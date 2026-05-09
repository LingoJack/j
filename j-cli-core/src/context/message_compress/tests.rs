use super::*;

#[test]
fn test_extract_agent_source() {
    assert_eq!(
        extract_agent_source("<Teammate@Frontend> hello"),
        Some(("Teammate@Frontend".to_string(), " hello"))
    );
    assert_eq!(
        extract_agent_source("  <Teammate@Backend> [调用工具 Read]"),
        Some(("Teammate@Backend".to_string(), " [调用工具 Read]"))
    );
    assert_eq!(
        extract_agent_source("<SubAgent@search_auth> text"),
        Some(("SubAgent@search_auth".to_string(), " text"))
    );
    assert_eq!(extract_agent_source("no prefix"), None);
    assert_eq!(extract_agent_source("<no-close"), None);
}

#[test]
fn test_is_tool_call_broadcast() {
    use crate::tools::tool_names::{EDIT, READ};

    assert_eq!(
        is_tool_call_broadcast(&format!("<Teammate@Frontend> [调用工具 {}]", READ)),
        Some(("Teammate@Frontend".to_string(), READ.to_string()))
    );
    assert_eq!(
        is_tool_call_broadcast(&format!("<Teammate@Backend>  [调用工具 {}] ", EDIT)),
        Some(("Teammate@Backend".to_string(), EDIT.to_string()))
    );
    // 不是 tool call
    assert_eq!(
        is_tool_call_broadcast("<Teammate@Frontend> hello world"),
        None
    );
    // 不是广播格式
    assert_eq!(is_tool_call_broadcast("regular message"), None);
}

#[test]
fn test_compress_no_other_agent() {
    // 只有自己的消息，不压缩
    let messages = vec![
        ChatMessage::text(MessageRole::User, "user question".to_string()),
        ChatMessage::text(MessageRole::Assistant, "response".to_string()),
    ];
    let result = compress_other_agent_toolcalls(&messages, "Main", 5);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].content, "user question");
    assert_eq!(result[1].content, "response");
}

#[test]
fn test_compress_within_threshold() {
    // 其他 agent 的 tool call 数量 <= threshold，不压缩
    let messages = vec![
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Read]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Edit]".to_string(),
        ),
    ];
    let result = compress_other_agent_toolcalls(&messages, "Main", 5);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].content, "<Teammate@Frontend> [调用工具 Read]");
    assert_eq!(result[1].content, "<Teammate@Frontend> [调用工具 Edit]");
}

#[test]
fn test_compress_exceed_threshold() {
    // 其他 agent 的 tool call 数量 > threshold，压缩早期消息
    let messages = vec![
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Read]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Edit]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Bash]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Read]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Edit]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Bash]".to_string(),
        ),
    ];
    let result = compress_other_agent_toolcalls(&messages, "Main", 3);
    // 前 3 条压缩为摘要 + 后 3 条保留 = 4 条
    assert_eq!(result.len(), 4);
    // 第一条应该是摘要
    assert!(result[0].content.contains("[早期工具调用摘要"));
    assert!(result[0].content.contains("Read×1"));
    assert!(result[0].content.contains("Edit×1"));
    assert!(result[0].content.contains("Bash×1"));
    assert!(result[0].content.contains("共 3 次"));
    // 后 3 条保留完整
    assert_eq!(result[1].content, "<Teammate@Frontend> [调用工具 Read]");
    assert_eq!(result[2].content, "<Teammate@Frontend> [调用工具 Edit]");
    assert_eq!(result[3].content, "<Teammate@Frontend> [调用工具 Bash]");
}

#[test]
fn test_compress_multiple_agents() {
    // 多个其他 agent，各自独立压缩
    let messages = vec![
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Read]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Edit]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Bash]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Backend> [调用工具 Bash]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Backend> [调用工具 Edit]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Backend> [调用工具 Read]".to_string(),
        ),
    ];
    let result = compress_other_agent_toolcalls(&messages, "Main", 2);
    // Frontend: 1 摘要 + 2 保留 = 3
    // Backend: 1 摘要 + 2 保留 = 3
    // 总共 6 条消息
    assert_eq!(result.len(), 6);
    // 验证 Frontend 摘要
    assert!(result[0].content.contains("<Teammate@Frontend>"));
    assert!(result[0].content.contains("[早期工具调用摘要"));
    // 验证 Backend 摘要（在其第一条消息的位置）
    let backend_summary_idx = result
        .iter()
        .position(|m| m.content.contains("<Teammate@Backend> [早期工具调用摘要"))
        .expect("Backend summary should exist");
    assert!(result[backend_summary_idx].content.contains("Bash×1"));
}

#[test]
fn test_compress_preserves_other_messages() {
    // 其他类型的消息保留完整
    let messages = vec![
        ChatMessage::text(MessageRole::User, "user question".to_string()),
        ChatMessage::text(MessageRole::Assistant, "assistant response".to_string()),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Read]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Edit]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Bash]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Read]".to_string(),
        ),
        ChatMessage::text(MessageRole::User, "another user message".to_string()),
    ];
    let result = compress_other_agent_toolcalls(&messages, "Main", 2);
    // user question + assistant response + 摘要 + 保留的 2 条 tool call + another user message
    assert!(result.iter().any(|m| m.content == "user question"));
    assert!(result.iter().any(|m| m.content == "assistant response"));
    assert!(result.iter().any(|m| m.content == "another user message"));
}

// ════════════════════════════════════════════════════════════════
// 回归测试：边界条件
// ════════════════════════════════════════════════════════════════

#[test]
fn compress_empty_messages_returns_empty() {
    let result = compress_other_agent_toolcalls(&[], "Main", 5);
    assert!(result.is_empty(), "空输入应返回空输出");
}

#[test]
fn compress_threshold_zero_returns_original() {
    let messages = vec![
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Read]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Edit]".to_string(),
        ),
    ];
    let result = compress_other_agent_toolcalls(&messages, "Main", 0);
    assert_eq!(result.len(), 2, "threshold=0 应原样返回");
    assert_eq!(result[0].content, "<Teammate@Frontend> [调用工具 Read]");
}

#[test]
fn compress_self_agent_excluded() {
    // 自己发出的 tool call 广播不应被压缩
    let messages = vec![
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Read]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Edit]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Bash]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Read]".to_string(),
        ),
    ];
    // self_agent_name 含类型前缀，与广播前缀一致
    let result = compress_other_agent_toolcalls(&messages, "Teammate@Frontend", 1);
    assert_eq!(result.len(), 4, "self agent 的消息不压缩");
}

#[test]
fn compress_mixed_broadcast_and_regular() {
    // 混合广播消息和普通消息
    let messages = vec![
        ChatMessage::text(MessageRole::User, "normal message".to_string()),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Read]".to_string(),
        ),
        ChatMessage::text(MessageRole::Assistant, "response".to_string()),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Edit]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Bash]".to_string(),
        ),
        ChatMessage::text(
            MessageRole::User,
            "<Teammate@Frontend> [调用工具 Read]".to_string(),
        ),
    ];
    let result = compress_other_agent_toolcalls(&messages, "Main", 2);
    // 普通消息和 assistant 消息保留，Frontend 超过 threshold 的被压缩
    assert!(result.iter().any(|m| m.content == "normal message"));
    assert!(result.iter().any(|m| m.content == "response"));
    // 应有摘要
    assert!(
        result
            .iter()
            .any(|m| m.content.contains("[早期工具调用摘要"))
    );
}
