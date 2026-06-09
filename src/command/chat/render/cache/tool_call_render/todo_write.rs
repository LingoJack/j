//! TodoWrite 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

use super::shared::{render_tag_line, truncate_str};

/// TodoWrite 工具展开渲染
pub(crate) fn render_todo_write_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(todos) = parsed.get("todos").and_then(|v| v.as_array()) {
        render_tag_line(
            &format!("待办列表 ({} 项)", todos.len()),
            content_w,
            lines,
            theme,
        );
        for todo in todos {
            let content = todo.get("content").and_then(|v| v.as_str()).unwrap_or("?");
            let status = todo
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("pending");
            let bullet = match status {
                "completed" => "[x]",
                "in_progress" => "[~]",
                "cancelled" => "[-]",
                _ => "[ ]",
            };
            let line_text = format!("{} {}", bullet, content);
            let display = truncate_str(&line_text, content_w);
            lines.push(Line::from(vec![
                Span::styled("      ", Style::default()),
                Span::styled(display, Style::default().fg(theme.text_normal)),
            ]));
        }
    }

    true
}
