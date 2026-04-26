use crate::command::chat::error::ChatError;
use crate::command::chat::storage::{ChatMessage, MessageRole, ModelProvider, ToolCallItem};
use crate::llm::{
    ChatRequest, Content, ContentPart, FunctionCall, ImageUrl, LlmClient, Message, Role, ToolCall,
    ToolDefinition,
};
use crate::util::log::{write_error_log, write_info_log};
use futures::StreamExt;
use std::collections::HashSet;

/// 根据 ModelProvider 配置创建 LlmClient
pub fn create_llm_client(provider: &ModelProvider) -> LlmClient {
    LlmClient::new(&provider.api_base, &provider.api_key)
}

/// 将内部 ChatMessage 转换为 llm::Message 格式
pub fn to_llm_messages(messages: &[ChatMessage]) -> Vec<Message> {
    messages
        .iter()
        .filter_map(|msg| match msg.role {
            MessageRole::System => Some(Message {
                role: Role::System,
                content: Some(Content::Text(msg.content.clone())),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            }),
            MessageRole::User => {
                if let Some(ref images) = msg.images
                    && !images.is_empty()
                {
                    // 多模态消息：Text + ImageUrl(s)
                    write_info_log(
                        "to_llm_messages",
                        &format!(
                            "构建多模态 user 消息: text_len={}, images_count={}",
                            msg.content.len(),
                            images.len()
                        ),
                    );
                    let mut parts = vec![ContentPart::Text {
                        text: msg.content.clone(),
                    }];
                    for img in images {
                        let data_url = format!("data:{};base64,{}", img.media_type, img.base64);
                        parts.push(ContentPart::ImageUrl {
                            image_url: ImageUrl {
                                url: data_url,
                                detail: None,
                            },
                        });
                    }
                    return Some(Message {
                        role: Role::User,
                        content: Some(Content::Parts(parts)),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    });
                }
                // 纯文本消息
                Some(Message {
                    role: Role::User,
                    content: Some(Content::Text(msg.content.clone())),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                })
            }
            MessageRole::Assistant => {
                let content = if msg.content.is_empty() {
                    None
                } else {
                    Some(Content::Text(msg.content.clone()))
                };
                let tool_calls = msg.tool_calls.as_ref().map(|tcs| {
                    tcs.iter()
                        .map(|tc| ToolCall {
                            id: tc.id.clone(),
                            call_type: "function".to_string(),
                            function: FunctionCall {
                                name: tc.name.clone(),
                                arguments: tc.arguments.clone(),
                            },
                        })
                        .collect()
                });
                Some(Message {
                    role: Role::Assistant,
                    content,
                    name: None,
                    tool_calls,
                    tool_call_id: None,
                    reasoning_content: msg.reasoning_content.clone(),
                })
            }
            MessageRole::Tool => {
                let tool_call_id = msg.tool_call_id.clone().unwrap_or_default();
                if tool_call_id.is_empty() {
                    write_error_log(
                        "to_llm_messages",
                        "跳过 tool_call_id 为空的 tool 消息（旧历史或异常消息），避免 API 报错",
                    );
                    return None;
                }
                Some(Message {
                    role: Role::Tool,
                    content: Some(Content::Text(msg.content.clone())),
                    name: None,
                    tool_calls: None,
                    tool_call_id: Some(tool_call_id),
                    reasoning_content: None,
                })
            }
        })
        .collect()
}

/// 预处理消息数组，保证 assistant tool_calls ↔ tool result 双向配对完整，
/// 避免 API 报 "tool_call_id not found" 或 "missing tool result" 错误。
pub fn sanitize_messages(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    let tool_result_ids: HashSet<String> = messages
        .iter()
        .filter(|m| m.role == MessageRole::Tool)
        .filter_map(|m| m.tool_call_id.clone())
        .filter(|id| !id.is_empty())
        .collect();

    let assistant_tool_call_ids: HashSet<String> = messages
        .iter()
        .filter(|m| m.role == MessageRole::Assistant)
        .flat_map(|m| {
            m.tool_calls
                .iter()
                .flatten()
                .filter(|tc| !tc.id.is_empty())
                .map(|tc| tc.id.clone())
        })
        .collect();

    let mut removed_count = 0usize;
    let result: Vec<ChatMessage> = messages
        .iter()
        .filter_map(|msg| {
            if msg.role == MessageRole::Tool {
                let id = msg.tool_call_id.as_deref().unwrap_or("");
                if id.is_empty() || !assistant_tool_call_ids.contains(id) {
                    write_error_log(
                        "sanitize_messages",
                        &format!(
                            "移除孤立 tool result tool_call_id={:?}（在 assistant tool_calls 中无对应项）",
                            msg.tool_call_id
                        ),
                    );
                    removed_count += 1;
                    return None;
                }
            }
            if msg.role == MessageRole::Assistant
                && let Some(ref tool_calls) = msg.tool_calls
            {
                let valid_tool_calls: Vec<_> = tool_calls
                    .iter()
                    .filter(|tool_call| {
                        !tool_call.id.is_empty() && tool_result_ids.contains(&tool_call.id)
                    })
                    .cloned()
                    .collect();
                if valid_tool_calls.len() != tool_calls.len() {
                    let dropped = tool_calls.len() - valid_tool_calls.len();
                    write_error_log(
                        "sanitize_messages",
                        &format!(
                            "assistant tool_calls 中 {} 个条目无对应 tool result，已移除",
                            dropped
                        ),
                    );
                    removed_count += dropped;
                    let mut sanitized_msg = msg.clone();
                    sanitized_msg.tool_calls = if valid_tool_calls.is_empty() {
                        None
                    } else {
                        Some(valid_tool_calls)
                    };
                    return Some(sanitized_msg);
                }
            }
            Some(msg.clone())
        })
        .collect();

    if removed_count > 0 {
        write_info_log(
            "sanitize_messages",
            &format!("共清理 {} 个孤立/无效 tool_call 相关条目", removed_count),
        );
    }
    result
}

