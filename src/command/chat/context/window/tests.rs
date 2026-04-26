use super::*;
use crate::command::chat::storage::{ChatMessage, MessageRole, ToolCallItem};

fn user_msg(content: &str) -> ChatMessage {
    ChatMessage::text(MessageRole::User, content)
}

fn assistant_msg(content: &str) -> ChatMessage {
    ChatMessage::text(MessageRole::Assistant, content)
}

fn tool_call_msg(names: &[&str]) -> ChatMessage {
    ChatMessage {
        role: MessageRole::Assistant,
        content: String::new(),
        tool_calls: Some(
            names
                .iter()
                .enumerate()
                .map(|(i, name)| ToolCallItem {
                    id: format!("call_{}", i),
                    name: name.to_string(),
                    arguments: "{}".to_string(),
                })
                .collect(),
        ),
        tool_call_id: None,
        images: None,
        reasoning_content: None,
        sender_name: None,
    }
}

fn tool_result_msg(call_id: &str, content: &str) -> ChatMessage {
    ChatMessage {
        role: MessageRole::Tool,
        content: content.to_string(),
        tool_calls: None,
        tool_call_id: Some(call_id.to_string()),
        images: None,
        reasoning_content: None,
        sender_name: None,
    }
}

#[test]
fn test_no_truncation_needed() {
    let msgs = vec![user_msg("hello"), assistant_msg("hi")];
    let result = select_messages(&msgs, 100, 0, 10, &[]); // 0 = 不限制
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].role, MessageRole::User);
    assert_eq!(result[1].role, MessageRole::Assistant);
}

#[test]
fn test_tool_group_dropped_first() {
    use crate::command::chat::tools::tool_names::BASH;

    // U1 → A1(text) → TG1(Bash+result) → U2 → A2(text)
    let msgs = vec![
        user_msg("do something"),
        assistant_msg("let me check"),
        tool_call_msg(&[BASH]),
        tool_result_msg("call_0", &"huge output ".repeat(1000)),
        user_msg("what about this"),
        assistant_msg("here's the answer"),
    ];

    // 设置极小预算 + keep_recent=0（禁用 Stage 1 保底），迫使丢弃
    let result = select_messages(&msgs, 100, 1, 0, &[]);

    // User 应该保留，ToolGroup 应该被占位符替换
    assert!(result.iter().any(|m| m.role == MessageRole::User));
    assert!(!result.iter().any(|m| m.role == MessageRole::Tool)); // tool result 丢弃
    assert!(result.iter().any(|m| m.content.contains("Previous: used"))); // 占位符
}

#[test]
fn test_time_order_preserved() {
    use crate::command::chat::tools::tool_names::BASH;

    let msgs = vec![
        user_msg("first"),
        assistant_msg("ok1"),
        tool_call_msg(&[BASH]),
        tool_result_msg("call_0", "output"),
        user_msg("second"),
        assistant_msg("ok2"),
    ];

    let result = select_messages(&msgs, 100, 0, 10, &[]); // 0 = 不限制

    // 时间顺序保持
    let user_positions: Vec<usize> = result
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role == MessageRole::User)
        .map(|(i, _)| i)
        .collect();
    assert!(user_positions[0] < user_positions[1]);
}

#[test]
fn test_placeholder_format() {
    use crate::command::chat::tools::tool_names::{BASH, READ};

    let msgs = vec![
        user_msg("run"),
        tool_call_msg(&[BASH, READ]),
        tool_result_msg("call_0", &"x".repeat(2000)),
        tool_result_msg("call_1", &"y".repeat(2000)),
    ];

    // 极小 token 预算 + keep_recent=0 迫使 ToolGroup 丢弃
    let result = select_messages(&msgs, 100, 1, 0, &[]);

    let placeholder = result.iter().find(|m| m.content.contains("Previous: used"));
    assert!(placeholder.is_some());
    let p = placeholder.unwrap();
    assert!(p.content.contains(BASH));
    assert!(p.content.contains(READ));
    assert!(p.tool_calls.is_none());
}

#[test]
fn test_exempt_tool_group_protected() {
    use crate::command::chat::tools::tool_names::LOAD_SKILL;

    // 豁免工具 ToolGroup 即使在极紧预算下也应保留
    let msgs = vec![
        user_msg("load a skill"),
        tool_call_msg(&[LOAD_SKILL]),
        tool_result_msg("call_0", &"skill content ".repeat(500)),
        user_msg("q1"),
        assistant_msg("a1"),
        user_msg("q2"),
        assistant_msg("a2"),
        user_msg("q3"),
        assistant_msg("a3"),
    ];

    // 预算足够容纳豁免的 skill 内容（~2K chars → ~700 tokens），keep_recent=0 禁用时间保底
    let result = select_messages(&msgs, 100, 5, 0, &[]);

    // 豁免 ToolGroup 的 tool result 应该保留，不会变占位符
    assert!(
        result.iter().any(|m| m.role == MessageRole::Tool),
        "exempt tool result 应该被保留"
    );
}

#[test]
fn test_stage1_time_fallback_keeps_recent_tool_group() {
    use crate::command::chat::tools::tool_names::BASH;

    // 即使早期有很多 User 消息，最近的 ToolGroup 也应该被 Stage 1 保底保留
    let mut msgs = Vec::new();
    for i in 0..20 {
        msgs.push(user_msg(&format!("old user {}", i).repeat(50)));
    }
    msgs.push(tool_call_msg(&[BASH]));
    msgs.push(tool_result_msg("call_0", "recent shell output"));
    msgs.push(user_msg("latest"));

    // keep_recent=2 → Stage 1 保留最近 4 个 unit（覆盖 ToolGroup + latest User）
    let result = select_messages(&msgs, 100, 2, 2, &[]);

    assert!(
        result.iter().any(|m| m.role == MessageRole::Tool),
        "最近的 tool result 应该被时间保底保留"
    );
    assert!(
        result
            .iter()
            .any(|m| m.role == MessageRole::User && m.content == "latest"),
        "最新 User 必须保留"
    );
}
