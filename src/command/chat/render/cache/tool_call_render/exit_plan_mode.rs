//! ExitPlanMode 工具调用渲染

use crate::command::chat::render::cache::bubble::bordered_line;
use crate::command::chat::render::theme::Theme;
use crate::util::text::wrap_text;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// 渲染 ExitPlanMode 工具调用请求（边框显示）
pub(crate) fn render_exit_plan_mode_request(
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let border_color = theme.text_dim;
    let result_bg = theme.bg_primary;
    let content_w = bubble_max_width.saturating_sub(6);

    // 顶边框
    let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(border_color).bg(result_bg),
    )));

    // 内容：提交计划审批提示
    let hint = "提交计划审批，等待用户批准后退出计划模式";
    for wrapped in wrap_text(hint, content_w) {
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

    // 底边框
    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color).bg(result_bg),
    )));
}
