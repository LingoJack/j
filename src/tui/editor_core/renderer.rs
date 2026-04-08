//! Markdown 渲染器
//!
//! 负责将文本渲染为 ratatui 的 Line/Widget。
//! 支持代码块围栏样式、表格、Markdown 语法高亮等高级渲染。

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::command::chat::markdown::highlight::highlight_code_line;
use crate::command::chat::theme::Theme;
use super::wrap_engine::{VisualLine, SizedSpan, RenderBlock, StyledVisualLine, wrap_spans};
use super::{search::SearchState, text_buffer::TextBuffer, vim::Mode};
use crate::util::text::display_width;

/// 表格对齐方式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableAlign {
    Left,
    Center,
    Right,
}

/// 表格上下文
#[derive(Debug, Clone)]
pub struct TableContext {
    pub start_idx: usize,
    pub end_idx: usize,
    pub col_widths: Vec<usize>,
    pub alignments: Vec<TableAlign>,
}

/// 代码块上下文
#[derive(Debug, Clone)]
pub struct CodeBlockContext {
    pub start_idx: usize,
    pub end_idx: usize,
    pub language: String,
    pub max_width: usize,
}

/// Markdown 渲染器
pub struct MarkdownRenderer {
    theme: Theme,
    /// 水平滚动偏移
    horizontal_scroll: usize,
}

impl MarkdownRenderer {
    /// 创建新的渲染器
    pub fn new(theme: Theme) -> Self {
        Self { 
            theme,
            horizontal_scroll: 0,
        }
    }

    /// 设置水平滚动偏移
    pub fn set_horizontal_scroll(&mut self, offset: usize) {
        self.horizontal_scroll = offset;
    }

    /// 获取水平滚动偏移
    pub fn horizontal_scroll(&self) -> usize {
        self.horizontal_scroll
    }

    // ========== 基础样式辅助方法 ==========

    /// 创建带背景色的 Style
    #[inline]
    pub fn style(&self, fg: Color) -> Style {
        Style::default().fg(fg).bg(self.theme.bg_primary)
    }

    /// 创建带背景色和加粗的 Style
    #[inline]
    pub fn style_bold(&self, fg: Color) -> Style {
        Style::default()
            .fg(fg)
            .bg(self.theme.bg_primary)
            .add_modifier(Modifier::BOLD)
    }

    /// 创建带背景色和斜体的 Style
    #[inline]
    pub fn style_italic(&self, fg: Color) -> Style {
        Style::default()
            .fg(fg)
            .bg(self.theme.bg_primary)
            .add_modifier(Modifier::ITALIC)
    }

    /// 创建代码块背景色的 Style
    #[inline]
    pub fn style_code(&self, fg: Color) -> Style {
        Style::default().fg(fg).bg(self.theme.code_bg)
    }

    /// 获取主题引用
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    // ========== 块级渲染 ==========

    /// 为一个逻辑行构建渲染块
    pub fn build_render_block(
        &self,
        logical_line: usize,
        lines: &[String],
        cursor_line: usize,
        max_width: usize,
    ) -> RenderBlock {
        let Some(line_content) = lines.get(logical_line) else {
            return RenderBlock::empty(logical_line);
        };

        // 代码块围栏行
        if Self::is_code_fence_line(line_content) {
            if self.is_fence_line_paired(logical_line, lines) {
                return self.build_code_fence_block(logical_line, line_content, lines);
            }
            return self.build_normal_block(logical_line, line_content, max_width);
        }

        // 代码块内容行
        if self.is_line_in_complete_code_block(logical_line, lines) {
            return self.build_code_content_block(logical_line, line_content, lines);
        }

        // 表格行
        if let Some(table_ctx) = self.find_table_context(logical_line, lines) {
            return self.build_table_block(logical_line, line_content, &table_ctx);
        }

        // 普通 Markdown 行
        self.build_normal_block(logical_line, line_content, max_width)
    }

    /// 构建普通 Markdown 行的渲染块
    fn build_normal_block(
        &self,
        logical_line: usize,
        line_content: &str,
        max_width: usize,
    ) -> RenderBlock {
        let trimmed = line_content.trim_start();
        let indent_len = line_content.len() - trimmed.len();
        let indent = " ".repeat(indent_len);

        let mut spans: Vec<SizedSpan> = Vec::new();

        // 缩进
        if !indent.is_empty() {
            spans.push(SizedSpan::plain(&indent, self.style(self.theme.text_normal)));
        }

        // 标题 h1
        if let Some(stripped) = trimmed.strip_prefix("# ") {
            let text = stripped.trim();
            spans.push(SizedSpan::styled(
                format!("◆ {}", text),
                self.style_bold(self.theme.md_h1),
            ));
            return RenderBlock::normal(logical_line, spans);
        }
        if let Some(stripped) = trimmed.strip_prefix("## ") {
            let text = stripped.trim();
            spans.push(SizedSpan::styled(
                format!("◇ {}", text),
                self.style_bold(self.theme.md_h2),
            ));
            return RenderBlock::normal(logical_line, spans);
        }
        if let Some(stripped) = trimmed.strip_prefix("### ") {
            let text = stripped.trim();
            spans.push(SizedSpan::styled(
                format!("〈 {} 〉", text),
                self.style_bold(self.theme.md_h3),
            ));
            return RenderBlock::normal(logical_line, spans);
        }
        if let Some(stripped) = trimmed.strip_prefix("#### ") {
            let text = stripped.trim();
            spans.push(SizedSpan::styled(
                format!("› {} ", text),
                self.style_bold(self.theme.md_h4),
            ));
            return RenderBlock::normal(logical_line, spans);
        }

        // 水平线
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            let width = max_width.saturating_sub(indent_len).min(40);
            spans.push(SizedSpan::styled(
                "─".repeat(width),
                self.style(self.theme.text_dim),
            ));
            return RenderBlock::normal(logical_line, spans);
        }

