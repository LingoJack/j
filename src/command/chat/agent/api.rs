use super::super::error::ChatError;
use super::super::storage::{ChatMessage, ModelProvider, ToolCallItem};
use crate::command::chat::constants;
use crate::util::log::{write_error_log, write_info_log};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestMessageContentPartImage, ChatCompletionRequestMessageContentPartText,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
        ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageArgs,
        ChatCompletionRequestUserMessageContent, ChatCompletionRequestUserMessageContentPart,
        ChatCompletionTools, CreateChatCompletionRequest, CreateChatCompletionRequestArgs,
        FunctionCall, ImageUrl,
    },
};
use constants::{ROLE_ASSISTANT, ROLE_SYSTEM, ROLE_TOOL, ROLE_USER};
use futures::StreamExt;
use serde::Deserialize;

/// 根据 ModelProvider 配置创建 async-openai Client
pub fn create_openai_client(provider: &ModelProvider) -> Client<OpenAIConfig> {
    let config = OpenAIConfig::new()
        .with_api_key(&provider.api_key)
        .with_api_base(&provider.api_base);
    Client::with_config(config)
}

/// 将内部 ChatMessage 转换为 async-openai 的请求消息格式
pub fn to_openai_messages(messages: &[ChatMessage]) -> Vec<ChatCompletionRequestMessage> {
    messages
        .iter()
        .filter_map(|msg| match msg.role.as_str() {
            ROLE_SYSTEM => ChatCompletionRequestSystemMessageArgs::default()
                .content(msg.content.as_str())
                .build()
                .ok()
                .map(ChatCompletionRequestMessage::System),
            ROLE_USER => {
                if let Some(ref images) = msg.images
                    && !images.is_empty()
                {
                    // 多模态消息：Text + ImageUrl(s)
                    write_info_log(
                        "to_openai_messages",
                        &format!(
                            "构建多模态 user 消息: text_len={}, images_count={}",
                            msg.content.len(),
                            images.len()
                        ),
                    );
                    let mut parts: Vec<ChatCompletionRequestUserMessageContentPart> =
                        vec![ChatCompletionRequestUserMessageContentPart::Text(
                            ChatCompletionRequestMessageContentPartText {
                                text: msg.content.clone(),
                            },
                        )];
                    for img in images {
                        let data_url = format!("data:{};base64,{}", img.media_type, img.base64);
                        parts.push(ChatCompletionRequestUserMessageContentPart::ImageUrl(
                            ChatCompletionRequestMessageContentPartImage {
                                image_url: ImageUrl {
                                    url: data_url,
                                    detail: None,
                                },
                            },
                        ));
                    }
                    let user_msg = ChatCompletionRequestUserMessage {
                        content: ChatCompletionRequestUserMessageContent::Array(parts),
                        name: None,
                    };
                    return Some(ChatCompletionRequestMessage::User(user_msg));
                }
                // 纯文本消息
                ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content.as_str())
                    .build()
                    .ok()
                    .map(ChatCompletionRequestMessage::User)
            }
            ROLE_ASSISTANT => {
                let mut builder = ChatCompletionRequestAssistantMessageArgs::default();
                if !msg.content.is_empty() {
                    builder.content(msg.content.as_str());
                }
                if let Some(ref tool_calls) = msg.tool_calls {
                    let tc_list: Vec<ChatCompletionMessageToolCalls> = tool_calls
                        .iter()
                        .map(|tc| {
                            ChatCompletionMessageToolCalls::Function(
                                ChatCompletionMessageToolCall {
                                    id: tc.id.clone(),
                                    function: FunctionCall {
                                        name: tc.name.clone(),
                                        arguments: tc.arguments.clone(),
                                    },
                                },
                            )
                        })
                        .collect();
                    builder.tool_calls(tc_list);
                }
                builder
                    .build()
                    .ok()
                    .map(ChatCompletionRequestMessage::Assistant)
            }
            ROLE_TOOL => {
                let tool_call_id = msg.tool_call_id.clone().unwrap_or_default();
                // tool_call_id 为空会导致 API 报 "tool_call_id is not found"，直接跳过
                if tool_call_id.is_empty() {
                    write_error_log(
                        "to_openai_messages",
                        "跳过 tool_call_id 为空的 tool 消息（旧历史或异常消息），避免 API 报错",
                    );
                    return None;
                }
                ChatCompletionRequestToolMessageArgs::default()
                    .content(msg.content.as_str())
                    .tool_call_id(tool_call_id)
                    .build()
                    .ok()
                    .map(ChatCompletionRequestMessage::Tool)
            }
            _ => None,
        })
        .collect()
}

