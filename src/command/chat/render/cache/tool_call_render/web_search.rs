//! WebSearch 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::render_kv_line;

/// WebSearch 工具展开渲染
pub(crate) fn render_web_search_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(query) = parsed.get("query").and_then(|v| v.as_str()) {
        render_kv_line("query", query, content_w, lines, theme);
    }

    if let Some(count) = parsed.get("count").and_then(|v| v.as_u64()) {
        render_kv_line("count", &count.to_string(), content_w, lines, theme);
    }

    if let Some(search_type) = parsed.get("type").and_then(|v| v.as_str()) {
        render_kv_line("type", search_type, content_w, lines, theme);
    }

    true
}
