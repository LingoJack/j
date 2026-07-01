//! Markdown 渲染器
//!
//! 负责将文本渲染为 ratatui 的 Line/Widget。
//! 支持代码块围栏样式、表格、Markdown 语法高亮等高级渲染。

mod block_cache;
mod code_block;
mod inline;
mod inline_width;
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
use crate::util::text::char_width;

use block_cache::BlockCache;

fn block_prefix_source_widths(line: &str) -> Option<Vec<u8>> {
    let trimmed = line.trim_start();
    if is_markdown_block_prefix_line(trimmed) {
        Some(
            line.chars()
                .map(|ch| char_width(ch).min(u8::MAX as usize) as u8)
                .collect(),
        )
    } else {
        None
    }
}

fn is_markdown_block_prefix_line(trimmed: &str) -> bool {
    trimmed.starts_with("# ")
        || trimmed.starts_with("## ")
        || trimmed.starts_with("### ")
        || trimmed.starts_with("#### ")
        || trimmed.starts_with('>')
        || trimmed.starts_with("- [ ]")
        || trimmed.starts_with("- [x]")
        || trimmed.starts_with("- [X]")
        || trimmed.starts_with("- ")
        || trimmed.starts_with("* ")
        || is_ordered_list_line(trimmed)
}

fn is_ordered_list_line(trimmed: &str) -> bool {
    let Some(marker_end) = trimmed.find(['.', ')']) else {
        return false;
    };
    let marker = &trimmed[..marker_end];
    if marker.is_empty() || !marker.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }
    trimmed
        .get(marker_end + 1..marker_end + 2)
        .is_some_and(|ch| ch == " ")
}

