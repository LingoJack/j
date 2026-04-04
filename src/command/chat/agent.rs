use super::api::{build_request_with_tools, call_openai_non_stream_lenient, create_openai_client};
use super::app::{StreamMsg, ToolResultMsg};
use super::compact::{self, CompactConfig};
use super::hook::{HookContext, HookEvent, HookManager};
use super::storage::{ChatMessage, ModelProvider, ToolCallItem};
use super::tools::background::BackgroundManager;
use super::tools::todo::TodoManager;
use crate::command::chat::constants::{ROLE_ASSISTANT, ROLE_TOOL, ROLE_USER};
use crate::command::chat::tools::Tool;
use crate::command::chat::tools::compact::CompactTool;
use crate::util::log::{write_error_log, write_info_log};
use crate::util::safe_lock;
use async_openai::types::chat::{ChatCompletionMessageToolCalls, ChatCompletionTools};
use futures::StreamExt;
use std::sync::{Arc, Mutex, mpsc};
use tokio_util::sync::CancellationToken;

/// 后台 Agent 循环：支持多轮工具调用
#[allow(clippy::too_many_arguments)]
pub async fn run_agent_loop(
    provider: ModelProvider,
    mut messages: Vec<ChatMessage>,
    tools: Vec<ChatCompletionTools>,
    mut system_prompt: Option<String>,
    use_stream: bool,
    streaming_content: Arc<Mutex<String>>,
    tx: mpsc::Sender<StreamMsg>,
    tool_result_rx: mpsc::Receiver<ToolResultMsg>,
    max_tool_rounds: usize,
    cancel_token: CancellationToken,
    pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,
    background_manager: Arc<BackgroundManager>,
    compact_config: CompactConfig,
    hook_manager: HookManager,
    todo_manager: Arc<TodoManager>,
    shared_messages: Arc<Mutex<Vec<ChatMessage>>>,
) {
    let client = create_openai_client(&provider);

    for _round in 0..max_tool_rounds {
        // 每轮开始时从待处理队列中 drain 用户在 agent loop 期间输入的新消息
        drain_pending_user_messages(&mut messages, &pending_user_messages);

        // ── Layer 1: micro_compact（替换旧 tool results）──
        // ── Layer 2: if tokens > threshold → auto_compact（LLM 摘要）──
        if compact_config.enabled {
            compact::micro_compact(&mut messages, compact_config.keep_recent);
            if compact::estimate_tokens(&messages) > compact_config.token_threshold {
                write_info_log(
                    "agent_loop",
                    "auto_compact triggered (token threshold exceeded)",
                );
                if let Err(e) = compact::auto_compact(&mut messages, &provider).await {
                    write_error_log("agent_loop", &format!("auto_compact failed: {}", e));
                }
            }
        }

        // Drain 后台任务完成通知，注入为系统消息
        {
            let notifications = background_manager.drain_notifications();
            for notif in notifications {
                let notif_msg = format!(
                    "[后台任务完成] task_id={}, command={}, status={}\n结果:\n{}",
                    notif.task_id, notif.command, notif.status, notif.result
                );
                messages.push(ChatMessage {
                    role: "user".to_string(),
                    content: notif_msg,
                    tool_calls: None,
                    tool_call_id: None,
                    images: None,
                });
                write_info_log(
                    "BackgroundNotification",
                    &format!("注入后台任务通知: task_id={}", notif.task_id),
                );
            }
        }

        // 检查是否有待办事项
        todo_manager.increment_turn();
        if todo_manager.has_todos() && todo_manager.turns_since_last_call() >= 15 {
            let todos_summary = todo_manager.format_todos_summary();
            messages.push(ChatMessage {
                role: ROLE_TOOL.to_string(),
                content: format!(
                    "<system-reminder>It seems that you have an active todo list but haven't updated it in 15+ rounds. forget to update or ignore this reminder if you are processing the item work\n\nCurrent todo items:\n{}</system-reminder>",
                    todos_summary
                ),
                tool_calls: None,
                tool_call_id: None,
                images: None,
            });
            write_info_log(
                "TodoNagReminder",
                &format!("Injected nag reminder with todos:\n{}", todos_summary),
            );
        }

        // 清空流式内容缓冲（每轮开始时）
        {
            let mut sc = safe_lock(&streaming_content, "agent::streaming_content_clear");
            sc.clear();
        }

        // 记录请求输入日志
        {
            let mut log_content = String::new();
            if let Some(ref sp) = system_prompt {
                log_content.push_str(&format!("[System] {}\n", sp));
            }
            for msg in &messages {
                match msg.role.as_str() {
                    ROLE_ASSISTANT => {
                        if !msg.content.is_empty() {
                            log_content.push_str(&format!("[Assistant] {}\n", msg.content));
                        }
                        if let Some(ref tcs) = msg.tool_calls {
                            for tc in tcs {
                                log_content.push_str(&format!(
                                    "[Assistant/ToolCall] {}: {}\n",
                                    tc.name, tc.arguments
                                ));
                            }
                        }
                    }
                    ROLE_TOOL => {
                        let id = msg.tool_call_id.as_deref().unwrap_or("?");
                        let tool_name = msg
                            .tool_calls
                            .as_ref()
                            .and_then(|tc| tc.first())
                            .map(|tc| tc.name.as_str())
                            .unwrap_or("unknown");
                        log_content.push_str(&format!(
                            "[Tool/Result({} with id `{}`)] result:\n{}\n",
                            tool_name, id, msg.content
                        ));
                    }
                    ROLE_USER => {
                        log_content.push_str(&format!("[User] {}\n", msg.content));
                    }
                    other => {
                        log_content.push_str(&format!("[{}] {}\n", other, msg.content));
                    }
                }
            }
            write_info_log("Chat 请求", &log_content);
        }

        // ★ PreLlmRequest hook（可修改 messages 和 system_prompt）
        if hook_manager.has_hooks_for(HookEvent::PreLlmRequest) {
            let ctx = HookContext {
                event: HookEvent::PreLlmRequest,
                messages: Some(messages.clone()),
                system_prompt: system_prompt.clone(),
                model: Some(provider.model.clone()),
                cwd: std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
                ..Default::default()
            };
            if let Some(result) = hook_manager.execute(HookEvent::PreLlmRequest, ctx) {
                if result.abort {
                    let _ = tx.send(StreamMsg::Error("LLM 请求被 hook 中止".to_string()));
                    return;
                }
                if let Some(new_msgs) = result.messages {
                    messages = new_msgs;
                }
                if let Some(new_prompt) = result.system_prompt {
                    system_prompt = Some(new_prompt);
                }
                if let Some(inject) = result.inject_messages {
                    messages.extend(inject);
                }
            }
        }

        // 记录本轮请求的消息统计
        {
            let has_images = messages
                .iter()
                .any(|m| m.images.as_ref().map_or(false, |imgs| !imgs.is_empty()));
            write_info_log(
                "agent_loop",
                &format!(
                    "第 {} 轮请求: messages={}, has_images={}, use_stream={}, supports_vision={}",
                    _round,
                    messages.len(),
                    has_images,
                    use_stream,
                    provider.supports_vision
                ),
            );
        }

        let request = match build_request_with_tools(
            &provider,
            &messages,
            tools.clone(),
            system_prompt.as_deref(),
        ) {
            Ok(req) => {
                write_info_log("agent_loop", "build_request_with_tools 成功");
                req
            }
            Err(e) => {
                let _ = tx.send(StreamMsg::Error(format!("构建请求失败: {}", e)));
                return;
            }
        };

        if use_stream {
            // 流式模式
            write_info_log("agent_loop", "开始创建流式请求...");
            let mut stream = match client.chat().create_stream(request.clone()).await {
                Ok(s) => {
                    write_info_log("agent_loop", "流式请求创建成功");
                    s
                }
                Err(e) => {
                    let error_msg = format!("API 请求失败: {}", e);
                    write_error_log("Chat API 流式请求创建", &error_msg);
                    let _ = tx.send(StreamMsg::Error(error_msg));
                    return;
                }
            };

            let mut finish_reason: Option<async_openai::types::chat::FinishReason> = None;
            let mut assistant_text = String::new();
            // 手动收集 tool_calls：按 index 聚合 (id, name, arguments)
            let mut raw_tool_calls: std::collections::BTreeMap<u32, (String, String, String)> =
                std::collections::BTreeMap::new();
            let mut stream_had_deserialize_error = false;

            let mut stream_chunk_count: u32 = 0;

            'stream: loop {
                tokio::select! {
                    result = stream.next() => {
                        match result {
                            Some(Ok(response)) => {
                                stream_chunk_count += 1;
                                // 记录前几个 chunk 的原始信息，便于调试
                                if stream_chunk_count <= 3 {
                                    let choices_debug: Vec<String> = response.choices.iter().map(|c| {
                                        format!(
                                            "idx={}, finish_reason={:?}, has_content={}, has_tool_calls={}",
                                            c.index,
                                            c.finish_reason,
                                            c.delta.content.is_some(),
                                            c.delta.tool_calls.is_some(),
                                        )
                                    }).collect();
                                    write_info_log(
                                        "stream_chunk",
                                        &format!("chunk #{}: choices=[{}]", stream_chunk_count, choices_debug.join("; ")),
                                    );
                                }
                                for choice in &response.choices {
                                    if let Some(ref content) = choice.delta.content {
                                        assistant_text.push_str(content);
                                        let mut sc = safe_lock(&streaming_content, "agent::stream_chunk");
                                        sc.push_str(content);
                                        drop(sc);
                                        let _ = tx.send(StreamMsg::Chunk);
                                    }
                                    // 尝试直接读取 tool_calls（若 async-openai 能反序列化）
                                    if let Some(ref toolcall_chunks) = choice.delta.tool_calls {
                                        for chunk in toolcall_chunks {
                                            let entry =
                                                raw_tool_calls.entry(chunk.index).or_insert_with(|| {
                                                    (
                                                        chunk.id.clone().unwrap_or_default(),
                                                        String::new(),
                                                        String::new(),
                                                    )
                                                });
                                            if entry.0.is_empty()
                                                && let Some(ref id) = chunk.id {
                                                    entry.0 = id.clone();
                                                }
                                            if let Some(ref tool_function) = chunk.function {
                                                if let Some(ref name) = tool_function.name {
                                                    entry.1.push_str(name);
                                                }
                                                if let Some(ref args) = tool_function.arguments {
                                                    entry.2.push_str(args);
                                                }
                                            }
                                        }
                                    }
                                    if let Some(ref fr) = choice.finish_reason {
                                        finish_reason = Some(*fr);
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                let error_str = format!("{}", e);
                                write_error_log("Chat API 流式响应 error", &error_str);
                                // 检测是否是 tool_calls 反序列化错误（Gemini 等不返回 chunk index）
                                if error_str.contains("missing field `index`")
                                    || error_str.contains("tool_calls")
                                {
                                    // 标记需要用非流式重做，跳出流式循环
                                    stream_had_deserialize_error = true;
                                    break 'stream;
                                }
                                write_error_log("Chat API 流式响应", &error_str);
                                let _ = tx.send(StreamMsg::Error(error_str));
                                return;
                            }
                            None => {
                                write_info_log("agent_loop", "流式结束 (stream returned None)");
                                break 'stream;
                            }
                        }
                    }
                    _ = cancel_token.cancelled() => {
                        let _ = tx.send(StreamMsg::Cancelled);
                        return;
                    }
                }
            }

            // 记录流式回复日志
            if !assistant_text.is_empty() {
                write_info_log("Sprite 回复", &assistant_text);
            }

            write_info_log(
                "agent_loop",
                &format!(
                    "流式循环结束: finish_reason={:?}, assistant_text_len={}, raw_tool_calls={}, stream_had_deserialize_error={}",
                    finish_reason,
                    assistant_text.len(),
                    raw_tool_calls.len(),
                    stream_had_deserialize_error
                ),
            );

            // 如果流式遇到 tool_calls 反序列化错误，或者流式返回空响应（finish_reason=None 且无内容），
            // fallback 到非流式获取完整响应
            if stream_had_deserialize_error
                || (finish_reason.is_none()
                    && assistant_text.is_empty()
                    && raw_tool_calls.is_empty()
                    && stream_chunk_count == 0)
            {
                if finish_reason.is_none() && stream_chunk_count == 0 {
                    write_info_log(
                        "agent_loop",
                        "流式返回空响应 (0 chunks)，fallback 到非流式重试",
                    );
                }
                // 清空流式内容（切换到非流式）
                {
                    let mut sc = safe_lock(&streaming_content, "agent::fallback_clear");
                    sc.clear();
                }
                // 使用宽松反序列化的非流式调用（兼容非标准 finish_reason）
                let create_fut = call_openai_non_stream_lenient(&provider, &request);
                tokio::select! {
                    result = create_fut => match result {
                    Ok(fallback_result) => {
                        if fallback_result.is_tool_calls
                            && let Some(tool_items) = fallback_result.tool_calls {
                                    if tool_items.is_empty() {
                                        break;
                                    }
                                    let assistant_text =
                                        fallback_result.content.clone().unwrap_or_default();
                                    match process_tool_calls(
                                        tool_items,
                                        assistant_text,
                                        &mut messages,
                                        &tx,
                                        &tool_result_rx,
                                        &pending_user_messages,
                                        &hook_manager,
                                        provider.supports_vision,
                                        &shared_messages,
                                        &streaming_content,
                                    ) {
                                        Ok(compact_requested) => {
                                            // ── Layer 3: compact tool 触发 ──
                                            if compact_requested && compact_config.enabled {
                                                let _ = compact::auto_compact(&mut messages, &provider).await;
                                            }
                                            continue;
                                        }
                                        Err(()) => return,
                                    }
                                }
                        // 普通文本回复（或非标准 finish_reason 如 network_error）
                        if let Some(ref content) = fallback_result.content
                            && !content.is_empty()
                        {
                                write_info_log("Sprite 回复", content);
                                let mut sc = safe_lock(&streaming_content, "agent::fallback_content");
                                sc.push_str(content);
                                drop(sc);
                                let _ = tx.send(StreamMsg::Chunk);
                        }
                        // 非标准 finish_reason 且无内容时，报告错误
                        if let Some(ref reason) = fallback_result.finish_reason
                            && !matches!(reason.as_str(), "stop" | "length" | "tool_calls" | "content_filter" | "function_call")
                            && fallback_result.content.as_deref().unwrap_or_default().is_empty()
                        {
                                let error_msg = format!("API 返回异常: finish_reason={}", reason);
                                write_error_log("Sprite API fallback 非流式", &error_msg);
                                let _ = tx.send(StreamMsg::Error(error_msg));
                                return;
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("API 请求失败(fallback): {}", e);
                        write_error_log("Sprite API fallback 非流式", &error_msg);
                        let _ = tx.send(StreamMsg::Error(error_msg));
                        return;
                    }
                },
                    _ = cancel_token.cancelled() => {
                        let _ = tx.send(StreamMsg::Cancelled);
                        return;
                    }
                }
                // fallback 非流式正常结束，但如果有用户增量消息则继续循环
                if !safe_lock(&pending_user_messages, "agent::pending_check_fallback").is_empty() {
                    flush_streaming_as_message(&streaming_content, &mut messages, &shared_messages);
                    continue;
                }
                break;
            }

            // 检查流式模式下的 tool_calls finish_reason
            let is_tool_calls = matches!(
                finish_reason,
                Some(async_openai::types::chat::FinishReason::ToolCalls)
            );

            if is_tool_calls && !raw_tool_calls.is_empty() {
                let tool_items: Vec<ToolCallItem> = raw_tool_calls
                    .into_values()
                    .map(|(id, name, arguments)| ToolCallItem {
                        id,
                        name,
                        arguments,
                    })
                    .collect();

                if tool_items.is_empty() {
                    break;
                }

                match process_tool_calls(
                    tool_items,
                    assistant_text,
                    &mut messages,
                    &tx,
                    &tool_result_rx,
                    &pending_user_messages,
                    &hook_manager,
                    provider.supports_vision,
                    &shared_messages,
                    &streaming_content,
                ) {
                    Ok(compact_requested) => {
                        // ── Layer 3: compact tool 触发 ──
                        if compact_requested && compact_config.enabled {
                            let _ = compact::auto_compact(&mut messages, &provider).await;
                        }
                        continue;
                    }
                    Err(()) => return,
                }
            } else {
                // 正常结束，但如果有用户增量消息则继续循环
                if !safe_lock(&pending_user_messages, "agent::pending_check_stream").is_empty() {
                    flush_streaming_as_message(&streaming_content, &mut messages, &shared_messages);
                    continue;
                }
                break;
            }
        } else {
            // 非流式模式
            let chat_client = client.chat();
            let create_fut = chat_client.create(request);
            tokio::select! {
                result = create_fut => match result {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        let is_tool_calls = matches!(
                            choice.finish_reason,
                            Some(async_openai::types::chat::FinishReason::ToolCalls)
                        );

                        if is_tool_calls
                            && let Some(ref tc_list) = choice.message.tool_calls {
                                let tool_items = extract_tool_items(tc_list);
                                if tool_items.is_empty() {
                                    break;
                                }
                                let assistant_text =
                                    choice.message.content.clone().unwrap_or_default();
                                match process_tool_calls(
                                    tool_items,
                                    assistant_text,
                                    &mut messages,
                                    &tx,
                                    &tool_result_rx,
                                    &pending_user_messages,
                                    &hook_manager,
                                    provider.supports_vision,
                                    &shared_messages,
                                    &streaming_content,
                                ) {
                                    Ok(compact_requested) => {
                                        // ── Layer 3: compact tool 触发 ──
                                        if compact_requested && compact_config.enabled {
                                            let _ = compact::auto_compact(&mut messages, &provider).await;
                                        }
                                        continue;
                                    }
                                    Err(()) => return,
                                }
                            }

                        // 正常文本回复
                        if let Some(ref content) = choice.message.content {
                            write_info_log("Chat 回复", content);
                            let mut sc = safe_lock(&streaming_content, "agent::non_stream_content");
                            sc.push_str(content);
                            drop(sc);
                            let _ = tx.send(StreamMsg::Chunk);
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("API 请求失败: {}", e);
                    write_error_log("Chat API 非流式请求", &error_msg);
                    let _ = tx.send(StreamMsg::Error(error_msg));
                    return;
                }
                },
                _ = cancel_token.cancelled() => {
                    let _ = tx.send(StreamMsg::Cancelled);
                    return;
                }
            }
            // 非流式正常结束，但如果有用户增量消息则继续循环
            if !safe_lock(&pending_user_messages, "agent::pending_check_non_stream").is_empty() {
                flush_streaming_as_message(&streaming_content, &mut messages, &shared_messages);
                continue;
            }
            break;
        }
    }

    let _ = tx.send(StreamMsg::Done);
}

/// 从待处理队列中 drain 用户在 agent loop 期间发送的新消息，追加到 messages
fn drain_pending_user_messages(
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

/// 从非流式响应的 tool_calls 列表中提取 ToolCallItem
fn extract_tool_items(raw_tool_list: &[ChatCompletionMessageToolCalls]) -> Vec<ToolCallItem> {
    raw_tool_list
        .iter()
        .filter_map(|tc| {
            if let ChatCompletionMessageToolCalls::Function(function) = tc {
                Some(ToolCallItem {
                    id: function.id.clone(),
                    name: function.function.name.clone(),
                    arguments: function.function.arguments.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

/// 向共享消息列表中追加一条消息（agent 线程写入，UI 线程读取）
fn push_shared(shared: &Arc<Mutex<Vec<ChatMessage>>>, msg: ChatMessage) {
    if let Ok(mut msgs) = shared.lock() {
        msgs.push(msg);
    }
}

/// 将 streaming_content 中的文本保存为 assistant 消息（多轮 agent loop 中间轮的文本回复）
/// 调用后 streaming_content 被清空，避免 UI 侧 finish_loading 再次保存导致重复
fn flush_streaming_as_message(
    streaming_content: &Arc<Mutex<String>>,
    messages: &mut Vec<ChatMessage>,
    shared_messages: &Arc<Mutex<Vec<ChatMessage>>>,
) {
    let mut sc = safe_lock(streaming_content, "agent::flush_streaming");
    if !sc.is_empty() {
        let text_msg = ChatMessage {
            role: ROLE_ASSISTANT.to_string(),
            content: sc.clone(),
            tool_calls: None,
            tool_call_id: None,
            images: None,
        };
        messages.push(text_msg.clone());
        push_shared(shared_messages, text_msg);
        sc.clear();
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
/// 返回 Ok(bool) 表示成功（应 continue 循环），bool 为 true 时表示有 compact tool 被调用
/// Err(()) 表示 channel 断开（应 return）
#[allow(clippy::too_many_arguments)]
fn process_tool_calls(
    tool_items: Vec<ToolCallItem>,
    assistant_text: String,
    messages: &mut Vec<ChatMessage>,
    tx: &mpsc::Sender<StreamMsg>,
    tool_result_rx: &mpsc::Receiver<ToolResultMsg>,
    pending_user_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    hook_manager: &HookManager,
    supports_vision: bool,
    shared_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    streaming_content: &Arc<Mutex<String>>,
) -> Result<bool, ()> {
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
        push_shared(shared_messages, text_msg);
        // 清空 streaming_content，文本已保存，避免 UI 继续显示流式内容
        if let Ok(mut sc) = streaming_content.lock() {
            sc.clear();
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
    push_shared(shared_messages, tool_call_msg);

    if tx
        .send(StreamMsg::ToolCallRequest(tool_items.clone()))
        .is_err()
    {
        return Err(());
    }

    let mut tool_results: Vec<ToolResultMsg> = Vec::new();
    for _ in &tool_items {
        match tool_result_rx.recv() {
            Ok(result) => tool_results.push(result),
            Err(_) => return Err(()),
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
        if hook_manager.has_hooks_for(HookEvent::PostToolExecution) {
            let ctx = HookContext {
                event: HookEvent::PostToolExecution,
                tool_name: tool_name.clone(),
                tool_result: Some(result_content.clone()),
                cwd: std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
                ..Default::default()
            };
            if let Some(hook_result) = hook_manager.execute(HookEvent::PostToolExecution, ctx)
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
        push_shared(shared_messages, tool_msg);

        // 如果模型支持视觉且工具返回了图片，先收集，稍后统一注入
        if !result_images.is_empty() {
            let tool_label = tool_name.as_deref().unwrap_or("unknown");
            let img_count = result_images.len();
            write_info_log(
                "ImageInjection",
                &format!(
                    "工具 {} 返回了 {} 张图片, supports_vision={}",
                    tool_label, img_count, supports_vision
                ),
            );
            if supports_vision {
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
                            .map(|img| super::storage::ImageData {
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
            messages.push(img_msg.clone());
            push_shared(shared_messages, img_msg);
        }
    }

    drain_pending_user_messages(messages, pending_user_messages);
    Ok(compact_requested)
}