/// 预处理消息数组，保证 assistant tool_calls ↔ tool result 双向配对完整，
/// 避免 API 报 "tool_call_id not found" 或 "missing tool result" 错误。
///
/// 处理逻辑（两步）：
/// Step 1 — 从 tool result 侧收集有效 id：
///   收集所有 role="tool" 且 tool_call_id 非空的 id，构成 "已有结果集"。
/// Step 2 — 双向清理：
///   - assistant 消息：将 tool_calls 中没有对应 result 的条目或 id 为空的条目删掉；
///     若 tool_calls 全部被清空，置为 None（保留消息的文本 content）。
///   - tool 消息：tool_call_id 为空或在任何 assistant tool_calls 中无对应 id 的，跳过。
pub fn sanitize_messages(messages: &[ChatMessage]) -> Vec<ChatMessage> {
    // Step 1：收集所有有效 tool result id（非空）
    let result_ids: std::collections::HashSet<String> = messages
        .iter()
        .filter(|m| m.role == ROLE_TOOL)
        .filter_map(|m| m.tool_call_id.clone())
        .filter(|id| !id.is_empty())
        .collect();

    // 收集所有 assistant 消息中合法（id 非空）的 tool_call id
    let assistant_ids: std::collections::HashSet<String> = messages
        .iter()
        .filter(|m| m.role == ROLE_ASSISTANT)
        .flat_map(|m| {
            m.tool_calls
                .iter()
                .flatten()
                .filter(|tc| !tc.id.is_empty())
                .map(|tc| tc.id.clone())
        })
        .collect();

    let mut removed = 0usize;
    let result: Vec<ChatMessage> = messages
        .iter()
        .filter_map(|msg| {
            if msg.role == ROLE_TOOL {
                let id = msg.tool_call_id.as_deref().unwrap_or("");
                // 孤立 tool result：id 为空，或在 assistant tool_calls 中无对应项
                if id.is_empty() || !assistant_ids.contains(id) {
                    write_error_log(
                        "sanitize_messages",
                        &format!(
                            "移除孤立 tool result tool_call_id={:?}（在 assistant tool_calls 中无对应项）",
                            msg.tool_call_id
                        ),
                    );
                    removed += 1;
                    return None;
                }
            }
            if msg.role == ROLE_ASSISTANT
                && let Some(ref tcs) = msg.tool_calls
            {
                // 仅保留：id 非空 且 已有对应 tool result 的条目
                let clean: Vec<_> = tcs
                    .iter()
                    .filter(|tc| !tc.id.is_empty() && result_ids.contains(&tc.id))
                    .cloned()
                    .collect();
                if clean.len() != tcs.len() {
                    let dropped = tcs.len() - clean.len();
                    write_error_log(
                        "sanitize_messages",
                        &format!(
                            "assistant tool_calls 中 {} 个条目无对应 tool result，已移除",
                            dropped
                        ),
                    );
                    removed += dropped;
                    let mut fixed = msg.clone();
                    fixed.tool_calls = if clean.is_empty() { None } else { Some(clean) };
                    return Some(fixed);
                }
            }
            Some(msg.clone())
        })
        .collect();

    if removed > 0 {
        write_info_log(
            "sanitize_messages",
            &format!("共清理 {} 个孤立/无效 tool_call 相关条目", removed),
        );
    }
    result
}