/// 后置验证：确保转换后的消息中 tool_call_id 双向一致。
fn sanitize_llm_messages(messages: &mut Vec<Message>) {
    let assistant_tool_call_ids: HashSet<String> = messages
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .flat_map(|m| m.tool_calls.iter().flatten().map(|tc| tc.id.clone()))
        .filter(|id| !id.is_empty())
        .collect();

    let tool_result_ids: HashSet<String> = messages
        .iter()
        .filter(|m| m.role == Role::Tool)
        .filter_map(|m| m.tool_call_id.clone())
        .filter(|id| !id.is_empty())
        .collect();

    let original_len = messages.len();

    messages.retain(|m| {
        if m.role == Role::Tool {
            let id = m.tool_call_id.as_deref().unwrap_or("");
            if !assistant_tool_call_ids.contains(id) {
                write_error_log(
                    "sanitize_llm_messages",
                    &format!(
                        "移除孤立 tool result (tool_call_id={})：在 assistant tool_calls 中无对应项",
                        id
                    ),
                );
                return false;
            }
        }
        true
    });

    for msg in messages.iter_mut() {
        if msg.role == Role::Assistant
            && let Some(ref mut tool_calls) = msg.tool_calls
        {
            let before = tool_calls.len();
            tool_calls.retain(|tc| tc.id.is_empty() || tool_result_ids.contains(&tc.id));
            if tool_calls.len() != before {
                write_error_log(
                    "sanitize_llm_messages",
                    &format!(
                        "assistant tool_calls 中 {} 个条目无对应 tool result，已移除",
                        before - tool_calls.len()
                    ),
                );
            }
            if tool_calls.is_empty() {
                msg.tool_calls = None;
            }
        }
    }

    let removed_count = original_len - messages.len();
    if removed_count > 0 {
        write_info_log(
            "sanitize_llm_messages",
            &format!("后置验证：共移除 {} 条孤立消息", removed_count),
        );
    }
}

/// 构建带工具定义的请求
pub fn build_request_with_tools(
    provider: &ModelProvider,
    messages: &[ChatMessage],
    tools: Vec<ToolDefinition>,
    system_prompt: Option<&str>,
) -> Result<ChatRequest, ChatError> {
    let sanitized_messages = sanitize_messages(messages);
    let mut llm_messages = Vec::with_capacity(sanitized_messages.len() + 1);
    if let Some(system_prompt_text) = system_prompt {
        let trimmed = system_prompt_text.trim();
        if !trimmed.is_empty() {
            llm_messages.push(Message {
                role: Role::System,
                content: Some(Content::Text(trimmed.to_string())),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            });
        }
    }
    llm_messages.extend(to_llm_messages(&sanitized_messages));

    // debug: 检查是否有 reasoning_content 被传递
    for (i, msg) in llm_messages.iter().enumerate() {
        if msg.reasoning_content.is_some() {
            write_info_log(
                "build_request_with_tools",
                &format!(
                    "消息[{}] role={:?} 携带 reasoning_content (len={})",
                    i,
                    msg.role,
                    msg.reasoning_content.as_ref().map(|s| s.len()).unwrap_or(0)
                ),
            );
        }
    }

    sanitize_llm_messages(&mut llm_messages);

    Ok(ChatRequest {
        model: provider.model.clone(),
        messages: llm_messages,
        tools: if tools.is_empty() { None } else { Some(tools) },
        stream: None,
        max_tokens: None,
        extra: serde_json::Map::new(),
    })
}

