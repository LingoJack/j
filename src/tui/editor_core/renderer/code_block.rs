//! 代码块渲染子模块（围栏行 + 内容行）。
//!
//! Block 检测已迁到 `super::block_cache::BlockCache`（基于 pulldown-cmark），
//! 本文件只保留把 fenced 代码块的源码行渲染成 ratatui `Line` 的方法。

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::util::text::display_width;

use super::MarkdownRenderer;

impl MarkdownRenderer {
    // ========== 代码块处理 ==========

    /// 代码块右侧内边距（字符数），防止竖线紧贴屏幕右边缘
    const CODE_BLOCK_RIGHT_PADDING: usize = 2;

    /// 渲染代码块围栏行（撑满 wrap_width）
    pub(super) fn render_code_fence_line(
        &self,
        line: &str,
        line_idx: usize,
        wrap_width: usize,
    ) -> Line<'static> {
        let line_num = self.format_line_number(line_idx);
        let trimmed = line.trim_start();

        // 起始围栏 vs 结束围栏：通过 BlockCache 查询
        let is_start = self.block_cache.is_code_block_start(line_idx);

        // wrap_width 传入时已减去行号宽度，total_width = wrap_width - padding
        let total_width = wrap_width
            .saturating_sub(Self::CODE_BLOCK_RIGHT_PADDING)
            .max(10);

        if is_start {
            // 开始围栏：╭─ lang ──────╮ 或 ┌─ lang ──────┐
            let lang = trimmed.strip_prefix("```").unwrap_or("").trim();
            let border_style = self.theme.code_border_style;

            let (left_part, left_width) = if lang.is_empty() {
                (format!("{}─", border_style.top_left()), 2)
            } else {
                let s = format!("{}─ {} ─", border_style.top_left(), lang);
                let w = display_width(&s);
                (s, w)
            };

            let dash_count = total_width.saturating_sub(left_width + 1).max(1);

            Line::from(vec![
                Span::styled(
                    line_num,
                    Style::default()
                        .fg(Color::DarkGray)
                        .bg(self.theme.bg_primary),
                ),
                Span::styled(left_part, self.style_code(self.theme.text_dim)),
                Span::styled("─".repeat(dash_count), self.style_code(self.theme.text_dim)),
                Span::styled(
                    border_style.top_right(),
                    self.style_code(self.theme.text_dim),
                ),
                Span::styled(
                    " ".repeat(Self::CODE_BLOCK_RIGHT_PADDING),
                    Style::default().bg(self.theme.bg_primary),
                ),
            ])
        } else {
            // 结束围栏：╰─────────────╯ 或 └─────────────┘
            let border_style = self.theme.code_border_style;
            let dash_count = total_width.saturating_sub(2).max(1);

            Line::from(vec![
                Span::styled(
                    line_num,
                    Style::default()
                        .fg(Color::DarkGray)
                        .bg(self.theme.bg_primary),
                ),
                Span::styled(
                    border_style.bottom_left(),
                    self.style_code(self.theme.text_dim),
                ),
                Span::styled("─".repeat(dash_count), self.style_code(self.theme.text_dim)),
                Span::styled(
                    border_style.bottom_right(),
                    self.style_code(self.theme.text_dim),
                ),
                Span::styled(
                    " ".repeat(Self::CODE_BLOCK_RIGHT_PADDING),
                    Style::default().bg(self.theme.bg_primary),
                ),
            ])
        }
    }

    /// 渲染代码块内容行（撑满 wrap_width）
    ///
    /// `text` 为要显示的文本（可能是完整的行内容，也可能是折行片段）。
    /// `is_continuation` 为 true 时使用续行行号（空格）。
    pub(super) fn render_code_block_line_content(
        &self,
        text: &str,
        line_idx: usize,
        _lines: &[String],
        wrap_width: usize,
        is_continuation: bool,
    ) -> Line<'static> {
        let line_num = if is_continuation {
            self.format_continuation_line_number()
        } else {
            self.format_line_number(line_idx)
        };

        // 通过 BlockCache 取语言并应用语法高亮
        let lang = self.block_cache.code_block_lang(line_idx).unwrap_or("");
        let highlighted_spans = (self.highlight_fn)(text, lang, &self.theme);

        // wrap_width 传入时已减去行号宽度，total_width = wrap_width - padding
        // 内部可用 = total_width - 4（│+sp+内容+sp+│）
        let total_width = wrap_width
            .saturating_sub(Self::CODE_BLOCK_RIGHT_PADDING)
            .max(10);
        let inner_width = total_width.saturating_sub(4);
        let content_width = display_width(text);
        let fill_width = inner_width.saturating_sub(content_width);

        let mut spans = vec![
            Span::styled(
                line_num,
                Style::default()
                    .fg(Color::DarkGray)
                    .bg(self.theme.bg_primary),
            ),
            Span::styled("│", self.style_code(self.theme.text_dim)),
            Span::styled(" ", Style::default().bg(self.theme.bg_primary)),
        ];

        for span in highlighted_spans {
            spans.push(Span::styled(
                span.content,
                span.style.bg(self.theme.bg_primary),
            ));
        }

        spans.push(Span::styled(
            " ".repeat(fill_width),
            Style::default().bg(self.theme.bg_primary),
        ));
        spans.push(Span::styled(
            " ",
            Style::default().bg(self.theme.bg_primary),
        ));
        spans.push(Span::styled("│", self.style_code(self.theme.text_dim)));
        spans.push(Span::styled(
            " ".repeat(Self::CODE_BLOCK_RIGHT_PADDING),
            Style::default().bg(self.theme.bg_primary),
        ));

        Line::from(spans)
    }
}
