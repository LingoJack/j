//! 行内光标组件
//!
//! 提供单行和多行折行光标渲染功能。

use crate::theme::Theme;
use crate::util::text::wrap_text;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// 构建带行内光标的 span 列表（单行）
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

/// 多行折行光标渲染结果
#[derive(Debug)]
pub struct WrappedCursorLines {
    /// 渲染后的行列表
    pub lines: Vec<Line<'static>>,
}

/// 将输入文本折行并在正确位置渲染光标
///
/// 返回折行后的行列表，每行中光标位置正确高亮显示。
///
/// # Arguments
/// * `input` - 输入文本
/// * `cursor_pos` - 光标位置（字符索引，从 0 开始）
/// * `width` - 每行最大宽度（用于折行）
/// * `placeholder` - 输入为空时显示的占位符（可选）
/// * `theme` - 主题样式
///
/// # Returns
/// `WrappedCursorLines` 包含渲染行信息
pub fn cursor_wrapped_lines(
    input: &str,
    cursor_pos: usize,
    width: usize,
    placeholder: Option<&str>,
    theme: &Theme,
) -> WrappedCursorLines {
    let cursor_style = Style::default().fg(theme.cursor_fg).bg(theme.cursor_bg);
    let text_style = Style::default().fg(theme.text_normal);
    let placeholder_style = Style::default().fg(theme.text_dim);

    // 空输入：显示占位符 + 光标
    if input.is_empty() {
        let placeholder_text = placeholder.unwrap_or("输入内容…");
        return WrappedCursorLines {
            lines: vec![Line::from(vec![
                Span::styled(" ", cursor_style),
                Span::styled(format!(" {}", placeholder_text), placeholder_style),
            ])],
        };
    }

    let wrapped = wrap_text(input, width);
    let mut char_offset = 0;
    let mut result = Vec::with_capacity(wrapped.len());
    let mut cursor_placed = false;

    for (line_idx, line_str) in wrapped.iter().enumerate() {
        let line_chars: Vec<char> = line_str.chars().collect();
        let line_len = line_chars.len();
        let is_last = line_idx == wrapped.len() - 1;

        // 判断光标是否在这一行
        let cursor_on_this_line = !cursor_placed
            && (cursor_pos < char_offset + line_len
                || (is_last && cursor_pos == char_offset + line_len));

        if cursor_on_this_line {
            cursor_placed = true;
            let pos_in_line = cursor_pos - char_offset;

            let before: String = line_chars[..pos_in_line].iter().collect();
            let (cursor_ch, after) = if pos_in_line < line_len {
                (
                    line_chars[pos_in_line].to_string(),
                    line_chars[pos_in_line + 1..].iter().collect::<String>(),
                )
            } else {
                (" ".to_string(), String::new())
            };

            result.push(Line::from(vec![
                Span::styled(before, text_style),
                Span::styled(cursor_ch, cursor_style),
                Span::styled(after, text_style),
            ]));
        } else {
            result.push(Line::from(Span::styled(line_str.clone(), text_style)));
        }

        char_offset += line_len;
    }

    WrappedCursorLines { lines: result }
}
