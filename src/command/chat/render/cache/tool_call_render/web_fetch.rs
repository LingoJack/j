//! WebFetch 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::render_kv_line;

/// WebFetch 工具展开渲染
pub(crate) fn render_web_fetch_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(url) = parsed.get("url").and_then(|v| v.as_str()) {
        render_kv_line("url", url, content_w, lines, theme);
    }

    if let Some(mode) = parsed.get("extract_mode").and_then(|v| v.as_str()) {
        render_kv_line("mode", mode, content_w, lines, theme);
    }

    true
}