        // 无序列表
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let text = &trimmed[2..];
            spans.push(SizedSpan::plain("• ", self.style(self.theme.text_normal)));
            spans.extend(self.render_inline_sized(text));
            return RenderBlock::normal(logical_line, spans);
        }

        // 任务列表 - 未完成
        if let Some(stripped) = trimmed.strip_prefix("- [ ]") {
            let text = stripped.trim();
            spans.push(SizedSpan::plain("○ ", self.style(self.theme.text_dim)));
            spans.extend(self.render_inline_sized(text));
            return RenderBlock::normal(logical_line, spans);
        }
        // 任务列表 - 已完成
        if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
            let text = trimmed[5..].trim();
            spans.push(SizedSpan::plain("● ", self.style(self.theme.md_list_bullet)));
            spans.extend(self.render_inline_sized(text));
            return RenderBlock::normal(logical_line, spans);
        }

        // 有序列表
        if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit())
            && let Some(num_end) = rest.find(['.', ')'])
            && (rest.get(num_end..num_end + 2) == Some(". ")
                || rest.get(num_end..num_end + 2) == Some(") "))
        {
            let num_str = &trimmed[..rest.len() - rest.len() + num_end + 1];
            let text = &rest[num_end + 2..];
            spans.push(SizedSpan::styled(
                format!("{} ", num_str),
                self.style(self.theme.text_normal),
            ));
            spans.extend(self.render_inline_sized(text));
            return RenderBlock::normal(logical_line, spans);
        }

        // 引用块
        if trimmed.starts_with('>') {
            let mut level = 0;
            let mut rest = trimmed;
            while rest.starts_with('>') {
                level += 1;
                rest = rest[1..].trim_start();
            }
            let prefix: String = (0..level).map(|_| "│").collect::<Vec<_>>().join("");
            let prefix_style = if level == 1 {
                self.style(self.theme.text_dim)
            } else {
                Style::default().fg(self.theme.text_dim).bg(self.theme.bg_primary)
            };
            spans.push(SizedSpan::styled(format!("{} ", prefix), prefix_style));
            spans.extend(self.render_inline_sized(rest));
            return RenderBlock::normal(logical_line, spans);
        }

        // 普通文本
        spans.extend(self.render_inline_sized(trimmed));
        RenderBlock::normal(logical_line, spans)
    }

    /// 构建代码围栏渲染块
    fn build_code_fence_block(
        &self,
        logical_line: usize,
        line_content: &str,
        lines: &[String],
    ) -> RenderBlock {
        let trimmed = line_content.trim_start();

        let is_start = {
            let mut in_block = false;
            for (i, l) in lines.iter().enumerate() {
                if i >= logical_line { break; }
                if Self::is_code_fence_line(l) { in_block = !in_block; }
            }
            !in_block
        };

        let content_max_width = self
            .find_code_block_range_for_fence(logical_line, lines)
            .map(|(start, end)| self.calculate_code_block_max_width(start, end, lines))
            .unwrap_or(10);
        let total_width = content_max_width + 2;

        let mut spans: Vec<SizedSpan> = Vec::new();

        if is_start {
            let lang = trimmed[3..].trim();
            let (left_part, left_width) = if lang.is_empty() {
                ("┌─".to_string(), 2)
            } else {
                let s = format!("┌─ {} ─", lang);
                let w = display_width(&s);
                (s, w)
            };
            let dash_count = total_width.saturating_sub(left_width + 1).max(1);
            spans.push(SizedSpan::styled(left_part, self.style_code(self.theme.text_dim)));
            spans.push(SizedSpan::styled("─".repeat(dash_count), self.style_code(self.theme.text_dim)));
            spans.push(SizedSpan::styled("┐", self.style_code(self.theme.text_dim)));
        } else {
            let dash_count = total_width.saturating_sub(2).max(1);
            spans.push(SizedSpan::styled("└", self.style_code(self.theme.text_dim)));
            spans.push(SizedSpan::styled("─".repeat(dash_count), self.style_code(self.theme.text_dim)));
            spans.push(SizedSpan::styled("┘", self.style_code(self.theme.text_dim)));
        }

        RenderBlock::code_fence(logical_line, spans)
    }

    /// 构建代码内容渲染块
    fn build_code_content_block(
        &self,
        logical_line: usize,
        line_content: &str,
        lines: &[String],
    ) -> RenderBlock {
        let lang = self.get_code_block_language(logical_line, lines).unwrap_or_default();
        let highlighted_spans = highlight_code_line(line_content, &lang, &self.theme);
        let content_width = display_width(line_content);

        let max_width = self
            .find_code_block_range(logical_line, lines)
            .map(|(start, end)| self.calculate_code_block_max_width(start, end, lines))
            .unwrap_or(content_width);
        let fill_width = max_width.saturating_sub(content_width);

        let mut spans: Vec<SizedSpan> = Vec::new();
        spans.push(SizedSpan::styled("│", self.style_code(self.theme.text_dim)));

        for span in highlighted_spans {
            let width = display_width(&span.content);
            spans.push(SizedSpan {
                content: span.content.into_owned(),
                style: span.style.bg(self.theme.code_bg),
                display_width: width,
            });
        }

        spans.push(SizedSpan::styled(
            " ".repeat(fill_width),
            Style::default().bg(self.theme.code_bg),
        ));
        spans.push(SizedSpan::styled("│", self.style_code(self.theme.text_dim)));

        RenderBlock::code_content(logical_line, spans)
    }

    /// 构建表格渲染块
    fn build_table_block(
        &self,
        logical_line: usize,
        line_content: &str,
        ctx: &TableContext,
    ) -> RenderBlock {
        let cells = Self::parse_table_cells(line_content);
        let border_style = Style::default().fg(self.theme.text_dim);

        let mut spans: Vec<SizedSpan> = Vec::new();

        // 分隔行
        if Self::is_table_separator_line(line_content) {
            spans.push(SizedSpan::styled("├", border_style));
            for (i, cw) in ctx.col_widths.iter().enumerate() {
                spans.push(SizedSpan::styled("─".repeat(*cw + 2), border_style));
                if i < ctx.col_widths.len() - 1 {
                    spans.push(SizedSpan::styled("┼", border_style));
                }
            }
            spans.push(SizedSpan::styled("┤", border_style));
            return RenderBlock::table(logical_line, spans);
        }

        // 表头或数据行
        let is_header = logical_line == ctx.start_idx;
        let content_style = if is_header {
            Style::default().fg(self.theme.text_bold).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.text_normal)
        };

        spans.push(SizedSpan::styled("│", border_style));

        for (i, cw) in ctx.col_widths.iter().enumerate() {
            let cell_text = cells.get(i).map(|s: &String| s.as_str()).unwrap_or("");
            let cell_width = display_width(cell_text);
            let fill = cw.saturating_sub(cell_width);
            let align = ctx.alignments.get(i).copied().unwrap_or(TableAlign::Left);
            let formatted = match align {
                TableAlign::Center => {
                    let left = fill / 2;
                    let right = fill - left;
                    format!(" {}{}{} ", " ".repeat(left), cell_text, " ".repeat(right))
                }
                TableAlign::Right => format!(" {}{} ", " ".repeat(fill), cell_text),
                TableAlign::Left => format!(" {}{} ", cell_text, " ".repeat(fill)),
            };
            spans.push(SizedSpan::styled(formatted, content_style));
            spans.push(SizedSpan::styled("│", border_style));
        }

        RenderBlock::table(logical_line, spans)
    }

    /// render_inline 的 SizedSpan 版本
    fn render_inline_sized(&self, text: &str) -> Vec<SizedSpan> {
        let spans = self.render_inline(text);
        spans.into_iter().map(|span| {
            let width = display_width(&span.content);
            SizedSpan {
                content: span.content.into_owned(),
                style: span.style,
                display_width: width,
            }
        }).collect()
    }

    /// 将 RenderBlock 折行为 StyledVisualLine 列表
    pub fn wrap_block(
        &self,
        block: &RenderBlock,
        wrap_enabled: bool,
        content_width: usize,
    ) -> Vec<StyledVisualLine> {
        if block.is_wrappable() && wrap_enabled {
            wrap_spans(&block.spans, content_width, block.logical_line)
        } else {
            let total_width: usize = block.spans.iter().map(|s| s.display_width).sum();
            vec![StyledVisualLine {
                logical_line: block.logical_line,
                is_continuation: false,
                spans: block.spans.clone(),
                display_width: total_width,
            }]
        }
    }

    /// 将 StyledVisualLine 转换为 ratatui Line（带行号）
    pub fn styled_visual_line_to_ratatui_line(&self, svl: &StyledVisualLine) -> Line<'static> {
        let line_num_str = if svl.is_continuation {
            "     ".to_string()
        } else {
            format!("{:>4} ", svl.logical_line + 1)
        };
        let line_num_style = Style::default().fg(Color::DarkGray);

        let mut spans = vec![Span::styled(line_num_str, line_num_style)];

        if svl.is_continuation {
            spans.push(Span::styled("↪ ", self.style(self.theme.text_dim)));
        }

        for sized_span in &svl.spans {
            spans.push(Span::styled(sized_span.content.clone(), sized_span.style));
        }

        Line::from(spans)
    }

    // ========== 视觉行渲染 ==========

    /// 渲染一个视觉行（Typora 风格）
    ///
    /// - 光标行：显示原始 Markdown 源码 + 光标
    /// - 非光标行：显示渲染后的 Markdown 效果（代码块围栏、表格、标题等）
    pub fn render_visual_line(
        &self,
        vl: &VisualLine,
        is_cursor_line: bool,
        cursor_col: Option<usize>,
        mode: &Mode,
        search: &SearchState,
        buffer: &TextBuffer,
    ) -> Line<'static> {
        let lines = buffer.lines();
        let logical_line = vl.logical_line;
        let Some(line_content) = lines.get(logical_line) else {
            return Line::default();
        };

        let is_continuation = vl.start_col > 0;

        // 行号：续行显示空格，否则显示行号
        let line_num_str = if is_continuation {
            "     ".to_string()
        } else {
            format!("{:>4} ", logical_line + 1)
        };
        let line_num_style = if is_cursor_line {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // ---- 光标行：显示源码 + 光标 ----
        if is_cursor_line {
            return self.render_cursor_visual_line(
                vl, &line_num_str, line_num_style, cursor_col, mode, search, is_continuation,
            );
        }

        // ---- 非光标行：Typora 风格渲染 ----
        // 续行（折行后的第二行及之后）无法独立渲染 Markdown，
        // 因为 Markdown 标记可能跨越折行边界，所以续行显示源码
        if is_continuation {
            let text = &vl.text;
            let mut spans = vec![Span::styled(line_num_str, line_num_style)];
            spans.push(Span::styled("→ ", self.style(self.theme.text_dim)));
            if !search.pattern.is_empty() && search.match_count() > 0 {
                spans.extend(search.highlight_line(logical_line, text, &self.theme));
            } else {
                spans.push(Span::styled(text.clone(), self.style(self.theme.text_normal)));
            }
            return Line::from(spans);
        }

        // 非续行的非光标行：完整 Markdown 渲染
        // 检查是否是代码块围栏行
        if Self::is_code_fence_line(line_content) {
            if self.is_fence_line_paired(logical_line, lines) {
                return self.render_code_fence_line(line_content, logical_line, lines);
            }
            // 不成对的围栏，渲染为普通文本
            let text = &vl.text;
            let mut spans = vec![Span::styled(line_num_str, line_num_style)];
            if !search.pattern.is_empty() && search.match_count() > 0 {
                spans.extend(search.highlight_line(logical_line, text, &self.theme));
            } else {
                spans.push(Span::styled(text.clone(), self.style(self.theme.text_normal)));
            }
            return Line::from(spans);
        }

        // 检查是否在完整的代码块内
        if self.is_line_in_complete_code_block(logical_line, lines) {
            return self.render_code_block_line(line_content, logical_line, lines);
        }

        // 检查是否在表格内
        if let Some(table_ctx) = self.find_table_context(logical_line, lines) {
            return self.render_table_line(line_content, logical_line, &table_ctx, lines);
        }

        // 其他行：Markdown 渲染（标题、列表、引用等）
        self.render_single_line_with_number(line_content, logical_line, 80)
    }

    /// 渲染光标行的视觉行（源码 + 光标高亮）
    fn render_cursor_visual_line(
        &self,
        vl: &VisualLine,
        line_num_str: &str,
        line_num_style: Style,
        cursor_col: Option<usize>,
        mode: &Mode,
        search: &SearchState,
        is_continuation: bool,
    ) -> Line<'static> {
        let text = &vl.text;
        let mut spans = vec![Span::styled(line_num_str.to_string(), line_num_style)];

        // 续行缩进提示
        if is_continuation {
            spans.push(Span::styled("→ ", self.style(self.theme.text_dim)));
        }

        // 搜索高亮
        if !search.pattern.is_empty() && search.match_count() > 0 {
            spans.extend(search.highlight_line(vl.logical_line, text, &self.theme));
            return Line::from(spans).patch_style(Style::default().bg(self.theme.bg_input));
        }

        // 处理光标位置
        if let Some(col) = cursor_col {
            // 判断光标是否在当前视觉行范围内
            let cursor_in_this_vl = col >= vl.start_col && col <= vl.end_col;

            if cursor_in_this_vl {
                // 光标在当前视觉行内
                let chars: Vec<char> = text.chars().collect();
                let char_idx_at_cursor = col.saturating_sub(vl.start_col);

                if char_idx_at_cursor > 0 {
                    let before: String = chars.iter().take(char_idx_at_cursor).collect();
                    spans.push(Span::styled(before, self.style(self.theme.text_normal)));
                }

                let cursor_style = match mode {
                    Mode::Insert => Style::default()
                        .fg(self.theme.bg_primary)
                        .bg(self.theme.text_normal)
                        .add_modifier(Modifier::BOLD),
                    Mode::Normal => Style::default()
                        .fg(self.theme.text_normal)
                        .bg(self.theme.cursor_bg)
                        .add_modifier(Modifier::UNDERLINED),
                    _ => Style::default()
                        .fg(self.theme.bg_primary)
                        .bg(Color::LightBlue),
                };

                if char_idx_at_cursor < chars.len() {
                    spans.push(Span::styled(chars[char_idx_at_cursor].to_string(), cursor_style));
                    if char_idx_at_cursor + 1 < chars.len() {
                        let after: String = chars.iter().skip(char_idx_at_cursor + 1).collect();
                        spans.push(Span::styled(after, self.style(self.theme.text_normal)));
                    }
                } else {
                    // 光标在行尾
                    spans.push(Span::styled(" ", cursor_style));
                }
            } else {
                // 光标不在当前视觉行，正常渲染文本
                spans.push(Span::styled(text.clone(), self.style(self.theme.text_normal)));
            }
        } else {
            // 无光标信息（不应该发生，但作为 fallback）
            spans.push(Span::styled(text.clone(), self.style(self.theme.text_normal)));
        }

        Line::from(spans).patch_style(Style::default().bg(self.theme.bg_input))
    }

    /// 获取显示列
    fn get_display_col(&self, text: &str, char_idx: usize) -> usize {
        let chars: Vec<char> = text.chars().collect();
        let sub_text: String = chars.iter().take(char_idx).collect();
        display_width(&sub_text)
    }

    // ========== 高级 Markdown 渲染 ==========

    /// 渲染单行（源码或渲染效果）
    pub fn render_line(
        &self,
        line_idx: usize,
        lines: &[String],
        cursor_line: usize,
        cursor_col: usize,
        mode: &Mode,
        search: &SearchState,
        max_width: usize,
    ) -> Line<'static> {
        let Some(line_content) = lines.get(line_idx) else {
            return Line::default();
        };

        // 应用水平滚动
        let _visible_line = self.apply_horizontal_scroll(line_content);

        // 检查是否是代码块围栏行
        if Self::is_code_fence_line(line_content) {
            // 只有成对的围栏才渲染围框样式
            if self.is_fence_line_paired(line_idx, lines) {
                return self.render_code_fence_line(line_content, line_idx, lines);
            }
            // 不成对的围栏，渲染为普通文本
            return self.render_source_line(line_content, line_idx, cursor_line, cursor_col, mode, search);
        }

        // 检查是否在完整的代码块内
        let is_in_code_block = self.is_line_in_complete_code_block(line_idx, lines);

        // 当前编辑行 - 显示源码
        if line_idx == cursor_line {
            return self.render_source_line(line_content, line_idx, cursor_line, cursor_col, mode, search);
        }

        // 代码块内的行 - 渲染代码块样式
        if is_in_code_block {
            return self.render_code_block_line(line_content, line_idx, lines);
        }

        // 检查是否在表格内（表格行不显示源码，直接渲染）
        if let Some(table_ctx) = self.find_table_context(line_idx, lines) {
            return self.render_table_line(line_content, line_idx, &table_ctx, lines);
        }

        // 其他行 - 尝试渲染（带行号）
        self.render_single_line_with_number(line_content, line_idx, max_width)
    }

    /// 应用水平滚动
    fn apply_horizontal_scroll(&self, line: &str) -> String {
        if self.horizontal_scroll == 0 {
            return line.to_string();
        }
        let chars: Vec<char> = line.chars().collect();
        chars.iter().skip(self.horizontal_scroll).collect()
    }

    // ========== 代码块处理 ==========

    /// 判断某行是否是代码块围栏 (```)
    pub fn is_code_fence_line(line: &str) -> bool {
        line.trim_start().starts_with("```")
    }

    /// 检测指定围栏行是否有配对的围栏
    pub fn is_fence_line_paired(&self, fence_line: usize, lines: &[String]) -> bool {
        self.find_complete_code_block(fence_line, lines).is_some()
    }

    /// 查找完整代码块
    fn find_complete_code_block(&self, fence_line: usize, lines: &[String]) -> Option<(usize, usize)> {
        let mut in_block = false;
        let mut block_start = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                if !in_block {
                    in_block = true;
                    block_start = i;
                } else {
                    if block_start == fence_line || i == fence_line {
                        return Some((block_start, i));
                    }
                    in_block = false;
                }
            }
        }
        None
    }

    /// 判断某行是否在完整的代码块内（不包括围栏行本身）
    fn is_line_in_complete_code_block(&self, line_idx: usize, lines: &[String]) -> bool {
        let mut in_block = false;
        let mut block_start = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                if !in_block {
                    in_block = true;
                    block_start = i;
                } else {
                    if block_start < line_idx && line_idx < i {
                        return true;
                    }
                    in_block = false;
                }
            }
        }
        false
    }

    /// 获取代码块语言
    fn get_code_block_language(&self, line_idx: usize, lines: &[String]) -> Option<String> {
        let mut in_block = false;
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if let Some(stripped) = trimmed.strip_prefix("```") {
                if !in_block {
                    let lang = stripped.trim();
                    if i <= line_idx {
                        in_block = true;
                        for (j, block_line) in lines.iter().enumerate().skip(i + 1) {
                            if Self::is_code_fence_line(block_line) {
                                if line_idx < j {
                                    return Some(lang.to_string());
                                }
                                break;
                            }
                        }
                    }
                } else {
                    in_block = false;
                }
            }
        }
        None
    }

    /// 查找代码块范围
    fn find_code_block_range(&self, line_idx: usize, lines: &[String]) -> Option<(usize, usize)> {
        let mut in_block = false;
        let mut block_start = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                if !in_block {
                    in_block = true;
                    block_start = i;
                } else {
                    if block_start < line_idx && line_idx < i {
                        return Some((block_start, i));
                    }
                    in_block = false;
                }
            }
        }
        None
    }

    /// 查找围栏行对应的代码块范围
    fn find_code_block_range_for_fence(&self, fence_line: usize, lines: &[String]) -> Option<(usize, usize)> {
        let mut in_block = false;
        let mut block_start = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                if !in_block {
                    in_block = true;
                    block_start = i;
                    if i == fence_line {
                        for (j, end_line) in lines.iter().enumerate().skip(i + 1) {
                            let t = end_line.trim_start();
                            if t.starts_with("```") {
                                return Some((block_start, j));
                            }
                        }
                    }
                } else {
                    if i == fence_line {
                        return Some((block_start, i));
                    }
                    in_block = false;
                }
            }
        }
        None
    }

    /// 计算代码块内容的最大显示宽度
    fn calculate_code_block_max_width(&self, start_idx: usize, end_idx: usize, lines: &[String]) -> usize {
        let mut max_width = 0;
        for i in (start_idx + 1)..end_idx {
            if let Some(line) = lines.get(i) {
                let chars: Vec<char> = line.chars().collect();
                let visible_chars: Vec<char> = chars.iter().skip(self.horizontal_scroll).copied().collect();
                let visible_line: String = visible_chars.iter().collect();
                let width = display_width(&visible_line);
                max_width = max_width.max(width);
            }
        }
        max_width.max(10)
    }

    /// 渲染代码块围栏行
    fn render_code_fence_line(
        &self,
        line: &str,
        line_idx: usize,
        lines: &[String],
    ) -> Line<'static> {
        let line_num = format!("{:4} ", line_idx + 1);
        let trimmed = line.trim_start();

        // 判断是开始围栏还是结束围栏
        let is_start = {
            let mut in_block = false;
            for (i, l) in lines.iter().enumerate() {
                if i >= line_idx {
                    break;
                }
                if Self::is_code_fence_line(l) {
                    in_block = !in_block;
                }
            }
            !in_block
        };

        // 计算代码块内容的最大宽度
        let content_max_width = self
            .find_code_block_range_for_fence(line_idx, lines)
            .map(|(start, end)| self.calculate_code_block_max_width(start, end, lines))
            .unwrap_or(10);

        let total_width = content_max_width + 2;

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
                Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                Span::styled(left_part, self.style_code(self.theme.text_dim)),
                Span::styled("─".repeat(dash_count), self.style_code(self.theme.text_dim)),
                Span::styled("┐", self.style_code(self.theme.text_dim)),
            ])
        } else {
            // 结束围栏：└─────────────┘
            let dash_count = total_width.saturating_sub(2).max(1);

            Line::from(vec![
                Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                Span::styled("└", self.style_code(self.theme.text_dim)),
                Span::styled("─".repeat(dash_count), self.style_code(self.theme.text_dim)),
                Span::styled("┘", self.style_code(self.theme.text_dim)),
            ])
        }
    }

    /// 渲染代码块内容行
    fn render_code_block_line(
        &self,
        line: &str,
        line_idx: usize,
        lines: &[String],
    ) -> Line<'static> {
        let line_num = format!("{:4} ", line_idx + 1);

        // 应用水平滚动
        let chars: Vec<char> = line.chars().collect();
        let visible_chars: Vec<char> = chars.iter().skip(self.horizontal_scroll).copied().collect();
        let visible_line: String = visible_chars.iter().collect();

        // 获取代码块语言
        let lang = self.get_code_block_language(line_idx, lines).unwrap_or_default();

        // 应用语法高亮
        let highlighted_spans = highlight_code_line(&visible_line, &lang, &self.theme);

        // 计算当前行的显示宽度
        let content_width = display_width(&visible_line);

        // 查找代码块范围并计算最大宽度
        let max_width = self
            .find_code_block_range(line_idx, lines)
            .map(|(start, end)| self.calculate_code_block_max_width(start, end, lines))
            .unwrap_or(content_width);

        let fill_width = max_width.saturating_sub(content_width);

        let mut spans = vec![
            Span::styled(line_num, Style::default().fg(Color::DarkGray).bg(self.theme.code_bg)),
            Span::styled("│", self.style_code(self.theme.text_dim)),
        ];

        for span in highlighted_spans {
            spans.push(Span::styled(span.content, span.style.bg(self.theme.code_bg)));
        }

        spans.push(Span::styled(" ".repeat(fill_width), Style::default().bg(self.theme.code_bg)));
        spans.push(Span::styled("│", self.style_code(self.theme.text_dim)));

        Line::from(spans)
    }

    // ========== 表格处理 ==========

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
    fn parse_table_alignments(line: &str) -> Vec<TableAlign> {
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
    fn parse_table_cells(line: &str) -> Vec<String> {
        let trimmed = line.trim();
        let inner = trimmed.trim_matches('|');
        inner.split('|').map(|s| s.trim().to_string()).collect()
    }

    /// 查找包含指定行的表格上下文
    fn find_table_context(&self, line_idx: usize, lines: &[String]) -> Option<TableContext> {
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

    /// 渲染表格行
    fn render_table_line(
        &self,
        line: &str,
        line_idx: usize,
        ctx: &TableContext,
        _lines: &[String],
    ) -> Line<'static> {
        let line_num = format!("{:4} ", line_idx + 1);
        let cells = Self::parse_table_cells(line);

        // 判断是否是分隔行
        if Self::is_table_separator_line(line) {
            let mut spans = vec![Span::styled(line_num, Style::default().fg(Color::DarkGray))];
            let border_style = Style::default().fg(self.theme.text_dim);

            spans.push(Span::styled("├", border_style));
            for (i, cw) in ctx.col_widths.iter().enumerate() {
                spans.push(Span::styled("─".repeat(*cw + 2), border_style));
                if i < ctx.col_widths.len() - 1 {
                    spans.push(Span::styled("┼", border_style));
                }
            }
            spans.push(Span::styled("┤", border_style));

            return Line::from(spans);
        }

        // 判断是否是表头行
        let is_header = line_idx == ctx.start_idx;

        let mut spans = vec![Span::styled(line_num, Style::default().fg(Color::DarkGray))];
        let border_style = Style::default().fg(self.theme.text_dim);
        let content_style = if is_header {
            Style::default()
                .fg(self.theme.text_bold)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.text_normal)
        };

        spans.push(Span::styled("│", border_style));

        for (i, cw) in ctx.col_widths.iter().enumerate() {
            let cell_text = cells.get(i).map(|s: &String| s.as_str()).unwrap_or("");
            let cell_width = display_width(cell_text);
            let fill = cw.saturating_sub(cell_width);

            let align = ctx.alignments.get(i).copied().unwrap_or(TableAlign::Left);
            let formatted = match align {
                TableAlign::Center => {
                    let left = fill / 2;
                    let right = fill - left;
                    format!(" {}{}{} ", " ".repeat(left), cell_text, " ".repeat(right))
                }
                TableAlign::Right => {
                    format!(" {}{} ", " ".repeat(fill), cell_text)
                }
                TableAlign::Left => {
                    format!(" {}{} ", cell_text, " ".repeat(fill))
                }
            };

            spans.push(Span::styled(formatted, content_style));
            spans.push(Span::styled("│", border_style));
        }

        Line::from(spans)
    }

    // ========== Markdown 单行渲染 ==========

    /// 渲染源码行（带行号和光标）
    fn render_source_line(
        &self,
        line: &str,
        line_idx: usize,
        cursor_line: usize,
        cursor_col: usize,
        mode: &Mode,
        search: &SearchState,
    ) -> Line<'static> {
        let line_num = format!("{:4} ", line_idx + 1);
        let is_cursor_line = line_idx == cursor_line;

        // 应用水平滚动
        let chars: Vec<char> = line.chars().collect();
        let visible_chars: Vec<char> = chars.iter().skip(self.horizontal_scroll).copied().collect();
        let visible_line: String = visible_chars.iter().collect();
        let visible_cursor_col = cursor_col.saturating_sub(self.horizontal_scroll);

        // 搜索高亮
        if !search.pattern.is_empty() {
            let mut spans = vec![Span::styled(line_num, self.style(Color::DarkGray))];
            spans.extend(search.highlight_line(line_idx, &visible_line, &self.theme));
            return Line::from(spans);
        }

        // Insert 模式光标
        if is_cursor_line && *mode == Mode::Insert {
            let visible_chars: Vec<char> = visible_line.chars().collect();
            let mut spans = vec![Span::styled(line_num, self.style(Color::DarkGray))];

            if visible_cursor_col > 0 {
                let before: String = visible_chars.iter().take(visible_cursor_col).collect();
                spans.push(Span::styled(before, self.style(self.theme.text_normal)));
            }

            if visible_cursor_col < visible_chars.len() {
                let cursor_char = visible_chars[visible_cursor_col];
                spans.push(Span::styled(
                    cursor_char.to_string(),
                    Style::default()
                        .fg(self.theme.bg_primary)
                        .bg(self.theme.text_normal)
                        .add_modifier(Modifier::BOLD),
                ));
                if visible_cursor_col + 1 < visible_chars.len() {
                    let after: String = visible_chars.iter().skip(visible_cursor_col + 1).collect();
                    spans.push(Span::styled(after, self.style(self.theme.text_normal)));
                }
            } else {
                spans.push(Span::styled(
                    " ",
                    Style::default()
                        .fg(self.theme.bg_primary)
                        .bg(self.theme.text_normal),
                ));
            }

            return Line::from(spans);
        }

        // Normal 模式光标
        if is_cursor_line && *mode == Mode::Normal {
            let visible_chars: Vec<char> = visible_line.chars().collect();
            let mut spans = vec![Span::styled(line_num, self.style(Color::DarkGray))];

            let line_style = Style::default()
                .fg(self.theme.text_normal)
                .bg(self.theme.bg_primary);

            if visible_cursor_col > 0 {
                let before: String = visible_chars.iter().take(visible_cursor_col).collect();
                spans.push(Span::styled(before, line_style));
            }

            if visible_cursor_col < visible_chars.len() {
                let cursor_char = visible_chars[visible_cursor_col];
                spans.push(Span::styled(
                    cursor_char.to_string(),
                    Style::default()
                        .fg(self.theme.text_normal)
                        .bg(self.theme.cursor_bg)
                        .add_modifier(Modifier::UNDERLINED),
                ));
                if visible_cursor_col + 1 < visible_chars.len() {
                    let after: String = visible_chars.iter().skip(visible_cursor_col + 1).collect();
                    spans.push(Span::styled(after, line_style));
                }
            } else if visible_chars.is_empty() {
                spans.push(Span::styled(
                    " ",
                    Style::default()
                        .fg(self.theme.bg_primary)
                        .bg(self.theme.cursor_bg),
                ));
            } else {
                spans.push(Span::styled(
                    " ",
                    Style::default()
                        .fg(self.theme.bg_primary)
                        .bg(self.theme.cursor_bg),
                ));
            }

            return Line::from(spans);
        }

        Line::from(vec![
            Span::styled(line_num, self.style(Color::DarkGray)),
            Span::styled(visible_line, self.style(self.theme.text_normal)),
        ])
    }

    /// 渲染单行 Markdown（带行号）
    fn render_single_line_with_number(
        &self,
        line: &str,
        line_idx: usize,
        max_width: usize,
    ) -> Line<'static> {
        let line_num = format!("{:4} ", line_idx + 1);

        // 应用水平滚动
        let chars: Vec<char> = line.chars().collect();
        let visible_chars: Vec<char> = chars.iter().skip(self.horizontal_scroll).copied().collect();
        let visible_line: String = visible_chars.iter().collect();

        let trimmed = visible_line.trim_start();
        let indent_len = visible_line.len() - trimmed.len();
        let indent = " ".repeat(indent_len);

        // 标题
        if let Some(stripped) = trimmed.strip_prefix("# ") {
            let text = stripped.trim();
            return Line::from(vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled(format!("◆ {}", text), self.style_bold(self.theme.md_h1)),
            ]);
        }
        if let Some(stripped) = trimmed.strip_prefix("## ") {
            let text = stripped.trim();
            return Line::from(vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled(format!("◇ {}", text), self.style_bold(self.theme.md_h2)),
            ]);
        }
        if let Some(stripped) = trimmed.strip_prefix("### ") {
            let text = stripped.trim();
            return Line::from(vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled(format!("〈 {} 〉", text), self.style_bold(self.theme.md_h3)),
            ]);
        }
        if let Some(stripped) = trimmed.strip_prefix("#### ") {
            let text = stripped.trim();
            return Line::from(vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled(format!("› {} ", text), self.style_bold(self.theme.md_h4)),
            ]);
        }

        // 水平线
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            let width = max_width.saturating_sub(indent_len).min(40);
            return Line::from(vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("─".repeat(width), self.style(self.theme.text_dim)),
            ]);
        }

        // 无序列表
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let text = &trimmed[2..];
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("• ", self.style(self.theme.text_normal)),
            ];
            spans.extend(rendered);
            return Line::from(spans);
        }

        // 任务列表
        if let Some(stripped) = trimmed.strip_prefix("- [ ]") {
            let text = stripped.trim();
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("○ ", self.style(self.theme.text_dim)),
            ];
            spans.extend(rendered);
            return Line::from(spans);
        }
        if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
            let text = trimmed[5..].trim();
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("● ", self.style(self.theme.md_list_bullet)),
            ];
            spans.extend(rendered);
            return Line::from(spans);
        }

        // 有序列表
        if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit())
            && let Some(num_end) = rest.find(['.', ')'])
            && (rest.get(num_end..num_end + 2) == Some(". ")
                || rest.get(num_end..num_end + 2) == Some(") "))
        {
            let num_str = &trimmed[..rest.len() - rest.len() + num_end + 1];
            let text = &rest[num_end + 2..];
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled(format!("{} ", num_str), self.style(self.theme.text_normal)),
            ];
            spans.extend(rendered);
            return Line::from(spans);
        }

        // 引用块
        if trimmed.starts_with('>') {
            let mut level = 0;
            let mut rest = trimmed;
            while rest.starts_with('>') {
                level += 1;
                rest = rest[1..].trim_start();
            }
            let text = rest;

            let prefix: String = (0..level).map(|_| "│").collect::<Vec<_>>().join("");
            let prefix_style = if level == 1 {
                self.style(self.theme.text_dim)
            } else {
                Style::default()
                    .fg(self.theme.text_dim)
                    .bg(self.theme.bg_primary)
            };

            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled(format!("{} ", prefix), prefix_style),
            ];
            spans.extend(rendered);
            return Line::from(spans);
        }

        // 普通文本
        let rendered = self.render_inline(trimmed);
        let mut spans = vec![
            Span::styled(line_num, self.style(Color::DarkGray)),
            Span::styled(indent, self.style(self.theme.text_normal)),
        ];
        spans.extend(rendered);
        Line::from(spans)
    }

    /// 渲染行内元素
    fn render_inline(&self, text: &str) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let mut remaining = text;

        while !remaining.is_empty() {
            let code_pos = remaining.find('`');
            let img_pos = remaining.find("![");
            let bold_pos = remaining.find("**");
            let strike_pos = remaining.find("~~");
            let italic_pos = remaining.find('*');
            let link_pos = remaining.find('[');

            let min_pos = [
                code_pos, img_pos, bold_pos, strike_pos, italic_pos, link_pos,
            ]
            .iter()
            .filter_map(|&p| p)
            .min();

            let Some(pos) = min_pos else {
                spans.push(Span::styled(
                    remaining.to_string(),
                    self.style(self.theme.text_normal),
                ));
                break;
            };

            let is_img = img_pos == Some(pos);
            let is_code = code_pos == Some(pos) && !is_img;
            let is_bold = bold_pos == Some(pos);
            let is_strike = strike_pos == Some(pos);
            let is_link = link_pos == Some(pos) && !is_img;
            let is_italic = italic_pos == Some(pos) && !is_bold && !is_img;

            if pos > 0 {
                spans.push(Span::styled(
                    remaining[..pos].to_string(),
                    self.style(self.theme.text_normal),
                ));
            }

            remaining = &remaining[pos..];

            // 行内代码
            if is_code {
                remaining = &remaining[1..];
                if let Some(end) = remaining.find('`') {
                    spans.push(Span::styled(
                        remaining[..end].to_string(),
                        Style::default()
                            .fg(self.theme.md_inline_code_fg)
                            .bg(self.theme.md_inline_code_bg),
                    ));
                    remaining = &remaining[end + 1..];
                } else {
                    spans.push(Span::styled(
                        format!("`{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 图片
            else if is_img {
                remaining = &remaining[2..];
                if let Some(alt_end) = remaining.find("](") {
                    let alt = &remaining[..alt_end];
                    remaining = &remaining[alt_end + 2..];
                    if let Some(url_end) = remaining.find(')') {
                        spans.push(Span::styled(
                            format!("🖼 {}", alt),
                            Style::default()
                                .fg(self.theme.text_dim)
                                .add_modifier(Modifier::ITALIC),
                        ));
                        remaining = &remaining[url_end + 1..];
                    } else {
                        spans.push(Span::styled(
                            format!("![{}{}", alt, remaining),
                            self.style(self.theme.text_normal),
                        ));
                        break;
                    }
                } else {
                    spans.push(Span::styled(
                        format!("![{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 粗体
            else if is_bold {
                remaining = &remaining[2..];
                if let Some(end) = remaining.find("**") {
                    spans.push(Span::styled(
                        remaining[..end].to_string(),
                        self.style_bold(self.theme.text_normal),
                    ));
                    remaining = &remaining[end + 2..];
                } else {
                    spans.push(Span::styled(
                        format!("**{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 删除线
            else if is_strike {
                remaining = &remaining[2..];
                if let Some(end) = remaining.find("~~") {
                    spans.push(Span::styled(
                        remaining[..end].to_string(),
                        self.style(self.theme.text_dim)
                            .add_modifier(Modifier::CROSSED_OUT),
                    ));
                    remaining = &remaining[end + 2..];
                } else {
                    spans.push(Span::styled(
                        format!("~~{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 斜体
            else if is_italic {
                remaining = &remaining[1..];
                if let Some(end) = remaining.find('*') {
                    spans.push(Span::styled(
                        remaining[..end].to_string(),
                        self.style_italic(self.theme.text_normal),
                    ));
                    remaining = &remaining[end + 1..];
                } else {
                    spans.push(Span::styled(
                        format!("*{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 链接
            else if is_link {
                remaining = &remaining[1..];
                if let Some(text_end) = remaining.find("](") {
                    let link_text = &remaining[..text_end];
                    remaining = &remaining[text_end + 2..];
                    if let Some(url_end) = remaining.find(')') {
                        spans.push(Span::styled(
                            link_text.to_string(),
                            self.style(self.theme.md_link)
                                .add_modifier(Modifier::UNDERLINED),
                        ));
                        spans.push(Span::styled(
                            " ↗".to_string(),
                            self.style(self.theme.text_dim),
                        ));
                        remaining = &remaining[url_end + 1..];
                    } else {
                        spans.push(Span::styled(
                            format!("[{}{}", link_text, remaining),
                            self.style(self.theme.text_normal),
                        ));
                        break;
                    }
                } else {
                    spans.push(Span::styled(
                        format!("[{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
        }

        spans
    }

    // ========== 简单 Markdown 渲染 ==========

    /// 渲染 Markdown 样式的行（简单版本）
    pub fn render_markdown_line(&self, line: &str, line_num: usize) -> Line<'static> {
        let mut spans = vec![Span::styled(
            format!("{:>4} ", line_num),
            Style::default().fg(Color::DarkGray),
        )];

        // 简单的 Markdown 高亮
        if line.starts_with('#') {
            // 标题
            let level = line.chars().take_while(|&c| c == '#').count();
            let color = match level {
                1 => Color::Red,
                2 => Color::Yellow,
                3 => Color::Green,
                4 => Color::Cyan,
                _ => Color::Blue,
            };
            spans.push(Span::styled(
                line.to_string(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));
        } else if line.starts_with("- ") || line.starts_with("* ") {
            // 列表
            spans.push(Span::styled(
                line.to_string(),
                Style::default().fg(self.theme.text_normal),
            ));
        } else if line.starts_with("```") {
            // 代码块边界
            spans.push(Span::styled(
                line.to_string(),
                Style::default().fg(Color::Magenta),
            ));
        } else if line.starts_with("> ") {
            // 引用
            spans.push(Span::styled(
                line.to_string(),
                Style::default().fg(Color::Yellow),
            ));
        } else {
            // 普通文本
            spans.push(Span::styled(
                line.to_string(),
                Style::default().fg(self.theme.text_normal),
            ));
        }

        Line::from(spans)
    }
}

/// 计算字符串的显示宽度
#[allow(dead_code)]
pub fn calc_display_width(s: &str) -> usize {
    s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_width() {
        assert_eq!(calc_display_width("hello"), 5);
        assert_eq!(calc_display_width("你好"), 4);
        assert_eq!(calc_display_width("hello世界"), 9);
    }

    #[test]
    fn test_block_wrap_pipeline() {
        use crate::command::chat::theme::Theme;

        let theme = Theme::dark();
        let renderer = MarkdownRenderer::new(theme);

        // 100 字符的普通文本行
        let long_line = "a".repeat(100);
        let lines = vec![long_line.clone()];
        let wrap_width = 40;

        let block = renderer.build_render_block(0, &lines, 99, wrap_width);
        assert!(block.is_wrappable(), "Normal text should be wrappable");
        eprintln!("block total_width = {}", block.total_width);
        eprintln!("block spans count = {}", block.spans.len());
        for (i, s) in block.spans.iter().enumerate() {
            eprintln!("  span[{}]: len={}, dw={}, content_preview='{}'",
                i, s.content.len(), s.display_width,
                &s.content[..s.content.len().min(20)]);
        }

        let styled_vlines = renderer.wrap_block(&block, true, wrap_width);
        eprintln!("styled_vlines count = {}", styled_vlines.len());
        for (i, svl) in styled_vlines.iter().enumerate() {
            eprintln!("  svl[{}]: dw={}, continuation={}, spans={}",
                i, svl.display_width, svl.is_continuation, svl.spans.len());
        }
        assert!(
            styled_vlines.len() >= 3,
            "100 chars at width 40 should produce at least 3 visual lines, got {}",
            styled_vlines.len()
        );
    }
}
