//! 分隔线组件

use crate::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::consts::INDENT;

/// 自适应宽度分隔线（替代硬编码 "─────…"）
pub fn separator_line(width: u16, theme: &Theme) -> Line<'static> {
    let w = (width as usize).saturating_sub(4); // 左缩进 2 字符 + 右留 2
    let bar: String = "\u{2500}".repeat(w);
    Line::from(Span::styled(
        format!("{INDENT}{bar}"),
        Style::default().fg(theme.separator),
    ))
}

/// 章节标题（如 "📖 快捷键帮助"）
pub fn section_header<'a>(icon: &str, title: &str, theme: &Theme) -> Line<'a> {
    Line::from(Span::styled(
        format!("{INDENT}{icon} {title}"),
        Style::default()
            .fg(theme.help_title)
            .add_modifier(Modifier::BOLD),
    ))
}
