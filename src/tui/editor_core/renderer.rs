//! Markdown 渲染器
//!
//! 负责将文本渲染为 ratatui 的 Line/Widget。
//! 支持代码块围栏样式、表格、Markdown 语法高亮等高级渲染。

mod code_block;
mod inline;
mod line;
mod table;
mod visual_line;

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use super::theme::{EditorTheme, HighlightFn};
use super::wrap_engine::VisualLine;
use super::{search::SearchState, text_buffer::TextBuffer};
use crate::util::text::{char_width, display_width};

use code_block::CodeBlockCache;

/// Markdown 渲染器
pub struct MarkdownRenderer {
    theme: EditorTheme,
    /// 水平滚动偏移
    horizontal_scroll: usize,
    /// 代码块缓存
    code_block_cache: CodeBlockCache,
    /// 语法高亮函数
    highlight_fn: HighlightFn,
    /// 是否显示行号
    show_line_numbers: bool,
}

impl MarkdownRenderer {
    /// 创建新的渲染器
    pub fn new(theme: EditorTheme, highlight_fn: HighlightFn) -> Self {
        Self {
            theme,
            horizontal_scroll: 0,
            code_block_cache: CodeBlockCache::new(),
            highlight_fn,
            show_line_numbers: true,
        }
    }

    /// 使代码块缓存失效
    pub fn invalidate_cache(&mut self) {
        self.code_block_cache.invalidate();
    }

    /// 切换主题
    pub fn set_theme(&mut self, theme: EditorTheme) {
        self.theme = theme;
        self.invalidate_cache();
    }

    /// 设置是否显示行号
    pub fn set_show_line_numbers(&mut self, show: bool) {
        self.show_line_numbers = show;
    }

    /// 获取是否显示行号
    pub fn is_show_line_numbers(&self) -> bool {
        self.show_line_numbers
    }

    /// 生成行号字符串
    fn format_line_number(&self, line_idx: usize) -> String {
        if self.show_line_numbers {
            format!("{:4} ", line_idx + 1)
        } else {
            String::new()
        }
    }

    /// 生成续行行号字符串（空格或空）
    fn format_continuation_line_number(&self) -> String {
        if self.show_line_numbers {
            "     ".to_string()
        } else {
            String::new()
        }
    }

    /// 确保代码块缓存有效
    pub fn ensure_cache_valid(&mut self, lines: &[String]) {
        if !self.code_block_cache.valid || self.code_block_cache.line_count != lines.len() {
            self.code_block_cache.build(lines);
        }
    }

    // ========== 基础样式辅助方法 ==========

    /// 创建带背景色的 Style
    #[inline]
    pub fn style(&self, fg: Color) -> Style {
        Style::default().fg(fg).bg(self.theme.bg_primary)
    }

    /// 创建带输入区背景色的 Style
    #[inline]
    pub fn style_input(&self, fg: Color) -> Style {
        Style::default().fg(fg).bg(self.theme.bg_input)
    }

    /// 创建带背景色和加粗的 Style
    #[inline]
    pub fn style_bold(&self, fg: Color) -> Style {
        Style::default()
            .fg(fg)
            .bg(self.theme.bg_primary)
            .add_modifier(Modifier::BOLD)
    }

    /// 创建代码块背景色的 Style
    #[inline]
    pub fn style_code(&self, fg: Color) -> Style {
        Style::default().fg(fg).bg(self.theme.code_bg)
    }

    // ========== 视觉行渲染 ==========

    /// 渲染一个视觉行（Typora 风格）
    ///
    /// - Insert 模式光标行：显示原始 Markdown 源码 + 光标
    /// - Normal 模式光标行：显示渲染后的 Markdown 效果 + 光标块
    /// - 非光标行：显示渲染后的 Markdown 效果（代码块围栏、表格、标题等）
    #[allow(clippy::too_many_arguments)]
    pub fn render_visual_line(
        &self,
        vl: &VisualLine,
        is_cursor_line: bool,
        cursor_col: Option<usize>,
        search: &SearchState,
        buffer: &TextBuffer,
        wrap_width: usize,
        is_insert_mode: bool,
    ) -> Vec<Line<'static>> {
        let lines = buffer.lines();
        let logical_line = vl.logical_line;
        let Some(raw_line_content) = lines.get(logical_line) else {
            return vec![Line::default()];
        };
        // 将 tab 替换为空格，防止终端展开 tab 导致二次折行
        let line_content = Self::normalize_tabs(raw_line_content);
        // 视觉行文本同样需要标准化（tab → 空格）
        let vl_text = Self::normalize_tabs(&vl.text);

