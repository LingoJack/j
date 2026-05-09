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

    /// Streaming chat completion — returns SSE stream.
    pub async fn chat_completion_stream(
        &self,
        request: &ChatRequest,
    ) -> Result<SseStream, LlmError> {
        // Ensure stream is set to true in the serialized body
        let mut body = serde_json::to_value(request)
            .map_err(|e| LlmError::RequestBuild(format!("Failed to serialize request: {}", e)))?;
        // SAFETY: `ChatRequest` always serializes as a JSON object (struct with named
        // fields), so `serde_json::to_value(request)` is guaranteed to produce a Value::Object.
        body.as_object_mut()
            .expect("ChatRequest must serialize to a JSON object")
            .insert("stream".to_string(), serde_json::Value::Bool(true));

        let resp = self
            .http_client
            .post(self.endpoint())
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .body(body.to_string())
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
