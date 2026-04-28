//! 表格渲染子模块

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::util::text::{char_width, display_width, wrap_text};

use super::MarkdownRenderer;

/// 表格对齐方式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableAlign {
    Left,
    Center,
    Right,
}

/// 表格边框类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum BorderKind {
    Top,
    Middle,
    Bottom,
}

/// 表格上下文
#[derive(Debug, Clone)]
pub struct TableContext {
    pub start_idx: usize,
    pub end_idx: usize,
    pub col_widths: Vec<usize>,
    pub alignments: Vec<TableAlign>,
}

impl MarkdownRenderer {
    // ========== 表格处理 ==========

    /// 计算表格行的总渲染宽度（包含边框）
    pub(super) fn calculate_table_render_width_from_col_widths(col_widths: &[usize]) -> usize {
        // 每列：│ + 空格 + 内容 + 空格 = 1 + cw + 2，最后一列加 │
        let cols_width: usize = col_widths.iter().map(|cw| cw + 2 + 1).sum();
        cols_width + 1 // 最左边的 │
    }

    /// 将 col_widths 缩小以适配 wrap_width（按比例缩减）
    pub(super) fn shrink_col_widths(col_widths: &[usize], wrap_width: usize) -> Vec<usize> {
        let current = Self::calculate_table_render_width_from_col_widths(col_widths);
        if current <= wrap_width {
            return col_widths.to_vec();
        }
        // 需要缩减的总量
        let excess = current.saturating_sub(wrap_width);
        let total_col_width: usize = col_widths.iter().sum();
        if total_col_width == 0 {
            return col_widths.to_vec();
        }
        // 按比例缩减每列
        let mut remaining_excess = excess;
        let mut result = Vec::with_capacity(col_widths.len());
        for (i, &cw) in col_widths.iter().enumerate() {
            let is_last = i == col_widths.len() - 1;
            let shrink = if is_last {
                // 最后一列承担剩余缩减量
                remaining_excess
            } else {
                // 按比例分配
                let s = (excess * cw) / total_col_width;
                remaining_excess = remaining_excess.saturating_sub(s);
                s
            };
            result.push(cw.saturating_sub(shrink).max(1));
        }
        result
    }