        let is_continuation = vl.start_col > 0;

        // 行号：续行显示空格，否则显示行号；若隐藏行号则为空
        let line_num_str = if !self.show_line_numbers {
            String::new()
        } else if is_continuation {
            "     ".to_string()
        } else {
            format!("{:>4} ", logical_line + 1)
        };
        let line_num_style = if is_cursor_line {
            Style::default()
                .fg(Color::Yellow)
                .bg(self.theme.bg_input)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::DarkGray)
                .bg(self.theme.bg_primary)
        };

        // ---- 光标行：显示源码 + 光标 ----
        // 代码块内的光标行使用 code_bg 背景以保持视觉一致性
        let code_block_max_width = if !Self::is_code_fence_line(&line_content)
            && self.is_line_in_complete_code_block(logical_line, lines)
        {
            self.find_code_block_range(logical_line, lines)
                .map(|(start, end)| self.calculate_code_block_max_width(start, end, lines))
        } else {
            None
        };

        if is_cursor_line && is_insert_mode {
            // 判断是否是逻辑行的最后一个视觉行
            let is_last_vl = vl.end_col >= line_content.chars().count();
            return vec![self.render_cursor_visual_line(
                vl_text,
                vl,
                &visual_line::CursorLineContext {
                    line_num_str: &line_num_str,
                    line_num_style,
                    cursor_col,
                    search,
                    code_block_max_width,
                    is_last_vl,
                },
            )];
        }

        // ---- 非光标行 / Normal 模式光标行：Typora 风格渲染 ----
        // Normal 模式光标行也走渲染路径，但需要额外叠加光标
        let rendered_lines = self.render_non_insert_line(
            vl,
            &line_content,
            &vl_text,
            search,
            buffer,
            wrap_width,
            line_num_str,
            line_num_style,
        );

        // Normal 模式光标行：在渲染结果上叠加光标块
        if is_cursor_line && !is_insert_mode {
            return self.overlay_cursor_on_rendered_lines(
                rendered_lines,
                vl,
                cursor_col,
                &line_content,
            );
        }

        rendered_lines
    }

    /// 渲染非 Insert 模式的视觉行（Typora 风格）
    ///
    /// 适用于非光标行和 Normal 模式的光标行。
    /// Normal 模式光标行的渲染结果会在上层叠加光标。
    #[allow(clippy::too_many_arguments)]
    fn render_non_insert_line(
        &self,
        vl: &VisualLine,
        line_content: &str,
        vl_text: &str,
        search: &SearchState,
        buffer: &TextBuffer,
        wrap_width: usize,
        line_num_str: String,
        line_num_style: Style,
    ) -> Vec<Line<'static>> {
        let lines = buffer.lines();
        let logical_line = vl.logical_line;
        let is_continuation = vl.start_col > 0;

        // 续行（折行后的第二行及之后）无法独立渲染 Markdown，
        // 因为 Markdown 标记可能跨越折行边界，所以续行显示源码
        // 但代码块内的续行需要保持 code_bg 样式
        if is_continuation {
            let text = vl_text;

            // 检查续行是否在代码块内（非围栏行）
            let in_code_block = !Self::is_code_fence_line(line_content)
                && self.is_line_in_complete_code_block(logical_line, lines);

            if in_code_block {
                // 代码块续行：保持 code_bg 背景 + 边框
                let mut spans = vec![
                    Span::styled(
                        line_num_str,
                        Style::default()
                            .fg(Color::DarkGray)
                            .bg(self.theme.bg_primary),
                    ),
                    Span::styled("│", self.style_code(self.theme.text_dim)),
                    Span::styled(" ", Style::default().bg(self.theme.code_bg)),
                ];
                // 续行用 code_bg 背景显示源码，不语法高亮（续行是折行片段）
                spans.push(Span::styled(
                    text.to_string(),
                    Style::default()
                        .fg(self.theme.text_normal)
                        .bg(self.theme.code_bg),
                ));
                // 计算填充宽度以对齐右边框
                let max_width = self
                    .find_code_block_range(logical_line, lines)
                    .map(|(start, end)| self.calculate_code_block_max_width(start, end, lines))
                    .unwrap_or(0);
                let content_width = display_width(text);
                let fill_width = max_width.saturating_sub(content_width);
                spans.push(Span::styled(
                    " ".repeat(fill_width),
                    Style::default().bg(self.theme.code_bg),
                ));
                spans.push(Span::styled(" ", Style::default().bg(self.theme.code_bg)));
                spans.push(Span::styled("│", self.style_code(self.theme.text_dim)));
                return vec![Line::from(spans)];
            }

            // 表格续行：保持表格边框样式
            if Self::is_table_row(line_content) {
                let mut spans = vec![Span::styled(line_num_str.clone(), line_num_style)];
                spans.push(Span::styled(
                    text.to_string(),
                    self.style(self.theme.text_normal),
                ));
                return vec![Line::from(spans)];
            }

            // 引用块续行：保持引用块样式
            let trimmed = line_content.trim_start();
            if trimmed.starts_with('>') {
                let mut level = 0;
                let mut rest = trimmed;
                while rest.starts_with('>') {
                    level += 1;
                    rest = rest[1..].trim_start();
                }
                let _ = rest; // 续行不需要 rest

                let bar: String = (0..level).map(|_| "▎").collect::<Vec<_>>().join("");
                let bar_style = Style::default()
                    .fg(self.theme.md_blockquote_bar)
                    .bg(self.theme.md_blockquote_bg)
                    .add_modifier(Modifier::BOLD);
                let text_style = Style::default()
                    .fg(self.theme.md_blockquote_text)
                    .bg(self.theme.md_blockquote_bg);

                let mut spans = vec![Span::styled(line_num_str.clone(), line_num_style)];
                spans.push(Span::styled(format!("{} ", bar), bar_style));
                spans.push(Span::styled(text.to_string(), text_style));
                return vec![Line::from(spans)];
            }

            // 普通续行
            let mut spans = vec![Span::styled(line_num_str.clone(), line_num_style)];
            if search.is_searching() && search.match_count() > 0 {
                spans.extend(search.highlight_line(logical_line, text, &self.theme, vl.start_col));
            } else {
                spans.push(Span::styled(
                    text.to_string(),
                    self.style(self.theme.text_normal),
                ));
            }
            return vec![Line::from(spans)];
        }

        // 非续行的非光标行：完整 Markdown 渲染
        // 截断到折行宽度，防止终端二次折行导致重复渲染
        let truncated = Self::truncate_to_display_width(line_content, wrap_width);

        // 检查是否是代码块围栏行
        if Self::is_code_fence_line(line_content) {
            if self.is_fence_line_paired(logical_line, lines) {
                return vec![self.render_code_fence_line(line_content, logical_line, lines)];
            }
            // 不成对的围栏，渲染为普通文本
            let mut spans = vec![Span::styled(line_num_str.clone(), line_num_style)];
            if search.is_searching() && search.match_count() > 0 {
                spans.extend(search.highlight_line(logical_line, &truncated, &self.theme, 0));
            } else {
                spans.push(Span::styled(truncated, self.style(self.theme.text_normal)));
            }
            return vec![Line::from(spans)];
        }

        // 检查是否在完整的代码块内
        if self.is_line_in_complete_code_block(logical_line, lines) {
            return vec![self.render_code_block_line(line_content, logical_line, lines)];
        }

        // 检查是否在表格内
        if let Some(table_ctx) = self.find_table_context(logical_line, lines) {
            return self.render_table_rows(
                line_content,
                logical_line,
                &table_ctx,
                lines,
                wrap_width,
            );
        }

        // 其他行：搜索高亮优先，否则 Markdown 渲染（标题、列表、引用等）
        if search.is_searching() && search.match_count() > 0 {
            let mut spans = vec![Span::styled(line_num_str.clone(), line_num_style)];
            spans.extend(search.highlight_line(logical_line, &truncated, &self.theme, 0));
            vec![Line::from(spans)]
        } else {
            vec![self.render_single_line_with_number(&truncated, logical_line, wrap_width)]
        }
    }

    /// 在 Normal 模式渲染后的行上叠加光标块
    fn overlay_cursor_on_rendered_lines(
        &self,
        rendered_lines: Vec<Line<'static>>,
        vl: &VisualLine,
        cursor_col: Option<usize>,
        line_content: &str,
    ) -> Vec<Line<'static>> {
        let Some(col) = cursor_col else {
            return rendered_lines;
        };

        // 判断光标是否在当前视觉行范围内
        let is_last_vl = vl.end_col >= line_content.chars().count();
        let cursor_in_this_vl = if col == vl.end_col {
            is_last_vl
        } else {
            col >= vl.start_col && col < vl.end_col
        };

        if !cursor_in_this_vl {
            return rendered_lines;
        }

        let cursor_style = Style::default()
            .fg(self.theme.cursor_fg)
            .bg(self.theme.cursor_bg)
            .add_modifier(Modifier::BOLD);

        // 计算光标在视觉行内的字符偏移
        // 需要加上行号占用的字符数，因为 overlay_cursor_on_spans 在包含行号的完整 spans 上定位
        let line_num_chars = if self.show_line_numbers { 5 } else { 0 };
        let char_idx_at_cursor = line_num_chars + col.saturating_sub(vl.start_col);

        // 对第一个（通常也是唯一一个）渲染行叠加光标
        let mut result = Vec::with_capacity(rendered_lines.len());
        for (i, line) in rendered_lines.into_iter().enumerate() {
            if i == 0 {
                let spans: Vec<Span<'static>> = line.spans;
                let overlaid =
                    Self::overlay_cursor_on_spans(spans, char_idx_at_cursor, cursor_style);
                result.push(Line::from(overlaid));
            } else {
                result.push(line);
            }
        }
        result
    }

    /// 将 tab 替换为空格（宽度为 1，与 char_width('\t') = 1 一致）
    ///
    /// 终端会将 tab 展开到下一个 tab stop（通常占 8 列），
    /// 但 char_width 计算时 tab = 1，导致显示宽度与计算宽度不一致，
    /// 引起终端二次折行和字符重复。替换为空格后两者保持一致。
    fn normalize_tabs(text: &str) -> String {
        text.replace('\t', " ")
    }

    /// 将文本截断到指定显示宽度（使用 unicode-width 精确计算）
    fn truncate_to_display_width(text: &str, max_width: usize) -> String {
        let mut result = String::new();
        let mut width = 0;
        for ch in text.chars() {
            let ch_width = char_width(ch);
            if width + ch_width > max_width {
                break;
            }
            result.push(ch);
            width += ch_width;
        }
        result
    }

    /// 在已渲染的 spans 上叠加光标样式（按字符索引定位）
    pub(super) fn overlay_cursor_on_spans(
        spans: Vec<Span<'static>>,
        cursor_char_idx: usize,
        cursor_style: Style,
    ) -> Vec<Span<'static>> {
        let mut result = Vec::with_capacity(spans.len() + 2);
        let mut chars_seen = 0;
        let mut placed = false;

        for span in spans {
            if placed {
                result.push(span);
                continue;
            }
            let span_chars: Vec<char> = span.content.chars().collect();
            let span_len = span_chars.len();
            let span_end = chars_seen + span_len;

            if cursor_char_idx >= span_end {
                result.push(span);
                chars_seen = span_end;
                continue;
            }

            let local = cursor_char_idx - chars_seen;
            if local > 0 {
                let before: String = span_chars[..local].iter().collect();
                result.push(Span::styled(before, span.style));
            }
            if local < span_len {
                result.push(Span::styled(span_chars[local].to_string(), cursor_style));
                if local + 1 < span_len {
                    let after: String = span_chars[local + 1..].iter().collect();
                    result.push(Span::styled(after, span.style));
                }
            }
            placed = true;
            chars_seen = span_end;
        }

        // 光标在所有 span 之后（行尾），追加一个空格光标块
        if !placed {
            result.push(Span::styled(" ", cursor_style));
        }

        result
    }
}