/// 后置验证：确保转换后的 OpenAI 消息中 tool_call_id 双向一致。
/// 移除孤立的 tool result 消息（其 tool_call_id 在任何 assistant tool_calls 中无对应项），
/// 以及移除 assistant 消息中无对应 tool result 的 tool_call 条目。
fn sanitize_openai_messages(messages: &mut Vec<ChatCompletionRequestMessage>) {
    // 1. 收集所有 assistant 消息中的 tool_call id
    let assistant_tc_ids: std::collections::HashSet<String> = messages
        .iter()
        .filter_map(|m| {
            if let ChatCompletionRequestMessage::Assistant(a) = m {
                Some(a)
            } else {
                None
            }
        })
        .flat_map(|a| {
            a.tool_calls.iter().flatten().filter_map(|tc| match tc {
                ChatCompletionMessageToolCalls::Function(f) => Some(f.id.clone()),
                _ => None,
            })
        })
        .filter(|id| !id.is_empty())
        .collect();

    // 2. 收集所有 tool result 消息中的 tool_call_id
    let tool_result_ids: std::collections::HashSet<String> = messages
        .iter()
        .filter_map(|m| {
            if let ChatCompletionRequestMessage::Tool(t) = m {
                Some(t.tool_call_id.clone())
            } else {
                None
            }
        })
        .filter(|id| !id.is_empty())
        .collect();

    let original_len = messages.len();

    // 3. 移除孤立的 tool result（tool_call_id 不在 assistant tool_calls 中）
    messages.retain(|m| {
        if let ChatCompletionRequestMessage::Tool(t) = m
            && !assistant_tc_ids.contains(&t.tool_call_id)
        {
            write_error_log(
                "sanitize_openai_messages",
                &format!(
                    "移除孤立 tool result (tool_call_id={})：在 assistant tool_calls 中无对应项",
                    t.tool_call_id
                ),
            );
            return false;
        }
        true
    });

    // 4. 清理 assistant 消息中无对应 tool result 的 tool_call 条目
    for msg in messages.iter_mut() {
        if let ChatCompletionRequestMessage::Assistant(a) = msg
            && let Some(ref mut tcs) = a.tool_calls
        {
            let before = tcs.len();
            tcs.retain(|tc| match tc {
                ChatCompletionMessageToolCalls::Function(f) => {
                    f.id.is_empty() || tool_result_ids.contains(&f.id)
                }
                _ => true,
            });
            if tcs.len() != before {
                write_error_log(
                    "sanitize_openai_messages",
                    &format!(
                        "assistant tool_calls 中 {} 个条目无对应 tool result，已移除",
                        before - tcs.len()
                    ),
                );
            }
            if tcs.is_empty() {
                a.tool_calls = None;
            }
        }
    }

    let removed = original_len - messages.len();
    if removed > 0 {
        write_info_log(
            "sanitize_openai_messages",
            &format!("后置验证：共移除 {} 条孤立消息", removed),
        );
    }
}

/// 构建带工具定义的请求
pub fn build_request_with_tools(
    provider: &ModelProvider,
    messages: &[ChatMessage],
    tools: Vec<ChatCompletionTools>,
    system_prompt: Option<&str>,
) -> Result<CreateChatCompletionRequest, ChatError> {
    let sanitized = sanitize_messages(messages);
    let mut openai_messages = Vec::with_capacity(sanitized.len());
    if let Some(sys) = system_prompt {
        let trimmed = sys.trim();
        if !trimmed.is_empty()
            && let Ok(msg) = ChatCompletionRequestSystemMessageArgs::default()
                .content(trimmed)
                .build()
        {
            openai_messages.push(ChatCompletionRequestMessage::System(msg));
        }
    }
    openai_messages.extend(to_openai_messages(&sanitized));

    // ── 后置验证：确保转换后的消息中 tool_call_id 双向一致 ──
    // to_openai_messages 可能通过 .build().ok() 静默丢弃某些消息，
    // 导致 assistant tool_calls ↔ tool result 不再配对，API 报 "tool_call_id not found"。
    sanitize_openai_messages(&mut openai_messages);

    let mut builder = CreateChatCompletionRequestArgs::default();
    builder.model(&provider.model).messages(openai_messages);
    let tools_count = tools.len();
    if !tools.is_empty() {
        builder.tools(tools);
    }
    builder.build().map_err(|e| {
        let err_msg = format!("构建请求失败: {}", e);
        let params_info = format!(
            "入参信息:\n  model: {}\n  api_base: {}\n  messages数量: {}\n  tools数量: {}\n  system_prompt: {:?}",
            provider.model, provider.api_base, sanitized.len(), tools_count, system_prompt
        );
        write_info_log("build_request_with_tools ERROR", &format!("{}\n{}", err_msg, params_info));
        ChatError::RequestBuild(e.to_string())
    })
}

