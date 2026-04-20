//! 状态栏输入框组件
//!
//! 提供统一的状态栏输入框渲染，用于 todo / notebook 等模块。

use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// 状态栏输入框参数
pub struct StatusInputParams<'a> {
    /// 标签文本（如 " 新建 ", " 重命名 "）
    pub label: &'a str,
    /// 标签颜色
    pub label_color: ratatui::style::Color,
    /// 输入文本
    pub input: &'a str,
    /// 光标位置（字符索引）
    pub cursor_pos: usize,
    /// 占位符文本（输入为空时显示）
    pub placeholder: &'a str,
    /// 提示文本（右侧灰色显示）
    pub hint: &'a str,
}

/// 在状态栏区域渲染输入框（带标签、文本、光标、占位符）
///
/// 渲染一个带边框的单行输入框，标签在左侧，输入文本在中间，提示在右侧。
/// 同时设置终端光标位置以支持光标闪烁。
pub fn draw_status_input(f: &mut Frame, area: Rect, params: &StatusInputParams<'_>, theme: &Theme) {
    let cursor_style = Style::default().fg(theme.cursor_fg).bg(theme.cursor_bg);
    let text_style = Style::default().fg(theme.text_normal);
    let placeholder_style = Style::default().fg(theme.text_dim);
    let hint_style = Style::default().fg(theme.text_dim);

    let label_display = format!(" {} ", params.label);
    let label_width = unicode_width::UnicodeWidthStr::width(label_display.as_str());

    let mut spans = vec![Span::styled(
        label_display,
        Style::default()
            .fg(params.label_color)
            .add_modifier(Modifier::BOLD),
    )];

    if params.input.is_empty() {
        // 空输入：显示占位符 + 闪烁光标
        spans.push(Span::styled(" ", cursor_style));
        spans.push(Span::styled(
            params.placeholder.to_string(),
            placeholder_style,
        ));
    } else {
        // 有输入：逐字符渲染，光标位置高亮
        let input_chars: Vec<char> = params.input.chars().collect();
        for (i, ch) in input_chars.iter().enumerate() {
            if i == params.cursor_pos {
                spans.push(Span::styled(ch.to_string(), cursor_style));
            } else {
                spans.push(Span::styled(ch.to_string(), text_style));
            }
        }
        if params.cursor_pos >= input_chars.len() {
            spans.push(Span::styled(" ", cursor_style));
        }
    }

    // 添加提示
    spans.push(Span::styled(format!("  {}", params.hint), hint_style));

    let status = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(params.label_color)),
    );
    f.render_widget(status, area);

    // 设置终端光标位置
    let cursor_x_offset = if params.input.is_empty() {
        0
    } else {
        let chars_before_cursor: String = params.input.chars().take(params.cursor_pos).collect();
        unicode_width::UnicodeWidthStr::width(chars_before_cursor.as_str())
    };
    let cursor_x = area.x + 1 + label_width as u16 + cursor_x_offset as u16;
    let cursor_y = area.y + 1;
    if cursor_x < area.x + area.width.saturating_sub(1) && cursor_y < area.y + area.height {
        f.set_cursor_position((cursor_x, cursor_y));
    }
}
