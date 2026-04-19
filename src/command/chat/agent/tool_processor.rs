use super::super::app::types::{PlanDecision, StreamMsg, ToolResultMsg};
use super::super::error::ChatError;
use super::super::hook::{HookContext, HookEvent, HookManager};
use super::super::storage::{ChatMessage, ImageData, ToolCallItem};
use super::compact;
use crate::command::chat::constants::{ROLE_ASSISTANT, ROLE_TOOL, ROLE_USER};
use crate::command::chat::tools::Tool;
use crate::command::chat::tools::compact::CompactTool;
use crate::util::log::write_info_log;
use crate::util::safe_lock;
use std::sync::{Arc, Mutex, mpsc};

/// process_tool_calls 所需的通道和共享状态
pub(super) struct ToolCallContext<'a> {
    pub(super) stream_msg_sender: &'a mpsc::Sender<StreamMsg>,
    pub(super) tool_result_receiver: &'a mpsc::Receiver<ToolResultMsg>,
    pub(super) pending_user_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    pub(super) hook_manager: &'a HookManager,
    pub(super) supports_vision: bool,
    pub(super) ui_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    pub(super) streaming_content: &'a Arc<Mutex<String>>,
    #[allow(dead_code)]
    pub(super) invoked_skills: &'a compact::InvokedSkillsMap,
    pub(super) session_id: &'a str,
}

/// process_tool_calls 的返回结果
pub(super) struct ToolCallResult {
    pub(super) compact_requested: bool,
    /// Plan 被批准且用户选择清空上下文，值为 plan 文件内容
    pub(super) plan_with_context_clear: Option<String>,
}

/// 从待处理队列中 drain 用户在 agent loop 期间发送的新消息，追加到 messages
pub(super) fn drain_pending_user_messages(
    messages: &mut Vec<ChatMessage>,
    pending_user_messages: &Arc<Mutex<Vec<ChatMessage>>>,
) {
    let mut pending = safe_lock(pending_user_messages, "agent::drain_pending");
    if !pending.is_empty() {
        // 给每条追加的用户消息添加 [User appended] 标记
        for msg in pending.iter_mut() {
            if msg.role == "user" {
                msg.content = format!("[User appended] {}", msg.content);
            }
        }
        messages.append(&mut *pending);
    }
}

/// 向共享消息列表中追加一条消息（agent 线程写入，UI 线程读取）
pub(super) fn push_ui(shared: &Arc<Mutex<Vec<ChatMessage>>>, msg: ChatMessage) {
    if let Ok(mut msgs) = shared.lock() {
        msgs.push(msg);
    }
}

/// auto_compact 后清空 ui_messages（旧消息已过时，由后续 push_ui 重建）
pub(super) fn clear_ui_messages(shared: &Arc<Mutex<Vec<ChatMessage>>>) {
    if let Ok(mut msgs) = shared.lock() {
        msgs.clear();
    }
}

/// 全量同步 ui_messages（仅用于 PostAutoCompact hook 修改了 messages 的罕见场景）
pub(super) fn sync_ui_full(shared: &Arc<Mutex<Vec<ChatMessage>>>, new_messages: &[ChatMessage]) {
    if let Ok(mut msgs) = shared.lock() {
        msgs.clear();
        msgs.extend_from_slice(new_messages);
    }
}

/// 将 streaming_content 中的文本保存为 assistant 消息（多轮 agent loop 中间轮的文本回复）
/// 调用后 streaming_content 被清空，避免 UI 侧 finish_loading 再次保存导致重复
pub(super) fn flush_streaming_as_message(
    streaming_content: &Arc<Mutex<String>>,
    messages: &mut Vec<ChatMessage>,
    ui_messages: &Arc<Mutex<Vec<ChatMessage>>>,
) {
    let mut stream_buf = safe_lock(streaming_content, "agent::flush_streaming");
    if !stream_buf.is_empty() {
        let text_msg = ChatMessage {
            role: ROLE_ASSISTANT.to_string(),
            content: std::mem::take(&mut *stream_buf),
            tool_calls: None,
            tool_call_id: None,
            images: None,
        };
        messages.push(text_msg.clone());
        push_ui(ui_messages, text_msg);
    }
}

/// 记录工具调用请求日志
fn log_tool_request(tool_items: &[ToolCallItem]) {
    let mut log_content = String::new();
    for item in tool_items {
        log_content.push_str(&format!("- {}: {}\n", item.name, item.arguments));
    }
    write_info_log("工具调用请求", &log_content);
}

/// 记录工具调用结果日志
fn log_tool_results(tool_items: &[ToolCallItem], tool_results: &[ToolResultMsg]) {
    let mut log_content = String::new();
    for (i, result) in tool_results.iter().enumerate() {
        let (tool_name, tool_args) = tool_items
            .get(i)
            .map(|t| (t.name.as_str(), t.arguments.as_str()))
            .unwrap_or(("unknown", ""));
        log_content.push_str(&format!(
            "- [{}] {}({}): {}\n",
            result.tool_call_id, tool_name, tool_args, result.result
        ));
    }
    write_info_log("工具调用结果", &log_content);
}

