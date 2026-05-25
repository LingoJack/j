//! SendMessage 工具结果渲染

use crate::command::chat::constants::NORMAL_RESULT_MAX_LINES;
use crate::command::chat::render::theme::Theme;
use crate::util::text::wrap_text;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// SendMessage 结果：发送确认
pub(crate) fn render_send_message_result(
    content: &str,
    tool_args: Option<&str>,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    // 从 tool_args 提取目标
    let target = tool_args.and_then(|args| {
        let pattern = "\"to\"";
        if let Some(idx) = args.find(pattern) {
            let rest = &args[idx + pattern.len()..];
            let rest = rest.trim_start_matches([' ', ':', '\t']);
            if let Some(stripped) = rest.strip_prefix('"')
                && let Some(end) = stripped.find('"')
            {
                return Some(stripped[..end].to_string());
            }
        }
        None
    });

    // 从 tool_args 提取消息内容
    let message = tool_args.and_then(|args| {
        let pattern = "\"message\"";
        if let Some(idx) = args.find(pattern) {
            let rest = &args[idx + pattern.len()..];
            let rest = rest.trim_start_matches([' ', ':', '\t']);
            if let Some(stripped) = rest.strip_prefix('"')
                && let Some(end) = stripped.find('"')
            {
                return Some(stripped[..end].to_string());
            }
        }
        None
    });

    // 第一行：发送目标
    if let Some(t) = &target {
        lines.push(Line::from(vec![
            Span::styled("    -> ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("@{}", t),
                Style::default()
                    .fg(theme.config_title)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    // 消息预览
    if let Some(msg) = &message {
        for wrapped in wrap_text(msg, 80) {
            lines.push(Line::from(Span::styled(
                format!("       {}", wrapped),
                Style::default().fg(theme.text_dim),
            )));
        }
    } else {
        // 无消息预览，显示原始 content
        for line in content.lines().take(NORMAL_RESULT_MAX_LINES) {
            lines.push(Line::from(Span::styled(
                format!("    {}", line),
                Style::default().fg(theme.text_dim),
            )));
        }
    }
}
