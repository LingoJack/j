//! oneshot 会话管理：会话 ID 生成、消息持久化、Hook 触发

use crate::command::chat::infra::hook::{HookContext, HookEvent, HookManager};
use crate::command::chat::storage::{ChatMessage, SessionEvent, append_session_event};

/// 基于 timestamp + pid 生成 oneshot 模式的唯一会话 ID
pub(crate) fn generate_oneshot_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    let pid = std::process::id();
    format!("{:x}-{:x}", ts, pid)
}

/// 从 `start_idx` 开始追加消息到会话存储
pub(crate) fn persist_messages(session_id: &str, messages: &[ChatMessage], start_idx: usize) {
    for msg in messages.iter().skip(start_idx) {
        append_session_event(session_id, &SessionEvent::msg(msg.clone()));
    }
}

/// 触发 SessionEnd hook
pub(crate) fn fire_session_end(
    hook_manager: &HookManager,
    disabled_hooks: &[String],
    messages: &[ChatMessage],
    session_id: &str,
    model: &str,
) {
    if hook_manager.has_hooks_for(HookEvent::SessionEnd) {
        let ctx = HookContext {
            event: HookEvent::SessionEnd,
            messages: Some(messages.to_vec()),
            model: Some(model.to_string()),
            session_id: Some(session_id.to_string()),
            ..Default::default()
        };
        hook_manager.execute(HookEvent::SessionEnd, ctx, disabled_hooks);
    }
}
