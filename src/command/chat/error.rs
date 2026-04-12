//! Chat 模块类型化错误
//!
//! 所有 chat 核心链路中的错误统一使用 `ChatError`，按语义分类，
//! 便于 UI 层给出差异化提示（认证失败 vs 网络超时 vs 服务端错误等）。

use async_openai::error::{ApiError, OpenAIError};

/// Chat 模块类型化错误
#[derive(Debug)]
pub enum ChatError {
    // ── API 错误（按 HTTP 语义分类）──
    /// 401/403 认证/授权失败
    ApiAuth(String),
    /// 429 请求过于频繁
    ApiRateLimit {
        message: String,
        retry_after_secs: Option<u64>,
    },
    /// 400 请求参数错误
    ApiBadRequest(String),
    /// 5xx 服务端错误（504/502/500 等）
    ApiServerError { status: u16, message: String },

    // ── 网络错误 ──
    /// 连接超时
    NetworkTimeout(String),
    /// DNS/TLS/连接拒绝等
    NetworkError(String),

    // ── 流式错误 ──
    /// 流式响应中断（SSE 连接断开）
    StreamInterrupted(String),
    /// 流式反序列化失败（触发 fallback 重试）
    StreamDeserialize(String),

    // ── 请求构建 ──
    /// 构建请求失败（参数无效等）
    RequestBuild(String),

    // ── Hook ──
    /// 被 hook 中止
    HookAborted,

    // ── 运行时 ──
    /// Agent 运行时错误
    RuntimeFailed(String),
    /// Agent 线程 panic
    AgentPanic(String),

    // ── 其他 ──
    /// 异常 finish_reason（如 network_error）
    AbnormalFinish(String),
    /// 兜底
    Other(String),
}

impl ChatError {
    /// 是否可重试
    #[allow(dead_code)]
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ApiRateLimit { .. }
                | Self::ApiServerError { .. }
                | Self::NetworkTimeout(_)
                | Self::NetworkError(_)
                | Self::StreamInterrupted(_)
        )
    }

    /// 是否为认证错误
    #[allow(dead_code)]
    pub fn is_auth_error(&self) -> bool {
        matches!(self, Self::ApiAuth(_))
    }

    /// 清理后的用户可读消息（剥离 HTML，截断长度，提取状态码）
    pub fn display_message(&self) -> String {
        match self {
            Self::ApiAuth(_) => "API 认证失败，请检查 API Key".to_string(),
            Self::ApiRateLimit {
                retry_after_secs, ..
            } => {
                if let Some(secs) = retry_after_secs {
                    format!("请求过于频繁，请在 {} 秒后重试", secs)
                } else {
                    "请求过于频繁，请稍后重试".to_string()
                }
            }
            Self::ApiBadRequest(msg) => {
                format!("请求参数错误: {}", truncate(&sanitize_html(msg), 150))
            }
            Self::ApiServerError { status, .. } => {
                format!("{} (HTTP {})", http_status_label(*status), status)
            }
            Self::NetworkTimeout(_) => "网络连接超时，请检查网络后重试".to_string(),
            Self::NetworkError(_) => "网络连接失败，请检查网络设置".to_string(),
            Self::StreamInterrupted(msg) => {
                format!("流式响应中断: {}", truncate(&sanitize_html(msg), 100))
            }
            Self::StreamDeserialize(msg) => {
                format!("响应解析失败: {}", truncate(&sanitize_html(msg), 100))
            }
            Self::RequestBuild(msg) => {
                format!("构建请求失败: {}", truncate(msg, 150))
            }
            Self::HookAborted => "请求被 hook 中止".to_string(),
            Self::RuntimeFailed(msg) => {
                format!("运行时错误: {}", truncate(msg, 100))
            }
            Self::AgentPanic(msg) => {
                format!("Agent 异常: {}", truncate(msg, 100))
            }
            Self::AbnormalFinish(reason) => {
                format!("API 返回异常: finish_reason={}", truncate(reason, 80))
            }
            Self::Other(msg) => truncate(&sanitize_html(msg), 200),
        }
    }
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiAuth(msg) => write!(f, "API 认证失败: {}", msg),
            Self::ApiRateLimit { message, .. } => write!(f, "API 请求过于频繁: {}", message),
            Self::ApiBadRequest(msg) => write!(f, "API 请求参数错误: {}", msg),
            Self::ApiServerError { status, message } => {
                write!(f, "API 服务端错误 (HTTP {}): {}", status, message)
            }
            Self::NetworkTimeout(msg) => write!(f, "网络连接超时: {}", msg),
            Self::NetworkError(msg) => write!(f, "网络错误: {}", msg),
            Self::StreamInterrupted(msg) => write!(f, "流式响应中断: {}", msg),
            Self::StreamDeserialize(msg) => write!(f, "流式反序列化失败: {}", msg),
            Self::RequestBuild(msg) => write!(f, "构建请求失败: {}", msg),
            Self::HookAborted => write!(f, "LLM 请求被 hook 中止"),
            Self::RuntimeFailed(msg) => write!(f, "运行时错误: {}", msg),
            Self::AgentPanic(msg) => write!(f, "Agent 异常: {}", msg),
            Self::AbnormalFinish(reason) => write!(f, "API 返回异常: finish_reason={}", reason),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ChatError {}

// ── 从 OpenAIError 转换（保留结构化信息）──

