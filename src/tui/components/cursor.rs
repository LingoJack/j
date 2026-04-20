//! 行内光标组件

use crate::theme::Theme;
use ratatui::{style::Style, text::Span};

/// 构建带行内光标的 span 列表
pub fn cursor_spans<'a>(value: &str, cursor: usize, style: Style, theme: &Theme) -> Vec<Span<'a>> {
    let chars: Vec<char> = value.chars().collect();
    let before: String = chars[..cursor.min(chars.len())].iter().collect();
    let cursor_ch = if cursor < chars.len() {
        chars[cursor].to_string()
    } else {
        " ".to_string()
    };
    let after: String = if cursor < chars.len() {
        chars[cursor + 1..].iter().collect()
    } else {
        String::new()
    };
    vec![
        Span::styled(before, style),
        Span::styled(
            cursor_ch,
            Style::default().fg(theme.cursor_fg).bg(theme.cursor_bg),
        ),
        Span::styled(after, style),
        Span::styled(" \u{270f}\u{fe0f}", Style::default()),
    ]
}
