//! 气泡布局工具：将渲染行包装为气泡样式

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::util::text::{char_width, display_width};

/// 将一行 Markdown 渲染行包装为气泡样式
///
/// 根据 `bubble_total_w` 在内容两侧填充背景色空格，并对超宽内容进行逐字符截断。
/// 若内容以 `\x00IMG:` 开头，则渲染为纯背景行并在行末保留图片标记。
pub fn wrap_md_line_in_bubble(
    md_line: Line<'static>,
    bubble_bg: Color,
    pad_left_w: usize,
    pad_right_w: usize,
    bubble_total_w: usize,
) -> Line<'static> {
    // 图片标记行：渲染为纯气泡背景空行，标记信息附加在末尾（不影响可见区域）
    for span in &md_line.spans {
        if span.content.starts_with("\x00IMG:") {
            let marker = span.content.clone();
            let spans: Vec<Span> = vec![
                // 整行用气泡背景色空格填充（与占位行一致）
                Span::styled(" ".repeat(bubble_total_w), Style::default().bg(bubble_bg)),
                // 标记信息附加在行末（超出可见区域，渲染 pass 通过扫描 spans 识别）
                Span::styled(marker, Style::default()),
            ];
            return Line::from(spans);
        }
    }
    let pad_left = " ".repeat(pad_left_w);
    let pad_right = " ".repeat(pad_right_w);
    let mut styled_spans: Vec<Span> = Vec::new();
    styled_spans.push(Span::styled(pad_left, Style::default().bg(bubble_bg)));
    let target_content_w = bubble_total_w.saturating_sub(pad_left_w + pad_right_w);
    let mut content_w: usize = 0;
    for span in md_line.spans {
        let sw = display_width(&span.content);
        if content_w + sw > target_content_w {
            // 安全钳制：逐字符截断以适应目标宽度
            let remaining = target_content_w.saturating_sub(content_w);
            if remaining > 0 {
                let mut truncated = String::new();
                let mut tw = 0;
                for ch in span.content.chars() {
                    let cw = char_width(ch);
                    if tw + cw > remaining {
                        break;
                    }
                    truncated.push(ch);
                    tw += cw;
                }
                if !truncated.is_empty() {
                    content_w += tw;
                    let merged_style = span.style.bg(bubble_bg);
                    styled_spans.push(Span::styled(truncated, merged_style));
                }
            }
            // 跳过后续 span（已溢出）
            break;
        }
        content_w += sw;
        let merged_style = span.style.bg(bubble_bg);
        styled_spans.push(Span::styled(span.content.to_string(), merged_style));
    }
    let fill = target_content_w.saturating_sub(content_w);
    if fill > 0 {
        styled_spans.push(Span::styled(
            " ".repeat(fill),
            Style::default().bg(bubble_bg),
        ));
    }
    styled_spans.push(Span::styled(pad_right, Style::default().bg(bubble_bg)));
    Line::from(styled_spans)
}

/// 带左边距的气泡行包装（用于 assistant 消息气泡）
/// 在 `wrap_md_line_in_bubble` 基础上，在行首插入一个无背景色的 margin span
pub(crate) fn wrap_md_line_in_bubble_with_margin(
    md_line: Line<'static>,
    bubble_bg: Color,
    pad_left_w: usize,
    pad_right_w: usize,
    bubble_total_w: usize,
    margin: &str,
) -> Line<'static> {
    // 先用原始函数生成气泡行
    let mut line =
        wrap_md_line_in_bubble(md_line, bubble_bg, pad_left_w, pad_right_w, bubble_total_w);
    // 在最前面插入 margin span（无背景色，透出消息区背景）
    line.spans
        .insert(0, Span::styled(margin.to_string(), Style::default()));
    line
}

/// 带边框的气泡行渲染（用于工具调用块等）
///
/// 左边框 `"  │ "` 占 4 列，右边框 `" │"` 占 2 列，中间内容按 `bubble_max_width` 钳制。
pub(crate) fn bordered_line(
    content_spans: Vec<Span<'static>>,
    bubble_max_width: usize,
    border_color: Color,
    bg: Color,
) -> Line<'static> {
    // 左边框 "  │ " 占 4 列，右边框 " │" 占 2 列
    let border_overhead = 4 + 2;
    let target_content_w = bubble_max_width.saturating_sub(border_overhead);

    // 溢出钳制：逐 span 逐字符截断，确保内容不超出目标宽度
    let mut clamped_spans: Vec<Span<'static>> = Vec::with_capacity(content_spans.len());
    let mut used: usize = 0;
    for span in content_spans {
        let sw = display_width(&span.content);
        if used + sw <= target_content_w {
            used += sw;
            clamped_spans.push(span);
        } else {
            // 当前 span 需要截断
            let remaining = target_content_w.saturating_sub(used);
            if remaining > 0 {
                let mut truncated = String::new();
                let mut tw = 0;
                for ch in span.content.chars() {
                    let cw = char_width(ch);
                    if tw + cw > remaining {
                        break;
                    }
                    truncated.push(ch);
                    tw += cw;
                }
                if !truncated.is_empty() {
                    used += tw;
                    clamped_spans.push(Span::styled(truncated, span.style));
                }
            }
            // 后续 span 全部跳过（已溢出）
            break;
        }
    }

    let fill = target_content_w.saturating_sub(used);

    let mut spans = Vec::with_capacity(clamped_spans.len() + 3);
    spans.push(Span::styled(
        "  │ ",
        Style::default().fg(border_color).bg(bg),
    ));
    spans.extend(clamped_spans);
    spans.push(Span::styled(" ".repeat(fill), Style::default().bg(bg)));
    spans.push(Span::styled(" │", Style::default().fg(border_color).bg(bg)));
    Line::from(spans)
}
