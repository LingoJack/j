use crate::command::chat::permission::JcliConfig;
use crate::command::chat::storage::{ChatMessage, ModelProvider};
use crate::command::chat::teammate::{TeammateManager, TeammateStatus};
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::agent_shared::{
    call_llm_non_stream, create_runtime_and_client, execute_tool_with_permission,
    extract_tool_items,
};
use crate::util::log::write_info_log;
use async_openai::types::chat::ChatCompletionTools;
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
    } = config;

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

        // Drain 来自广播的消息
        let had_new_messages = drain_broadcast_messages(&mut messages, &pending_user_messages);

        // 如果之前是空闲状态但收到了新消息，重置空闲计数
        if had_new_messages {
            idle_rounds = 0;
        }

        // 同步 messages 快照（供 /dump 读取）
        sync_messages(&messages);

        write_info_log(
            "TeammateLoop",
            &format!(
                "{}: Round {}/{}, messages={}, new_broadcast={}",
                name,
                round + 1,
                max_rounds,
                messages.len(),
                had_new_messages,
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
                && let Ok(mut shared) = manager.shared_messages.lock()
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

            let has_pending = pending_user_messages
                .lock()
                .map(|p| !p.is_empty())
                .unwrap_or(false);

            if has_pending {
                idle_rounds = 0;
                continue;
            }

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
                std::thread::sleep(std::time::Duration::from_millis(100));

                let new_pending = pending_user_messages
                    .lock()
                    .map(|p| !p.is_empty())
                    .unwrap_or(false);
                if new_pending {
                    idle_rounds = 0;
                    break;
                }
            }
            continue;
        }

        // 处理工具调用
        let tool_items = extract_tool_items(choice.message.tool_calls.as_ref().unwrap());
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

        // 在 TUI 中显示 teammate 的工具调用（SendMessage 不显示，因为 broadcast 会单独显示消息内容）
        if let Ok(manager) = teammate_manager.lock()
            && let Ok(mut shared) = manager.shared_messages.lock()
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
    if let Ok(manager) = teammate_manager.lock()
        && let Ok(mut shared) = manager.shared_messages.lock()
    {
        shared.push(ChatMessage::text(
            "assistant",
            format!("<{}> [已完成工作]", name),
        ));
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
        - 用 `@AgentName` 指定消息接收者（消息仍广播给所有人）\n\
        - 完成任务后，用 SendMessage 通知 @Main\n\n\
        ## Rules\n\
        - 专注于你的角色职责，不要越界做其他角色的工作\n\
        - 如果需要其他 agent 的配合，通过 SendMessage 沟通\n\
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
