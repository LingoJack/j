//! Write 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::{render_kv_line, truncate_path};

/// Write 工具展开渲染
pub(crate) fn render_write_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    // path / file_path
    let path = parsed
        .get("path")
        .or_else(|| parsed.get("file_path"))
        .and_then(|v| v.as_str());

    if let Some(path) = path {
        let display_path = truncate_path(path, 60);
        render_kv_line("path", &display_path, content_w, lines, theme);
    }

    true
}
