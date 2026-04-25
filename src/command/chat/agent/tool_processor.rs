use super::super::app::types::{PlanDecision, StreamMsg, ToolResultMsg};
use super::super::error::ChatError;
use super::super::hook::{HookContext, HookEvent, HookManager};
use crate::command::chat::storage::{
    ChatMessage, ImageData, MessageRole, SessionOp, SessionOpKind, ToolCallItem, append_session_op,
};
use crate::command::chat::tools::Tool;
use crate::command::chat::tools::compact_tool::CompactTool;
use crate::util::log::write_info_log;
use crate::util::safe_lock;
use std::collections::HashSet;
use std::env::current_dir;
use std::mem::take;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{SystemTime, UNIX_EPOCH};

/// process_tool_calls 所需的通道和共享状态
pub(super) struct ToolCallContext<'a> {
    pub(super) stream_msg_sender: &'a mpsc::Sender<StreamMsg>,
    pub(super) tool_result_receiver: &'a mpsc::Receiver<ToolResultMsg>,
    pub(super) pending_user_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    pub(super) hook_manager: &'a HookManager,
    pub(super) disabled_hooks: &'a [String],
    pub(super) supports_vision: bool,
    /// 仅 UI 显示通道（不作为 LLM context 数据源）
    pub(super) display_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    /// LLM context 同步通道（poll_stream_actions 从此增量同步到 session.messages）
    pub(super) context_messages: &'a Arc<Mutex<Vec<ChatMessage>>>,
    pub(super) streaming_content: &'a Arc<Mutex<String>>,
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
            if msg.role == MessageRole::User {
                msg.content = format!("[User appended] {}", msg.content);
            }
        }
        messages.append(&mut *pending);
    }
}

/// 向 display 和 context 双通道同时推送消息。
///
/// Main Agent 的对话消息（text reply、tool_call、tool result）走此函数，
/// 因为这些消息既要在 UI 显示，也要进入 Main Agent 的 LLM context。
///
/// **设计说明**：
/// - `display_messages`：仅 UI 显示，`poll_stream_actions` 不从此同步到 `session.messages`
/// - `context_messages`：LLM context 同步通道，`poll_stream_actions` 从此增量同步到 `session.messages`
///
/// SubAgent/Teammate 的消息由各自的推送逻辑决定走哪个通道（见 sub_agent.rs / teammate_loop.rs）。
pub(super) fn push_both(
    display: &Arc<Mutex<Vec<ChatMessage>>>,
    context: &Arc<Mutex<Vec<ChatMessage>>>,
    msg: ChatMessage,
) {
    if let Ok(mut msgs) = display.lock() {
        msgs.push(msg.clone());
    }
    if let Ok(mut msgs) = context.lock() {
        msgs.push(msg);
    }
}

/// auto_compact 后清空双通道（旧消息已过时，由后续 push 重建）
pub(super) fn clear_channels(
    display: &Arc<Mutex<Vec<ChatMessage>>>,
    context: &Arc<Mutex<Vec<ChatMessage>>>,
) {
    if let Ok(mut msgs) = display.lock() {
        msgs.clear();
    }
    if let Ok(mut msgs) = context.lock() {
        msgs.clear();
    }
}

/// 全量同步 context 通道（仅用于 PostAutoCompact hook 修改了 messages 的罕见场景）
pub(super) fn sync_context_full(
    display: &Arc<Mutex<Vec<ChatMessage>>>,
    context: &Arc<Mutex<Vec<ChatMessage>>>,
    new_messages: &[ChatMessage],
) {
    if let Ok(mut msgs) = context.lock() {
        msgs.clear();
        msgs.extend_from_slice(new_messages);
    }
    if let Ok(mut msgs) = display.lock() {
        msgs.clear();
        msgs.extend_from_slice(new_messages);
    }
}

