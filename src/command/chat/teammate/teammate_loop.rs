use crate::command::chat::permission::JcliConfig;
use crate::command::chat::storage::{
    ChatMessage, ModelProvider, SessionEvent, SessionPaths, append_event_to_path, sanitize_filename,
};
use crate::command::chat::teammate::{TeammateManager, TeammateStatus};
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::agent_shared::{
    call_llm_non_stream, create_runtime_and_client, execute_tool_with_permission,
    extract_tool_items,
};
use crate::util::log::write_info_log;
use async_openai::types::chat::ChatCompletionTools;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio_util::sync::CancellationToken;

/// Teammate agent loop 的配置
pub struct TeammateLoopConfig {
    pub name: String,
    pub role: String,
    pub initial_prompt: String,
    pub provider: ModelProvider,
    pub base_system_prompt: Option<String>,
    /// 共享的当前 session id 槽（session 切换时会被主线程更新）
    pub session_id: Arc<Mutex<String>>,
    pub tools: Vec<ChatCompletionTools>,
    pub registry: Arc<ToolRegistry>,
    pub jcli_config: Arc<JcliConfig>,
    pub teammate_manager: Arc<Mutex<TeammateManager>>,
    pub pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,
    pub cancel_token: CancellationToken,
    /// 供 /dump 读取的 system prompt 快照
    pub system_prompt_snapshot: Arc<Mutex<String>>,
    /// 供 /dump 读取的 messages 快照
    pub messages_snapshot: Arc<Mutex<Vec<ChatMessage>>>,
    /// 细粒度运行状态（与 TeammateHandle 共享）
    pub status: Arc<Mutex<TeammateStatus>>,
    /// 累计工具调用次数（与 TeammateHandle 共享）
    pub tool_calls_count: Arc<AtomicUsize>,
    /// 当前正在执行的工具名（与 TeammateHandle 共享）
    pub current_tool: Arc<Mutex<Option<String>>>,
    /// 唤醒标志（与 TeammateHandle 共享）：@self 或 from==Main 的广播会 set 它
    /// WorkDone 后仅此标志能触发重新激活（清除 work_done），未 WorkDone 时任何消息都唤醒
    pub wake_flag: Arc<AtomicBool>,
    /// WorkDone 终态标志（与 TeammateHandle 共享）：WorkDone 工具调用后 set，loop 看到后退出
    pub work_done: Arc<AtomicBool>,
}

