//! Markdown 渲染器
//!
//! 负责将文本渲染为 ratatui 的 Line/Widget。
//! 支持代码块围栏样式、表格、Markdown 语法高亮等高级渲染。

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use super::theme::{EditorTheme, HighlightFn};
use super::wrap_engine::VisualLine;
use super::{search::SearchState, text_buffer::TextBuffer};
use crate::util::text::{char_width, display_width, wrap_text};

/// 表格对齐方式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableAlign {
    Left,
    Center,
    Right,
}

/// 表格边框类型
#[derive(Debug, Clone, Copy, PartialEq)]
enum BorderKind {
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

/// 代码块范围缓存（用于加速渲染）
#[derive(Debug, Clone, Default)]
struct CodeBlockCache {
    /// 每行所在的代码块范围 (start, end)，None 表示不在代码块内
    line_to_block: Vec<Option<(usize, usize)>>,
    /// 代码块语言信息
    block_languages: Vec<(usize, usize, String)>, // (start, end, language)
    /// 缓存是否有效
    valid: bool,
    /// 缓存对应的文件行数
    line_count: usize,
}

impl CodeBlockCache {
    fn new() -> Self {
        Self::default()
    }

    /// 使缓存失效
    fn invalidate(&mut self) {
        self.valid = false;
    }

