/// LLM client errors (lower level than ChatError)
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    /// HTTP-level errors (reqwest)
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    /// Non-success HTTP status with body
    #[error("API error ({status}): {body}")]
    Api { status: u16, body: String },
    /// JSON deserialization failures
    #[error("Deserialize error: {0}")]
    Deserialize(String),
    /// SSE stream interrupted
    #[error("Stream interrupted: {0}")]
    StreamInterrupted(String),
    /// Request build error
    #[error("Request build error: {0}")]
    RequestBuild(String),
}

impl From<serde_json::Error> for LlmError {
    fn from(e: serde_json::Error) -> Self {
        LlmError::Deserialize(e.to_string())
    }
}
