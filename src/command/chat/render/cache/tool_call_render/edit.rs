//! Edit 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::{render_kv_line, summarize_string_content, truncate_path};

/// Edit 工具展开渲染
pub(crate) fn render_edit_call_request_expanded(
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

    // old_string 摘要
    if let Some(old) = parsed.get("old_string").and_then(|v| v.as_str()) {
        let summary = summarize_string_content(old, 40);
        render_kv_line("old", &summary, content_w, lines, theme);
    }

    // new_string 摘要
    if let Some(new) = parsed.get("new_string").and_then(|v| v.as_str()) {
        let summary = summarize_string_content(new, 40);
        render_kv_line("new", &summary, content_w, lines, theme);
    }

    // replace_all
    if let Some(replace_all) = parsed.get("replace_all").and_then(|v| v.as_bool())
        && replace_all
    {
        render_kv_line("mode", "全部替换", content_w, lines, theme);
    }

    true
}
