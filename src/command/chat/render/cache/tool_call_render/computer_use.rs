//! ComputerUse 工具调用渲染

#[cfg(target_os = "macos")]
use crate::command::chat::render::theme::Theme;
#[cfg(target_os = "macos")]
use ratatui::text::Line;

#[cfg(target_os = "macos")]
use super::shared::render_kv_line;

/// ComputerUse 工具展开渲染
#[cfg(target_os = "macos")]
pub(crate) fn render_computer_use_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(action) = parsed.get("action").and_then(|v| v.as_str()) {
        render_kv_line("action", action, content_w, lines, theme);
    }

    if let Some(display_num) = parsed.get("display_number").and_then(|v| v.as_u64()) {
        render_kv_line("display", &display_num.to_string(), content_w, lines, theme);
    }

    true
}
