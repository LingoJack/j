//! Chat 模块类型化错误
//!
//! 所有 chat 核心链路中的错误统一使用 `ChatError`，按语义分类，
//! 便于 UI 层给出差异化提示（认证失败 vs 网络超时 vs 服务端错误等）。

use crate::constants::{
    TEAMMATE_LOG_RESULT_MAX_CHARS, TEAMMATE_PROMPT_PREVIEW_MAX_CHARS,
};
use crate::llm::LlmError;

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
                format!(
                    "流式响应中断: {}",
                    truncate(&sanitize_html(msg), TEAMMATE_PROMPT_PREVIEW_MAX_CHARS)
                )
            }
            Self::StreamDeserialize(msg) => {
                format!(
                    "响应解析失败: {}",
                    truncate(&sanitize_html(msg), TEAMMATE_PROMPT_PREVIEW_MAX_CHARS)
                )
            }
            Self::RequestBuild(msg) => {
                format!("构建请求失败: {}", truncate(msg, 150))
            }
            Self::HookAborted => "请求被 hook 中止".to_string(),
            Self::RuntimeFailed(msg) => {
                format!(
                    "运行时错误: {}",
                    truncate(msg, TEAMMATE_PROMPT_PREVIEW_MAX_CHARS)
                )
            }
            Self::AgentPanic(msg) => {
                format!(
                    "Agent 异常: {}",
                    truncate(msg, TEAMMATE_PROMPT_PREVIEW_MAX_CHARS)
                )
            }
            Self::AbnormalFinish(reason) => {
                format!("API 返回异常: finish_reason={}", truncate(reason, 80))
            }
            Self::Other(msg) => truncate(&sanitize_html(msg), TEAMMATE_LOG_RESULT_MAX_CHARS),
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

// ── 从 LlmError 转换 ──

impl From<LlmError> for ChatError {
    fn from(e: LlmError) -> Self {
        match e {
            LlmError::Http(re) => ChatError::from(re),
            LlmError::Api { status, body } => {
                // 尝试从 body 中解析 { "error": { "code": ..., "message": ... } }
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&body) {
                    let error_obj = parsed.get("error").unwrap_or(&parsed);
                    let code = error_obj.get("code").and_then(|v| {
                        v.as_str()
                            .map(|s| s.to_string())
                            .or_else(|| v.as_i64().map(|n| n.to_string()))
                    });
                    let message = error_obj
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&body);
                    ChatError::from_api_error(code.as_deref(), message)
                } else {
                    ChatError::from_http_status(status, sanitize_html(&body))
                }
            }
            LlmError::Deserialize(msg) => ChatError::StreamDeserialize(truncate(&msg, 500)),
            LlmError::StreamInterrupted(msg) => ChatError::StreamInterrupted(msg),
            LlmError::RequestBuild(msg) => ChatError::RequestBuild(msg),
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

    /// 从结构化 API 错误（code + message）转换
    fn from_api_error(code: Option<&str>, message: &str) -> Self {
        match code {
            Some("rate_limit_exceeded") => ChatError::ApiRateLimit {
                message: message.to_string(),
                retry_after_secs: None,
            },
            Some("invalid_api_key") | Some("authentication_required") => {
                ChatError::ApiAuth(message.to_string())
            }
            Some("invalid_request_error") => ChatError::ApiBadRequest(message.to_string()),
            Some("1305") => ChatError::ApiRateLimit {
                message: message.to_string(),
                retry_after_secs: None,
            },
            Some(code) if code.contains("rpm_exceeded") || code.contains("tpm_exceeded") => {
                ChatError::ApiRateLimit {
                    message: message.to_string(),
                    retry_after_secs: None,
                }
            }
            _ => {
                let msg_lower = message.to_lowercase();
                if msg_lower.contains("api key")
                    || msg_lower.contains("unauthorized")
                    || msg_lower.contains("authentication")
                {
                    ChatError::ApiAuth(message.to_string())
                } else if msg_lower.contains("rate limit")
                    || msg_lower.contains("too many requests")
                    || msg_lower.contains("访问量过大")
                    || msg_lower.contains("请稍后再试")
                    || msg_lower.contains("过载")
                    || msg_lower.contains("overloaded")
                    || msg_lower.contains("too busy")
                    || msg_lower.contains("速率限制")
                    || msg_lower.contains("网络错误")
                    || msg_lower.contains("quota exceeded")
                    || msg_lower.contains("concurrency limit")
                    || msg_lower.contains("请求频率")
                    || msg_lower.contains("busy")
                    || msg_lower.contains("rpm_exceeded")
                    || msg_lower.contains("rpm exceeded")
                    || msg_lower.contains("tpm_exceeded")
                    || msg_lower.contains("tpm exceeded")
                    || msg_lower.contains("channel:rpm")
                    || msg_lower.contains("channel:tpm")
                {
                    ChatError::ApiRateLimit {
                        message: message.to_string(),
                        retry_after_secs: None,
                    }
                } else if msg_lower.contains("invalid") || msg_lower.contains("bad request") {
                    ChatError::ApiBadRequest(message.to_string())
                } else {
                    ChatError::Other(sanitize_html(message))
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
            '>' => {
                in_tag = false;
                // 标签闭合时插入空格，以便相邻标签内容之间有分隔
                if !result.is_empty() && !result.ends_with(char::is_whitespace) {
                    result.push(' ');
                }
            }
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
mod tests;