/// 使用 async-openai 流式调用 API，通过回调逐步输出
/// 返回完整的助手回复内容
pub async fn call_openai_stream_async(
    provider: &ModelProvider,
    messages: &[ChatMessage],
    system_prompt: Option<&str>,
    on_chunk: &mut dyn FnMut(&str),
) -> Result<String, ChatError> {
    let client = create_openai_client(provider);
    let mut openai_messages = Vec::with_capacity(messages.len());

    if let Some(sys) = system_prompt {
        let trimmed = sys.trim();
        if !trimmed.is_empty()
            && let Ok(msg) = ChatCompletionRequestSystemMessageArgs::default()
                .content(trimmed)
                .build()
        {
            openai_messages.push(ChatCompletionRequestMessage::System(msg));
        }
    }
    openai_messages.extend(to_openai_messages(messages));

    let request = CreateChatCompletionRequestArgs::default()
        .model(&provider.model)
        .messages(openai_messages)
        .build()
        .map_err(|e| ChatError::RequestBuild(e.to_string()))?;

    // 在 request 被 move 之前，序列化完整的 request body 用于错误日志
    let request_body =
        serde_json::to_string(&request).unwrap_or_else(|e| format!("序列化request失败: {}", e));

    let mut stream = client.chat().create_stream(request).await.map_err(|e| {
        let err_msg = ChatError::from(e);
        write_info_log(
            "call_openai_stream_async API请求 ERROR",
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
                    "call_openai_stream_async 流式响应 ERROR",
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

// ==================== 宽松反序列化结构（兼容非标准 finish_reason）====================

/// 宽松版 tool call function
#[derive(Debug, Deserialize)]
pub struct LenientFunctionCall {
    pub name: String,
    pub arguments: String,
}

/// 宽松版 tool call
#[derive(Debug, Deserialize)]
pub struct LenientToolCall {
    pub id: String,
    pub function: LenientFunctionCall,
}

/// 宽松版 choice message
#[derive(Debug, Deserialize)]
pub struct LenientMessage {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<LenientToolCall>>,
}

/// 宽松版 choice —— finish_reason 用 String 接收，兼容任意非标准值
#[derive(Debug, Deserialize)]
pub struct LenientChoice {
    pub message: LenientMessage,
    pub finish_reason: Option<String>,
}

/// 宽松版 API 响应
#[derive(Debug, Deserialize)]
pub struct LenientChatResponse {
    pub choices: Vec<LenientChoice>,
}

/// fallback 非流式调用结果
#[derive(Debug)]
pub struct FallbackResult {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCallItem>>,
    pub is_tool_calls: bool,
    pub finish_reason: Option<String>,
}

/// 使用 reqwest 发送非流式请求，用宽松结构反序列化，兼容非标准 finish_reason
pub async fn call_openai_non_stream_lenient(
    provider: &ModelProvider,
    request: &CreateChatCompletionRequest,
) -> Result<FallbackResult, ChatError> {
    let url = format!(
        "{}/chat/completions",
        provider.api_base.trim_end_matches('/')
    );
    let request_body =
        serde_json::to_string(request).unwrap_or_else(|e| format!("序列化request失败: {}", e));

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", provider.api_key))
        .body(request_body.clone())
        .send()
        .await
        .map_err(|e| {
            let err = ChatError::from(e);
            write_error_log(
                "call_openai_non_stream_lenient HTTP",
                &format!("{}\nrequest body:\n{}", err, request_body),
            );
            err
        })?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| ChatError::Other(format!("读取响应 body 失败: {}", e)))?;

    if !status.is_success() {
        let err = ChatError::from_http_status(status.as_u16(), sanitize_api_body(&body));
        write_error_log(
            "call_openai_non_stream_lenient HTTP status",
            &format!("{}\nrequest body:\n{}", err, request_body),
        );
        return Err(err);
    }

    let parsed: LenientChatResponse =
        serde_json::from_str(&body).map_err(|e| ChatError::StreamDeserialize(format!("{}", e)))?;

    let choice = match parsed.choices.first() {
        Some(c) => c,
        None => {
            return Ok(FallbackResult {
                content: None,
                tool_calls: None,
                is_tool_calls: false,
                finish_reason: None,
            });
        }
    };

    let is_tool_calls = choice
        .finish_reason
        .as_deref()
        .is_some_and(|r| r == "tool_calls");

    let tool_items = choice.message.tool_calls.as_ref().map(|tcs| {
        tcs.iter()
            .map(|tc| {
                // 与流式路径保持一致：API 未返回 id 时生成随机 id，避免下一轮报 tool_call_id not found
                let id = if tc.id.is_empty() {
                    use rand::Rng;
                    let rand_id = format!("call_{:016x}", rand::thread_rng().r#gen::<u64>());
                    write_info_log(
                        "call_openai_non_stream_lenient",
                        &format!(
                            "tool_call id 为空，已生成随机 id: {} (tool: {})",
                            rand_id, tc.function.name
                        ),
                    );
                    rand_id
                } else {
                    tc.id.clone()
                };
                ToolCallItem {
                    id,
                    name: tc.function.name.clone(),
                    arguments: tc.function.arguments.clone(),
                }
            })
            .collect()
    });

    // 如果 finish_reason 是非标准值（如 network_error），记录警告日志
    if let Some(ref reason) = choice.finish_reason
        && !matches!(
            reason.as_str(),
            "stop" | "length" | "tool_calls" | "content_filter" | "function_call"
        )
    {
        write_info_log(
            "call_openai_non_stream_lenient",
            &format!("非标准 finish_reason: {}", reason),
        );
    }

    Ok(FallbackResult {
        content: choice.message.content.clone(),
        tool_calls: tool_items,
        is_tool_calls,
        finish_reason: choice.finish_reason.clone(),
    })
}

/// 同步包装：创建 tokio runtime 执行异步流式调用
pub fn call_openai_stream(
    provider: &ModelProvider,
    messages: &[ChatMessage],
    system_prompt: Option<&str>,
    on_chunk: &mut dyn FnMut(&str),
) -> Result<String, ChatError> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        let err = ChatError::RuntimeFailed(e.to_string());
        let params_info = format!(
            "入参信息:\n  model: {}\n  api_base: {}\n  messages数量: {}\n  system_prompt: {:?}",
            provider.model,
            provider.api_base,
            messages.len(),
            system_prompt
        );
        write_info_log(
            "call_openai_stream 创建runtime ERROR",
            &format!("{}\n{}", err, params_info),
        );
        err
    })?;
    rt.block_on(call_openai_stream_async(
        provider,
        messages,
        system_prompt,
        on_chunk,
    ))
}

/// 清理 API 响应 body 用于错误消息：剥离 HTML 标签，截断超长内容
fn sanitize_api_body(body: &str) -> String {
    let max_len = crate::command::chat::constants::API_ERROR_BODY_MAX_LEN;
    let truncated = &body[..body.len().min(max_len)];
    // 剥离 HTML 标签
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
