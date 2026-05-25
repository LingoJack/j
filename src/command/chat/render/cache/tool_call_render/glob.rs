//! Glob 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::{render_kv_line, truncate_path};

/// Glob 工具展开渲染：只显示关键参数（path + pattern）
pub(crate) fn render_glob_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // path（截断显示）
    if let Some(path) = parsed.get("path").and_then(|v| v.as_str()) {
        let display_path = truncate_path(path, 60);
        render_kv_line("path", &display_path, content_w, lines, theme);
    }

    // pattern
    if let Some(pattern) = parsed.get("pattern").and_then(|v| v.as_str()) {
        render_kv_line("pattern", pattern, content_w, lines, theme);
    }

    true
}
