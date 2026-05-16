//! 代码块渲染子模块

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::util::text::display_width;

use super::MarkdownRenderer;

/// 代码块范围缓存（用于加速渲染）
#[derive(Debug, Clone, Default)]
pub(crate) struct CodeBlockCache {
    /// 每行所在的代码块范围 (start, end)，None 表示不在代码块内
    line_to_block: Vec<Option<(usize, usize)>>,
    /// 代码块语言信息
    block_languages: Vec<(usize, usize, String)>, // (start, end, language)
    /// 缓存是否有效
    pub(super) valid: bool,
    /// 缓存对应的文件行数
    pub(super) line_count: usize,
}

impl CodeBlockCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 使缓存失效
    pub(crate) fn invalidate(&mut self) {
        self.valid = false;
    }

    /// 构建缓存
    pub(crate) fn build(&mut self, lines: &[String]) {
        self.line_to_block.clear();
        self.block_languages.clear();
        self.line_to_block.resize(lines.len(), None);

        let mut in_block = false;
        let mut block_start = 0;
        let mut current_lang = String::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if let Some(stripped) = trimmed.strip_prefix("```") {
                if !in_block {
                    // 开始代码块
                    in_block = true;
                    block_start = i;
                    current_lang = stripped.trim().to_string();
                } else {
                    // 结束代码块
                    // 记录语言信息
                    self.block_languages
                        .push((block_start, i, current_lang.clone()));
                    // 标记代码块内的所有行
                    for j in block_start..=i {
                        if j < self.line_to_block.len() {
                            self.line_to_block[j] = Some((block_start, i));
                        }
                    }
                    in_block = false;
                }
            }
        }

        self.line_count = lines.len();
        self.valid = true;
    }

    /// 获取行所在的代码块范围
    pub(crate) fn get_block_range(&self, line_idx: usize) -> Option<(usize, usize)> {
        if line_idx < self.line_to_block.len() {
            self.line_to_block[line_idx]
        } else {
            None
        }
    }

    /// 获取代码块语言
    pub(crate) fn get_language(&self, line_idx: usize) -> Option<&str> {
        if let Some((start, end)) = self.get_block_range(line_idx) {
            for (s, e, lang) in &self.block_languages {
                if *s == start && *e == end {
                    return Some(lang);
                }
            }
        }
        None
    }

    /// 返回所有代码块内容行的闭区间范围（不含围栏行本身）。
    ///
    /// 返回值如 `[(3, 8), (12, 20)]`，表示第 3~8 行和第 12~20 行是代码块内容行。
    pub(crate) fn content_ranges(&self) -> Vec<(usize, usize)> {
        let mut ranges = Vec::new();
        let mut last_end: Option<usize> = None;
        for (start, end) in self.line_to_block.iter().flatten() {
            // 只取内容行（不含围栏行），且每个块只取一次
            if *end > *start && last_end != Some(*end) {
                ranges.push((*start + 1, *end - 1));
                last_end = Some(*end);
            }
        }
        ranges
    }
}

impl MarkdownRenderer {
    // ========== 代码块处理 ==========

    /// 判断某行是否是代码块围栏 (```)
    pub fn is_code_fence_line(line: &str) -> bool {
        line.trim_start().starts_with("```")
    }

    /// 检测指定围栏行是否有配对的围栏
    pub fn is_fence_line_paired(&self, fence_line: usize, _lines: &[String]) -> bool {
        self.code_block_cache.get_block_range(fence_line).is_some()
    }

    /// 判断某行是否在完整的代码块内（不包括围栏行本身）
    pub(super) fn is_line_in_complete_code_block(
        &self,
        line_idx: usize,
        _lines: &[String],
    ) -> bool {
        if let Some((start, end)) = self.code_block_cache.get_block_range(line_idx) {
            // 围栏行本身不算"在代码块内"
            line_idx > start && line_idx < end
        } else {
            false
        }
    }

    /// 获取代码块语言
    pub(super) fn get_code_block_language(
        &self,
        line_idx: usize,
        _lines: &[String],
    ) -> Option<String> {
        self.code_block_cache
            .get_language(line_idx)
            .map(|s| s.to_string())
    }

    /// 渲染代码块围栏行（撑满 wrap_width）
    pub(super) fn render_code_fence_line(
        &self,
        line: &str,
        line_idx: usize,
        wrap_width: usize,
    ) -> Line<'static> {
        let line_num = self.format_line_number(line_idx);
        let trimmed = line.trim_start();

        // 判断是开始围栏还是结束围栏（通过缓存查询）
        let is_start = self
            .code_block_cache
            .get_block_range(line_idx)
            .is_some_and(|(start, _)| start == line_idx);

        // wrap_width 传入时已减去行号宽度，total_width = wrap_width
        let total_width = wrap_width.max(10);

        if is_start {
            // 开始围栏：┌─ lang ──────┐
            let lang = trimmed[3..].trim();

            let (left_part, left_width) = if lang.is_empty() {
                ("┌─".to_string(), 2)
            } else {
                let s = format!("┌─ {} ─", lang);
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
                Span::styled("┐", self.style_code(self.theme.text_dim)),
            ])
        } else {
            // 结束围栏：└─────────────┘
            let dash_count = total_width.saturating_sub(2).max(1);

            Line::from(vec![
                Span::styled(
                    line_num,
                    Style::default()
                        .fg(Color::DarkGray)
                        .bg(self.theme.bg_primary),
                ),
                Span::styled("└", self.style_code(self.theme.text_dim)),
                Span::styled("─".repeat(dash_count), self.style_code(self.theme.text_dim)),
                Span::styled("┘", self.style_code(self.theme.text_dim)),
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
        lines: &[String],
        wrap_width: usize,
        is_continuation: bool,
    ) -> Line<'static> {
        let line_num = if is_continuation {
            self.format_continuation_line_number()
        } else {
            self.format_line_number(line_idx)
        };

        // 获取代码块语言并应用语法高亮
        let lang = self
            .get_code_block_language(line_idx, lines)
            .unwrap_or_default();
        let highlighted_spans = (self.highlight_fn)(text, &lang, &self.theme);

        // wrap_width 传入时已减去行号宽度，total_width = wrap_width
        // 内部可用 = total_width - 4（│+sp+内容+sp+│）
        let total_width = wrap_width.max(10);
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

        Line::from(spans)
    }
}