/// 处理工具调用的公共逻辑：发送请求、等待结果、更新 messages
/// 返回 Ok(ToolCallResult) 表示成功（应 continue 循环）
/// Err(ChatError) 表示 channel 断开或执行失败
pub(super) fn process_tool_calls(
    tool_items: Vec<ToolCallItem>,
    assistant_text: String,
    messages: &mut Vec<ChatMessage>,
    ctx: &ToolCallContext<'_>,
) -> Result<ToolCallResult, ChatError> {
    log_tool_request(&tool_items);

    if !assistant_text.is_empty() {
        write_info_log("Sprite 回复", &assistant_text);
    }

    // 检查是否有 compact tool 被调用
    let compact_requested = tool_items.iter().any(|t| t.name == CompactTool {}.name());

    // ★ 如果 LLM 同时返回了文本和 tool_calls，拆成两条消息：
    //   1. 纯文本 assistant 消息（让 UI 先渲染文字）
    //   2. tool_call assistant 消息（content 为空，只带 tool_calls）
    //   这样渲染时文字在上面，tool_call 在下面
    if !assistant_text.is_empty() {
        let text_msg = ChatMessage {
            role: ROLE_ASSISTANT.to_string(),
            content: assistant_text,
            tool_calls: None,
            tool_call_id: None,
            images: None,
        };
        messages.push(text_msg.clone());
        push_ui(ctx.ui_messages, text_msg);
        // 清空 streaming_content，文本已保存，避免 UI 继续显示流式内容
        if let Ok(mut stream_buf) = ctx.streaming_content.lock() {
            stream_buf.clear();
        }
    }

    let tool_call_msg = ChatMessage {
        role: ROLE_ASSISTANT.to_string(),
        content: String::new(),
        tool_calls: Some(tool_items.clone()),
        tool_call_id: None,
        images: None,
    };
    messages.push(tool_call_msg.clone());
    push_ui(ctx.ui_messages, tool_call_msg);

    if ctx
        .stream_msg_sender
        .send(StreamMsg::ToolCallRequest(tool_items.clone()))
        .is_err()
    {
        return Err(ChatError::Other("工具调用通道已断开".to_string()));
    }

    let mut tool_results: Vec<ToolResultMsg> = Vec::new();
    let mut plan_clear_context: Option<String> = None;
    for _ in &tool_items {
        match ctx.tool_result_receiver.recv() {
            Ok(result) => {
                // 检测 ExitPlanMode 返回清空上下文信号
                if result.plan_decision == PlanDecision::ApproveAndClearContext {
                    plan_clear_context = Some(result.result.clone());
                }
                tool_results.push(result);
            }
            Err(_) => return Err(ChatError::Other("工具执行结果通道已断开".to_string())),
        }
    }

    log_tool_results(&tool_items, &tool_results);

    // 收集需要延迟注入的图片消息（在所有 tool results 之后统一注入，
    // 避免在 tool results 中间插入 user 消息导致 API 报错）
    let mut deferred_image_msgs: Vec<ChatMessage> = Vec::new();

    for result in tool_results {
        let mut result_content = result.result;
        let result_images = result.images;

        // 查找工具名
        let tool_name = tool_items
            .iter()
            .find(|t| t.id == result.tool_call_id)
            .map(|t| t.name.clone());

        // ★ PostToolExecution hook
        if ctx.hook_manager.has_hooks_for(HookEvent::PostToolExecution) {
            let hook_ctx = HookContext {
                event: HookEvent::PostToolExecution,
                tool_name: tool_name.clone(),
                tool_result: Some(result_content.clone()),
                session_id: Some(ctx.session_id.to_string()),
                cwd: std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
                ..Default::default()
            };
            if let Some(hook_result) = ctx
                .hook_manager
                .execute(HookEvent::PostToolExecution, hook_ctx)
                && let Some(new_result) = hook_result.tool_result
            {
                result_content = new_result;
            }
        }

        let tool_msg = ChatMessage {
            role: ROLE_TOOL.to_string(),
            content: result_content,
            tool_calls: None,
            tool_call_id: Some(result.tool_call_id.clone()),
            images: None,
        };
        messages.push(tool_msg.clone());
        push_ui(ctx.ui_messages, tool_msg);

        // 如果模型支持视觉且工具返回了图片，先收集，稍后统一注入
        if !result_images.is_empty() {
            let tool_label = tool_name.as_deref().unwrap_or("unknown");
            let img_count = result_images.len();
            write_info_log(
                "ImageInjection",
                &format!(
                    "工具 {} 返回了 {} 张图片, supports_vision={}",
                    tool_label, img_count, ctx.supports_vision
                ),
            );
            if ctx.supports_vision {
                let img_msg = ChatMessage {
                    role: ROLE_USER.to_string(),
                    content: format!(
                        "[{tool_label} 返回了 {img_count} 张图片，请查看图片内容并继续帮助完成任务]"
                    ),
                    tool_calls: None,
                    tool_call_id: None,
                    images: Some(
                        result_images
                            .into_iter()
                            .map(|img| ImageData {
                                base64: img.base64,
                                media_type: img.media_type,
                            })
                            .collect(),
                    ),
                };
                deferred_image_msgs.push(img_msg);
            } else {
                write_info_log(
                    "ImageInjection",
                    &format!(
                        "supports_vision=false，丢弃 {} 返回的 {} 张图片",
                        tool_label, img_count
                    ),
                );
            }
        }
    }

    // ★ 所有 tool results 处理完毕后，统一注入图片 user messages
    if !deferred_image_msgs.is_empty() {
        write_info_log(
            "ImageInjection",
            &format!(
                "在所有 tool results 之后注入 {} 条图片消息",
                deferred_image_msgs.len()
            ),
        );
        for img_msg in deferred_image_msgs {
            // 只加入 LLM 上下文，不推送到 ui_messages（避免 UI 渲染这条内部消息）
            messages.push(img_msg);
        }
    }

    drain_pending_user_messages(messages, ctx.pending_user_messages);

    Ok(ToolCallResult {
        compact_requested,
        plan_with_context_clear: plan_clear_context,
    })
}
