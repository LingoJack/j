use crate::command::chat::infra::hook::{HookContext, HookEvent, HookManager};
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::storage::{
    ChatMessage, MessageRole, ModelProvider, SessionEvent, SessionPaths, append_event_to_path,
    sanitize_filename,
};
use crate::command::chat::teammate::{TeammateManager, TeammateStatus};
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::derived_shared::{
    AgentContextConfig, call_llm_non_stream, create_runtime_and_client,
    execute_tool_with_permission, extract_tool_items,
};
use crate::llm::ToolDefinition;
use crate::util::log::write_info_log;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio_util::sync::CancellationToken;

/// Teammate loop 最大轮数（足够大，实际由 cancel_token 控制生命周期）
const MAX_TEAMMATE_ROUNDS: u32 = 200;
/// 连续空闲轮询上限（约 2 分钟后退出）
const MAX_CONSECUTIVE_IDLE_POLLS: u32 = 120;
/// 轮询等待期间的内层循环次数（每次休眠 POLL_SLEEP_MILLIS）
const POLL_CHECK_INTERVAL: u32 = 10;
/// 轮询等待期间每次休眠的毫秒数
const POLL_SLEEP_MILLIS: u64 = 100;

/// Teammate agent loop 的配置
pub struct TeammateLoopConfig {
    pub name: String,
    pub role: String,
    pub initial_prompt: String,
    pub provider: ModelProvider,
    pub base_system_prompt: Option<String>,
    /// 共享的当前 session id 槽（session 切换时会被主线程更新）
    pub session_id: Arc<Mutex<String>>,
    pub tools: Vec<ToolDefinition>,
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
    /// 父 agent 共享的 HookManager（Teammate 调 LLM 前走 PreLlmRequest hook 链）
    pub hook_manager: Arc<Mutex<HookManager>>,
    /// 父 agent 的 disabled_hooks 快照（Teammate 走 hook 链时用）
    pub disabled_hooks: Arc<Mutex<Vec<String>>>,
    /// 父 agent 的上下文配置快照（供 select_messages + micro_compact 复用）
    pub context_config: Arc<Mutex<AgentContextConfig>>,
}