/// Teammate 专用的 agent loop
///
/// 与 headless agent loop 的关键区别：
/// 1. 无 TUI 交互式确认（通过 permission 规则自动决定）
/// 2. 每轮开始检查 pending_user_messages（来自广播）
/// 3. 使用 SendMessage 工具与其他 agent 通信
/// 4. idle polling — 无工具调用时不立即退出，而是轮询等待新消息
/// 5. loop 结束后通知团队
pub fn run_teammate_loop(config: TeammateLoopConfig) -> String {
    let TeammateLoopConfig {
        name,
        role,
        initial_prompt,
        provider,
        base_system_prompt,
        session_id,
        tools,
        registry,
        jcli_config,
        teammate_manager,
        pending_user_messages,
        cancel_token,
        system_prompt_snapshot,
        messages_snapshot,
        status,
        tool_calls_count,
        current_tool,
        wake_flag,
        work_done,
    } = config;

    // 定位当前 teammate 的 transcript JSONL 路径（按 session_id 实时解析，切换 session 也能落到正确位置）
    let transcript_path = |name: &str| -> PathBuf {
        let sid = session_id
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| "unknown".to_string());
        SessionPaths::new(&sid).teammate_transcript(&sanitize_filename(name))
    };

    let append_messages = |msgs: &[ChatMessage]| {
        let path = transcript_path(&name);
        for m in msgs {
            let _ = append_event_to_path(&path, &SessionEvent::msg(m.clone()));
        }
    };

    // 辅助闭包：更新状态
    let set_status = |new_status: TeammateStatus| {
        if let Ok(mut s) = status.lock() {
            *s = new_status;
        }
    };

    set_status(TeammateStatus::Initializing);

    let max_rounds = 200; // 足够大，实际由 cancel_token 控制生命周期
    let max_consecutive_idle = 120; // 连续空闲 120 次（约 2 分钟）后退出

    let (rt, client) = match create_runtime_and_client(&provider) {
        Ok(pair) => pair,
        Err(e) => return e,
    };

    // 构建 teammate 专用 system prompt
    let system_prompt = build_teammate_system_prompt(
        &name,
        &role,
        base_system_prompt.as_deref(),
        &teammate_manager,
    );

    // 写入 system prompt 快照（供 /dump 读取）
    if let Ok(mut sp) = system_prompt_snapshot.lock() {
        *sp = system_prompt.clone();
    }

    let mut messages: Vec<ChatMessage> = vec![ChatMessage {
        role: "user".to_string(),
        content: initial_prompt,
        tool_calls: None,
        tool_call_id: None,
        images: None,
    }];
    // 初始 prompt 也要写入 transcript，便于恢复时重现对话
    append_messages(&messages);

    // 辅助闭包：将当前 messages clone 到共享快照
    let sync_messages = |msgs: &Vec<ChatMessage>| {
        if let Ok(mut snap) = messages_snapshot.lock() {
            *snap = msgs.clone();
        }
    };
    sync_messages(&messages);

    let mut final_text = String::new();
    let mut idle_rounds = 0;

    // 创建 AtomicBool 作为取消信号（与 CancellationToken 桥接）
    let cancelled = Arc::new(AtomicBool::new(false));

    for round in 0..max_rounds {
        // 检查取消
        if cancel_token.is_cancelled() || cancelled.load(Ordering::Relaxed) {
            set_status(TeammateStatus::Cancelled);
            return format!("{}\n[Teammate '{}' cancelled]", final_text, name);
        }

        // WorkDone 终态检查：teammate 明确声明完成工作后立即退出
        if work_done.load(Ordering::Relaxed) {
            write_info_log(
                "TeammateLoop",
                &format!("{}: WorkDone flag set, exiting", name),
            );
            break;
        }

        // Drain 来自广播的消息（包括旁听消息，保留上下文）
        // 注意：idle_rounds 的管理下放到 WaitingForMessage 分支，
        // 本处不再根据 had_new_messages 重置，避免"任何消息都触发 LLM"。
        let prev_len = messages.len();
        let _ = drain_broadcast_messages(&mut messages, &pending_user_messages);
        if messages.len() > prev_len {
            append_messages(&messages[prev_len..]);
        }

        // 同步 messages 快照（供 /dump 读取）
        sync_messages(&messages);

        write_info_log(
            "TeammateLoop",
            &format!(
                "{}: Round {}/{}, messages={}",
                name,
                round + 1,
                max_rounds,
                messages.len(),
            ),
        );

        // 更新状态为 Working（即将调用 LLM）
        set_status(TeammateStatus::Working);

        let choice = match call_llm_non_stream(
            &rt,
            &client,
            &provider,
            &messages,
            &tools,
            Some(&system_prompt),
        ) {
            Ok(c) => c,
            Err(e) => {
                set_status(TeammateStatus::Error(e.clone()));
                return format!("{}\n{}", final_text, e);
            }
        };

        let assistant_text = choice.message.content.clone().unwrap_or_default();
        if !assistant_text.is_empty() {
            final_text = assistant_text.clone();
            // 将 teammate 的文字回复通过广播显示在聊天室
            if let Ok(manager) = teammate_manager.lock()
                && let Ok(mut shared) = manager.ui_messages.lock()
            {
                shared.push(ChatMessage::text(
                    "assistant",
                    format!("<{}> {}", name, &assistant_text),
                ));
            }
        }

        // 检查是否有工具调用
        let is_tool_calls = matches!(
            choice.finish_reason,
            Some(async_openai::types::chat::FinishReason::ToolCalls)
        );

        if !is_tool_calls || choice.message.tool_calls.is_none() {
            // 无工具调用 — 进入轮询等待模式
            set_status(TeammateStatus::WaitingForMessage);

            // 文字回复也写入 messages + jsonl
            // 否则独立 jsonl 缺少这部分，resume 时 existing_count > synthesized.len() 导致 delta 补齐失效
            if !assistant_text.is_empty() {
                messages.push(ChatMessage::text("assistant", assistant_text.clone()));
                if let Some(last) = messages.last() {
                    append_messages(std::slice::from_ref(last));
                }
            }

            // 先把已到达的旁听消息 drain 到 messages（保留上下文，但不自动唤醒）
            let prev_len = messages.len();
            let _ = drain_broadcast_messages(&mut messages, &pending_user_messages);
            if messages.len() > prev_len {
                append_messages(&messages[prev_len..]);
            }

            // 唤醒判断：有 pending 消息就唤醒（除非已 WorkDone 且未被 @）
            // work_done=true 时，只有 @self 才能重新激活（清除 work_done 继续工作）
            let has_new = messages.len() > prev_len;
            if has_new {
                if work_done.load(Ordering::Relaxed) {
                    // WorkDone 后只有 @self 才能重新激活
                    if wake_flag.swap(false, Ordering::Relaxed) {
                        work_done.store(false, Ordering::Relaxed);
                        write_info_log(
                            "TeammateLoop",
                            &format!("{}: re-activated after WorkDone by @mention", name),
                        );
                        idle_rounds = 0;
                        continue;
                    }
                    // WorkDone 且未被 @，忽略消息
                } else {
                    // 未 WorkDone，任何消息都唤醒
                    let _ = wake_flag.swap(false, Ordering::Relaxed); // 清理 wake_flag
                    idle_rounds = 0;
                    continue;
                }
            }
            let _ = wake_flag.swap(false, Ordering::Relaxed); // 清理残留 wake_flag

            idle_rounds += 1;
            if idle_rounds >= max_consecutive_idle {
                write_info_log(
                    "TeammateLoop",
                    &format!("{}: idle for {} rounds (~2min), exiting", name, idle_rounds),
                );
                break;
            }

            // 等待 1 秒后再检查（可被 cancel_token 中断）
            for _ in 0..10 {
                if cancel_token.is_cancelled() {
                    set_status(TeammateStatus::Cancelled);
                    return format!("{}\n[Teammate '{}' cancelled]", final_text, name);
                }
                if work_done.load(Ordering::Relaxed) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));

                // 在轮询期间也 drain 消息到 messages
                let prev_len = messages.len();
                let _ = drain_broadcast_messages(&mut messages, &pending_user_messages);
                if messages.len() > prev_len {
                    append_messages(&messages[prev_len..]);

                    // 有新消息：未 WorkDone 就唤醒，WorkDone 后只有 @self 才重新激活
                    if work_done.load(Ordering::Relaxed) {
                        if wake_flag.swap(false, Ordering::Relaxed) {
                            work_done.store(false, Ordering::Relaxed);
                            write_info_log(
                                "TeammateLoop",
                                &format!("{}: re-activated after WorkDone by @mention", name),
                            );
                            idle_rounds = 0;
                            break;
                        }
                    } else {
                        let _ = wake_flag.swap(false, Ordering::Relaxed);
                        idle_rounds = 0;
                        break;
                    }
                }
                let _ = wake_flag.swap(false, Ordering::Relaxed);
            }
            continue;
        }

        // 上面已检查 tool_calls.is_none() 会 continue，此处用 let else 确保安全
        let Some(tool_calls) = choice.message.tool_calls.as_ref() else {
            continue;
        };
        let tool_items = extract_tool_items(tool_calls);
        if tool_items.is_empty() {
            break;
        }

        // 重置空闲计数（有工具调用说明正在工作）
        idle_rounds = 0;

        messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: assistant_text,
            tool_calls: Some(tool_items.clone()),
            tool_call_id: None,
            images: None,
        });
        if let Some(last) = messages.last() {
            append_messages(std::slice::from_ref(last));
        }

        // 在 TUI 中显示 teammate 的工具调用（SendMessage 不显示，因为 broadcast 会单独显示消息内容）
        if let Ok(manager) = teammate_manager.lock()
            && let Ok(mut shared) = manager.ui_messages.lock()
        {
            for item in &tool_items {
                if item.name != "SendMessage" {
                    shared.push(ChatMessage::text(
                        "assistant",
                        format!("<{}> [调用工具 {}]", name, item.name),
                    ));
                }
            }
        }

        // 执行工具
        for item in &tool_items {
            if cancel_token.is_cancelled() {
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: "[Cancelled]".to_string(),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                });
                if let Some(last) = messages.last() {
                    append_messages(std::slice::from_ref(last));
                }
                continue;
            }

            // 更新当前工具名
            if let Ok(mut ct) = current_tool.lock() {
                *ct = Some(item.name.clone());
            }
            tool_calls_count.fetch_add(1, Ordering::Relaxed);

            let result_msg = execute_tool_with_permission(
                item,
                &registry,
                &jcli_config,
                &cancelled,
                "TeammateLoop",
                false,
            );
            messages.push(result_msg);
            if let Some(last) = messages.last() {
                append_messages(std::slice::from_ref(last));
            }

            // 清除当前工具名
            if let Ok(mut ct) = current_tool.lock() {
                *ct = None;
            }
        }

        // 本轮工具结果写入后同步快照
        sync_messages(&messages);
    }

    // 通知团队：teammate 已完成
    set_status(TeammateStatus::Completed);
    // WorkDone 工具自己已广播过 [已完成工作]，避免重复；其他路径（idle 超时等）补一次
    if !work_done.load(Ordering::Relaxed)
        && let Ok(manager) = teammate_manager.lock()
        && let Ok(mut shared) = manager.ui_messages.lock()
    {
        shared.push(ChatMessage::text(
            "assistant",
            format!("<{}> [已完成工作]", name),
        ));
        // 同步写入独立 jsonl（不带 <Name> 前缀，合成时会加前缀）
        let done_msg = ChatMessage::text("assistant", "[已完成工作]".to_string());
        append_messages(std::slice::from_ref(&done_msg));
    }

    if final_text.is_empty() {
        format!("[Teammate '{}' completed with no output]", name)
    } else {
        final_text
    }
}

