//! Chat 模块类型化错误
//!
//! 所有 chat 核心链路中的错误统一使用 `ChatError`，按语义分类，
//! 便于 UI 层给出差异化提示（认证失败 vs 网络超时 vs 服务端错误等）。

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
        assert_eq!(truncate("hello world", 8), "hello wo...");
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

    // ════════════════════════════════════════════════════════════════
    // 回归测试：from_http_status 映射
    // 如果以下测试失败，说明 HTTP 状态码到 ChatError 变体的映射被意外修改
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn from_http_status_401_is_auth() {
        let err = ChatError::from_http_status(401, "unauthorized".into());
        assert!(
            matches!(err, ChatError::ApiAuth(msg) if msg == "unauthorized"),
            "401 应映射为 ApiAuth"
        );
    }

    #[test]
    fn from_http_status_403_is_auth() {
        let err = ChatError::from_http_status(403, "forbidden".into());
        assert!(
            matches!(err, ChatError::ApiAuth(msg) if msg == "forbidden"),
            "403 应映射为 ApiAuth"
        );
    }

    #[test]
    fn from_http_status_429_is_rate_limit() {
        let err = ChatError::from_http_status(429, "slow down".into());
        assert!(
            matches!(
                err,
                ChatError::ApiRateLimit {
                    retry_after_secs: None,
                    ..
                }
            ),
            "429 应映射为 ApiRateLimit { retry_after_secs: None }"
        );
    }

    #[test]
    fn from_http_status_400_is_bad_request() {
        let err = ChatError::from_http_status(400, "bad".into());
        assert!(
            matches!(err, ChatError::ApiBadRequest(msg) if msg == "bad"),
            "400 应映射为 ApiBadRequest"
        );
    }

    #[test]
    fn from_http_status_5xx_are_server_error() {
        for status in [500, 502, 503, 504, 529] {
            let err = ChatError::from_http_status(status, "err".into());
            assert!(
                matches!(err, ChatError::ApiServerError { status: s, .. } if s == status),
                "{status} 应映射为 ApiServerError {{ status: {status} }}"
            );
        }
    }

    #[test]
    fn from_http_status_unknown_is_other() {
        let err = ChatError::from_http_status(418, "I'm a teapot".into());
        assert!(
            matches!(err, ChatError::Other(_)),
            "未知状态码应映射为 Other"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 回归测试：from_api_error code + message 映射
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn from_api_error_rate_limit_code() {
        let err = ChatError::from_api_error(Some("rate_limit_exceeded"), "msg");
        assert!(
            matches!(
                err,
                ChatError::ApiRateLimit {
                    retry_after_secs: None,
                    ..
                }
            ),
            "rate_limit_exceeded → ApiRateLimit"
        );
    }

    #[test]
    fn from_api_error_auth_codes() {
        for code in ["invalid_api_key", "authentication_required"] {
            let err = ChatError::from_api_error(Some(code), "msg");
            assert!(matches!(err, ChatError::ApiAuth(_)), "{code} → ApiAuth");
        }
    }

    #[test]
    fn from_api_error_bad_request_code() {
        let err = ChatError::from_api_error(Some("invalid_request_error"), "msg");
        assert!(
            matches!(err, ChatError::ApiBadRequest(_)),
            "invalid_request_error → ApiBadRequest"
        );
    }

    #[test]
    fn from_api_error_code_1305() {
        let err = ChatError::from_api_error(Some("1305"), "msg");
        assert!(
            matches!(err, ChatError::ApiRateLimit { .. }),
            "1305 → ApiRateLimit"
        );
    }

    #[test]
    fn from_api_error_message_auth_heuristics() {
        let auth_keywords = [
            "Invalid API key provided",
            "Unauthorized access",
            "Authentication failed",
        ];
        for kw in auth_keywords {
            let err = ChatError::from_api_error(None, kw);
            assert!(
                matches!(err, ChatError::ApiAuth(_)),
                "message='{kw}' 应通过启发式识别为 ApiAuth"
            );
        }
    }

    #[test]
    fn from_api_error_message_rate_limit_heuristics() {
        let rate_keywords = [
            "Rate limit exceeded",
            "Too many requests",
            "访问量过大，请稍后",
            "请稍后再试",
            "服务过载",
            "server overloaded",
            "server too busy",
            "速率限制",
            "网络错误",
            "quota exceeded",
            "concurrency limit reached",
            "请求频率过高",
            "busy",
        ];
        for kw in rate_keywords {
            let err = ChatError::from_api_error(None, kw);
            assert!(
                matches!(err, ChatError::ApiRateLimit { .. }),
                "message='{kw}' 应通过启发式识别为 ApiRateLimit"
            );
        }
    }

    #[test]
    fn from_api_error_message_bad_request_heuristics() {
        let bad_keywords = ["Invalid parameter", "Bad request format"];
        for kw in bad_keywords {
            let err = ChatError::from_api_error(None, kw);
            assert!(
                matches!(err, ChatError::ApiBadRequest(_)),
                "message='{kw}' 应通过启发式识别为 ApiBadRequest"
            );
        }
    }

    #[test]
    fn from_api_error_fallback_to_other() {
        let err = ChatError::from_api_error(None, "something went wrong");
        assert!(
            matches!(err, ChatError::Other(_)),
            "无法识别的错误应兜底为 Other"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 回归测试：From<LlmError> 转换
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn from_llm_error_deserialize() {
        let llm = LlmError::Deserialize("bad json".into());
        let chat: ChatError = llm.into();
        assert!(
            matches!(chat, ChatError::StreamDeserialize(msg) if msg.contains("bad json")),
            "LlmError::Deserialize → ChatError::StreamDeserialize"
        );
    }

    #[test]
    fn from_llm_error_stream_interrupted() {
        let llm = LlmError::StreamInterrupted("disconnected".into());
        let chat: ChatError = llm.into();
        assert!(
            matches!(chat, ChatError::StreamInterrupted(msg) if msg.contains("disconnected")),
            "LlmError::StreamInterrupted → ChatError::StreamInterrupted"
        );
    }

    #[test]
    fn from_llm_error_request_build() {
        let llm = LlmError::RequestBuild("bad args".into());
        let chat: ChatError = llm.into();
        assert!(
            matches!(chat, ChatError::RequestBuild(msg) if msg.contains("bad args")),
            "LlmError::RequestBuild → ChatError::RequestBuild"
        );
    }

    #[test]
    fn from_llm_error_api_with_structured_body() {
        // 模拟 { "error": { "code": "rate_limit_exceeded", "message": "slow down" } }
        let body = r#"{"error":{"code":"rate_limit_exceeded","message":"slow down"}}"#;
        let llm = LlmError::Api {
            status: 429,
            body: body.to_string(),
        };
        let chat: ChatError = llm.into();
        assert!(
            matches!(chat, ChatError::ApiRateLimit { .. }),
            "LlmError::Api with rate_limit_exceeded code → ApiRateLimit"
        );
    }

    #[test]
    fn from_llm_error_api_with_unparseable_body() {
        let llm = LlmError::Api {
            status: 500,
            body: "not json".to_string(),
        };
        let chat: ChatError = llm.into();
        assert!(
            matches!(chat, ChatError::ApiServerError { status: 500, .. }),
            "LlmError::Api with unparseable body → ApiServerError"
        );
    }

    // ════════════════════════════════════════════════════════════════
    // 回归测试：display_message 所有变体不 panic 且有合理输出
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn display_message_all_variants() {
        let variants: Vec<ChatError> = vec![
            ChatError::ApiAuth("bad key".into()),
            ChatError::ApiRateLimit {
                message: "msg".into(),
                retry_after_secs: None,
            },
            ChatError::ApiRateLimit {
                message: "msg".into(),
                retry_after_secs: Some(60),
            },
            ChatError::ApiBadRequest("param".into()),
            ChatError::ApiServerError {
                status: 500,
                message: "err".into(),
            },
            ChatError::ApiServerError {
                status: 503,
                message: "err".into(),
            },
            ChatError::NetworkTimeout("timeout".into()),
            ChatError::NetworkError("dns".into()),
            ChatError::StreamInterrupted("msg".into()),
            ChatError::StreamDeserialize("json".into()),
            ChatError::RequestBuild("args".into()),
            ChatError::HookAborted,
            ChatError::RuntimeFailed("err".into()),
            ChatError::AgentPanic("panic".into()),
            ChatError::AbnormalFinish("reason".into()),
            ChatError::Other("unknown".into()),
        ];
        for err in &variants {
            let msg = err.display_message();
            assert!(!msg.is_empty(), "{err:?} 的 display_message 不应为空");
        }
    }

    // ════════════════════════════════════════════════════════════════
    // 回归测试：sanitize_html 边界条件
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn sanitize_html_empty() {
        assert_eq!(sanitize_html(""), "");
    }

    #[test]
    fn sanitize_html_nested_tags() {
        assert_eq!(sanitize_html("<div><p><span>deep</span></p></div>"), "deep");
    }

    #[test]
    fn sanitize_html_unclosed_tag() {
        // 未闭合标签：内容在 > 之前不会被输出
        assert_eq!(sanitize_html("before <div>after"), "before after");
    }

    #[test]
    fn sanitize_html_adjacent_tags() {
        // 相邻标签内容之间应有空格
        assert_eq!(sanitize_html("<b>bold</b><i>italic</i>"), "bold italic");
    }

    // ════════════════════════════════════════════════════════════════
    // 回归测试：truncate UTF-8 边界安全
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn truncate_short_input_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_exact_boundary() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_multibyte_safe() {
        // "你好世界" 每个字符 3 字节，在字节边界 7 处应回退到 6（"你好"）
        let input = "你好世界";
        let result = truncate(input, 7);
        assert!(
            result.contains("你好"),
            "UTF-8 截断应在字符边界处：{result}"
        );
    }

    #[test]
    fn truncate_adds_ellipsis() {
        let result = truncate("hello world", 8);
        assert!(result.ends_with("..."), "截断后应添加省略号：{result}");
    }

    // ════════════════════════════════════════════════════════════════
    // 回归测试：http_status_label 覆盖
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn http_status_label_known_codes() {
        assert_eq!(http_status_label(500), "服务端内部错误");
        assert_eq!(http_status_label(502), "网关错误");
        assert_eq!(http_status_label(503), "服务暂不可用");
        assert_eq!(http_status_label(504), "网关超时");
        assert_eq!(http_status_label(529), "服务过载");
        assert_eq!(http_status_label(599), "服务端错误"); // fallback
    }
}
