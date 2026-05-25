//! TaskOutput 工具结果渲染：结构化任务输出

use crate::command::chat::constants::{BASH_OUTPUT_MAX_LINES, NORMAL_RESULT_MAX_LINES};
use crate::command::chat::render::theme::Theme;
use crate::util::text::wrap_text;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// TaskOutput 结果：结构化任务输出
///
/// 输入格式（JSON）：
/// ```json
/// {
///   "task_id": "bg_1",
///   "command": "cargo build",
///   "status": "completed",
///   "output": "..."
/// }
/// ```
pub(crate) fn render_task_output_result(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    use serde_json::Value;

    let parsed: Option<Value> = serde_json::from_str(content).ok();

    if let Some(Value::Object(obj)) = parsed {
        // 状态行
        let task_id = obj.get("task_id").and_then(|v| v.as_str()).unwrap_or("?");
        let status = obj
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let command = obj.get("command").and_then(|v| v.as_str());

        // 状态图标 + 颜色
        let (status_icon, status_color) = match status {
            "completed" => ("●", theme.label_ai),
            "running" => ("◉", theme.title_loading),
            "error" => ("✕", theme.toast_error_border),
            "timeout" => ("⏱", theme.title_loading),
            "dead" => ("✕", theme.toast_error_border),
            _ => ("·", theme.text_dim),
        };

        // 第一行：task_id + 状态
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(
                format!("{} ", status_icon),
                Style::default().fg(status_color),
            ),
            Span::styled(
                format!("[{}] ", task_id),
                Style::default().fg(theme.text_dim),
            ),
            Span::styled(status.to_string(), Style::default().fg(status_color)),
        ]));

        // 命令行
        if let Some(cmd) = command {
            let cmd_w = content_w.saturating_sub(10); // "    $ " prefix
            for wrapped in wrap_text(cmd, cmd_w) {
                lines.push(Line::from(vec![
                    Span::styled("    $ ", Style::default().fg(theme.label_ai)),
                    Span::styled(
                        wrapped,
                        Style::default()
                            .fg(theme.text_white)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
        }

        // note 字段（如超时/取消提示）
        if let Some(note) = obj.get("note").and_then(|v| v.as_str()) {
            lines.push(Line::from(Span::styled(
                format!("    {}", note),
                Style::default().fg(theme.title_loading),
            )));
        }

        // 输出内容
        if let Some(output) = obj.get("output").and_then(|v| v.as_str())
            && !output.is_empty()
        {
            // 命令输出与结果之间加空行
            if command.is_some() {
                lines.push(Line::from(""));
            }

            let output_lines: Vec<&str> = output.lines().take(BASH_OUTPUT_MAX_LINES).collect();
            for line in &output_lines {
                for wrapped in wrap_text(line, content_w) {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", wrapped),
                        Style::default().fg(theme.text_dim),
                    )));
                }
            }

            let total_lines = output.lines().count();
            if total_lines > BASH_OUTPUT_MAX_LINES {
                lines.push(Line::from(Span::styled(
                    format!(
                        "    ... (共 {} 行，显示前 {} 行)",
                        total_lines, BASH_OUTPUT_MAX_LINES
                    ),
                    Style::default().fg(theme.text_dim),
                )));
            }
        }
    } else {
        // 非 JSON 格式，回退到普通渲染
        let all_lines: Vec<&str> = content.lines().take(NORMAL_RESULT_MAX_LINES).collect();
        for line in all_lines {
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_dim),
                )));
            }
        }

        let total_lines = content.lines().count();
        if total_lines > NORMAL_RESULT_MAX_LINES {
            lines.push(Line::from(Span::styled(
                format!(
                    "    ... (共 {} 行，显示前 {} 行)",
                    total_lines, NORMAL_RESULT_MAX_LINES
                ),
                Style::default().fg(theme.text_dim),
            )));
        }
    }
}