/// 将 streaming_content 中的文本保存为 assistant 消息（多轮 agent loop 中间轮的文本回复）
/// 调用后 streaming_content 被清空，避免 UI 侧 finish_loading 再次保存导致重复
pub(super) fn flush_streaming_as_message(
    streaming_content: &Arc<Mutex<String>>,
    streaming_reasoning_content: &Arc<Mutex<String>>,
    messages: &mut Vec<ChatMessage>,
    display: &Arc<Mutex<Vec<ChatMessage>>>,
    context: &Arc<Mutex<Vec<ChatMessage>>>,
    reasoning_content: Option<String>,
) {
    let mut stream_buf = safe_lock(streaming_content, "agent::flush_streaming");
    if !stream_buf.is_empty() {
        let mut text_msg = ChatMessage::text(MessageRole::Assistant, take(&mut *stream_buf));
        text_msg.reasoning_content = reasoning_content;
        messages.push(text_msg.clone());
        push_both(display, context, text_msg);
    }
    // 清空 reasoning 缓冲区
    safe_lock(
        streaming_reasoning_content,
        "agent::flush_streaming_reasoning",
    )
    .clear();
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
/// Err(ChatError) 仅在 stream_msg_sender 通道断开时返回；
/// tool 执行侧的错误（result 通道断开/结果缺失）会被合成为 tool_result，不上抛。
pub(super) fn process_tool_calls(
    tool_items: Vec<ToolCallItem>,
    assistant_text: String,
    messages: &mut Vec<ChatMessage>,
    ctx: &ToolCallContext<'_>,
    reasoning_content: Option<String>,
) -> Result<ToolCallResult, ChatError> {
    log_tool_request(&tool_items);

    if !assistant_text.is_empty() {
        write_info_log("Sprite 回复", &assistant_text);
    }

    // 检查是否有 compact tool 被调用
    let compact_requested = tool_items.iter().any(|t| t.name == CompactTool {}.name());

    // ★ content + reasoning_content + tool_calls 合为一条 assistant message
    //   DeepSeek 等 API 要求 reasoning_content 与 tool_calls 在同一条消息中传回
    let tool_call_msg = ChatMessage {
        role: MessageRole::Assistant,
        content: assistant_text,
        tool_calls: Some(tool_items.clone()),
        tool_call_id: None,
        images: None,
        reasoning_content,
        sender_name: None,
    };
    messages.push(tool_call_msg.clone());
    push_both(ctx.display_messages, ctx.context_messages, tool_call_msg);
    // 清空 streaming_content，文本已保存
    if let Ok(mut stream_buf) = ctx.streaming_content.lock() {
        stream_buf.clear();
    }

    if ctx
        .stream_msg_sender
        .send(StreamMsg::ToolCallRequest(tool_items.clone()))
        .is_err()
    {
        return Err(ChatError::Other("工具调用通道已断开".to_string()));
    }

    let mut tool_results: Vec<ToolResultMsg> = Vec::with_capacity(tool_items.len());
    let mut plan_clear_context: Option<String> = None;
    let mut channel_broken = false;
    for _ in &tool_items {
        if channel_broken {
            break;
        }
        match ctx.tool_result_receiver.recv() {
            Ok(result) => {
                // 检测 ExitPlanMode 返回清空上下文信号
                if result.plan_decision == PlanDecision::ApproveAndClearContext {
                    plan_clear_context = Some(result.result.clone());
                }
                tool_results.push(result);
            }
            Err(_) => {
                channel_broken = true;
            }
        }
    }

    // ★ 配对兜底：凡 tool_items 中的 id 未收到对应 result，合成错误 tool_result。
    //   这样 tool_call 与 tool_result 在内存中永远成对，下游 persist 才能原子提交。
    let received_ids: HashSet<String> = tool_results
        .iter()
        .map(|r| r.tool_call_id.clone())
        .collect();
    for item in &tool_items {
        if !received_ids.contains(&item.id) {
            let reason = if channel_broken {
                "[工具执行中断: 结果通道已断开]"
            } else {
                "[工具执行中断: 未收到结果]"
            };
            tool_results.push(ToolResultMsg {
                tool_call_id: item.id.clone(),
                result: reason.to_string(),
                is_error: true,
                images: Vec::new(),
                plan_decision: PlanDecision::None,
            });
        }
    }

    log_tool_results(&tool_items, &tool_results);

    // ★ 记录写入操作到 ops.jsonl
    append_write_ops(&tool_items, &tool_results, ctx.session_id);

    // 收集需要延迟注入的图片消息（在所有 tool results 之后统一注入，
    // 避免在 tool results 中间插入 user 消息导致 API 报错）
    // 预分配：最多每个 result 可能产生一条图片消息
    let mut deferred_image_msgs: Vec<ChatMessage> = Vec::with_capacity(tool_results.len());

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
                cwd: current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
                ..Default::default()
            };
            if let Some(hook_result) =
                ctx.hook_manager
                    .execute(HookEvent::PostToolExecution, hook_ctx, ctx.disabled_hooks)
                && let Some(new_result) = hook_result.tool_result
            {
                result_content = new_result;
            }
        }

        let tool_msg = ChatMessage {
            role: MessageRole::Tool,
            content: result_content,
            tool_calls: None,
            tool_call_id: Some(result.tool_call_id.clone()),
            images: None,
            reasoning_content: None,
            sender_name: None,
        };
        messages.push(tool_msg.clone());
        push_both(ctx.display_messages, ctx.context_messages, tool_msg);

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
                    role: MessageRole::User,
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
                    reasoning_content: None,
                    sender_name: None,
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
            // 只加入 LLM 上下文，不推送到 display（避免 UI 渲染这条内部消息）
            messages.push(img_msg);
        }
    }

    drain_pending_user_messages(messages, ctx.pending_user_messages);

    Ok(ToolCallResult {
        compact_requested,
        plan_with_context_clear: plan_clear_context,
    })
}

/// 从 Edit/Write 工具的 arguments JSON 中提取 path 字段
fn extract_path_from_args(args: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(args)
        .ok()
        .and_then(|v| v.get("path")?.as_str().map(String::from))
}

/// 从 Bash 工具的 arguments JSON 中提取 command 字段
fn extract_command_from_args(args: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(args)
        .ok()
        .and_then(|v| v.get("command")?.as_str().map(String::from))
}

