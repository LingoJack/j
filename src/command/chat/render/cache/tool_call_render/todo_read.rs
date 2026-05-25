//! TodoRead 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::render_tag_line;

/// TodoRead 工具展开渲染
pub(crate) fn render_todo_read_call_request_expanded(
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    render_tag_line("读取待办列表", content_w, lines, theme);
    true
}
