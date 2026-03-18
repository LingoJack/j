use super::storage::{ChatMessage, ModelProvider};
use crate::command::chat::constants;
use crate::util::log::write_info_log;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionTools, CreateChatCompletionRequest,
        CreateChatCompletionRequestArgs, FunctionCall,
    },
};
use constants::{ROLE_ASSISTANT, ROLE_SYSTEM, ROLE_TOOL, ROLE_USER};
use futures::StreamExt;

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
            ROLE_USER => ChatCompletionRequestUserMessageArgs::default()
                .content(msg.content.as_str())
                .build()
                .ok()
                .map(ChatCompletionRequestMessage::User),
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

/// 构建带工具定义的请求
pub fn build_request_with_tools(
    provider: &ModelProvider,
    messages: &[ChatMessage],
    tools: Vec<ChatCompletionTools>,
    system_prompt: Option<&str>,
) -> Result<CreateChatCompletionRequest, String> {
    let mut openai_messages = Vec::new();
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
            provider.model, provider.api_base, messages.len(), tools_count, system_prompt
        );
        write_info_log("build_request_with_tools ERROR", &format!("{}\n{}", err_msg, params_info));
        err_msg
    })
}

/// 使用 async-openai 流式调用 API，通过回调逐步输出
/// 返回完整的助手回复内容
pub async fn call_openai_stream_async(
    provider: &ModelProvider,
    messages: &[ChatMessage],
    system_prompt: Option<&str>,
    on_chunk: &mut dyn FnMut(&str),
) -> Result<String, String> {
    let client = create_openai_client(provider);
    let mut openai_messages = Vec::new();
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
        .map_err(|e| {
            let err_msg = format!("构建请求失败: {}", e);
            let params_info = format!(
                "入参信息:\n  model: {}\n  api_base: {}\n  messages数量: {}\n  system_prompt: {:?}",
                provider.model,
                provider.api_base,
                messages.len(),
                system_prompt
            );
            write_info_log(
                "call_openai_stream_async 构建请求 ERROR",
                &format!("{}\n{}", err_msg, params_info),
            );
            err_msg
        })?;

    // 在 request 被 move 之前，序列化完整的 request body 用于错误日志
    let request_body =
        serde_json::to_string(&request).unwrap_or_else(|e| format!("序列化request失败: {}", e));

    let mut stream = client.chat().create_stream(request).await.map_err(|e| {
        let err_msg = format!("API 请求失败: {}", e);
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
                let err_msg = format!("流式响应错误: {}", e);
                write_info_log(
                    "call_openai_stream_async 流式响应 ERROR",
                    &format!(
                        "{}\n已接收内容长度: {}\nrequest body:\n{}",
                        err_msg,
                        full_content.len(),
                        request_body
                    ),
                );
                return Err(err_msg);
            }
        }
    }

    Ok(full_content)
}

/// 同步包装：创建 tokio runtime 执行异步流式调用
pub fn call_openai_stream(
    provider: &ModelProvider,
    messages: &[ChatMessage],
    system_prompt: Option<&str>,
    on_chunk: &mut dyn FnMut(&str),
) -> Result<String, String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| {
        let err_msg = format!("创建异步运行时失败: {}", e);
        let params_info = format!(
            "入参信息:\n  model: {}\n  api_base: {}\n  messages数量: {}\n  system_prompt: {:?}",
            provider.model,
            provider.api_base,
            messages.len(),
            system_prompt
        );
        write_info_log(
            "call_openai_stream 创建runtime ERROR",
            &format!("{}\n{}", err_msg, params_info),
        );
        err_msg
    })?;
    rt.block_on(call_openai_stream_async(
        provider,
        messages,
        system_prompt,
        on_chunk,
    ))
}