/// 记录 Edit/Write/Bash 写入操作到 ops.jsonl
fn append_write_ops(tool_items: &[ToolCallItem], tool_results: &[ToolResultMsg], session_id: &str) {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    for item in tool_items {
        let is_error = tool_results
            .iter()
            .any(|r| r.tool_call_id == item.id && r.is_error);

        let op_kind = match item.name.as_str() {
            "Edit" => {
                extract_path_from_args(&item.arguments).map(|path| SessionOpKind::Edit { path })
            }
            "Write" => {
                extract_path_from_args(&item.arguments).map(|path| SessionOpKind::Write { path })
            }
            "Bash" => extract_command_from_args(&item.arguments)
                .map(|cmd| SessionOpKind::Bash { command: cmd }),
            _ => None,
        };

        if let Some(op) = op_kind {
            let _ = append_session_op(
                session_id,
                &SessionOp {
                    op,
                    timestamp_ms: now_ms,
                    is_error,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::chat::storage::MessageRole;

    // ════════════════════════════════════════════════════════════════
    // 回归测试：drain_pending_user_messages
    // 如果以下测试失败，说明 agent loop 的用户消息注入逻辑被破坏
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn drain_pending_appends_user_messages_with_marker() {
        let pending: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![
            ChatMessage::text(MessageRole::User, "hello"),
            ChatMessage::text(MessageRole::User, "world"),
        ]));
        let mut messages: Vec<ChatMessage> = vec![ChatMessage::text(MessageRole::Assistant, "hi")];

        drain_pending_user_messages(&mut messages, &pending);

        assert_eq!(messages.len(), 3, "应有 3 条消息");
        assert_eq!(messages[1].content, "[User appended] hello");
        assert_eq!(messages[2].content, "[User appended] world");
        // pending 已被 drain
        assert!(pending.lock().unwrap().is_empty(), "pending 应被清空");
    }

    #[test]
    fn drain_pending_preserves_non_user_role() {
        let pending: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![ChatMessage::text(
            MessageRole::System,
            "system note",
        )]));
        let mut messages: Vec<ChatMessage> = vec![];

        drain_pending_user_messages(&mut messages, &pending);

        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content, "system note",
            "非 User 角色的消息不应加 [User appended] 前缀"
        );
    }

    #[test]
    fn drain_pending_empty_noop() {
        let pending: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
        let mut messages: Vec<ChatMessage> = vec![ChatMessage::text(MessageRole::User, "keep")];

        drain_pending_user_messages(&mut messages, &pending);

        assert_eq!(messages.len(), 1, "空 pending 不应改变 messages");
        assert_eq!(messages[0].content, "keep");
    }

    // ════════════════════════════════════════════════════════════════
    // 回归测试：push_both / clear_channels
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn push_both_appends_to_both_channels() {
        let display: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
        let context: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
        let msg = ChatMessage::text(MessageRole::Assistant, "response");

        push_both(&display, &context, msg);

        assert_eq!(display.lock().unwrap().len(), 1, "display 应有 1 条消息");
        assert_eq!(context.lock().unwrap().len(), 1, "context 应有 1 条消息");
        assert_eq!(display.lock().unwrap()[0].content, "response");
        assert_eq!(context.lock().unwrap()[0].content, "response");
    }

    #[test]
    fn push_both_clones_message() {
        let display: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
        let context: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
        let msg = ChatMessage::text(MessageRole::User, "test");

        push_both(&display, &context, msg);

        // 两个通道应各自持有独立的 clone
        let d = display.lock().unwrap();
        let c = context.lock().unwrap();
        assert_eq!(d[0].content, c[0].content);
    }

    #[test]
    fn clear_channels_empties_both() {
        let display: Arc<Mutex<Vec<ChatMessage>>> =
            Arc::new(Mutex::new(vec![ChatMessage::text(MessageRole::User, "a")]));
        let context: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![
            ChatMessage::text(MessageRole::Assistant, "b"),
            ChatMessage::text(MessageRole::Tool, "c"),
        ]));

        clear_channels(&display, &context);

        assert!(
            display.lock().unwrap().is_empty(),
            "clear 后 display 应为空"
        );
        assert!(
            context.lock().unwrap().is_empty(),
            "clear 后 context 应为空"
        );
    }

    #[test]
    fn sync_context_full_replaces_both() {
        let display: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![ChatMessage::text(
            MessageRole::User,
            "old",
        )]));
        let context: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![ChatMessage::text(
            MessageRole::Assistant,
            "old",
        )]));
        let new_msgs = vec![
            ChatMessage::text(MessageRole::System, "new1"),
            ChatMessage::text(MessageRole::User, "new2"),
        ];

        sync_context_full(&display, &context, &new_msgs);

        let d = display.lock().unwrap();
        let c = context.lock().unwrap();
        assert_eq!(d.len(), 2, "display 应被替换为 2 条新消息");
        assert_eq!(c.len(), 2, "context 应被替换为 2 条新消息");
        assert_eq!(d[0].content, "new1");
        assert_eq!(c[1].content, "new2");
    }
}
