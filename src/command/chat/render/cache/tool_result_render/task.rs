//! Task 工具结果渲染：结构化任务列表

use crate::command::chat::constants::NORMAL_RESULT_MAX_LINES;
use crate::command::chat::render::theme::Theme;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// Task 结果：结构化任务列表
///
/// 输入格式：JSON 数组
/// ```json
/// [
///   { "taskId": "1", "title": "...", "status": "completed", "blockedBy": [] }
/// ]
/// ```
pub(crate) fn render_task_result(
    content: &str,
    _content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    use serde_json::Value;
    // 尝试解析 JSON
    let parsed: Option<Value> = serde_json::from_str(content).ok();
    if let Some(Value::Array(arr)) = parsed {
        if arr.is_empty() {
            lines.push(Line::from(Span::styled(
                "    (无任务)",
                Style::default().fg(theme.text_dim),
            )));
            return;
        }

        for item in &arr {
            let task_id = item.get("taskId").and_then(|v| v.as_str()).unwrap_or("?");
            let title = item
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("(无标题)");
            let status = item
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("pending");

            // 状态图标 + 颜色
            let (icon, color) = match status {
                "completed" => ("●", theme.label_ai),
                "in_progress" => ("◉", theme.title_loading),
                "pending" => ("○", theme.text_dim),
                "deleted" => ("✕", theme.toast_error_border),
                _ => ("·", theme.text_dim),
            };

            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(format!("{} ", icon), Style::default().fg(color)),
                Span::styled(
                    format!("#{} ", task_id),
                    Style::default().fg(theme.text_dim),
                ),
                Span::styled(
                    format!("[{}] ", status),
                    Style::default().fg(theme.text_dim),
                ),
                Span::styled(title.to_string(), Style::default().fg(theme.text_normal)),
            ]));
        }

        lines.push(Line::from(Span::styled(
            format!("    (共 {} 项任务)", arr.len()),
            Style::default().fg(theme.text_dim),
        )));
    } else {
        // 非 JSON 格式，普通渲染
        for line in content.lines().take(NORMAL_RESULT_MAX_LINES) {
            lines.push(Line::from(Span::styled(
                format!("    {}", line),
                Style::default().fg(theme.text_dim),
            )));
        }
    }
}
