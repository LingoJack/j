//! 指针组件

use crate::theme::Theme;
use ratatui::{style::Style, text::Span};

use super::consts::{POINTER_EMPTY, POINTER_SELECTED};

/// 选中指针 span
pub fn pointer_span<'a>(selected: bool, theme: &Theme) -> Span<'a> {
    if selected {
        Span::styled(POINTER_SELECTED, Style::default().fg(theme.config_pointer))
    } else {
        Span::styled(POINTER_EMPTY, Style::default())
    }
}
