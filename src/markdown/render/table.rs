use crate::markdown::ir::{Inline, TableData};
use crate::markdown::theme::MdStyle;
use crate::util::text::{chars_with_display_width, display_width};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::inline::inline_display_width;

/// 渲染表格
pub fn render_table(
    data: &TableData,
    alignments: &[pulldown_cmark::Alignment],
    content_width: usize,
    theme: &dyn MdStyle,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    if data.rows.is_empty() {
        return lines;
    }

    let num_cols = data.rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if num_cols == 0 {
        return lines;
    }

    let mut col_widths: Vec<usize> = vec![0; num_cols];
    for row in &data.rows {
        for (i, cell) in row.iter().enumerate() {
            let w = inline_display_width(cell);
            if w > col_widths[i] {
                col_widths[i] = w;
            }
        }
    }

    // 列宽压缩逻辑
    let sep_w = num_cols + 1;
    let pad_w = num_cols * 2;
    let avail = content_width.saturating_sub(sep_w + pad_w);
    let max_col_w = avail * 2 / 3;
    for cw in col_widths.iter_mut() {
        if *cw > max_col_w {
            *cw = max_col_w;
        }
    }
    let total_col_w: usize = col_widths.iter().sum();
    if total_col_w > avail && total_col_w > 0 {
        let mut remaining = avail;
        for (i, cw) in col_widths.iter_mut().enumerate() {
            if i == num_cols - 1 {
                *cw = remaining.max(1);
            } else {
                *cw = ((*cw) * avail / total_col_w).max(1);
                remaining = remaining.saturating_sub(*cw);
            }
        }
    }

    let table_style = Style::default().fg(theme.table_body());
    let header_style = Style::default()
        .fg(theme.table_header())
        .add_modifier(Modifier::BOLD);
    let border_style = Style::default().fg(theme.text_dim());

    let total_col_w_final: usize = col_widths.iter().sum();
    let table_row_w = sep_w + pad_w + total_col_w_final;
    // 居中放置：把剩余宽度平分到左右两侧，形成左右 padding
    let total_extra = content_width.saturating_sub(table_row_w);
    let table_left_pad = total_extra / 2;
    let table_right_pad = total_extra - table_left_pad;

    let make_left_pad = || -> Option<Span<'static>> {
        if table_left_pad > 0 {
            Some(Span::raw(" ".repeat(table_left_pad)))
        } else {
            None
        }
    };

    // 顶边框 ┌─┬─┐
    let mut top = String::from("┌");
    for (i, cw) in col_widths.iter().enumerate() {
        top.push_str(&"─".repeat(cw + 2));
        if i < num_cols - 1 {
            top.push('┬');
        }
    }
    top.push('┐');
    let mut top_spans: Vec<Span> = Vec::new();
    if let Some(p) = make_left_pad() {
        top_spans.push(p);
    }
    top_spans.push(Span::styled(top, border_style));
    if table_right_pad > 0 {
        top_spans.push(Span::raw(" ".repeat(table_right_pad)));
    }
    lines.push(Line::from(top_spans));

    let code_style = Style::default()
        .fg(theme.md_inline_code_fg())
        .bg(theme.bg_primary());

    for (row_idx, row) in data.rows.iter().enumerate() {
        let base_style = if row_idx == 0 {
            header_style
        } else {
            table_style
        };

        // 对每个单元格按显示宽度折行
        let wrapped_cells: Vec<Vec<(Vec<Span<'static>>, usize)>> = col_widths
            .iter()
            .enumerate()
            .map(|(i, cw)| {
                wrap_cell_inlines(
                    row.get(i).map(|v| v.as_slice()).unwrap_or(&[]),
                    *cw,
                    base_style,
                    code_style,
                    theme,
                )
            })
            .collect();

        let max_rows = wrapped_cells.iter().map(|r| r.len()).max().unwrap_or(1);

        for sub_row in 0..max_rows {
            let mut row_spans: Vec<Span> = Vec::new();
            if let Some(p) = make_left_pad() {
                row_spans.push(p);
            }
            row_spans.push(Span::styled("│", border_style));
            for (i, cw) in col_widths.iter().enumerate() {
                let empty_line: (Vec<Span<'static>>, usize) = (Vec::new(), 0);
                let (mut cell_spans, _cell_line_w) = wrapped_cells
                    .get(i)
                    .and_then(|lines| lines.get(sub_row))
                    .cloned()
                    .unwrap_or(empty_line);

                // 单元格内容截断逻辑
                let mut actual_w: usize = cell_spans
                    .iter()
                    .map(|s| {
                        chars_with_display_width(&s.content)
                            .map(|(_, w)| w)
                            .sum::<usize>()
                    })
                    .sum();
                if actual_w > *cw {
                    let mut truncated = Vec::new();
                    let mut w = 0;
                    for span in cell_spans {
                        let span_w: usize = chars_with_display_width(&span.content)
                            .map(|(_, w)| w)
                            .sum();
                        if w + span_w <= *cw {
                            w += span_w;
                            truncated.push(span);
                        } else {
                            let remain = *cw - w;
                            let mut buf = String::new();
                            let mut bw = 0;
                            for (ch, chw) in chars_with_display_width(&span.content) {
                                if bw + chw > remain {
                                    break;
                                }
                                buf.push(ch);
                                bw += chw;
                            }
                            if !buf.is_empty() {
                                truncated.push(Span::styled(buf, span.style));
                                w += bw;
                            }
                            break;
                        }
                    }
                    cell_spans = truncated;
                    actual_w = w;
                }
                let fill = cw.saturating_sub(actual_w);
                let align = alignments
                    .get(i)
                    .copied()
                    .unwrap_or(pulldown_cmark::Alignment::None);
                let (left_pad, right_pad) = match align {
                    pulldown_cmark::Alignment::Center => {
                        let left = fill / 2;
                        (left, fill - left)
                    }
                    pulldown_cmark::Alignment::Right => (fill, 0),
                    _ => (0, fill),
                };
                row_spans.push(Span::styled(
                    format!(" {}", " ".repeat(left_pad)),
                    base_style,
                ));
                row_spans.extend(cell_spans);
                row_spans.push(Span::styled(
                    format!("{} ", " ".repeat(right_pad)),
                    base_style,
                ));
                row_spans.push(Span::styled("│", border_style));
            }
            if table_right_pad > 0 {
                row_spans.push(Span::raw(" ".repeat(table_right_pad)));
            }
            lines.push(Line::from(row_spans));
        }

        // 行间分隔线
        if row_idx < data.rows.len() - 1 {
            let mut sep = String::from("├");
            for (i, cw) in col_widths.iter().enumerate() {
                sep.push_str(&"─".repeat(cw + 2));
                if i < num_cols - 1 {
                    sep.push('┼');
                }
            }
            sep.push('┤');
            let mut sep_spans: Vec<Span> = Vec::new();
            if let Some(p) = make_left_pad() {
                sep_spans.push(p);
            }
            sep_spans.push(Span::styled(sep, border_style));
            if table_right_pad > 0 {
                sep_spans.push(Span::raw(" ".repeat(table_right_pad)));
            }
            lines.push(Line::from(sep_spans));
        }
    }

    // 底边框 └─┴─┘
    let mut bottom = String::from("└");
    for (i, cw) in col_widths.iter().enumerate() {
        bottom.push_str(&"─".repeat(cw + 2));
        if i < num_cols - 1 {
            bottom.push('┴');
        }
    }
    bottom.push('┘');
    let mut bottom_spans: Vec<Span> = Vec::new();
    if let Some(p) = make_left_pad() {
        bottom_spans.push(p);
    }
    bottom_spans.push(Span::styled(bottom, border_style));
    if table_right_pad > 0 {
        bottom_spans.push(Span::raw(" ".repeat(table_right_pad)));
    }
    lines.push(Line::from(bottom_spans));

    lines
}

/// 按显示宽度对 inline 元素列表折行。
/// 返回每个子行的 (spans, 显示宽度)。
pub fn wrap_cell_inlines(
    inlines: &[Inline],
    max_width: usize,
    base_style: Style,
    code_style: Style,
    theme: &dyn MdStyle,
) -> Vec<(Vec<Span<'static>>, usize)> {
    // 最小宽度保证至少能放一个宽字符
    let max_width = max_width.max(2);

    // 先将所有 inline 渲染为 span 片段
    let pieces = inlines_to_cell_pieces(inlines, base_style, code_style, theme);

    let mut lines: Vec<(Vec<Span<'static>>, usize)> = Vec::new();
    let mut cur_line: Vec<Span<'static>> = Vec::new();
    let mut cur_w: usize = 0;
    let mut cur_buf: String = String::new();
    let mut cur_style: Style = base_style;

    for (text, style) in pieces {
        if !cur_buf.is_empty() && style != cur_style {
            cur_line.push(Span::styled(std::mem::take(&mut cur_buf), cur_style));
        }
        cur_style = style;
        for (ch, cw) in chars_with_display_width(&text) {
            if ch == '\n' {
                if !cur_buf.is_empty() {
                    cur_line.push(Span::styled(std::mem::take(&mut cur_buf), cur_style));
                }
                lines.push((std::mem::take(&mut cur_line), cur_w));
                cur_w = 0;
                continue;
            }
            if cur_w + cw > max_width && cur_w > 0 {
                if !cur_buf.is_empty() {
                    cur_line.push(Span::styled(std::mem::take(&mut cur_buf), cur_style));
                }
                lines.push((std::mem::take(&mut cur_line), cur_w));
                cur_w = 0;
            }
            cur_buf.push(ch);
            cur_w += cw;
        }
    }
    if !cur_buf.is_empty() {
        cur_line.push(Span::styled(cur_buf, cur_style));
    }
    if !cur_line.is_empty() || lines.is_empty() {
        lines.push((cur_line, cur_w));
    }
    lines
}

/// 将 inline 元素列表转换为 (text, style) 片段
fn inlines_to_cell_pieces(
    inlines: &[Inline],
    base_style: Style,
    code_style: Style,
    _theme: &dyn MdStyle,
) -> Vec<(String, Style)> {
    let mut pieces = Vec::new();
    for inline in inlines {
        inline_to_cell_pieces_recursive(inline, base_style, code_style, &mut pieces);
    }
    pieces
}

fn inline_to_cell_pieces_recursive(
    inline: &Inline,
    base_style: Style,
    code_style: Style,
    out: &mut Vec<(String, Style)>,
) {
    match inline {
        Inline::Text(s) => {
            out.push((s.clone(), base_style));
        }
        Inline::Code(s) => {
            out.push((s.clone(), code_style));
        }
        Inline::Strong(children) => {
            let style = base_style.add_modifier(Modifier::BOLD);
            for child in children {
                inline_to_cell_pieces_recursive(child, style, code_style, out);
            }
        }
        Inline::Emphasis(children) => {
            let style = base_style.add_modifier(Modifier::ITALIC);
            for child in children {
                inline_to_cell_pieces_recursive(child, style, code_style, out);
            }
        }
        Inline::Strikethrough(children) => {
            let style = base_style.add_modifier(Modifier::CROSSED_OUT);
            for child in children {
                inline_to_cell_pieces_recursive(child, style, code_style, out);
            }
        }
        Inline::Link { text, .. } => {
            let style = base_style.add_modifier(Modifier::UNDERLINED);
            for child in text {
                inline_to_cell_pieces_recursive(child, style, code_style, out);
            }
        }
        Inline::SoftBreak => {
            out.push((" ".to_string(), base_style));
        }
        Inline::HardBreak => {
            out.push(("\n".to_string(), base_style));
        }
        Inline::Image { alt, url } => {
            // 终端表格中图片退化为 [图片: alt](url) 文本占位
            let placeholder = if alt.is_empty() {
                format!("[图片]({})", url)
            } else {
                format!("[图片: {}]({})", alt, url)
            };
            out.push((placeholder, base_style.add_modifier(Modifier::DIM)));
        }
    }
}

/// 仅计算单元格内容按 `max_width` 折行后产出多少子行（不构造 Span，等价于
/// `wrap_cell_inlines(...).len()`，但避免分配）。
///
/// 必须与 `wrap_cell_inlines` 的折行规则保持一致：硬换行立即起新行；当下一个字符
/// 加入会超出 `max_width` 且当前行非空时换行；空内容仍算 1 行。
pub fn measure_cell_wrap_lines(inlines: &[Inline], max_width: usize) -> usize {
    let max_width = max_width.max(2);

    fn collect_text(inline: &Inline, out: &mut String) {
        match inline {
            Inline::Text(s) | Inline::Code(s) => {
                out.push_str(s);
            }
            Inline::Strong(children)
            | Inline::Emphasis(children)
            | Inline::Strikethrough(children) => {
                for child in children {
                    collect_text(child, out);
                }
            }
            Inline::Link { text, .. } => {
                for child in text {
                    collect_text(child, out);
                }
            }
            Inline::Image { alt, url } => {
                // 与 inline_to_cell_pieces_recursive 占位保持一致
                if alt.is_empty() {
                    out.push_str(&format!("[图片]({})", url));
                } else {
                    out.push_str(&format!("[图片: {}]({})", alt, url));
                }
            }
            Inline::SoftBreak => out.push(' '),
            Inline::HardBreak => out.push('\n'),
        }
    }

    let mut text = String::new();
    for inline in inlines {
        collect_text(inline, &mut text);
    }

    let mut lines: usize = 0;
    let mut cur_w: usize = 0;
    let mut cur_has_content: bool = false;

    for (ch, cw) in chars_with_display_width(&text) {
        if ch == '\n' {
            // wrap_cell_inlines 中 '\n' 始终把当前行 push 入 lines（即便为空），
            // 这里复刻同样行为。
            lines += 1;
            cur_w = 0;
            cur_has_content = false;
            continue;
        }
        if cur_w + cw > max_width && cur_w > 0 {
            lines += 1;
            cur_w = 0;
        }
        cur_w += cw;
        cur_has_content = true;
    }
    if cur_has_content || lines == 0 {
        lines += 1;
    }
    lines
}

/// 仅测量整个表格在给定 `content_width` 下的渲染高度（行数），不分配 Span。
///
/// 计算公式：
///   1（顶边框）
/// + Σ 每行 max(1, max_rows_of_wrapped_cells)
/// + (rows - 1)（行间分隔线）
/// + 1（底边框）
///
/// 列宽计算逻辑必须与 `render_table` 一致——这是高度准确性的唯一前提。
pub fn measure_table_height(data: &TableData, content_width: usize) -> usize {
    if data.rows.is_empty() {
        return 0;
    }
    let num_cols = data.rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if num_cols == 0 {
        return 0;
    }

    // 列宽：与 render_table 完全同步
    let mut col_widths: Vec<usize> = vec![0; num_cols];
    for row in &data.rows {
        for (i, cell) in row.iter().enumerate() {
            let w = inline_display_width(cell);
            if w > col_widths[i] {
                col_widths[i] = w;
            }
        }
    }

    let sep_w = num_cols + 1;
    let pad_w = num_cols * 2;
    let avail = content_width.saturating_sub(sep_w + pad_w);
    let max_col_w = avail * 2 / 3;
    for cw in col_widths.iter_mut() {
        if *cw > max_col_w {
            *cw = max_col_w;
        }
    }
    let total_col_w: usize = col_widths.iter().sum();
    if total_col_w > avail && total_col_w > 0 {
        let mut remaining = avail;
        for (i, cw) in col_widths.iter_mut().enumerate() {
            if i == num_cols - 1 {
                *cw = remaining.max(1);
            } else {
                *cw = ((*cw) * avail / total_col_w).max(1);
                remaining = remaining.saturating_sub(*cw);
            }
        }
    }

    // 顶边框 + 底边框
    let mut height: usize = 2;
    let row_count = data.rows.len();

    for (row_idx, row) in data.rows.iter().enumerate() {
        let max_sub = col_widths
            .iter()
            .enumerate()
            .map(|(i, cw)| {
                let cell = row.get(i).map(|v| v.as_slice()).unwrap_or(&[]);
                measure_cell_wrap_lines(cell, *cw)
            })
            .max()
            .unwrap_or(1)
            .max(1);
        height += max_sub;

        if row_idx < row_count - 1 {
            height += 1; // 行间分隔线
        }
    }

    height
}

/// 计算 inline 元素列表的显示宽度（用于列宽计算）
#[allow(dead_code)]
pub fn display_width_inlines(inlines: &[Inline]) -> usize {
    let mut width = 0;
    for inline in inlines {
        match inline {
            Inline::Text(s) => width += display_width(s),
            Inline::Code(s) => width += display_width(s),
            Inline::Strong(children)
            | Inline::Emphasis(children)
            | Inline::Strikethrough(children) => width += display_width_inlines(children),
            Inline::SoftBreak => width += 1,
            Inline::HardBreak => {}
            Inline::Link { text, .. } => width += display_width_inlines(text),
            Inline::Image { alt, url } => {
                // 与文本占位宽度一致
                width += if alt.is_empty() {
                    display_width(&format!("[图片]({})", url))
                } else {
                    display_width(&format!("[图片: {}]({})", alt, url))
                };
            }
        }
    }
    width
}