/// Teammate 专用的 agent loop
///
/// 与 sub_agent_loop 的关键区别：
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
        hook_manager,
        disabled_hooks,
        context_config,
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

    let mut messages: Vec<ChatMessage> = Vec::with_capacity(1 + initial_prompt.len());
    messages.push(ChatMessage {
        role: MessageRole::User,
        content: initial_prompt,
        tool_calls: None,
        tool_call_id: None,
        images: None,
        reasoning_content: None,
    });
    // 初始 prompt 也要写入 transcript，便于恢复时重现对话
    append_messages(&messages);

    // 辅助闭包：将当前 messages clone 到共享快照
    let sync_messages = |msgs: &Vec<ChatMessage>| {
        if let Ok(mut snap) = messages_snapshot.lock() {
            *snap = msgs.clone();
        }
    };
    sync_messages(&messages);

    let mut last_assistant_text = String::new();
    let mut consecutive_idle_polls = 0;

    // 创建 AtomicBool 作为取消信号（与 CancellationToken 桥接）
    let cancel_flag = Arc::new(AtomicBool::new(false));

    for round in 0..MAX_TEAMMATE_ROUNDS {
        // 检查取消
        if cancel_token.is_cancelled() || cancel_flag.load(Ordering::Relaxed) {
            set_status(TeammateStatus::Cancelled);
            return format!("{}\n[Teammate '{}' cancelled]", last_assistant_text, name);
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
        // 注意：consecutive_idle_polls 的管理下放到 WaitingForMessage 分支，
        // 本处不再根据 had_new_messages 重置，避免"任何消息都触发 LLM"。
        let len_before_drain = messages.len();
        let _ = drain_broadcast_messages(&mut messages, &pending_user_messages);
        if messages.len() > len_before_drain {
            append_messages(&messages[len_before_drain..]);
        }

        // 同步 messages 快照（供 /dump 读取）
        sync_messages(&messages);

        write_info_log(
            "TeammateLoop",
            &format!(
                "{}: Round {}/{}, messages={}",
                name,
                round + 1,
                MAX_TEAMMATE_ROUNDS,
                messages.len(),
            ),
        );

        // 更新状态为 Working（即将调用 LLM）
        set_status(TeammateStatus::Working);

        // 复用父 agent 的 context 配置，对齐 Main 管线：
        //   select_messages → micro_compact → PreLlmRequest hook 链 (含 broadcast_compress)
        let ctx_cfg = match context_config.lock() {
            Ok(g) => g.clone(),
            Err(e) => {
                set_status(TeammateStatus::Error(format!("context_config lock: {}", e)));
                return format!("{}\ncontext_config lock poisoned", last_assistant_text);
            }
        };
        let mut api_messages = crate::command::chat::context::window::select_messages(
            &messages,
            ctx_cfg.max_history_messages,
            ctx_cfg.max_context_tokens,
            ctx_cfg.compact.keep_recent,
            &ctx_cfg.compact.micro_compact_exempt_tools,
        );
        if ctx_cfg.compact.enabled {
            crate::command::chat::context::compact::micro_compact(
                &mut api_messages,
                ctx_cfg.compact.keep_recent,
                &ctx_cfg.compact.micro_compact_exempt_tools,
            );
        }

        // PreLlmRequest hook 链（内置 broadcast_compress 会按线程本地身份折叠其他 agent 广播）
        let mut effective_system_prompt = system_prompt.clone();
        {
            let hook_mgr = match hook_manager.lock() {
                Ok(g) => g,
                Err(e) => {
                    set_status(TeammateStatus::Error(format!("hook_manager lock: {}", e)));
                    return format!("{}\nhook_manager lock poisoned", last_assistant_text);
                }
            };
            if hook_mgr.has_hooks_for(HookEvent::PreLlmRequest) {
                let disabled_snapshot: Vec<String> =
                    disabled_hooks.lock().map(|g| g.clone()).unwrap_or_default();
                let ctx = HookContext {
                    event: HookEvent::PreLlmRequest,
                    messages: Some(api_messages.clone()),
                    system_prompt: Some(effective_system_prompt.clone()),
                    model: Some(provider.model.clone()),
                    session_id: session_id.lock().ok().map(|g| g.clone()),
                    cwd: std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| ".".to_string()),
                    ..Default::default()
                };
                if let Some(result) =
                    hook_mgr.execute(HookEvent::PreLlmRequest, ctx, &disabled_snapshot)
                {
                    if result.is_stop() {
                        set_status(TeammateStatus::Error("hook requested stop".to_string()));
                        return format!(
                            "{}\n[Teammate halted by PreLlmRequest hook]",
                            last_assistant_text
                        );
                    }
                    if let Some(new_msgs) = result.messages {
                        api_messages = new_msgs;
                    }
                    if let Some(new_prompt) = result.system_prompt {
                        effective_system_prompt = new_prompt;
                    }
                    if let Some(inject) = result.inject_messages {
                        api_messages.extend(inject);
                    }
                }
            }
        }

        let response_choice = match call_llm_non_stream(
            &rt,
            &client,
            &provider,
            &api_messages,
            &tools,
            Some(&effective_system_prompt),
            None, // teammate 暂不使用重试回调
        ) {
            Ok(c) => c,
            Err(e) => {
                set_status(TeammateStatus::Error(e.clone()));
                return format!("{}\n{}", last_assistant_text, e);
            }
        };

        let assistant_text = response_choice.message.content.clone().unwrap_or_default();
        let reasoning_content = response_choice.message.reasoning_content.clone();
        if !assistant_text.is_empty() {
            last_assistant_text = assistant_text.clone();
            // 将 teammate 的文字回复通过广播显示在聊天室
            // ★ 此消息通过双通道推送（display + context），会同步到 Main Agent 的 LLM 上下文（有意为之的设计）。
            if let Ok(manager) = teammate_manager.lock() {
                let msg = ChatMessage::text(
                    MessageRole::Assistant,
                    format!("<{}> {}", name, &assistant_text),
                );
                if let Ok(mut display) = manager.display_messages.lock() {
                    display.push(msg.clone());
                }
                if let Ok(mut context) = manager.context_messages.lock() {
                    context.push(msg);
                }
            }
        }

        // 检查是否有工具调用
        let has_tool_calls = response_choice.finish_reason.as_deref() == Some("tool_calls");

        if !has_tool_calls || response_choice.message.tool_calls.is_none() {
            // 无工具调用 — 进入轮询等待模式
            set_status(TeammateStatus::WaitingForMessage);

            // 文字回复也写入 messages + jsonl
            // 否则独立 jsonl 缺少这部分，resume 时 existing_count > synthesized.len() 导致 delta 补齐失效
            if !assistant_text.is_empty() {
                messages.push(ChatMessage::text(
                    MessageRole::Assistant,
                    assistant_text.clone(),
                ));
                if let Some(last) = messages.last() {
                    append_messages(std::slice::from_ref(last));
                }
            }

            // 先把已到达的旁听消息 drain 到 messages（保留上下文，但不自动唤醒）
            let len_before_drain = messages.len();
            let _ = drain_broadcast_messages(&mut messages, &pending_user_messages);
            if messages.len() > len_before_drain {
                append_messages(&messages[len_before_drain..]);
            }

            // 唤醒判断：有 pending 消息就唤醒（除非已 WorkDone 且未被 @）
            // work_done=true 时，只有 @self 才能重新激活（清除 work_done 继续工作）
            let has_new = messages.len() > len_before_drain;
            if has_new {
                if work_done.load(Ordering::Relaxed) {
                    // WorkDone 后只有 @self 才能重新激活
                    if wake_flag.swap(false, Ordering::Relaxed) {
                        work_done.store(false, Ordering::Relaxed);
                        write_info_log(
                            "TeammateLoop",
                            &format!("{}: re-activated after WorkDone by @mention", name),
                        );
                        consecutive_idle_polls = 0;
                        continue;
                    }
                    // WorkDone 且未被 @，忽略消息
                } else {
                    // 未 WorkDone，任何消息都唤醒
                    let _ = wake_flag.swap(false, Ordering::Relaxed); // 清理 wake_flag
                    consecutive_idle_polls = 0;
                    continue;
                }
            }
            let _ = wake_flag.swap(false, Ordering::Relaxed); // 清理残留 wake_flag

            consecutive_idle_polls += 1;
            if consecutive_idle_polls >= MAX_CONSECUTIVE_IDLE_POLLS {
                write_info_log(
                    "TeammateLoop",
                    &format!(
                        "{}: idle for {} rounds (~2min), exiting",
                        name, consecutive_idle_polls
                    ),
                );
                break;
            }

            // 等待 1 秒后再检查（可被 cancel_token 中断）
            for _ in 0..POLL_CHECK_INTERVAL {
                if cancel_token.is_cancelled() {
                    set_status(TeammateStatus::Cancelled);
                    return format!("{}\n[Teammate '{}' cancelled]", last_assistant_text, name);
                }
                if work_done.load(Ordering::Relaxed) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(POLL_SLEEP_MILLIS));

                // 在轮询期间也 drain 消息到 messages
                let len_before_drain = messages.len();
                let _ = drain_broadcast_messages(&mut messages, &pending_user_messages);
                if messages.len() > len_before_drain {
                    append_messages(&messages[len_before_drain..]);

                    // 有新消息：未 WorkDone 就唤醒，WorkDone 后只有 @self 才重新激活
                    if work_done.load(Ordering::Relaxed) {
                        if wake_flag.swap(false, Ordering::Relaxed) {
                            work_done.store(false, Ordering::Relaxed);
                            write_info_log(
                                "TeammateLoop",
                                &format!("{}: re-activated after WorkDone by @mention", name),
                            );
                            consecutive_idle_polls = 0;
                            break;
                        }
                    } else {
                        let _ = wake_flag.swap(false, Ordering::Relaxed);
                        consecutive_idle_polls = 0;
                        break;
                    }
                }
                let _ = wake_flag.swap(false, Ordering::Relaxed);
            }
            continue;
        }

        // 上面已检查 tool_calls.is_none() 会 continue，此处用 let else 确保安全
        let Some(tool_calls) = response_choice.message.tool_calls.as_ref() else {
            continue;
        };
        let tool_items = extract_tool_items(tool_calls);
        if tool_items.is_empty() {
            break;
        }

        // 重置空闲计数（有工具调用说明正在工作）
        consecutive_idle_polls = 0;

        messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: assistant_text,
            tool_calls: Some(tool_items.clone()),
            tool_call_id: None,
            images: None,
            reasoning_content,
        });
        if let Some(last) = messages.last() {
            append_messages(std::slice::from_ref(last));
        }

        // 在 TUI 中显示 teammate 的工具调用（SendMessage 不显示，因为 broadcast 会单独显示消息内容）
        // ★ 此消息通过双通道推送（display + context），会同步到 Main Agent 的 LLM 上下文（有意为之的设计）。
        if let Ok(manager) = teammate_manager.lock() {
            for item in &tool_items {
                if item.name != "SendMessage" {
                    let msg = ChatMessage::text(
                        MessageRole::Assistant,
                        format!("<{}> [调用工具 {}]", name, item.name),
                    );
                    if let Ok(mut display) = manager.display_messages.lock() {
                        display.push(msg.clone());
                    }
                    if let Ok(mut context) = manager.context_messages.lock() {
                        context.push(msg);
                    }
                }
            }
        }

        // 执行工具
        for item in &tool_items {
            if cancel_token.is_cancelled() {
                messages.push(ChatMessage {
                    role: MessageRole::Tool,
                    content: "[Cancelled]".to_string(),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                    reasoning_content: None,
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
                &cancel_flag,
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
    // ★ 此消息通过双通道推送（display + context），会同步到 Main Agent 的 LLM 上下文（有意为之的设计）。
    if !work_done.load(Ordering::Relaxed)
        && let Ok(manager) = teammate_manager.lock()
    {
        let msg = ChatMessage::text(MessageRole::Assistant, format!("<{}> [已完成工作]", name));
        if let Ok(mut display) = manager.display_messages.lock() {
            display.push(msg.clone());
        }
        if let Ok(mut context) = manager.context_messages.lock() {
            context.push(msg);
        }
        // 同步写入独立 jsonl（不带 <Name> 前缀，合成时会加前缀）
        let done_msg = ChatMessage::text(MessageRole::Assistant, "[已完成工作]".to_string());
        append_messages(std::slice::from_ref(&done_msg));
    }

    if last_assistant_text.is_empty() {
        format!("[Teammate '{}' completed with no output]", name)
    } else {
        last_assistant_text
    }
}

/// 构建 teammate 专用的 system prompt
///
/// 从嵌入的模板文件加载并替换占位符：
/// - `{{.base_prompt}}` — 主 agent 的 base system prompt
/// - `{{.name}}` — teammate 名字
/// - `{{.role}}` — teammate 角色
/// - `{{.team_summary}}` — 团队成员列表摘要
fn build_teammate_system_prompt(
    name: &str,
    role: &str,
    base_prompt: Option<&str>,
    teammate_manager: &Arc<Mutex<TeammateManager>>,
) -> String {
    let template = crate::assets::teammate_system_prompt_template();
    let base = base_prompt.unwrap_or("You are a helpful assistant.");
    let team_summary = teammate_manager
        .lock()
        .map(|m| m.team_summary())
        .unwrap_or_default();

    template
        .as_ref()
        .replace("{{.base_prompt}}", base)
        .replace("{{.name}}", name)
        .replace("{{.role}}", role)
        .replace("{{.team_summary}}", &team_summary)
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
