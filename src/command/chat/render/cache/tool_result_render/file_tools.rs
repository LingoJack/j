//! 文件操作工具结果渲染（Write/Edit）

use crate::command::chat::constants::{ERROR_RESULT_MAX_LINES, NORMAL_RESULT_MAX_LINES};
use crate::command::chat::render::theme::Theme;
use crate::util::text::wrap_text;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::shared::parse_file_path_from_json;

/// Write/Edit 结果：文件路径高亮
pub(crate) fn render_write_edit_result(
    content: &str,
    tool_args: Option<&str>,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    // 检测是否为失败结果（Edit 找不到匹配、匹配不唯一等）
    let is_failure = content.contains("未找到匹配")
        || content.contains("not unique")
        || content.contains("failed")
        || content.contains("Failed");

    if is_failure {
        // 失败：文件路径 + 完整错误信息（红色）
        let file_path = tool_args.and_then(parse_file_path_from_json);
        if let Some(path) = file_path {
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    path,
                    Style::default()
                        .fg(theme.config_title)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
        // 显示完整错误信息
        let error_style = Style::default().fg(theme.toast_error_border);
        for line in content.lines().take(ERROR_RESULT_MAX_LINES) {
            for wrapped in wrap_text(line, content_w.saturating_sub(6)) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    error_style,
                )));
            }
        }
        return;
    }

    // 成功：文件路径 + 操作摘要
    let file_path = tool_args.and_then(parse_file_path_from_json);

    if let Some(path) = file_path {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(
                path,
                Style::default()
                    .fg(theme.config_title)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" — ", Style::default().fg(theme.text_dim)),
            Span::styled(
                content.lines().next().unwrap_or("").to_string(),
                Style::default().fg(theme.text_dim),
            ),
        ]));
    } else {
        for line in content.lines().take(NORMAL_RESULT_MAX_LINES) {
            lines.push(Line::from(Span::styled(
                format!("    {}", line),
                Style::default().fg(theme.text_dim),
            )));
        }
    }
}
