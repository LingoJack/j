use crate::command::chat::api::{build_request_with_tools, create_openai_client};
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::storage::{ChatMessage, ModelProvider, ToolCallItem};
use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::ToolRegistry;
use crate::util::log::write_info_log;
use async_openai::types::chat::ChatCompletionTools;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
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
}

/// Teammate 专用的 agent loop
///
/// 与主 agent loop 的关键区别：
/// 1. 无 TUI 交互式确认（通过 permission 规则自动决定）
/// 2. 每轮开始检查 pending_user_messages（来自广播）
/// 3. 使用 SendMessage 工具与其他 agent 通信
/// 4. loop 结束后标记 is_running = false
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
    } = config;

    let max_rounds = 200; // 足够大，实际由 cancel_token 控制生命周期
    let max_consecutive_idle = 120; // 连续空闲 120 次（约 2 分钟）后退出

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            return format!("Failed to create runtime: {}", e);
        }
    };

    let client = create_openai_client(&provider);

    // 构建 teammate 专用 system prompt
    let system_prompt = build_teammate_system_prompt(
        &name,
        &role,
        base_system_prompt.as_deref(),
        &teammate_manager,
    );

    let mut messages: Vec<ChatMessage> = vec![ChatMessage {
        role: "user".to_string(),
        content: initial_prompt,
        tool_calls: None,
        tool_call_id: None,
        images: None,
    }];

    let mut final_text = String::new();
    let mut idle_rounds = 0;

    // 创建 AtomicBool 作为取消信号（与 CancellationToken 桥接）
    let cancelled = Arc::new(AtomicBool::new(false));

    for round in 0..max_rounds {
        // 检查取消
        if cancel_token.is_cancelled() || cancelled.load(Ordering::Relaxed) {
            return format!("{}\n[Teammate '{}' cancelled]", final_text, name);
        }

        // Drain 来自广播的消息
        let had_new_messages =
            drain_broadcast_messages(&mut messages, &pending_user_messages, &name);

        // 如果之前是空闲状态但收到了新消息，重置空闲计数
        if had_new_messages {
            idle_rounds = 0;
        }

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

        let request = match build_request_with_tools(
            &provider,
            &messages,
            tools.clone(),
            Some(&system_prompt),
        ) {
            Ok(req) => req,
            Err(e) => {
                return format!("Failed to build request: {}", e);
            }
        };

        let response = rt.block_on(async {
            let chat_client = client.chat();
            chat_client.create(request).await
        });

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                return format!("API error: {}", e);
            }
        };

        let choice = match response.choices.first() {
            Some(c) => c,
            None => {
                return format!("{}\n[No response from API]", final_text);
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
            let has_pending = pending_user_messages
                .lock()
                .map(|p| !p.is_empty())
                .unwrap_or(false);

            if has_pending {
                // 有待处理消息，继续循环
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
                    return format!("{}\n[Teammate '{}' cancelled]", final_text, name);
                }
                std::thread::sleep(std::time::Duration::from_millis(100));

                // 检查是否有新消息到达
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
        let tool_calls = choice.message.tool_calls.as_ref().unwrap();
        let tool_items: Vec<ToolCallItem> = tool_calls
            .iter()
            .filter_map(|tc| {
                if let async_openai::types::chat::ChatCompletionMessageToolCalls::Function(f) = tc {
                    Some(ToolCallItem {
                        id: f.id.clone(),
                        name: f.function.name.clone(),
                        arguments: f.function.arguments.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

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

            // 权限检查
            if jcli_config.is_denied(&item.name, &item.arguments) {
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: format!("Tool '{}' denied by permission rules.", item.name),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                });
                continue;
            }

            let tool_ref = registry.get(&item.name);
            let requires_confirm = tool_ref.map(|t| t.requires_confirmation()).unwrap_or(false);

            if requires_confirm && !jcli_config.is_allowed(&item.name, &item.arguments) {
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: format!(
                        "Tool '{}' requires confirmation. Add a permission rule to allow it.",
                        item.name
                    ),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                });
                continue;
            }

            let result = registry.execute(&item.name, &item.arguments, &cancelled);

            messages.push(ChatMessage {
                role: "tool".to_string(),
                content: result.output,
                tool_calls: None,
                tool_call_id: Some(item.id.clone()),
                images: None,
            });
        }
    }

    // 通知团队：teammate 已完成
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
    _agent_name: &str,
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