/// Markdown 渲染器
pub struct MarkdownRenderer {
    theme: EditorTheme,
    /// 水平滚动偏移
    horizontal_scroll: usize,
    /// 块级缓存（基于 pulldown-cmark 解析结果）
    pub(crate) block_cache: BlockCache,
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
            block_cache: BlockCache::new(),
            highlight_fn,
            show_line_numbers: true,
        }
    }

    /// 使代码块缓存失效
    pub fn invalidate_cache(&mut self) {
        self.block_cache.invalidate();
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
            format!("{:>4}  ", line_idx + 1)
        } else {
            String::new()
        }
    }

    /// 生成续行行号字符串（空格或空）
    fn format_continuation_line_number(&self) -> String {
        if self.show_line_numbers {
            "      ".to_string()
        } else {
            String::new()
        }
    }

    /// 确保块级缓存有效（按需重建）
    pub fn ensure_cache_valid(&mut self, lines: &[String], width: usize) {
        if !self.block_cache.is_valid_for(lines, width) {
            self.block_cache.build(lines, width);
        }
    }

    /// 返回所有代码块内容行的闭区间范围（不含围栏行本身）。
    ///
    /// 返回值如 `[(3, 8), (12, 20)]`，表示第 3~8 行和第 12~20 行是代码块内容行。
    pub fn code_block_content_ranges(&self) -> Vec<(usize, usize)> {
        self.block_cache.content_ranges()
    }

    /// 为给定的源码行集合计算"按渲染后宽度折行"用的 per-char 宽度数组。
    ///
    /// 返回 `Vec<Option<Vec<u8>>>`，长度 = `lines.len()`：
    /// - `Some(widths)`：该行折行宽度按渲染后算（Markdown 标记符号 = 0）；
    /// - `None`：该行按源码 `char_width` 算（fence 行、代码块内容行、表格行、
    ///   以及光标所在行 `cursor_line`）。
    ///
    /// **调用方约定**：先调 `ensure_cache_valid` 让 `BlockCache` 对齐当前 lines/width，
    /// 再调本方法。否则 fence / 代码块 / 表格行的判断可能基于旧的 block 缓存。
    pub fn compute_line_visible_widths(
        &self,
        lines: &[String],
        cursor_line: Option<usize>,
    ) -> Vec<Option<Vec<u8>>> {
        lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                // 光标所在行：严格按源码（让用户编辑标记符号时所见即所得）
                if cursor_line == Some(i) {
                    return None;
                }
                // 围栏行 / 代码块内 / 表格行：渲染逻辑特殊，不参与 inline 渲染宽度补偿
                if self.block_cache.is_fence_line(i)
                    || self.block_cache.is_in_code_block_content(i)
                    || self.block_cache.is_table_line(i)
                {
                    return None;
                }
                if let Some(widths) = block_prefix_source_widths(line) {
                    return Some(widths);
                }
                Some(inline_width::compute_visible_widths(line))
            })
            .collect()
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

    /// 创建代码块样式（背景统一用 bg_primary，与正文背景一致，
    /// 避免代码块在编辑器里形成与主背景突兀的色块）。
    #[inline]
    pub fn style_code(&self, fg: Color) -> Style {
        Style::default().fg(fg).bg(self.theme.bg_primary)
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
            "      ".to_string()
        } else {
            format!("{:>4}  ", logical_line + 1)
        };
        let line_num_style = if is_cursor_line {
            Style::default()
                .fg(Color::Yellow)
                .bg(self.theme.bg_primary)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::DarkGray)
                .bg(self.theme.bg_primary)
        };

        // ---- 光标行：显示源码 + 光标 ----
        // 代码块内的光标行用 wrap_width 驱动右边框对齐
        let code_block_max_width = if !self.block_cache.is_fence_line(logical_line)
            && self.block_cache.is_in_code_block_content(logical_line)
        {
            // wrap_width 已减去行号宽度，再减去围栏 4 + 右内边距 2 = 6 列
            // （与 wrap_engine::CODE_BLOCK_FRAME_WIDTH 和 code_block.rs 的 inner_width 保持一致）
            Some(wrap_width.max(10).saturating_sub(6))
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
        // 但代码块内的续行需要保持代码块的边框样式
        if is_continuation {
            // 检查续行是否在代码块内（非围栏行）
            let in_code_block = !self.block_cache.is_fence_line(logical_line)
                && self.block_cache.is_in_code_block_content(logical_line);

            // 代码块续行走统一的代码块内容渲染（带边框），不再单独处理
            if in_code_block {
                return vec![self.render_code_block_line_content(
                    vl_text,
                    logical_line,
                    lines,
                    wrap_width,
                    true, // is_continuation
                )];
            }

            // 表格续行：不渲染（关键修复点，改前请先看完）。
            //
            // 背景：当一行表格源码（如 `| col1 | col2 | ... |`）的显示宽度超过
            // wrap_width 时，wrap_engine 会按宽度把它拆成多个 VisualLine：
            //   - VL1（start_col=0）：走下方 render_table_rows 分支，产出完整的
            //     多行表格渲染（边框 + 单元格折行）。
            //   - VL2..K（start_col>0）：进入这里——is_continuation 分支。
            //
            // 历史 bug：此处曾把续行的源码尾段（含原始 `|` 字符）按 line_num + text
            // 原样塞回输出。结果在屏幕上：渲染好的表格行下面紧跟一段错位的源码片段，
            // 其中的 `|` 与上方表格的 `│` 位置对不上，视觉上就是"该行右侧边框被挤
            // 开、没闭合"。窄终端下源码被拆出的续行越多，垃圾文本越长，越像 bug。
            // 这就是用户描述的"窄变宽情况下表框线右边被挤开"现象。
            //
            // 修复：续行直接返回 vec![]。完整表格已由 VL1 渲染，续行不需要再贡献输出。
            //
            // 副作用注意：这会让 wrap_engine 给出的视觉行计数 K 与实际渲染行数 T
            // 不再相等（render_table_rows 自行决定 T）。这种不一致本来就存在，
            // 此修复只是让"多余的输出"消失，不引入新的不一致。Insert 模式下光标
            // 行走 render_cursor_visual_line（不经过这里），编辑长表格行的体验不变。
            if self.block_cache.is_table_line(logical_line) {
                return vec![];
            }

            // 引用块续行：保持引用块样式（与 thinking block 一致），渲染 inline
            let trimmed = line_content.trim_start();
            if trimmed.starts_with('>') {
                let mut level = 0;
                let mut rest = trimmed;
                while rest.starts_with('>') {
                    level += 1;
                    rest = rest[1..].trim_start();
                }

                let bg_color = self.theme.bg_primary;
                let bar: String = (0..level).map(|_| "|").collect::<Vec<_>>().join("");
                let bar_style = Style::default()
                    .fg(self.theme.md_blockquote_bar)
                    .bg(bg_color)
                    .add_modifier(Modifier::BOLD);
                let bq_text_style = Style::default()
                    .fg(self.theme.md_blockquote_text)
                    .bg(bg_color);

                // 对引用内容部分渲染 inline，然后提取续行片段
                let inline_spans = self.render_inline(rest);
                // vl.start_col / vl.end_col 已经是字符偏移（wrap_engine 使用字符索引）
                // 无需再用 char_idx_at_display_col 转换（该函数把参数当显示宽度，对中文会出错）
                let vl_start_char = vl.start_col;
                let vl_end_char = vl.end_col;

                // 计算引用前缀 "> " 的字符数（字节长度不等于字符数，需用 chars().count()）
                let prefix_chars = line_content.chars().count() - rest.chars().count();
                let adjusted_start = vl_start_char.saturating_sub(prefix_chars);
                let adjusted_end = vl_end_char.saturating_sub(prefix_chars);

                let vl_spans = extract_span_range(&inline_spans, adjusted_start, adjusted_end);

                let mut spans = vec![Span::styled(line_num_str.clone(), line_num_style)];
                spans.push(Span::styled("  ", Style::default()));
                spans.push(Span::styled(format!("{} ", bar), bar_style));
                // 应用引用块文本样式
                for span in vl_spans {
                    spans.push(Span::styled(span.content, span.style.patch(bq_text_style)));
                }
                return vec![Line::from(spans)];
            }

            // 普通续行：对完整逻辑行渲染 inline，然后提取对应视觉行的片段
            // 这样可以正确处理跨折行边界的 **bold** 等标记
            let full_line_spans = self.render_inline(line_content);
            // 注意坐标系：`full_line_spans` 是渲染后的产物，char 数 ≤ 源码 char 数
            // （`**`/`[`/`]`/`(url)` 等标记符号已被消费）。所以这里要用 vl 上的
            // "渲染端 char 索引" `visible_start_char / visible_end_char`，
            // 而不是 `vl.start_col / vl.end_col`（那是源码 char 索引）。
            let vl_spans =
                extract_span_range(&full_line_spans, vl.visible_start_char, vl.visible_end_char);

            let mut spans = vec![Span::styled(line_num_str.clone(), line_num_style)];
            if search.is_searching() && search.match_count() > 0 {
                // 搜索高亮叠加
                spans.extend(search.highlight_line(
                    logical_line,
                    vl_text,
                    &self.theme,
                    vl.start_col,
                ));
            } else {
                spans.extend(vl_spans);
            }
            return vec![Line::from(spans)];
        }

        // 非续行的非光标行：完整 Markdown 渲染
        //
        // 旧实现这里用 `truncate_to_display_width(line_content, wrap_width)` 按源码
        // `char_width` 截断到 wrap_width，防止终端二次折行。但当 wrap_engine 改成
        // 按"渲染后宽度"折行后，源码 char 数 ≠ 渲染 char 数：源码累加到 wrap_width
        // 会把可见正文砍掉（标记符号占 0 列，必须读到更多源码 char 才凑够 wrap_width
        // 个渲染列）。
        //
        // 正确的做法是用 wrap_engine 已经算好的 vl 范围（源码 char 边界）来截：
        //   - 若该行只占 1 个视觉行（vl 覆盖整行）：保留全部 `line_content`；
        //   - 若该行被折成多段：vl 是第一段（start_col=0..end_col），按 char 数
        //     截到 `vl.end_col` 即可。
        // 这两种情况都可统一成"截到 vl.end_col 个 char"。
        let truncated: String = if vl.end_col >= line_content.chars().count() {
            line_content.to_string()
        } else {
            line_content.chars().take(vl.end_col).collect()
        };

        // 检查是否是代码块围栏行
        if self.block_cache.is_fence_line(logical_line) {
            // BlockCache 中标记的 fence 行一定有配对的 CodeBlock
            return vec![self.render_code_fence_line(line_content, logical_line, wrap_width)];
        }
        // 解析器未识别的 ``` 行（极少见：pulldown-cmark 对未闭合 fence 也会
        // 产出延伸到 EOF 的 CodeBlock，但若 fence 出现在非块语法上下文里
        // 偶尔会被吞），回退到普通文本渲染，避免空白屏。
        if line_content.trim_start().starts_with("```") {
            let mut spans = vec![Span::styled(line_num_str.clone(), line_num_style)];
            if search.is_searching() && search.match_count() > 0 {
                spans.extend(search.highlight_line(logical_line, &truncated, &self.theme, 0));
            } else {
                spans.push(Span::styled(truncated, self.style(self.theme.text_normal)));
            }
            return vec![Line::from(spans)];
        }

        // 检查是否在完整的代码块内
        if self.block_cache.is_in_code_block_content(logical_line) {
            // 代码块内容行：需要用 vl_text（折行片段）而非完整 line_content，
            // 否则首行 VL 会渲染完整内容，续行又重复渲染尾部，造成字符重复。
            let text_for_render = if is_continuation || line_content.chars().count() > vl.end_col {
                // 被折行了（续行或非续行但行内容超出首个 VL 范围）
                vl_text
            } else {
                line_content
            };
            return vec![self.render_code_block_line_content(
                text_for_render,
                logical_line,
                lines,
                wrap_width,
                is_continuation,
            )];
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
        //
        // 关键决策：当 wrap_engine 按"渲染后宽度"折行后，**独立把第一段
        // `truncated` 扔给 `render_single_line_with_number` 会出现 char 数与
        // 整行渲染前 N char 不一致**（pulldown-cmark 会吃掉 Strong 闭合后的
        // 末尾空格、把未闭合的 `**` 当 Text 多出 char……），导致 vl[0] 和
        // vl[1+] 续行切片拼起来出现"少 / 多字符"。
        //
        // 修复：**只要该行被折成多段**，第一段也走和续行一致的"整行 inline
        // 渲染 + 按 visible char 索引切片"路径，保证两条路径的 char 序列
        // 严格连续。代价：折行场景下 heading 图标 / bullet / blockquote 竖条
        // 等块级前缀不再渲染（前缀字符以源码形式 fall through 到 inline）；
        // 这是与"字符不丢失"的取舍。未折行场景沿用 `render_single_line_with_number`。
        let is_wrapped = vl.end_col < line_content.chars().count();
        if search.is_searching() && search.match_count() > 0 {
            let mut spans = vec![Span::styled(line_num_str.clone(), line_num_style)];
            spans.extend(search.highlight_line(logical_line, &truncated, &self.theme, 0));
            vec![Line::from(spans)]
        } else if is_wrapped {
            // 折行的第一段：走整行 inline 切片路径（与续行一致），统一渲染产物坐标系
            let full_line_spans = self.render_inline(line_content);
            let vl_spans =
                extract_span_range(&full_line_spans, vl.visible_start_char, vl.visible_end_char);
            let mut spans = vec![Span::styled(line_num_str.clone(), line_num_style)];
            spans.extend(vl_spans);
            vec![Line::from(spans)]
        } else {
            // 未折行：完整一行，走块级前缀渲染（heading 图标 / bullet / blockquote 等）
            vec![self.render_single_line_with_number(line_content, logical_line, wrap_width)]
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

        let cursor_style = self
            .theme
            .cursor_fg
            .apply_fg(Style::default())
            .add_modifier(Modifier::BOLD);
        let cursor_style = self.theme.cursor_bg.apply_bg(cursor_style);

        // 计算光标在视觉行内的字符偏移
        // 需要加上行号占用的字符数，因为 overlay_cursor_on_spans 在包含行号的完整 spans 上定位
        let line_num_chars = if self.show_line_numbers { 6 } else { 0 };
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

    /// 将 tab 替换为空格（宽度为 4，与 char_width('\t') = 4 一致）
    ///
    /// 终端会将 tab 展开到下一个 tab stop（通常占 8 列），
    /// 但 char_width 计算时 tab = 4，必须用相同数量的空格替换，
    /// 否则显示宽度与计算宽度不一致，引起折行错位和鼠标点击偏移。
    fn normalize_tabs(text: &str) -> String {
        text.replace('\t', "    ")
    }

    /// 将文本截断到指定显示宽度（使用 unicode-width 精确计算）
    #[allow(dead_code)]
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

// ---------------------------------------------------------------------------
// 续行渲染辅助函数
// ---------------------------------------------------------------------------

/// 将显示列位置（基于 `char_width` 计算）转换为字符索引（0-based, exclusive）。
///
/// 从已渲染的 Span 列表中提取指定字符范围的片段。
///
/// `start_char` / `end_char` 是字符索引（0-based）。
/// 返回的 Span 保留原始样式。
fn extract_span_range(
    spans: &[Span<'static>],
    start_char: usize,
    end_char: usize,
) -> Vec<Span<'static>> {
    let mut result = Vec::with_capacity(spans.len());
    let mut chars_seen = 0;

    for span in spans {
        let span_chars: Vec<char> = span.content.chars().collect();
        let span_len = span_chars.len();
        let span_end = chars_seen + span_len;

        // 跳过完全在范围之前的 span
        if span_end <= start_char {
            chars_seen = span_end;
            continue;
        }

        // 超出范围，停止
        if chars_seen >= end_char {
            break;
        }

        // 计算本 span 内的截取范围
        let local_start = start_char.saturating_sub(chars_seen);
        let local_end = (end_char - chars_seen).min(span_len);

        if local_start < local_end {
            let text: String = span_chars[local_start..local_end].iter().collect();
            result.push(Span::styled(text, span.style));
        }

        chars_seen = span_end;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::{block_prefix_source_widths, inline_width};
    use crate::util::text::display_width;

    #[test]
    fn block_prefix_widths_keep_source_prefix_visible() {
        let heading = "### tail should not be hidden";
        let widths = block_prefix_source_widths(heading).expect("heading should use source widths");
        let inline_widths = inline_width::compute_visible_widths(heading);

        assert_eq!(
            widths.iter().map(|w| *w as usize).sum::<usize>(),
            display_width(heading)
        );
        assert!(
            inline_widths.iter().take(4).any(|w| *w == 0),
            "pulldown-cmark consumes heading marker as block syntax; block lines must not use that width map"
        );
    }

    #[test]
    fn regular_inline_widths_still_hide_inline_markers() {
        assert!(block_prefix_source_widths("normal **bold** text").is_none());
        let widths = inline_width::compute_visible_widths("normal **bold** text");
        assert!(widths.iter().any(|w| *w == 0));
    }
}
