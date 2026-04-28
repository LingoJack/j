//! Span 选区高亮工具
//!
//! 提供精确到字符级别的选区高亮功能，供 Markdown 编辑器和 Chat UI 复用。

use ratatui::style::{Color, Style};
use ratatui::text::Span;

/// 选区样式上下文，用于减少辅助函数的参数数量。
pub(crate) struct SelectionStyle {
    normal: Style,
    selected: Style,
    local_start: usize,
    local_end: usize,
}

/// 对已渲染的 spans 列表应用选区高亮（精确到字符级别）。
///
/// # 参数
///
/// - `spans`: 原始 spans 列表
/// - `skip_chars`: 开头跳过的字符数（如 Markdown 编辑器的行号，Chat UI 传 0）
/// - `local_start` / `local_end`: 内容部分的字符偏移（0-based, exclusive end）
/// - `sel_fg` / `sel_bg`: 选区的文字色和背景色
pub fn rebuild_spans_with_selection(
    spans: &[Span<'static>],
    skip_chars: usize,
    local_start: usize,
    local_end: usize,
    sel_fg: Color,
    sel_bg: Color,
) -> Vec<Span<'static>> {
    let ss = SelectionStyle {
        normal: Style::default(),
        selected: Style::default().fg(sel_fg).bg(sel_bg),
        local_start,
        local_end,
    };
    let mut result = Vec::with_capacity(spans.len() + 4);
    let mut chars_seen = 0usize;

    for span in spans {
        let span_chars: Vec<char> = span.content.chars().collect();
        let span_len = span_chars.len();
        let span_end = chars_seen + span_len;

        // 跳过 skip_chars 区域（如行号）
        if span_end <= skip_chars {
            result.push(span.clone());
            chars_seen = span_end;
            continue;
        }

        // 当前 span 跨越 skip_chars 边界，需分割
        if chars_seen < skip_chars && span_end > skip_chars {
            let skip_part_len = skip_chars - chars_seen;
            let skip_text: String = span_chars[..skip_part_len].iter().collect();
            result.push(Span::styled(skip_text, span.style));

            // 剩余内容作为新 span 处理
            let content_chars = &span_chars[skip_part_len..];
            let content_len = content_chars.len();
            // 相对于内容起始的偏移（内容部分从 0 开始计算）
            let c_start = 0usize;
            let c_end = content_len;
            let content_ss = SelectionStyle {
                normal: span.style,
                ..ss
            };
            append_content_spans(content_chars, c_start, c_end, &content_ss, &mut result);
            chars_seen = span_end;
            continue;
        }

        // 纯内容 span
        let content_offset = chars_seen - skip_chars;
        let c_start = content_offset;
        let c_end = content_offset + span_len;
        let content_ss = SelectionStyle {
            normal: span.style,
            ..ss
        };
        append_content_spans(&span_chars, c_start, c_end, &content_ss, &mut result);
        chars_seen = span_end;
    }

    result
}

/// 将内容 span 按 `[local_start, local_end)` 选区范围分割并附加到 result。
fn append_content_spans(
    chars: &[char],
    c_start: usize,
    c_end: usize,
    ss: &SelectionStyle,
    result: &mut Vec<Span<'static>>,
) {
    let SelectionStyle {
        normal,
        selected,
        local_start,
        local_end,
    } = *ss;

    // 无交集
    if c_end <= local_start || c_start >= local_end {
        let text: String = chars.iter().collect();
        result.push(Span::styled(text, normal));
        return;
    }

    // 选中前的部分
    if c_start < local_start {
        let before_len = local_start - c_start;
        let text: String = chars[..before_len].iter().collect();
        result.push(Span::styled(text, normal));
    }

    // 选中的部分
    {
        let sel_begin = local_start.saturating_sub(c_start);
        let sel_finish = local_end.min(c_end).saturating_sub(c_start);
        if sel_begin < sel_finish && sel_finish <= chars.len() {
            let text: String = chars[sel_begin..sel_finish].iter().collect();
            result.push(Span::styled(text, selected));
        }
    }

    // 选中后的部分
    if c_end > local_end {
        let after_begin = local_end.saturating_sub(c_start);
        if after_begin < chars.len() {
            let text: String = chars[after_begin..].iter().collect();
            result.push(Span::styled(text, normal));
        }
    }
}
