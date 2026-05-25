//! SendMessage 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::{render_kv_line, render_tag_line, truncate_str};

/// SendMessage 工具展开渲染
pub(crate) fn render_send_message_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let to = parsed.get("to").and_then(|v| v.as_str());
    if let Some(target) = to {
        render_kv_line("to", &format!("@{}", target), content_w, lines, theme);
    } else {
        render_tag_line("广播消息", content_w, lines, theme);
    }

    if let Some(message) = parsed.get("message").and_then(|v| v.as_str()) {
        render_kv_line(
            "message",
            &truncate_str(message, 100),
            content_w,
            lines,
            theme,
        );
    }

    true
}
