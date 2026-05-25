//! Grep 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::{render_kv_line, truncate_path};

/// Grep 工具展开渲染：显示 path + pattern + mode + context
pub(crate) fn render_grep_call_request_expanded(
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

    // output_mode
    if let Some(mode) = parsed.get("output_mode").and_then(|v| v.as_str())
        && mode != "content"
    {
        render_kv_line("mode", mode, content_w, lines, theme);
    }

    // context（若有）
    if let Some(ctx) = parsed.get("context").and_then(|v| v.as_u64())
        && ctx > 0
    {
        render_kv_line("context", &format!("{} 行", ctx), content_w, lines, theme);
    }

    true
}
