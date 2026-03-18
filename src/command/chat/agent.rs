use super::api::{build_request_with_tools, create_openai_client};
use super::app::{StreamMsg, ToolResultMsg};
use super::storage::{ChatMessage, ModelProvider, ToolCallItem};
use super::tools::background::BackgroundManager;
use crate::util::log::{write_error_log, write_info_log};
use async_openai::types::chat::{ChatCompletionMessageToolCalls, ChatCompletionTools};
use futures::StreamExt;
use std::sync::{Arc, Mutex, mpsc};
use tokio_util::sync::CancellationToken;

/// 从待处理队列中 drain 用户在 agent loop 期间发送的新消息，追加到 messages
fn drain_pending_user_messages(
    messages: &mut Vec<ChatMessage>,
    pending_user_messages: &Arc<Mutex<Vec<ChatMessage>>>,
) {
    let mut pending = pending_user_messages.lock().unwrap();
    if !pending.is_empty() {
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
/// 返回 Ok(()) 表示成功（应 continue 循环），Err(()) 表示 channel 断开（应 return）
fn process_tool_calls(
    tool_items: Vec<ToolCallItem>,
    assistant_text: String,
    messages: &mut Vec<ChatMessage>,
    tx: &mpsc::Sender<StreamMsg>,
    tool_result_rx: &mpsc::Receiver<ToolResultMsg>,
    pending_user_messages: &Arc<Mutex<Vec<ChatMessage>>>,
) -> Result<(), ()> {
    log_tool_request(&tool_items);

    if !assistant_text.is_empty() {
        write_info_log("Chat 回复", &assistant_text);
    }

    messages.push(ChatMessage {
        role: "assistant".to_string(),
        content: assistant_text,
        tool_calls: Some(tool_items.clone()),
        tool_call_id: None,
    });

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

    for result in tool_results {
        messages.push(ChatMessage {
            role: "tool".to_string(),
            content: result.result,
            tool_calls: None,
            tool_call_id: Some(result.tool_call_id),
        });
    }

    drain_pending_user_messages(messages, pending_user_messages);
    Ok(())
}

/// 后台 Agent 循环：支持多轮工具调用
#[allow(clippy::too_many_arguments)]
pub async fn run_agent_loop(
    provider: ModelProvider,
    mut messages: Vec<ChatMessage>,
    tools: Vec<ChatCompletionTools>,
    system_prompt: Option<String>,
    use_stream: bool,
    streaming_content: Arc<Mutex<String>>,
    tx: mpsc::Sender<StreamMsg>,
    tool_result_rx: mpsc::Receiver<ToolResultMsg>,
    max_tool_rounds: usize,
    cancel_token: CancellationToken,
    pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,
    background_manager: Arc<BackgroundManager>,
) {
    let client = create_openai_client(&provider);

    for _round in 0..max_tool_rounds {
        // 每轮开始时从待处理队列中 drain 用户在 agent loop 期间输入的新消息
        drain_pending_user_messages(&mut messages, &pending_user_messages);

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
                });
                write_info_log(
                    "BackgroundNotification",
                    &format!("注入后台任务通知: task_id={}", notif.task_id),
                );
            }
        }

        // 清空流式内容缓冲（每轮开始时）
        {
            let mut sc = streaming_content.lock().unwrap();
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
                    "assistant" => {
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
                    "tool" => {
                        let id = msg.tool_call_id.as_deref().unwrap_or("?");
                        log_content.push_str(&format!("[Tool/Result({})] {}\n", id, msg.content));
                    }
                    "user" => {
                        log_content.push_str(&format!("[User] {}\n", msg.content));
                    }
                    other => {
                        log_content.push_str(&format!("[{}] {}\n", other, msg.content));
                    }
                }
            }
            write_info_log("Chat 请求", &log_content);
        }

        let request = match build_request_with_tools(
            &provider,
            &messages,
            tools.clone(),
            system_prompt.as_deref(),
        ) {
            Ok(req) => req,
            Err(e) => {
                let _ = tx.send(StreamMsg::Error(format!("构建请求失败: {}", e)));
                return;
            }
        };

        if use_stream {
            // 流式模式
            let mut stream = match client.chat().create_stream(request.clone()).await {
                Ok(s) => s,
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

            'stream: loop {
                tokio::select! {
                    result = stream.next() => {
                        match result {
                            Some(Ok(response)) => {
                                for choice in &response.choices {
                                    if let Some(ref content) = choice.delta.content {
                                        assistant_text.push_str(content);
                                        let mut sc = streaming_content.lock().unwrap();
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
                            None => break 'stream,
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
                write_info_log("Chat 回复", &assistant_text);
            }

            // 如果流式遇到 tool_calls 反序列化错误，fallback 到非流式获取完整响应
            if stream_had_deserialize_error {
                // 清空流式内容（切换到非流式）
                {
                    let mut sc = streaming_content.lock().unwrap();
                    sc.clear();
                }
                // 重新构建请求（不带 stream）
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
                                    ) {
                                        Ok(()) => continue,
                                        Err(()) => return,
                                    }
                                }
                            // 普通文本回复
                            if let Some(ref content) = choice.message.content {
                                write_info_log("Chat 回复", content);
                                let mut sc = streaming_content.lock().unwrap();
                                sc.push_str(content);
                                drop(sc);
                                let _ = tx.send(StreamMsg::Chunk);
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("API 请求失败(fallback): {}", e);
                        write_error_log("Chat API fallback 非流式", &error_msg);
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
                if !pending_user_messages.lock().unwrap().is_empty() {
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
                ) {
                    Ok(()) => continue,
                    Err(()) => return,
                }
            } else {
                // 正常结束，但如果有用户增量消息则继续循环
                if !pending_user_messages.lock().unwrap().is_empty() {
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
                                ) {
                                    Ok(()) => continue,
                                    Err(()) => return,
                                }
                            }

                        // 正常文本回复
                        if let Some(ref content) = choice.message.content {
                            write_info_log("Chat 回复", content);
                            let mut sc = streaming_content.lock().unwrap();
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
            if !pending_user_messages.lock().unwrap().is_empty() {
                continue;
            }
            break;
        }
    }

    let _ = tx.send(StreamMsg::Done);
}
