//! 提示栏组件

use crate::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::consts::INDENT;

/// 帮助页快捷键行
pub fn help_key_row<'a>(key: &str, desc: &str, key_width: usize, theme: &Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(INDENT, Style::default()),
        Span::styled(
            format!("{:<width$}", key, width = key_width),
            Style::default()
                .fg(theme.help_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_string(), Style::default().fg(theme.help_desc)),
    ])
}

/// 底部提示栏单项 spans
pub fn hint_spans<'a>(key: &str, desc: &str, theme: &Theme) -> Vec<Span<'a>> {
    vec![
        Span::styled(
            format!(" {key} "),
            Style::default()
                .fg(theme.text_very_dim)
                .bg(theme.bg_primary),
        ),
        Span::styled(format!(" {desc}"), Style::default().fg(theme.text_very_dim)),
    ]
}
