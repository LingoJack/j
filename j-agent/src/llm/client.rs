use super::error::LlmError;
use super::stream::SseStream;
use super::types::{ChatRequest, ChatResponse};

/// OpenAI-compatible Chat Completions client backed by reqwest.
#[derive(Debug)]
pub struct LlmClient {
    http_client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl LlmClient {
    /// Create a new LLM client with the given API base URL and API key.
    pub fn new(api_base: &str, api_key: &str) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url: api_base.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        }
    }

    const CHAT_COMPLETIONS_PATH: &'static str = "/chat/completions";

    fn endpoint(&self) -> String {
        format!("{}{}", self.base_url, Self::CHAT_COMPLETIONS_PATH)
    }

    /// Non-streaming chat completion.
    pub async fn chat_completion(&self, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
        let resp = self
            .http_client
            .post(self.endpoint())
            .bearer_auth(&self.api_key)
            .json(request)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Api {
                status: status.as_u16(),
                body,
            });
        }

        let body = resp.text().await?;
        serde_json::from_str::<ChatResponse>(&body).map_err(|e| {
            LlmError::Deserialize(format!("Failed to parse response: {} | body: {}", e, body))
        })
    }

    /// Build the JSON body used for streaming chat completion requests.
    ///
    /// This is shared by the request sender and error logging so diagnostics match the
    /// exact payload sent to the OpenAI-compatible endpoint.
    pub fn stream_request_body(&self, request: &ChatRequest) -> Result<String, LlmError> {
        let mut body = serde_json::to_value(request)
            .map_err(|e| LlmError::RequestBuild(format!("Failed to serialize request: {}", e)))?;
        let Some(obj) = body.as_object_mut() else {
            return Err(LlmError::RequestBuild(
                "ChatRequest must serialize to a JSON object".to_string(),
            ));
        };
        obj.insert("stream".to_string(), serde_json::Value::Bool(true));
        Ok(body.to_string())
    }

    /// Streaming chat completion — returns SSE stream.
    pub async fn chat_completion_stream(
        &self,
        request: &ChatRequest,
    ) -> Result<SseStream, LlmError> {
        let body = self.stream_request_body(request)?;

        let resp = self
            .http_client
            .post(self.endpoint())
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let resp_body = resp.text().await.unwrap_or_default();
            return Err(LlmError::Api {
                status: status.as_u16(),
                body: resp_body,
            });
        }

        Ok(SseStream::new(resp))
    }
}
