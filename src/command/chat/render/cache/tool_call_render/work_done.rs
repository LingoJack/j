//! WorkDone 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::{render_kv_line, render_tag_line};

/// WorkDone 工具展开渲染
pub(crate) fn render_work_done_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    render_tag_line("工作完成声明", content_w, lines, theme);

    if let Some(summary) = parsed.get("summary").and_then(|v| v.as_str()) {
        render_kv_line("summary", summary, content_w, lines, theme);
    }

    true
}
