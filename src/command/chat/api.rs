use super::model::{ChatMessage, ModelProvider};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
};
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
            "system" => ChatCompletionRequestSystemMessageArgs::default()
                .content(msg.content.as_str())
                .build()
                .ok()
                .map(ChatCompletionRequestMessage::System),
            "user" => ChatCompletionRequestUserMessageArgs::default()
                .content(msg.content.as_str())
                .build()
                .ok()
                .map(ChatCompletionRequestMessage::User),
            "assistant" => ChatCompletionRequestAssistantMessageArgs::default()
                .content(msg.content.as_str())
                .build()
                .ok()
                .map(ChatCompletionRequestMessage::Assistant),
            _ => None,
        })
        .collect()
}

/// 使用 async-openai 流式调用 API，通过回调逐步输出
/// 返回完整的助手回复内容
pub async fn call_openai_stream_async(
    provider: &ModelProvider,
    messages: &[ChatMessage],
    on_chunk: &mut dyn FnMut(&str),
) -> Result<String, String> {
    let client = create_openai_client(provider);
    let openai_messages = to_openai_messages(messages);

    let request = CreateChatCompletionRequestArgs::default()
        .model(&provider.model)
        .messages(openai_messages)
        .build()
        .map_err(|e| format!("构建请求失败: {}", e))?;

    let mut stream = client
        .chat()
        .create_stream(request)
        .await
        .map_err(|e| format!("API 请求失败: {}", e))?;

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
                return Err(format!("流式响应错误: {}", e));
            }
        }
    }

    Ok(full_content)
}

/// 同步包装：创建 tokio runtime 执行异步流式调用
pub fn call_openai_stream(
    provider: &ModelProvider,
    messages: &[ChatMessage],
    on_chunk: &mut dyn FnMut(&str),
) -> Result<String, String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("创建异步运行时失败: {}", e))?;
    rt.block_on(call_openai_stream_async(provider, messages, on_chunk))
}
