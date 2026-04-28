use super::ParserState;
use crate::util::text::{char_width, display_width};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

// ---------------------------------------------------------------------------
// ParserState table-related methods
// ---------------------------------------------------------------------------

impl<'a> ParserState<'a> {
    /// Handle `Event::Start(Tag::Table(alignments))`.
    pub(crate) fn handle_table_start(&mut self, alignments: Vec<pulldown_cmark::Alignment>) {
        self.flush_line();
        self.in_table = true;
        self.table_rows.clear();
        self.table_alignments = alignments;
    }

    /// Handle `Event::End(TagEnd::Table)`.
    pub(crate) fn handle_table_end(&mut self) {
        self.flush_line();
        self.in_table = false;

        if self.table_rows.is_empty() {
            self.table_alignments.clear();
            return;
        }

        let num_cols = self.table_rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if num_cols == 0 {
            self.table_rows.clear();
            self.table_alignments.clear();
            return;
        }

        let mut col_widths: Vec<usize> = vec![0; num_cols];
        for row in &self.table_rows {
            for (i, cell) in row.iter().enumerate() {
                let w = display_width_cell(cell);
                if w > col_widths[i] {
                    col_widths[i] = w;
                }
            }
        }

        // 列宽压缩逻辑
        let sep_w = num_cols + 1;
        let pad_w = num_cols * 2;
        let avail = self.content_width.saturating_sub(sep_w + pad_w);
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

        let table_style = Style::default().fg(self.theme.table_body());
        let header_style = Style::default()
            .fg(self.theme.table_header())
            .add_modifier(Modifier::BOLD);
        let border_style = Style::default().fg(self.theme.text_dim());

        let total_col_w_final: usize = col_widths.iter().sum();
        let table_row_w = sep_w + pad_w + total_col_w_final;
        let table_right_pad = self.content_width.saturating_sub(table_row_w);

        // 渲染顶边框 ┌─┬─┐
        let mut top = String::from("┌");
        for (i, cw) in col_widths.iter().enumerate() {
            top.push_str(&"─".repeat(cw + 2));
            if i < num_cols - 1 {
                top.push('┬');
            }
        }
        top.push('┐');
        let mut top_spans = vec![Span::styled(top, border_style)];
        if table_right_pad > 0 {
            top_spans.push(Span::raw(" ".repeat(table_right_pad)));
        }
        self.lines.push(Line::from(top_spans));

        for (row_idx, row) in self.table_rows.iter().enumerate() {
            let base_style = if row_idx == 0 {
                header_style
            } else {
                table_style
            };
            let code_style = Style::default()
                .fg(self.theme.md_inline_code_fg())
                .bg(self.theme.md_inline_code_bg());

            // 对每个单元格按显示宽度折行，保留行内代码样式
            let wrapped_cells: Vec<Vec<(Vec<Span<'static>>, usize)>> = col_widths
                .iter()
                .enumerate()
                .map(|(i, cw)| {
                    let cell_text = row.get(i).map(|s| s.as_str()).unwrap_or("");
                    wrap_cell_styled(cell_text, *cw, base_style, code_style)
                })
                .collect();

            let max_rows = wrapped_cells.iter().map(|r| r.len()).max().unwrap_or(1);

            for sub_row in 0..max_rows {
                let mut row_spans: Vec<Span> = Vec::new();
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
                        .map(|s| s.content.chars().map(char_width).sum::<usize>())
                        .sum();
                    if actual_w > *cw {
                        let mut truncated = Vec::new();
                        let mut w = 0;
                        for span in cell_spans {
                            let span_w: usize = span.content.chars().map(char_width).sum();
                            if w + span_w <= *cw {
                                w += span_w;
                                truncated.push(span);
                            } else {
                                let remain = *cw - w;
                                let mut buf = String::new();
                                let mut bw = 0;
                                for ch in span.content.chars() {
                                    let chw = char_width(ch);
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
                    let align = self
                        .table_alignments
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
                self.lines.push(Line::from(row_spans));
            }

            // 行间分隔线
            if row_idx < self.table_rows.len() - 1 {
                let mut sep = String::from("├");
                for (i, cw) in col_widths.iter().enumerate() {
                    sep.push_str(&"─".repeat(cw + 2));
                    if i < num_cols - 1 {
                        sep.push('┼');
                    }
                }
                sep.push('┤');
                let mut sep_spans = vec![Span::styled(sep, border_style)];
                if table_right_pad > 0 {
                    sep_spans.push(Span::raw(" ".repeat(table_right_pad)));
                }
                self.lines.push(Line::from(sep_spans));
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
        let mut bottom_spans = vec![Span::styled(bottom, border_style)];
        if table_right_pad > 0 {
            bottom_spans.push(Span::raw(" ".repeat(table_right_pad)));
        }
        self.lines.push(Line::from(bottom_spans));

        self.table_rows.clear();
        self.table_alignments.clear();
    }

    /// Handle `Event::Start(Tag::TableHead)`.
    pub(crate) fn handle_table_head_start(&mut self) {
        self.current_row.clear();
    }

    /// Handle `Event::End(TagEnd::TableHead)`.
    pub(crate) fn handle_table_head_end(&mut self) {
        self.table_rows.push(self.current_row.clone());
        self.current_row.clear();
    }

    /// Handle `Event::Start(Tag::TableRow)`.
    pub(crate) fn handle_table_row_start(&mut self) {
        self.current_row.clear();
    }

    /// Handle `Event::End(TagEnd::TableRow)`.
    pub(crate) fn handle_table_row_end(&mut self) {
        self.table_rows.push(self.current_row.clone());
        self.current_row.clear();
    }

    /// Handle `Event::Start(Tag::TableCell)`.
    pub(crate) fn handle_table_cell_start(&mut self) {
        self.current_cell.clear();
    }

    /// Handle `Event::End(TagEnd::TableCell)`.
    pub(crate) fn handle_table_cell_end(&mut self) {
        self.current_row.push(self.current_cell.clone());
        self.current_cell.clear();
    }

    /// Handle inline code when inside a table cell.
    pub(crate) fn handle_code_in_table(&mut self, text: &str) {
        self.current_cell.push('`');
        self.current_cell.push_str(text);
        self.current_cell.push('`');
    }

    /// Handle soft break inside a table cell.
    pub(crate) fn handle_soft_break_in_table(&mut self) {
        self.current_cell.push(' ');
    }

    /// Handle hard break inside a table cell.
    pub(crate) fn handle_hard_break_in_table(&mut self) {
        self.current_cell.push(' ');
    }
}

// ---------------------------------------------------------------------------
// Private helper functions for table cell rendering
// ---------------------------------------------------------------------------

/// 将单元格拆成 (text, style) 片段：配对反引号内的是行内代码样式，其余为 base 样式。
/// 反引号作为标记被剥离，不进入返回文本。未配对的反引号保留为普通文本。
pub(crate) fn cell_to_pieces(cell: &str, base: Style, code: Style) -> Vec<(String, Style)> {
    let mut out = Vec::new();
    let mut remaining = cell;
    while !remaining.is_empty() {
        if let Some(s) = remaining.find('`') {
            if s > 0 {
                out.push((remaining[..s].to_string(), base));
            }
            let after = &remaining[s + 1..];
            if let Some(e) = after.find('`') {
                if e > 0 {
                    out.push((after[..e].to_string(), code));
                }
                remaining = &after[e + 1..];
            } else {
                out.push((remaining[s..].to_string(), base));
                break;
            }
        } else {
            out.push((remaining.to_string(), base));
            break;
        }
    }
    out
}

/// 按显示宽度对单元格折行，保留行内代码样式。
/// 返回每个子行的 (spans, 显示宽度)。
pub(crate) fn wrap_cell_styled(
    cell: &str,
    max_width: usize,
    base: Style,
    code: Style,
) -> Vec<(Vec<Span<'static>>, usize)> {
    // IMPORTANT: 这里将 max_width 提升到至少 2，是为了让宽字符（如中文，宽度=2）
    // 至少能放一个字符，避免死循环（一个字符都放不下时无法折行）。
    // 但这会导致返回的子行宽度可能超过调用方传入的 max_width（即 col_widths[i]）。
    // 渲染层必须对此做截断处理，见渲染层的 cell_spans 截断逻辑。
    let max_width = max_width.max(2);
    let pieces = cell_to_pieces(cell, base, code);

    let mut lines: Vec<(Vec<Span<'static>>, usize)> = Vec::new();
    let mut cur_line: Vec<Span<'static>> = Vec::new();
    let mut cur_w: usize = 0;
    let mut cur_buf: String = String::new();
    let mut cur_style: Style = base;

    for (text, style) in pieces {
        if !cur_buf.is_empty() && style != cur_style {
            cur_line.push(Span::styled(std::mem::take(&mut cur_buf), cur_style));
        }
        cur_style = style;
        for ch in text.chars() {
            if ch == '\n' {
                if !cur_buf.is_empty() {
                    cur_line.push(Span::styled(std::mem::take(&mut cur_buf), cur_style));
                }
                lines.push((std::mem::take(&mut cur_line), cur_w));
                cur_w = 0;
                continue;
            }
            let cw = char_width(ch);
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

/// 计算表格单元格文本的显示宽度，扣除行内代码标记反引号的宽度。
pub(crate) fn display_width_cell(cell: &str) -> usize {
    let mut width = 0;
    let mut remaining = cell;
    while !remaining.is_empty() {
        if let Some(start) = remaining.find('`') {
            width += display_width(&remaining[..start]);
            let after_tick = &remaining[start + 1..];
            if let Some(end) = after_tick.find('`') {
                width += display_width(&after_tick[..end]);
                remaining = &after_tick[end + 1..];
            } else {
                width += display_width(&remaining[start..]);
                break;
            }
        } else {
            width += display_width(remaining);
            break;
        }
    }
    width
}