/// 流式调用 API，通过回调逐步输出，返回完整的助手回复内容
pub async fn call_llm_stream_async(
    provider: &ModelProvider,
    messages: &[ChatMessage],
    system_prompt: Option<&str>,
    on_chunk: &mut dyn FnMut(&str),
) -> Result<String, ChatError> {
    let client = create_llm_client(provider);
    let mut llm_messages = Vec::with_capacity(messages.len() + 1);

    if let Some(system_prompt_text) = system_prompt {
        let trimmed = system_prompt_text.trim();
        if !trimmed.is_empty() {
            llm_messages.push(Message {
                role: Role::System,
                content: Some(Content::Text(trimmed.to_string())),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            });
        }
    }
    llm_messages.extend(to_llm_messages(messages));

    let request = ChatRequest {
        model: provider.model.clone(),
        messages: llm_messages,
        tools: None,
        stream: Some(true),
        max_tokens: None,
        extra: serde_json::Map::new(),
    };

    let request_body =
        serde_json::to_string(&request).unwrap_or_else(|e| format!("序列化request失败: {}", e));

    let mut stream = client.chat_completion_stream(&request).await.map_err(|e| {
        let err_msg = ChatError::from(e);
        write_info_log(
            "call_llm_stream_async API请求 ERROR",
            &format!("{}\nrequest body:\n{}", err_msg, request_body),
        );
        err_msg
    })?;

    let mut full_content = String::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                for choice in &response.choices {
                    if let Some(ref content) = choice.delta.content {
                        full_content.push_str(content);
                        on_chunk(content);
                    }
                }
            }
            Err(e) => {
                let err = ChatError::from(e);
                write_info_log(
                    "call_llm_stream_async 流式响应 ERROR",
                    &format!(
                        "{}\n已接收内容长度: {}\nrequest body:\n{}",
                        err,
                        full_content.len(),
                        request_body
                    ),
                );
                return Err(err);
            }
        }
    }

    Ok(full_content)
}

/// fallback 非流式调用结果
#[derive(Debug)]
pub struct FallbackResult {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCallItem>>,
    pub finish_reason: Option<String>,
    pub reasoning_content: Option<String>,
}

impl FallbackResult {
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls.is_some()
    }
}

/// 非流式请求（已内置宽松反序列化：finish_reason 为 String）
pub async fn call_llm_non_stream(
    provider: &ModelProvider,
    request: &ChatRequest,
) -> Result<FallbackResult, ChatError> {
    let client = create_llm_client(provider);
    let request_body =
        serde_json::to_string(request).unwrap_or_else(|e| format!("序列化request失败: {}", e));

    let response = client.chat_completion(request).await.map_err(|e| {
        let err = ChatError::from(e);
        write_error_log(
            "call_llm_non_stream",
            &format!("{}\nrequest body:\n{}", err, request_body),
        );
        err
    })?;

    let choice = match response.choices.first() {
        Some(c) => c,
        None => {
            return Ok(FallbackResult {
                content: None,
                tool_calls: None,
                finish_reason: None,
                reasoning_content: None,
            });
        }
    };

    let tool_items = choice.message.tool_calls.as_ref().map(|tool_calls| {
        tool_calls
            .iter()
            .map(|tool_call| {
                let id = if tool_call.id.is_empty() {
                    use rand::Rng;
                    let rand_id = format!("call_{:016x}", rand::thread_rng().r#gen::<u64>());
                    write_info_log(
                        "call_llm_non_stream",
                        &format!(
                            "tool_call id 为空，已生成随机 id: {} (tool: {})",
                            rand_id, tool_call.function.name
                        ),
                    );
                    rand_id
                } else {
                    tool_call.id.clone()
                };
                ToolCallItem {
                    id,
                    name: tool_call.function.name.clone(),
                    arguments: tool_call.function.arguments.clone(),
                }
            })
            .collect()
    });

    if let Some(ref reason) = choice.finish_reason
        && !matches!(
            reason.as_str(),
            "stop" | "length" | "tool_calls" | "content_filter" | "function_call"
        )
    {
        write_info_log(
            "call_llm_non_stream",
            &format!("非标准 finish_reason: {}", reason),
        );
    }

    Ok(FallbackResult {
        content: choice.message.content.clone(),
        tool_calls: tool_items,
        finish_reason: choice.finish_reason.clone(),
        reasoning_content: choice.message.reasoning_content.clone(),
    })
}

/// 同步包装：创建 tokio runtime 执行异步流式调用
pub fn call_llm_stream(
    provider: &ModelProvider,
    messages: &[ChatMessage],
    system_prompt: Option<&str>,
    on_chunk: &mut dyn FnMut(&str),
) -> Result<String, ChatError> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        let err = ChatError::RuntimeFailed(e.to_string());
        write_info_log("call_llm_stream 创建runtime ERROR", &format!("{}", err));
        err
    })?;
    rt.block_on(call_llm_stream_async(
        provider,
        messages,
        system_prompt,
        on_chunk,
    ))
}

/// 清理 API 响应 body 用于错误消息：剥离 HTML 标签，截断超长内容
#[allow(dead_code)]
fn sanitize_api_body(body: &str) -> String {
    let max_len = crate::command::chat::constants::API_ERROR_BODY_MAX_LEN;
    let truncated = &body[..body.len().min(max_len)];
    let mut result = String::with_capacity(truncated.len());
    let mut in_tag = false;
    for ch in truncated.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}