/// 构建 teammate 专用的 system prompt
fn build_teammate_system_prompt(
    name: &str,
    role: &str,
    base_prompt: Option<&str>,
    teammate_manager: &Arc<Mutex<TeammateManager>>,
) -> String {
    let team_summary = teammate_manager
        .lock()
        .map(|m| m.team_summary())
        .unwrap_or_default();

    let base = base_prompt.unwrap_or("You are a helpful assistant.");

    format!(
        "{}\n\n\
        ## Your Identity\n\
        你是团队中的 **{}**，角色: {}。\n\
        你的名字是 `{}`，在发送消息和被提及时使用这个名字。\n\n\
        {}\n\
        ## Communication\n\
        - 使用 `SendMessage` 工具与其他 agent 通信\n\
        - 收到的广播消息以 `<AgentName>` 前缀出现在对话中\n\
        - 用 `@AgentName` 指定消息接收者（消息仍广播给所有人，但只有 @目标 会被真正「唤醒」）\n\n\
        ## Message Wake Semantics（重要）
        聊天室里你会看到三类消息，处理方式不同：
        - **@你自己 的消息** 或 **来自 @Main 的消息**：会立即唤醒你去思考和回复
        - **你不是接收者的其他 agent 间广播**：也会唤醒你（保持上下文感知），但**不要**主动回复无关消息，否则会造成无限循环
        - 旁听消息只是让你了解团队动态；除非其中包含你必须处理的信息，否则简单确认后继续工作\n\n\
        ## Completing Your Work（重要）\n\
        - 任务做完后：先用 SendMessage 告知 @Main 结果摘要，然后调用 `WorkDone` 工具退出\n\
        - `WorkDone` 调用后你将进入完成状态，普通消息不再唤醒你\n\
        - **但如果有人 @你**，你会被重新激活（WorkDone 被撤销），可以继续工作\n\
        - 如果任务还可能需要你配合，**不要**调用 WorkDone，保持空闲等待即可\n\n\
        ## Rules\n\
        - 专注于你的角色职责，不要越界做其他角色的工作\n\
        - 如果需要其他 agent 的配合，通过 SendMessage @对方 沟通\n\
        - 如果遇到文件编辑冲突（被其他 agent 锁定），等待后重试\n",
        base, name, role, name, team_summary
    )
}

/// 从 pending_user_messages 中 drain 广播消息到 messages
/// 返回 true 表示有新消息
fn drain_broadcast_messages(
    messages: &mut Vec<ChatMessage>,
    pending: &Arc<Mutex<Vec<ChatMessage>>>,
) -> bool {
    if let Ok(mut pending_msgs) = pending.lock() {
        if pending_msgs.is_empty() {
            return false;
        }
        messages.append(&mut *pending_msgs);
        true
    } else {
        false
    }
}
