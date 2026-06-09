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

/// 说明文字 span，选中时使用选中前景色，避免次要说明文字在高亮行里继续 dim。
pub fn desc_span_with_selected<'a>(
    text: &str,
    max_width: usize,
    selected: bool,
    theme: &Theme,
) -> Span<'a> {
    use unicode_width::UnicodeWidthStr;
    if text.is_empty() {
        return Span::styled(String::new(), Style::default());
    }
    let style = if selected {
        Style::default().fg(theme.config_label_selected)
    } else {
        Style::default().fg(theme.config_dim)
    };
    let display_w = UnicodeWidthStr::width(text);
    if display_w <= max_width {
        let padding = " ".repeat(max_width - display_w);
        Span::styled(format!("  {text}{padding}"), style)
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
        Span::styled(format!("  {truncated}...{padding}"), style)
    }
}

/// 值的样式（普通/选中/编辑中）
pub fn value_style(selected: bool, editing: bool, theme: &Theme) -> Style {
    if editing && selected {
        Style::default()
            .fg(theme.text_normal)
            .bg(theme.config_edit_bg)
    } else if selected {
        Style::default().fg(theme.text_normal)
    } else {
        Style::default().fg(theme.config_value)
    }
}
