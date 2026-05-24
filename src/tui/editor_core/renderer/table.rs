//! 表格渲染子模块
//!
//! 使用共享层 `markdown::render::table` 渲染表格，支持内联语法（bold, code, emphasis 等）。
//! Editor 在逐行调用时，仅在表格首行（start_idx）触发完整表格渲染，
//! 后续行返回 `vec![]` 以避免重复输出。

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::markdown::parser::parse_table_from_source;
use crate::markdown::render::table::render_table;

use super::MarkdownRenderer;

/// 表格上下文
#[derive(Debug, Clone)]
pub struct TableContext {
    pub start_idx: usize,
    pub end_idx: usize,
}

impl MarkdownRenderer {
    // ========== 表格处理 ==========

    /// 判断一行是否是表格行
    pub fn is_table_row(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.contains('|')
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

        Some(TableContext { start_idx, end_idx })
    }

    /// 渲染表格行。
    ///
    /// 只在 `line_idx == ctx.start_idx` 时渲染完整表格（一次性），
    /// 后续行返回 `vec![]`。
    pub(super) fn render_table_rows(
        &self,
        _line: &str,
        line_idx: usize,
        ctx: &TableContext,
        lines: &[String],
        wrap_width: usize,
    ) -> Vec<Line<'static>> {
        // 仅在表格首行触发渲染；后续源码行不再重复输出
        if line_idx != ctx.start_idx {
            return vec![];
        }

        let line_num = self.format_line_number(line_idx);
        let cont_num = self.format_continuation_line_number();

        // 收集表格源码行
        let source_lines: Vec<&str> = lines[ctx.start_idx..=ctx.end_idx]
            .iter()
            .map(|s| s.as_str())
            .collect();

        // 通过共享层解析为 IR TableData
        let Some(table_data) = parse_table_from_source(&source_lines) else {
            return vec![];
        };

        // 使用共享层渲染表格
        let table_lines =
            render_table(&table_data, &table_data.alignments, wrap_width, &self.theme);

        // 给每行添加行号前缀
        let mut result = Vec::with_capacity(table_lines.len());
        for (i, tbl_line) in table_lines.into_iter().enumerate() {
            let num_str = if i == 0 {
                line_num.clone()
            } else {
                cont_num.clone()
            };
            let num_span = Span::styled(
                num_str,
                Style::default()
                    .fg(Color::DarkGray)
                    .bg(self.theme.bg_primary),
            );
            // 将共享层渲染的 Line 的 spans 前面插入行号 span
            let mut spans = vec![num_span];
            // ratatui::text::Line 实现了 IntoIterator<Item=Span>
            spans.extend(tbl_line.spans);
            result.push(Line::from(spans));
        }

        result
    }
}