impl From<OpenAIError> for ChatError {
    fn from(e: OpenAIError) -> Self {
        match e {
            OpenAIError::Reqwest(re) => {
                if re.is_timeout() {
                    ChatError::NetworkTimeout(re.to_string())
                } else if let Some(status) = re.status() {
                    ChatError::from_http_status(status.as_u16(), re.to_string())
                } else {
                    ChatError::NetworkError(re.to_string())
                }
            }
            OpenAIError::ApiError(api_err) => ChatError::from_api_error(api_err),
            OpenAIError::JSONDeserialize(_, content) => {
                ChatError::StreamDeserialize(truncate(&content, 500))
            }
            OpenAIError::StreamError(_) => ChatError::StreamInterrupted(e.to_string()),
            OpenAIError::InvalidArgument(msg) => ChatError::RequestBuild(msg),
            _ => ChatError::Other(e.to_string()),
        }
    }
}

// ── 从 reqwest::Error 转换 ──

impl From<reqwest::Error> for ChatError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            ChatError::NetworkTimeout(e.to_string())
        } else if let Some(status) = e.status() {
            ChatError::from_http_status(status.as_u16(), e.to_string())
        } else {
            ChatError::NetworkError(e.to_string())
        }
    }
}

impl ChatError {
    /// 根据 HTTP 状态码构造对应的 ChatError 变体
    pub(crate) fn from_http_status(status: u16, message: String) -> Self {
        match status {
            401 | 403 => ChatError::ApiAuth(message),
            429 => ChatError::ApiRateLimit {
                message,
                retry_after_secs: None,
            },
            400 => ChatError::ApiBadRequest(message),
            500..=599 => ChatError::ApiServerError {
                status,
                message: sanitize_html(&message),
            },
            _ => ChatError::Other(format!("HTTP {}: {}", status, message)),
        }
    }

    /// 从 async-openai 的 ApiError（结构化 JSON 错误）转换
    fn from_api_error(api_err: ApiError) -> Self {
        // 优先根据 code 分类（OpenAI 兼容 API 通常返回标准 code）
        match api_err.code.as_deref() {
            Some("rate_limit_exceeded") => ChatError::ApiRateLimit {
                message: api_err.message.clone(),
                retry_after_secs: None,
            },
            Some("invalid_api_key") | Some("authentication_required") => {
                ChatError::ApiAuth(api_err.message.clone())
            }
            Some("invalid_request_error") => ChatError::ApiBadRequest(api_err.message.clone()),
            _ => {
                // code 不明确时，尝试从 message 中推断
                let msg_lower = api_err.message.to_lowercase();
                if msg_lower.contains("api key")
                    || msg_lower.contains("unauthorized")
                    || msg_lower.contains("authentication")
                {
                    ChatError::ApiAuth(api_err.message)
                } else if msg_lower.contains("rate limit")
                    || msg_lower.contains("too many requests")
                {
                    ChatError::ApiRateLimit {
                        message: api_err.message,
                        retry_after_secs: None,
                    }
                } else if msg_lower.contains("invalid") || msg_lower.contains("bad request") {
                    ChatError::ApiBadRequest(api_err.message)
                } else {
                    // 无法分类，保留原始 message（但清理 HTML）
                    ChatError::Other(sanitize_html(&api_err.message))
                }
            }
        }
    }
}

// ── 工具函数 ──

/// 剥离 HTML 标签，保留纯文本内容
fn sanitize_html(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    // 清理多余空白
    let collapsed: String = result.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
}

/// 截断超长字符串
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    // 尝试在字符边界截断
    let mut end = max_len;
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

/// HTTP 状态码 → 中文标签
fn http_status_label(status: u16) -> &'static str {
    match status {
        500 => "服务端内部错误",
        502 => "网关错误",
        503 => "服务暂不可用",
        504 => "网关超时",
        529 => "服务过载",
        _ => "服务端错误",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_html() {
        assert_eq!(sanitize_html("<html><body>504</body></html>"), "504");
        assert_eq!(
            sanitize_html("<html><body>504 Gateway Timeout</body></html>"),
            "504 Gateway Timeout"
        );
        assert_eq!(sanitize_html("plain text"), "plain text");
        assert_eq!(sanitize_html("<div>hello</div><p>world</p>"), "hello world");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_display_message_504() {
        let err = ChatError::ApiServerError {
            status: 504,
            message: "<html><body>504 Gateway Timeout</body></html>".to_string(),
        };
        assert_eq!(err.display_message(), "网关超时 (HTTP 504)");
    }

    #[test]
    fn test_display_message_auth() {
        let err = ChatError::ApiAuth("invalid api key".to_string());
        assert_eq!(err.display_message(), "API 认证失败，请检查 API Key");
    }

    #[test]
    fn test_display_message_rate_limit() {
        let err = ChatError::ApiRateLimit {
            message: "too many requests".to_string(),
            retry_after_secs: Some(30),
        };
        assert_eq!(err.display_message(), "请求过于频繁，请在 30 秒后重试");
    }

    #[test]
    fn test_is_retryable() {
        assert!(
            ChatError::ApiServerError {
                status: 504,
                message: String::new()
            }
            .is_retryable()
        );
        assert!(ChatError::NetworkTimeout("".into()).is_retryable());
        assert!(!ChatError::ApiAuth("".into()).is_retryable());
        assert!(!ChatError::HookAborted.is_retryable());
    }
}