    /// 判断一行是否是表格分隔行
    pub fn is_table_separator_line(line: &str) -> bool {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            return false;
        }
        let inner = trimmed.trim_matches('|');
        inner.split('|').all(|cell| {
            let cell = cell.trim();
            cell.chars().all(|c| c == '-' || c == ':' || c == ' ')
        })
    }

    /// 判断一行是否是表格行
    pub fn is_table_row(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.contains('|')
    }

    /// 解析表格对齐方式
    pub(super) fn parse_table_alignments(line: &str) -> Vec<TableAlign> {
        let trimmed = line.trim();
        let inner = trimmed.trim_matches('|');
        inner
            .split('|')
            .map(|cell| {
                let cell = cell.trim();
                let left = cell.starts_with(':');
                let right = cell.ends_with(':');
                if left && right {
                    TableAlign::Center
                } else if right {
                    TableAlign::Right
                } else {
                    TableAlign::Left
                }
            })
            .collect()
    }

    /// 解析表格单元格
    pub(super) fn parse_table_cells(line: &str) -> Vec<String> {
        let trimmed = line.trim();
        let inner = trimmed.trim_matches('|');
        inner.split('|').map(|s| s.trim().to_string()).collect()
    }

    /// 查找包含指定行的表格上下文
    pub(super) fn find_table_context(
        &self,
        line_idx: usize,
        lines: &[String],
    ) -> Option<TableContext> {
        let line = lines.get(line_idx)?;
        if !Self::is_table_row(line) {
            return None;
        }

        // 向上查找表格开始
        let mut start_idx = line_idx;
        while start_idx > 0 {
            if let Some(prev) = lines.get(start_idx - 1) {
                if Self::is_table_row(prev) {
                    start_idx -= 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // 向下查找表格结束
        let mut end_idx = line_idx;
        while end_idx < lines.len() - 1 {
            if let Some(next) = lines.get(end_idx + 1) {
                if Self::is_table_row(next) {
                    end_idx += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if end_idx - start_idx < 1 {
            return None;
        }

        let alignments = if let Some(sep_line) = lines.get(start_idx + 1) {
            Self::parse_table_alignments(sep_line)
        } else {
            return None;
        };

        let num_cols = alignments.len();
        let mut col_widths = vec![1; num_cols];

        for row_idx in start_idx..=end_idx {
            let row_line = lines.get(row_idx)?;
            let cells = Self::parse_table_cells(row_line);
            for (i, cell) in cells.iter().enumerate() {
                if i < num_cols {
                    col_widths[i] = col_widths[i].max(display_width(cell));
                }
            }
        }

        Some(TableContext {
            start_idx,
            end_idx,
            col_widths,
            alignments,
        })
    }

    /// 渲染表格行（支持单元格折行，返回多行）
    pub(super) fn render_table_rows(
        &self,
        line: &str,
        line_idx: usize,
        ctx: &TableContext,
        _lines: &[String],
        wrap_width: usize,
    ) -> Vec<Line<'static>> {
        let line_num = self.format_line_number(line_idx);
        let line_num_width = display_width(&line_num);
        let available_width = wrap_width.saturating_sub(line_num_width);

        // 缩小 col_widths 以适配可用宽度
        let col_widths = Self::shrink_col_widths(&ctx.col_widths, available_width);

        let border_style = Style::default().fg(self.theme.text_dim);

        // 判断是否是分隔行 — 不再直接渲染，由上下文决定边框样式
        if Self::is_table_separator_line(line) {
            return vec![];
        }

        // 判断是否是表头行
        let is_header = line_idx == ctx.start_idx;
        let is_last_data_row = line_idx == ctx.end_idx;
        let content_style = if is_header {
            Style::default()
                .fg(self.theme.text_bold)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.text_normal)
        };

        let cells = Self::parse_table_cells(line);

        // 对每个单元格进行折行
        let wrapped_cells: Vec<Vec<String>> = col_widths
            .iter()
            .enumerate()
            .map(|(i, cw)| {
                let cell_text = cells.get(i).map(|s| s.as_str()).unwrap_or("");
                wrap_text(cell_text, *cw)
            })
            .collect();

        // 计算最大折行数
        let max_rows = wrapped_cells.iter().map(|r| r.len()).max().unwrap_or(1);

        let mut result = Vec::new();

        // 如果是表头行，先渲染顶部边框 ┌─┬─┐
        if is_header {
            result.push(Self::render_table_border(
                &self.format_continuation_line_number(),
                &col_widths,
                border_style,
                BorderKind::Top,
                self.theme.bg_primary,
            ));
        }

        // 渲染每一行折行内容
        for sub_row in 0..max_rows {
            let num_str = if sub_row == 0 {
                line_num.clone()
            } else {
                self.format_continuation_line_number()
            };

            let mut spans = vec![Span::styled(
                num_str,
                Style::default()
                    .fg(Color::DarkGray)
                    .bg(self.theme.bg_primary),
            )];
            spans.push(Span::styled("│", border_style));

            for (i, cw) in col_widths.iter().enumerate() {
                let cell_line_raw = wrapped_cells
                    .get(i)
                    .and_then(|lines| lines.get(sub_row))
                    .map(|s| s.as_str())
                    .unwrap_or("");

                // 单元格内容严格截断到 cw 宽度（防御性兜底）。
                //
                // 为什么需要：wrap_text 内部用了 `max_width.max(2)` 防止宽度 1 时
                // 无限循环，但 shrink_col_widths 在极窄终端下可能把列压到 cw=1。
                // 此时若该列单元格里有中文/全角字符（宽度 2），wrap_text 返回的
                // 子行宽度仍是 2 > cw=1，渲染出的 cell 块（` <cell> `）实际宽度
                // = 1+2+1 = 4，与该列边框块（`──...─`）固定宽度 cw+2 = 3 失配。
                // 多列累加后整行宽度超出 wrap_width，终端把右侧 `│` 折到下一行，
                // 表现为边框未闭合。
                //
                // 与历史修复 8db7333（src/command/chat/markdown/parser.rs）同思路。
                // 该 bug 在 chat 渲染里早已修过，editor_core 表格是后写的另一套
                // 渲染，最初没带这个截断保护，是同一个 bug 的回归。
                let (cell_line, cell_width) = {
                    let mut buf = String::new();
                    let mut w = 0;
                    for ch in cell_line_raw.chars() {
                        let chw = char_width(ch);
                        if w + chw > *cw {
                            break;
                        }
                        buf.push(ch);
                        w += chw;
                    }
                    (buf, w)
                };
                let fill = cw.saturating_sub(cell_width);

                let align = ctx.alignments.get(i).copied().unwrap_or(TableAlign::Left);
                let formatted = match align {
                    TableAlign::Center => {
                        let left = fill / 2;
                        let right = fill - left;
                        format!(" {}{}{} ", " ".repeat(left), cell_line, " ".repeat(right))
                    }
                    TableAlign::Right => {
                        format!(" {}{} ", " ".repeat(fill), cell_line)
                    }
                    TableAlign::Left => {
                        format!(" {}{} ", cell_line, " ".repeat(fill))
                    }
                };

                spans.push(Span::styled(formatted, content_style));
                spans.push(Span::styled("│", border_style));
            }

            result.push(Line::from(spans));
        }

        // 表头行后渲染分隔边框 ├─┼─┤
        if is_header {
            result.push(Self::render_table_border(
                &self.format_continuation_line_number(),
                &col_widths,
                border_style,
                BorderKind::Middle,
                self.theme.bg_primary,
            ));
        }

        // 非表头、非最后一行的数据行后渲染行间分割线 ├─┼─┤
        if !is_header && !is_last_data_row {
            result.push(Self::render_table_border(
                &self.format_continuation_line_number(),
                &col_widths,
                border_style,
                BorderKind::Middle,
                self.theme.bg_primary,
            ));
        }

        // 最后一行后渲染底部边框 └─┴─┘
        if is_last_data_row {
            result.push(Self::render_table_border(
                &self.format_continuation_line_number(),
                &col_widths,
                border_style,
                BorderKind::Bottom,
                self.theme.bg_primary,
            ));
        }

        result
    }

    /// 表格边框类型
    pub(super) fn render_table_border(
        line_num: &str,
        col_widths: &[usize],
        border_style: Style,
        kind: BorderKind,
        line_num_bg: Color,
    ) -> Line<'static> {
        let (left, mid, right, fill) = match kind {
            BorderKind::Top => ("┌", "┬", "┐", "─"),
            BorderKind::Middle => ("├", "┼", "┤", "─"),
            BorderKind::Bottom => ("└", "┴", "┘", "─"),
        };

        let mut spans = vec![Span::styled(
            line_num.to_string(),
            Style::default().fg(Color::DarkGray).bg(line_num_bg),
        )];
        spans.push(Span::styled(left, border_style));
        for (i, cw) in col_widths.iter().enumerate() {
            spans.push(Span::styled(fill.repeat(*cw + 2), border_style));
            if i < col_widths.len() - 1 {
                spans.push(Span::styled(mid, border_style));
            }
        }
        spans.push(Span::styled(right, border_style));

        Line::from(spans)
    }
}
