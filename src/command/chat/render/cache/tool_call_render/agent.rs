//! Agent 工具调用渲染

use crate::command::chat::constants::AGENT_CALL_PROMPT_MAX_LINES;
use crate::command::chat::render::cache::bubble::bordered_line;
use crate::command::chat::render::theme::Theme;
use crate::util::text::wrap_text;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// Agent 工具参数结构（用于渲染）
pub(crate) struct AgentCallArgs {
    pub prompt: String,
    pub description: Option<String>,
    pub run_in_background: bool,
}

/// 从 Agent 工具的 arguments JSON 中提取参数
pub(crate) fn extract_agent_args(arguments: &str) -> Option<AgentCallArgs> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;
    Some(AgentCallArgs {
        prompt: parsed.get("prompt")?.as_str()?.to_string(),
        description: parsed
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        run_in_background: parsed
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

/// 渲染 Agent 工具调用请求的展开模式（边框 + prompt + 元信息）
pub(crate) fn render_agent_call_request_expanded(
    args: &AgentCallArgs,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let border_color = theme.text_dim;
    let result_bg = theme.bg_primary;
    let content_w = bubble_max_width.saturating_sub(6);

    // 元信息行：[background] 标识
    if args.run_in_background {
        for wrapped in wrap_text("[background]", content_w) {
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default().bg(result_bg)),
                Span::styled(wrapped, Style::default().fg(theme.text_dim).bg(result_bg)),
            ]));
        }
    }

    // Prompt 边框显示（复用 render_agent_result_nested 的边框风格）
    let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(border_color).bg(result_bg),
    )));

    let prompt_lines: Vec<&str> = args.prompt.lines().collect();
    let total = prompt_lines.len();
    let max_display = AGENT_CALL_PROMPT_MAX_LINES;
    let display_lines = &prompt_lines[..total.min(max_display)];

    for line in display_lines {
        for wrapped in wrap_text(line, content_w) {
            lines.push(bordered_line(
                vec![Span::styled(
                    wrapped,
                    Style::default().fg(theme.text_dim).bg(result_bg),
                )],
                bubble_max_width,
                border_color,
                result_bg,
            ));
        }
    }

    // 截断提示
    if total > max_display {
        lines.push(bordered_line(
            vec![Span::styled(
                format!("... (共 {} 行)", total),
                Style::default().fg(theme.text_dim).bg(result_bg),
            )],
            bubble_max_width,
            border_color,
            result_bg,
        ));
    }

    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color).bg(result_bg),
    )));
}
