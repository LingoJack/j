//! 标签 / 说明文字组件

use crate::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

/// 标签 span（固定显示宽度，左对齐，CJK 感知）
pub fn label_span<'a>(text: &str, width: usize, selected: bool, theme: &Theme) -> Span<'a> {
    use unicode_width::UnicodeWidthStr;
    let style = if selected {
        Style::default()
            .fg(theme.config_label_selected)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.config_label)
    };
    let display_w = UnicodeWidthStr::width(text);
    let padding = if display_w < width {
        " ".repeat(width - display_w)
    } else {
        String::new()
    };
    Span::styled(format!("{text}{padding}"), style)
}

/// 说明文字 span（固定显示宽度，超长截断加 "..."，CJK 感知）
pub fn desc_span<'a>(text: &str, max_width: usize, theme: &Theme) -> Span<'a> {
    use unicode_width::UnicodeWidthStr;
    if text.is_empty() {
        return Span::styled(String::new(), Style::default());
    }
    let display_w = UnicodeWidthStr::width(text);
    if display_w <= max_width {
        let padding = " ".repeat(max_width - display_w);
        Span::styled(
            format!("  {text}{padding}"),
            Style::default().fg(theme.config_dim),
        )
    } else {
        let mut w = 0;
        let end = max_width.saturating_sub(3);
        let truncated: String = text
            .chars()
            .take_while(|c| {
                let cw = UnicodeWidthStr::width(c.to_string().as_str());
                if w + cw > end {
                    false
                } else {
                    w += cw;
                    true
                }
            })
            .collect();
        let padding = " ".repeat(max_width - w - 3);
        Span::styled(
            format!("  {truncated}...{padding}"),
            Style::default().fg(theme.config_dim),
        )
    }
}

/// 值的样式（普通/选中/编辑中）
pub fn value_style(selected: bool, editing: bool, theme: &Theme) -> Style {
    if editing && selected {
        Style::default()
            .fg(theme.text_white)
            .bg(theme.config_edit_bg)
    } else if selected {
        Style::default().fg(theme.text_white)
    } else {
        Style::default().fg(theme.config_value)
    }
}