    /// 构建缓存
    fn build(&mut self, lines: &[String]) {
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
    fn get_block_range(&self, line_idx: usize) -> Option<(usize, usize)> {
        if line_idx < self.line_to_block.len() {
            self.line_to_block[line_idx]
        } else {
            None
        }
    }

    /// 获取代码块语言
    fn get_language(&self, line_idx: usize) -> Option<&str> {
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

/// 光标视觉行渲染所需的上下文参数
struct CursorLineContext<'a> {
    line_num_str: &'a str,
    line_num_style: Style,
    cursor_col: Option<usize>,
    search: &'a SearchState,
    code_block_max_width: Option<usize>,
    is_last_vl: bool,
}

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
    /// - 光标行：显示原始 Markdown 源码 + 光标
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

        if is_cursor_line {
            // 判断是否是逻辑行的最后一个视觉行
            let is_last_vl = vl.end_col >= line_content.chars().count();
            return vec![self.render_cursor_visual_line(
                vl_text,
                vl,
                &CursorLineContext {
                    line_num_str: &line_num_str,
                    line_num_style,
                    cursor_col,
                    search,
                    code_block_max_width,
                    is_last_vl,
                },
            )];
        }

        // ---- 非光标行：Typora 风格渲染 ----
        // 续行（折行后的第二行及之后）无法独立渲染 Markdown，
        // 因为 Markdown 标记可能跨越折行边界，所以续行显示源码
        // 但代码块内的续行需要保持 code_bg 样式
        if is_continuation {
            let text = &vl_text;

            // 检查续行是否在代码块内（非围栏行）
            let in_code_block = !Self::is_code_fence_line(&line_content)
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
                    text.clone(),
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
            if Self::is_table_row(&line_content) {
                let mut spans = vec![Span::styled(line_num_str, line_num_style)];
                spans.push(Span::styled(
                    text.clone(),
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

                let mut spans = vec![Span::styled(line_num_str, line_num_style)];
                spans.push(Span::styled(format!("{} ", bar), bar_style));
                spans.push(Span::styled(text.clone(), text_style));
                return vec![Line::from(spans)];
            }

            // 普通续行
            let mut spans = vec![Span::styled(line_num_str, line_num_style)];
            if search.is_searching() && search.match_count() > 0 {
                spans.extend(search.highlight_line(logical_line, text, &self.theme, vl.start_col));
            } else {
                spans.push(Span::styled(
                    text.clone(),
                    self.style(self.theme.text_normal),
                ));
            }
            return vec![Line::from(spans)];
        }

        // 非续行的非光标行：完整 Markdown 渲染
        // 截断到折行宽度，防止终端二次折行导致重复渲染
        let truncated = Self::truncate_to_display_width(&line_content, wrap_width);

        // 检查是否是代码块围栏行
        if Self::is_code_fence_line(&line_content) {
            if self.is_fence_line_paired(logical_line, lines) {
                return vec![self.render_code_fence_line(&line_content, logical_line, lines)];
            }
            // 不成对的围栏，渲染为普通文本
            let mut spans = vec![Span::styled(line_num_str, line_num_style)];
            if search.is_searching() && search.match_count() > 0 {
                spans.extend(search.highlight_line(logical_line, &truncated, &self.theme, 0));
            } else {
                spans.push(Span::styled(truncated, self.style(self.theme.text_normal)));
            }
            return vec![Line::from(spans)];
        }

        // 检查是否在完整的代码块内
        if self.is_line_in_complete_code_block(logical_line, lines) {
            return vec![self.render_code_block_line(&line_content, logical_line, lines)];
        }

        // 检查是否在表格内
        if let Some(table_ctx) = self.find_table_context(logical_line, lines) {
            return self.render_table_rows(
                &line_content,
                logical_line,
                &table_ctx,
                lines,
                wrap_width,
            );
        }

        // 其他行：Markdown 渲染（标题、列表、引用等）
        vec![self.render_single_line_with_number(&truncated, logical_line, wrap_width)]
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

    /// 渲染光标行的视觉行（源码 + 光标高亮）
    fn render_cursor_visual_line(
        &self,
        text: String,
        vl: &VisualLine,
        ctx: &CursorLineContext<'_>,
    ) -> Line<'static> {
        let in_code_block = ctx.code_block_max_width.is_some();

        // 代码块内的光标行：文本用 code_bg，行号保持 bg_input
        let (text_style, line_num_bg) = if in_code_block {
            (
                Style::default()
                    .fg(self.theme.text_normal)
                    .bg(self.theme.code_bg),
                self.theme.bg_input,
            )
        } else {
            (
                self.style_input(self.theme.text_normal),
                self.theme.bg_input,
            )
        };

        let effective_line_num_style = ctx.line_num_style.bg(line_num_bg);
        let mut spans = vec![Span::styled(
            ctx.line_num_str.to_string(),
            effective_line_num_style,
        )];

        // 代码块光标行：添加左边框
        if in_code_block {
            spans.push(Span::styled("│", self.style_code(self.theme.text_dim)));
            spans.push(Span::styled(" ", Style::default().bg(self.theme.code_bg)));
        }

        // 计算显示宽度（在 text 被消费之前）
        let text_display_width = display_width(&text);

        // 搜索高亮
        if ctx.search.is_searching() && ctx.search.match_count() > 0 {
            spans.extend(ctx.search.highlight_line(
                vl.logical_line,
                &text,
                &self.theme,
                vl.start_col,
            ));
            return Line::from(spans).patch_style(Style::default().bg(line_num_bg));
        }

        // 处理光标位置
        if let Some(col) = ctx.cursor_col {
            // 判断光标是否在当前视觉行范围内
            // 当 col == vl.end_col 时：
            //   - 如果是最后一个视觉行，光标在行尾，属于当前视觉行
            //   - 如果不是最后一个视觉行，光标属于下一个视觉行（end_col == next start_col）
            let cursor_in_this_vl = if col == vl.end_col {
                ctx.is_last_vl
            } else {
                col >= vl.start_col && col < vl.end_col
            };

            if cursor_in_this_vl {
                // 光标在当前视觉行内
                let chars: Vec<char> = text.chars().collect();
                let char_idx_at_cursor = col.saturating_sub(vl.start_col);

                if char_idx_at_cursor > 0 {
                    let before: String = chars.iter().take(char_idx_at_cursor).collect();
                    spans.push(Span::styled(before, text_style));
                }

                let cursor_style = Style::default()
                    .fg(self.theme.cursor_fg)
                    .bg(self.theme.cursor_bg)
                    .add_modifier(Modifier::BOLD);

                if char_idx_at_cursor < chars.len() {
                    spans.push(Span::styled(
                        chars[char_idx_at_cursor].to_string(),
                        cursor_style,
                    ));
                    if char_idx_at_cursor + 1 < chars.len() {
                        let after: String = chars.iter().skip(char_idx_at_cursor + 1).collect();
                        spans.push(Span::styled(after, text_style));
                    }
                } else {
                    // 光标在行尾，用空格显示背景色，与字上光标一致
                    spans.push(Span::styled(" ", cursor_style));
                }
            } else {
                // 光标不在当前视觉行，正常渲染文本
                spans.push(Span::styled(text, text_style));
            }
        } else {
            // 无光标信息（不应该发生，但作为 fallback）
            spans.push(Span::styled(text, text_style));
        }

        // 代码块光标行：添加右边框 + 填充
        if let Some(max_width) = ctx.code_block_max_width {
            let fill_width = max_width.saturating_sub(text_display_width);
            spans.push(Span::styled(
                " ".repeat(fill_width),
                Style::default().bg(self.theme.code_bg),
            ));
            spans.push(Span::styled(" ", Style::default().bg(self.theme.code_bg)));
            spans.push(Span::styled("│", self.style_code(self.theme.text_dim)));
        }

        Line::from(spans)
    }

    // ========== 高级 Markdown 渲染 ==========

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
    fn is_line_in_complete_code_block(&self, line_idx: usize, _lines: &[String]) -> bool {
        if let Some((start, end)) = self.code_block_cache.get_block_range(line_idx) {
            // 围栏行本身不算"在代码块内"
            line_idx > start && line_idx < end
        } else {
            false
        }
    }

    /// 获取代码块语言
    fn get_code_block_language(&self, line_idx: usize, _lines: &[String]) -> Option<String> {
        self.code_block_cache
            .get_language(line_idx)
            .map(|s| s.to_string())
    }

    /// 查找代码块范围（统一通过缓存）
    fn find_code_block_range(&self, line_idx: usize, _lines: &[String]) -> Option<(usize, usize)> {
        self.code_block_cache.get_block_range(line_idx)
    }

    /// 查找围栏行对应的代码块范围（统一通过缓存）
    fn find_code_block_range_for_fence(
        &self,
        fence_line: usize,
        _lines: &[String],
    ) -> Option<(usize, usize)> {
        self.code_block_cache.get_block_range(fence_line)
    }

    /// 计算代码块内容的最大显示宽度
    fn calculate_code_block_max_width(
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
    fn render_code_fence_line(
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
    fn render_code_block_line(
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

    // ========== 表格处理 ==========

    /// 计算表格行的总渲染宽度（包含边框）
    fn calculate_table_render_width_from_col_widths(col_widths: &[usize]) -> usize {
        // 每列：│ + 空格 + 内容 + 空格 = 1 + cw + 2，最后一列加 │
        let cols_width: usize = col_widths.iter().map(|cw| cw + 2 + 1).sum();
        cols_width + 1 // 最左边的 │
    }

    /// 将 col_widths 缩小以适配 wrap_width（按比例缩减）
    fn shrink_col_widths(col_widths: &[usize], wrap_width: usize) -> Vec<usize> {
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

    /// 渲染表格行（支持单元格折行，返回多行）
    fn render_table_rows(
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
                let cell_line = wrapped_cells
                    .get(i)
                    .and_then(|lines| lines.get(sub_row))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let cell_width = display_width(cell_line);
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
    fn render_table_border(
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

    // ========== Markdown 单行渲染 ==========

    /// 渲染单行 Markdown（带行号）
    fn render_single_line_with_number(
        &self,
        line: &str,
        line_idx: usize,
        max_width: usize,
    ) -> Line<'static> {
        let line_num = self.format_line_number(line_idx);

        // 应用水平滚动（使用迭代器避免 Vec<char> 分配）
        let visible_line: String = line.chars().skip(self.horizontal_scroll).collect();

        let trimmed = visible_line.trim_start();
        let indent_chars = visible_line.chars().count() - trimmed.chars().count();
        let indent_width: usize = visible_line
            .chars()
            .take(indent_chars)
            .map(char_width)
            .sum();
        let indent = " ".repeat(indent_width);

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
            let width = max_width.saturating_sub(indent_width).min(40);
            return Line::from(vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("─".repeat(width), self.style(self.theme.text_dim)),
            ]);
        }

        // 任务列表（必须在无序列表之前检查，因为 `- [ ]` 也以 `- ` 开头）
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

            // 引用块竖线: 每级一个 ▎，加粗显示
            let bar: String = (0..level).map(|_| "▎").collect::<Vec<_>>().join("");
            let bar_style = Style::default()
                .fg(self.theme.md_blockquote_bar)
                .bg(self.theme.md_blockquote_bg)
                .add_modifier(Modifier::BOLD);

            // 引用块文字: 使用主题专用颜色 + 背景
            let text_style = Style::default()
                .fg(self.theme.md_blockquote_text)
                .bg(self.theme.md_blockquote_bg);

            // 渲染行内元素，然后覆盖基础文字颜色和背景
            let rendered = self.render_inline(text);
            let styled_rendered: Vec<Span<'static>> = rendered
                .into_iter()
                .map(|span| {
                    // 保留行内代码等特殊样式，只覆盖基础文字颜色
                    let has_special_bg =
                        span.style.bg.is_some_and(|bg| bg != self.theme.bg_primary);
                    if has_special_bg {
                        span // 保留行内代码等自带背景的样式
                    } else {
                        Span::styled(
                            span.content,
                            span.style.fg.map_or(text_style, |fg| {
                                // 对于使用 text_normal 的普通文本，替换为 md_blockquote_text
                                if fg == self.theme.text_normal {
                                    text_style
                                } else {
                                    // 加粗/斜体等保留原前景色，只加 blockquote 背景
                                    Style::default().fg(fg).bg(self.theme.md_blockquote_bg)
                                }
                            }),
                        )
                    }
                })
                .collect();

            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled(format!("{} ", bar), bar_style),
            ];
            spans.extend(styled_rendered);
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
                    let inner = &remaining[..end];
                    let inner_spans = self.render_inline(inner);
                    for span in inner_spans {
                        spans.push(Span::styled(
                            span.content,
                            span.style.add_modifier(Modifier::BOLD),
                        ));
                    }
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
                    let inner = &remaining[..end];
                    let inner_spans = self.render_inline(inner);
                    for span in inner_spans {
                        spans.push(Span::styled(
                            span.content,
                            span.style.add_modifier(Modifier::CROSSED_OUT),
                        ));
                    }
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
                    let inner = &remaining[..end];
                    let inner_spans = self.render_inline(inner);
                    for span in inner_spans {
                        spans.push(Span::styled(
                            span.content,
                            span.style.add_modifier(Modifier::ITALIC),
                        ));
                    }
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
}
