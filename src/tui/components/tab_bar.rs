//! Tab 栏组件

use crate::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::consts::SEPARATOR_V;

/// Tab 栏（支持任意 tab 列表）
pub fn tab_bar<'a>(tabs: &[(&str, bool)], hint: &str, theme: &Theme) -> Line<'a> {
    let mut spans: Vec<Span<'a>> = vec![Span::styled("  ", Style::default())];
    for (i, (label, active)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                format!(" {SEPARATOR_V} "),
                Style::default().fg(theme.separator),
            ));
        }
        let text = format!(" {label} ");
        if *active {
            spans.push(Span::styled(
                text,
                Style::default()
                    .fg(theme.config_tab_active_fg)
                    .bg(theme.config_tab_active_bg)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                text,
                Style::default().fg(theme.config_tab_inactive),
            ));
        }
    }
    if !hint.is_empty() {
        spans.push(Span::styled(
            format!("    ({hint})"),
            Style::default().fg(theme.config_dim),
        ));
    }
    Line::from(spans)
}
