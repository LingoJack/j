use super::*;
use crate::command::chat::storage::MessageRole;

// ════════════════════════════════════════════════════════════════
// 回归测试：drain_pending_user_messages
// 如果以下测试失败，说明 agent loop 的用户消息注入逻辑被破坏
// ════════════════════════════════════════════════════════════════

#[test]
fn drain_pending_appends_user_messages_with_marker() {
    let pending: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![
        ChatMessage::text(MessageRole::User, "hello"),
        ChatMessage::text(MessageRole::User, "world"),
    ]));
    let mut messages: Vec<ChatMessage> = vec![ChatMessage::text(MessageRole::Assistant, "hi")];

    drain_pending_user_messages(&mut messages, &pending);

    assert_eq!(messages.len(), 3, "应有 3 条消息");
    assert_eq!(messages[1].content, "[User appended] hello");
    assert_eq!(messages[2].content, "[User appended] world");
    // pending 已被 drain
    assert!(pending.lock().unwrap().is_empty(), "pending 应被清空");
}

#[test]
fn drain_pending_preserves_non_user_role() {
    let pending: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![ChatMessage::text(
        MessageRole::System,
        "system note",
    )]));
    let mut messages: Vec<ChatMessage> = vec![];

    drain_pending_user_messages(&mut messages, &pending);

    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content, "system note",
        "非 User 角色的消息不应加 [User appended] 前缀"
    );
}

#[test]
fn drain_pending_empty_noop() {
    let pending: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
    let mut messages: Vec<ChatMessage> = vec![ChatMessage::text(MessageRole::User, "keep")];

    drain_pending_user_messages(&mut messages, &pending);

    assert_eq!(messages.len(), 1, "空 pending 不应改变 messages");
    assert_eq!(messages[0].content, "keep");
}

// ════════════════════════════════════════════════════════════════
// 回归测试：push_both / clear_channels
// ════════════════════════════════════════════════════════════════

#[test]
fn push_both_appends_to_both_channels() {
    let display: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
    let context: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
    let msg = ChatMessage::text(MessageRole::Assistant, "response");

    push_both(&display, &context, msg);

    assert_eq!(display.lock().unwrap().len(), 1, "display 应有 1 条消息");
    assert_eq!(context.lock().unwrap().len(), 1, "context 应有 1 条消息");
    assert_eq!(display.lock().unwrap()[0].content, "response");
    assert_eq!(context.lock().unwrap()[0].content, "response");
}

#[test]
fn push_both_clones_message() {
    let display: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
    let context: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
    let msg = ChatMessage::text(MessageRole::User, "test");

    push_both(&display, &context, msg);

    // 两个通道应各自持有独立的 clone
    let d = display.lock().unwrap();
    let c = context.lock().unwrap();
    assert_eq!(d[0].content, c[0].content);
}

#[test]
fn clear_channels_empties_both() {
    let display: Arc<Mutex<Vec<ChatMessage>>> =
        Arc::new(Mutex::new(vec![ChatMessage::text(MessageRole::User, "a")]));
    let context: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![
        ChatMessage::text(MessageRole::Assistant, "b"),
        ChatMessage::text(MessageRole::Tool, "c"),
    ]));

    clear_channels(&display, &context);

    assert!(
        display.lock().unwrap().is_empty(),
        "clear 后 display 应为空"
    );
    assert!(
        context.lock().unwrap().is_empty(),
        "clear 后 context 应为空"
    );
}

#[test]
fn sync_context_full_replaces_both() {
    let display: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![ChatMessage::text(
        MessageRole::User,
        "old",
    )]));
    let context: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![ChatMessage::text(
        MessageRole::Assistant,
        "old",
    )]));
    let new_msgs = vec![
        ChatMessage::text(MessageRole::System, "new1"),
        ChatMessage::text(MessageRole::User, "new2"),
    ];

    sync_context_full(&display, &context, &new_msgs);

    let d = display.lock().unwrap();
    let c = context.lock().unwrap();
    assert_eq!(d.len(), 2, "display 应被替换为 2 条新消息");
    assert_eq!(c.len(), 2, "context 应被替换为 2 条新消息");
    assert_eq!(d[0].content, "new1");
    assert_eq!(c[1].content, "new2");
}
