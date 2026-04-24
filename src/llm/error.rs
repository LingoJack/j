use std::fmt;

/// LLM client errors (lower level than ChatError)
#[derive(Debug)]
pub enum LlmError {
    /// HTTP-level errors (reqwest)
    Http(reqwest::Error),
    /// Non-success HTTP status with body
    Api { status: u16, body: String },
    /// JSON deserialization failures
    Deserialize(String),
    /// SSE stream interrupted
    StreamInterrupted(String),
    /// Request build error
    RequestBuild(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::Http(e) => write!(f, "HTTP error: {}", e),
            LlmError::Api { status, body } => write!(f, "API error ({}): {}", status, body),
            LlmError::Deserialize(msg) => write!(f, "Deserialize error: {}", msg),
            LlmError::StreamInterrupted(msg) => write!(f, "Stream interrupted: {}", msg),
            LlmError::RequestBuild(msg) => write!(f, "Request build error: {}", msg),
        }
    }
}

impl std::error::Error for LlmError {}

impl From<reqwest::Error> for LlmError {
    fn from(e: reqwest::Error) -> Self {
        LlmError::Http(e)
    }
}

impl From<serde_json::Error> for LlmError {
    fn from(e: serde_json::Error) -> Self {
        LlmError::Deserialize(e.to_string())
    }
}
