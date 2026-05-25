//! RegisterHook 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::{render_kv_line, render_tag_line, truncate_str};

/// RegisterHook 工具展开渲染
pub(crate) fn render_register_hook_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let action = parsed
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("register");
    render_tag_line(&format!("[{}]", action), content_w, lines, theme);

    if let Some(event) = parsed.get("event").and_then(|v| v.as_str()) {
        render_kv_line("event", event, content_w, lines, theme);
    }

    if let Some(hook_type) = parsed.get("type").and_then(|v| v.as_str()) {
        render_kv_line("type", hook_type, content_w, lines, theme);
    }

    if let Some(command) = parsed.get("command").and_then(|v| v.as_str()) {
        render_kv_line("command", command, content_w, lines, theme);
    }

    if let Some(prompt) = parsed.get("prompt").and_then(|v| v.as_str()) {
        render_kv_line(
            "prompt",
            &truncate_str(prompt, 100),
            content_w,
            lines,
            theme,
        );
    }

    true
}
