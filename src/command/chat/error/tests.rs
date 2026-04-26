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
        "429 应映射为 ApiRateLimit {{ retry_after_secs: None }}"
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
