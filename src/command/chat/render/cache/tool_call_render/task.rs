//! Task 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::{render_kv_line, render_tag_line};

/// Task 工具展开渲染
pub(crate) fn render_task_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let action = parsed
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("task");

    // action 标签
    render_tag_line(&format!("[{}]", action), content_w, lines, theme);

    // title
    if let Some(title) = parsed.get("title").and_then(|v| v.as_str()) {
        render_kv_line("title", title, content_w, lines, theme);
    }

    // description
    if let Some(desc) = parsed.get("description").and_then(|v| v.as_str()) {
        render_kv_line("description", desc, content_w, lines, theme);
    }

    // taskId（update/get 时）
    if let Some(task_id) = parsed.get("taskId").and_then(|v| v.as_str()) {
        render_kv_line("taskId", task_id, content_w, lines, theme);
    }

    // status（update 时）
    if let Some(status) = parsed.get("status").and_then(|v| v.as_str()) {
        render_kv_line("status", status, content_w, lines, theme);
    }

    true
}
