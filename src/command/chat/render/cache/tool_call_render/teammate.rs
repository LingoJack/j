//! Teammate 工具调用渲染

use crate::command::chat::constants::AGENT_CALL_PROMPT_MAX_LINES;
use crate::command::chat::render::cache::bubble::bordered_line;
use crate::command::chat::render::theme::Theme;
use crate::util::text::wrap_text;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// Teammate 工具参数结构（用于渲染）
pub(crate) struct TeammateCallArgs {
    pub name: String,
    pub role: String,
    pub prompt: String,
    pub worktree: bool,
}

/// 从 Teammate 工具的 arguments JSON 中提取参数
pub(crate) fn extract_teammate_args(arguments: &str) -> Option<TeammateCallArgs> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;
    Some(TeammateCallArgs {
        name: parsed.get("name")?.as_str()?.to_string(),
        role: parsed
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        prompt: parsed.get("prompt")?.as_str()?.to_string(),
        worktree: parsed
            .get("worktree")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

/// 渲染 Teammate 工具调用请求的展开模式（边框 + name/role + prompt + 元信息）
pub(crate) fn render_teammate_call_request_expanded(
    args: &TeammateCallArgs,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let border_color = theme.text_dim;
    let result_bg = theme.bg_primary;
    let content_w = bubble_max_width.saturating_sub(6);

    // 元信息行：name(role) [worktree]
    let mut meta_parts = vec![format!(
        "{}({})",
        args.name,
        if args.role.is_empty() {
            &args.name
        } else {
            &args.role
        }
    )];
    if args.worktree {
        meta_parts.push("[worktree]".to_string());
    }
    let meta_line = meta_parts.join("  ");
    for wrapped in wrap_text(&meta_line, content_w) {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default().bg(result_bg)),
            Span::styled(wrapped, Style::default().fg(theme.text_dim).bg(result_bg)),
        ]));
    }

    // Prompt 边框显示
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
