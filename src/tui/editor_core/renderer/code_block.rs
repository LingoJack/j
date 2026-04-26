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

    /// 查找代码块范围（统一通过缓存）
    pub(super) fn find_code_block_range(
        &self,
        line_idx: usize,
        _lines: &[String],
    ) -> Option<(usize, usize)> {
        self.code_block_cache.get_block_range(line_idx)
    }

    /// 查找围栏行对应的代码块范围（统一通过缓存）
    pub(super) fn find_code_block_range_for_fence(
        &self,
        fence_line: usize,
        _lines: &[String],
    ) -> Option<(usize, usize)> {
        self.code_block_cache.get_block_range(fence_line)
    }

    /// 计算代码块内容的最大显示宽度
    pub(super) fn calculate_code_block_max_width(
        &self,
        start_idx: usize,
        end_idx: usize,
        lines: &[String],
    ) -> usize {
        let mut max_width = 0;
        for i in (start_idx + 1)..end_idx {
            if let Some(line) = lines.get(i) {
                if self.horizontal_scroll == 0 {
                    max_width = max_width.max(display_width(line));
                } else {
                    // 跳过 horizontal_scroll 个字符后计算宽度
                    let visible: String = line.chars().skip(self.horizontal_scroll).collect();
                    max_width = max_width.max(display_width(&visible));
                }
            }
        }
        max_width.max(10)
    }

    /// 渲染代码块围栏行
    pub(super) fn render_code_fence_line(
        &self,
        line: &str,
        line_idx: usize,
        lines: &[String],
    ) -> Line<'static> {
        let line_num = self.format_line_number(line_idx);
        let trimmed = line.trim_start();

        // 判断是开始围栏还是结束围栏（通过缓存查询）
        let is_start = self
            .code_block_cache
            .get_block_range(line_idx)
            .is_some_and(|(start, _)| start == line_idx);

        // 计算代码块内容的最大宽度
        let content_max_width = self
            .find_code_block_range_for_fence(line_idx, lines)
            .map(|(start, end)| self.calculate_code_block_max_width(start, end, lines))
            .unwrap_or(10);

        let total_width = content_max_width + 2 + 2; // +2 for left/right padding

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

    /// 渲染代码块内容行
    pub(super) fn render_code_block_line(
        &self,
        line: &str,
        line_idx: usize,
        lines: &[String],
    ) -> Line<'static> {
        let line_num = self.format_line_number(line_idx);

        // 应用水平滚动（使用迭代器避免 Vec<char> 分配）
        let visible_line: String = line.chars().skip(self.horizontal_scroll).collect();

        // 获取代码块语言
        let lang = self
            .get_code_block_language(line_idx, lines)
            .unwrap_or_default();

        // 应用语法高亮
        let highlighted_spans = (self.highlight_fn)(&visible_line, &lang, &self.theme);

        // 计算当前行的显示宽度
        let content_width = display_width(&visible_line);

        // 查找代码块范围并计算最大宽度
        let max_width = self
            .find_code_block_range(line_idx, lines)
            .map(|(start, end)| self.calculate_code_block_max_width(start, end, lines))
            .unwrap_or(content_width);

        let fill_width = max_width.saturating_sub(content_width);

        let mut spans = vec![
            Span::styled(
                line_num,
                Style::default()
                    .fg(Color::DarkGray)
                    .bg(self.theme.bg_primary),
            ),
            Span::styled("│", self.style_code(self.theme.text_dim)),
            Span::styled(" ", Style::default().bg(self.theme.code_bg)),
        ];

        for span in highlighted_spans {
            spans.push(Span::styled(
                span.content,
                span.style.bg(self.theme.code_bg),
            ));
        }

        spans.push(Span::styled(
            " ".repeat(fill_width),
            Style::default().bg(self.theme.code_bg),
        ));
        spans.push(Span::styled(" ", Style::default().bg(self.theme.code_bg)));
        spans.push(Span::styled("│", self.style_code(self.theme.text_dim)));

        Line::from(spans)
    }
}
