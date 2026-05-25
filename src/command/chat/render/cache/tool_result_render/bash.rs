//! Bash 工具结果渲染

use crate::command::chat::constants::BASH_OUTPUT_MAX_LINES;
use crate::command::chat::render::cache::ContentContext;
use crate::util::text::wrap_text;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// 渲染 Bash 工具结果（命令行高亮 + 输出）
pub(crate) fn render_bash_result(
    content: &str,
    tool_args: Option<&str>,
    ctx: &mut ContentContext<'_>,
) {
    let lines = &mut *ctx.lines;
    let theme = ctx.theme;
    let content_w = ctx.content_w;
    // 提取命令
    let command = tool_args
        .and_then(|args| serde_json::from_str::<serde_json::Value>(args).ok())
        .and_then(|v| {
            v.get("command")
                .and_then(|c| c.as_str().map(|s| s.to_string()))
        });

    if let Some(cmd) = command {
        // 命令行用高亮颜色显示
        let cmd_w = content_w.saturating_sub(6); // "    $ " 前缀
        for (i, cmd_line) in cmd.lines().enumerate() {
            let prefix = if i == 0 { "    $ " } else { "      " };
            for wrapped in wrap_text(cmd_line, cmd_w) {
                lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(theme.label_ai)),
                    Span::styled(
                        wrapped,
                        Style::default()
                            .fg(theme.text_white)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
        }
    }

    // 输出内容（灰色）
    let output_lines: Vec<&str> = content.lines().take(BASH_OUTPUT_MAX_LINES).collect();
    for line in &output_lines {
        for wrapped in wrap_text(line, content_w) {
            lines.push(Line::from(Span::styled(
                format!("    {}", wrapped),
                Style::default().fg(theme.text_dim),
            )));
        }
    }

    let total_lines = content.lines().count();
    if total_lines > 100 {
        lines.push(Line::from(Span::styled(
            format!("    ... (共 {} 行，显示前 100 行)", total_lines),
            Style::default().fg(theme.text_dim),
        )));
    }
}
