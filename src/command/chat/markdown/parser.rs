mod table;
#[cfg(test)]
mod tests;
mod text;

use super::highlight::highlight_code_line;
use crate::command::chat::render::theme::Theme;
use crate::tui::editor_core::EditorTheme;
use crate::util::text::{display_width, wrap_text};
use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

// ---------------------------------------------------------------------------
// ParserState — shared state for the markdown event loop
// ---------------------------------------------------------------------------

/// Encapsulates all mutable state used during pulldown-cmark event processing.
pub(crate) struct ParserState<'a> {
    pub(crate) lines: Vec<Line<'static>>,
    pub(crate) current_spans: Vec<Span<'static>>,
    pub(crate) style_stack: Vec<Style>,

    // Code block
    pub(crate) in_code_block: bool,
    pub(crate) code_block_content: String,
    pub(crate) code_block_lang: String,

    // List
    pub(crate) list_depth: usize,
    pub(crate) ordered_index: Option<u64>,

    // Heading
    pub(crate) heading_level: Option<u8>,

    // Block quote
    pub(crate) in_blockquote: bool,

    // Link
    pub(crate) link_url: Option<String>,

    // Image
    pub(crate) image_url: Option<String>,
    pub(crate) image_alt: String,

    // Table
    pub(crate) in_table: bool,
    pub(crate) table_rows: Vec<Vec<String>>,
    pub(crate) current_row: Vec<String>,
    pub(crate) current_cell: String,
    pub(crate) table_alignments: Vec<pulldown_cmark::Alignment>,

    // Layout
    pub(crate) content_width: usize,
    pub(crate) theme: &'a Theme,
    pub(crate) base_style: Style,
}

impl<'a> ParserState<'a> {
    fn new(content_width: usize, theme: &'a Theme) -> Self {
        Self {
            lines: Vec::new(),
            current_spans: Vec::new(),
            style_stack: vec![Style::default().fg(theme.text_normal)],
            in_code_block: false,
            code_block_content: String::new(),
            code_block_lang: String::new(),
            list_depth: 0,
            ordered_index: None,
            heading_level: None,
            in_blockquote: false,
            link_url: None,
            image_url: None,
            image_alt: String::new(),
            in_table: false,
            table_rows: Vec::new(),
            current_row: Vec::new(),
            current_cell: String::new(),
            table_alignments: Vec::new(),
            content_width,
            theme,
            base_style: Style::default().fg(theme.text_normal),
        }
    }

    /// Flush current_spans into lines.
    pub(crate) fn flush_line(&mut self) {
        if !self.current_spans.is_empty() {
            self.lines
                .push(Line::from(std::mem::take(&mut self.current_spans)));
        }
    }
}

// ---------------------------------------------------------------------------
// Table separator normalization
// ---------------------------------------------------------------------------

/// 检测 markdown 文本中是否存在列数不足的表格分隔行。
///
/// 快速扫描：找到以 `|` 开头且仅含 `|---|`（一个分隔列）的行，
/// 而其上一行（表头）含有多个 `|` 分隔列的情况。
fn needs_table_separator_fix(md: &str) -> bool {
    let lines: Vec<&str> = md.lines().collect();
    for i in 1..lines.len() {
        let prev = lines[i - 1].trim();
        let curr = lines[i].trim();
        if prev.starts_with('|') && is_separator_row(curr) {
            let header_cols = count_pipe_cells(prev);
            let sep_cols = count_pipe_cells(curr);
            if header_cols > 1 && sep_cols < header_cols {
                return true;
            }
        }
    }
    false
}

/// 补齐所有表格分隔行的列数，使其与对应的表头列数匹配。
fn normalize_table_separators(md: &str) -> String {
    let lines: Vec<&str> = md.lines().collect();
    let mut result = String::with_capacity(md.len());
    let mut modified = false;

    for i in 0..lines.len() {
        if i > 0 {
            let prev = lines[i - 1].trim();
            let curr = lines[i].trim();
            if prev.starts_with('|') && is_separator_row(curr) {
                let header_cols = count_pipe_cells(prev);
                let sep_cols = count_pipe_cells(curr);
                if header_cols > 1 && sep_cols < header_cols {
                    // 直接重建分隔行：每列一个 "---"，用 "|" 包裹
                    let mut fixed = String::from("|");
                    for _ in 0..header_cols {
                        fixed.push_str("---|");
                    }
                    result.push_str(&fixed);
                    modified = true;
                    result.push('\n');
                    continue;
                }
            }
        }
        result.push_str(lines[i]);
        result.push('\n');
    }

    if modified {
        // 移除末尾多余的换行，保持与原文本一致
        if md.ends_with('\n') && !result.ends_with('\n') {
            result.push('\n');
        } else if !md.ends_with('\n') && result.ends_with('\n') {
            result.pop();
        }
        result
    } else {
        md.to_string()
    }
}

/// 判断一行是否是表格分隔行（仅含 `|`、`-`、`:`、空格）。
fn is_separator_row(line: &str) -> bool {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return false;
    }
    // 去掉首尾 | 后，内容应仅由 - : 空格 组成
    let inner = trimmed.trim_matches('|').trim();
    !inner.is_empty()
        && inner
            .chars()
            .all(|c| c == '-' || c == ':' || c == ' ' || c == '|')
}

/// 统计以 `|` 分隔的行的列数。
/// `| A | B |` → 2，`|---|` → 1，`|---|---|` → 2
fn count_pipe_cells(line: &str) -> usize {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return 0;
    }
    // 按非转义的 | 分割，减去首尾空段
    let segments: Vec<&str> = trimmed.split('|').collect();
    // "| A | B |" → ["", " A ", " B ", ""] → 2 cells
    // "|---|" → ["", "---", ""] → 1 cell
    let count = segments.len().saturating_sub(1);
    // 如果最后一个段是空（尾 |），减 1
    if count > 0 && segments.last().is_some_and(|s| s.trim().is_empty()) {
        count - 1
    } else {
        count
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// 将 Markdown 文本渲染为 TUI 可显示的 `Line` 列表，应用主题着色和自动换行。
pub fn markdown_to_lines(md: &str, max_width: usize, theme: &Theme) -> Vec<Line<'static>> {
    // 内容区宽度 = max_width - 2（左侧 "  " 缩进由外层负责）
    let content_width = max_width.saturating_sub(2);

    // 预处理：修复 **"text"** 加粗不生效的问题。
    // CommonMark 规范规定：左侧分隔符 ** 后面是标点（如 " U+201C）且前面是字母（如中文字符）时，
    // 不被识别为有效的加粗开始标记。
    // 解决方案：在 ** 与中文引号之间插入零宽空格（U+200B），使 ** 后面不再紧跟标点，
    // 从而满足 CommonMark 规范。零宽空格在终端中不可见，不影响显示。
    let mut md_owned;
    let md = if md.contains("**\u{201C}")
        || md.contains("**\u{2018}")
        || md.contains("\u{201D}**")
        || md.contains("\u{2019}**")
    {
        md_owned = md
            .replace("**\u{201C}", "**\u{200B}\u{201C}")
            .replace("**\u{2018}", "**\u{200B}\u{2018}")
            .replace("\u{201D}**", "\u{201D}\u{200B}**")
            .replace("\u{2019}**", "\u{2019}\u{200B}**");
        &md_owned as &str
    } else {
        md
    };

    // 预处理：补齐表格分隔行列数。
    // pulldown-cmark 要求分隔行的列数与表头匹配，否则整个表格不被识别。
    // LLM 生成的表格有时分隔行只写 |---| 而表头有多列，需要自动补齐为 |---|---|。
    let separator_fixed;
    let md = if needs_table_separator_fix(md) {
        separator_fixed = normalize_table_separators(md);
        md_owned = separator_fixed;
        &md_owned as &str
    } else {
        md
    };

    let options = pulldown_cmark::Options::ENABLE_STRIKETHROUGH
        | pulldown_cmark::Options::ENABLE_TABLES
        | pulldown_cmark::Options::ENABLE_TASKLISTS;
    let parser = pulldown_cmark::Parser::new_ext(md, options);

    let mut state = ParserState::new(content_width, theme);

    for event in parser {
        match event {
            // ===== Heading =====
            Event::Start(Tag::Heading { level, .. }) => {
                state.flush_line();
                state.heading_level = Some(level as u8);
                if !state.lines.is_empty() {
                    state.lines.push(Line::from(""));
                }
                let heading_style = match level as u8 {
                    1 => Style::default()
                        .fg(theme.md_h1)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    2 => Style::default()
                        .fg(theme.md_h2)
                        .add_modifier(Modifier::BOLD),
                    3 => Style::default()
                        .fg(theme.md_h3)
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default()
                        .fg(theme.md_h4)
                        .add_modifier(Modifier::BOLD),
                };
                state.style_stack.push(heading_style);

                // 添加前缀
                let (prefix, prefix_style) = match level as u8 {
                    1 => (
                        "◆ ",
                        Style::default()
                            .fg(theme.md_h1)
                            .add_modifier(Modifier::BOLD),
                    ),
                    2 => (
                        "◇ ",
                        Style::default()
                            .fg(theme.md_h2)
                            .add_modifier(Modifier::BOLD),
                    ),
                    3 => (
                        "〈",
                        Style::default()
                            .fg(theme.md_h3)
                            .add_modifier(Modifier::BOLD),
                    ),
                    _ => (
                        "› ",
                        Style::default()
                            .fg(theme.md_h4)
                            .add_modifier(Modifier::BOLD),
                    ),
                };
                state
                    .current_spans
                    .push(Span::styled(prefix.to_string(), prefix_style));
            }
            Event::End(TagEnd::Heading(level)) => {
                let level_u8 = level as u8;
                // H3 添加文艺风格后缀
                if level_u8 == 3 {
                    state.current_spans.push(Span::styled(
                        "〉".to_string(),
                        Style::default()
                            .fg(theme.md_h3)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                state.flush_line();
                // H1/H2 显示分隔线
                if level_u8 <= 2 {
                    let sep_char = if level_u8 == 1 { "━" } else { "─" };
                    state.lines.push(Line::from(Span::styled(
                        sep_char.repeat(content_width),
                        Style::default().fg(theme.md_heading_sep),
                    )));
                }
                state.style_stack.pop();
                state.heading_level = None;
            }

            // ===== Strong / Emphasis / Strikethrough =====
            Event::Start(Tag::Strong) => {
                let current = *state.style_stack.last().unwrap_or(&state.base_style);
                state
                    .style_stack
                    .push(current.add_modifier(Modifier::BOLD).fg(theme.text_bold));
            }
            Event::End(TagEnd::Strong) => {
                state.style_stack.pop();
            }
            Event::Start(Tag::Emphasis) => {
                let current = *state.style_stack.last().unwrap_or(&state.base_style);
                state
                    .style_stack
                    .push(current.add_modifier(Modifier::ITALIC));
            }
            Event::End(TagEnd::Emphasis) => {
                state.style_stack.pop();
            }
            Event::Start(Tag::Strikethrough) => {
                let current = *state.style_stack.last().unwrap_or(&state.base_style);
                state
                    .style_stack
                    .push(current.add_modifier(Modifier::CROSSED_OUT));
            }
            Event::End(TagEnd::Strikethrough) => {
                state.style_stack.pop();
            }

            // ===== Link =====
            Event::Start(Tag::Link { dest_url, .. }) => {
                let link_style = Style::default()
                    .fg(theme.md_link)
                    .add_modifier(Modifier::UNDERLINED);
                state.style_stack.push(link_style);
                state.link_url = Some(dest_url.to_string());
            }
            Event::End(TagEnd::Link) => {
                // 如果链接文本和 URL 不同，在文本后追加显示 URL
                if let Some(url) = state.link_url.take() {
                    let text_content: String = state
                        .current_spans
                        .iter()
                        .rev()
                        .take_while(|s| s.style.fg == Some(theme.md_link))
                        .map(|s| s.content.to_string())
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                    if !text_content.is_empty() && text_content != url {
                        state.current_spans.push(Span::styled(
                            format!(" ({})", url),
                            Style::default()
                                .fg(theme.md_link)
                                .add_modifier(Modifier::DIM),
                        ));
                    }
                }
                state.style_stack.pop();
            }

            // ===== Code Block =====
            Event::Start(Tag::CodeBlock(kind)) => {
                state.flush_line();
                state.in_code_block = true;
                state.code_block_content.clear();
                state.code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                let label = if state.code_block_lang.is_empty() {
                    " code ".to_string()
                } else {
                    format!(" {} ", state.code_block_lang)
                };
                let label_w = display_width(&label);
                // 顶边框：┌─ label ───┐
                let border_fill = content_width.saturating_sub(3 + label_w);
                let top_border = format!("┌─{}{}┐", label, "─".repeat(border_fill));
                state.lines.push(Line::from(Span::styled(
                    top_border,
                    Style::default().fg(theme.code_border).bg(theme.code_bg),
                )));
            }
            Event::End(TagEnd::CodeBlock) => {
                let code_inner_w = content_width.saturating_sub(4);
                let code_content_expanded = state.code_block_content.replace('\t', "    ");
                for code_line in code_content_expanded.lines() {
                    let wrapped = wrap_text(code_line, code_inner_w);
                    for wl in wrapped {
                        let editor_theme = EditorTheme::from(theme);
                        let highlighted =
                            highlight_code_line(&wl, &state.code_block_lang, &editor_theme);
                        let text_w: usize =
                            highlighted.iter().map(|s| display_width(&s.content)).sum();
                        let fill = code_inner_w.saturating_sub(text_w);
                        let mut spans_vec = Vec::new();
                        // 左侧边框：│ (2字符，有背景色)
                        spans_vec.push(Span::styled(
                            "│ ",
                            Style::default().fg(theme.code_border).bg(theme.code_bg),
                        ));
                        // 代码内容（有背景色）
                        for hs in highlighted {
                            spans_vec.push(Span::styled(
                                hs.content.to_string(),
                                hs.style.bg(theme.code_bg),
                            ));
                        }
                        // 右侧填充 + 边框
                        spans_vec.push(Span::styled(
                            format!("{} │", " ".repeat(fill)),
                            Style::default().fg(theme.code_border).bg(theme.code_bg),
                        ));
                        state.lines.push(Line::from(spans_vec));
                    }
                }
                // 底边框
                let bottom_border = format!("└{}┘", "─".repeat(content_width.saturating_sub(2)));
                state.lines.push(Line::from(Span::styled(
                    bottom_border,
                    Style::default().fg(theme.code_border).bg(theme.code_bg),
                )));
                state.in_code_block = false;
                state.code_block_content.clear();
                state.code_block_lang.clear();
            }

            // ===== Inline Code =====
            Event::Code(text) => {
                if state.in_table {
                    state.handle_code_in_table(&text);
                } else {
                    let code_str = format!(" {} ", text);
                    let code_w = display_width(&code_str);
                    let effective_prefix_w = if state.in_blockquote { 2 } else { 0 };
                    let full_line_w = content_width.saturating_sub(effective_prefix_w);
                    let existing_w: usize = state
                        .current_spans
                        .iter()
                        .map(|s| display_width(&s.content))
                        .sum();
                    let content_w_on_line = existing_w.saturating_sub(effective_prefix_w);
                    if content_w_on_line + code_w > full_line_w && !state.current_spans.is_empty() {
                        state.flush_line();
                        if state.in_blockquote {
                            state.current_spans.push(Span::styled(
                                "| ".to_string(),
                                Style::default()
                                    .fg(theme.md_blockquote_bar)
                                    .bg(theme.md_blockquote_bg)
                                    .add_modifier(Modifier::BOLD),
                            ));
                        }
                    }
                    state.current_spans.push(Span::styled(
                        code_str,
                        Style::default()
                            .fg(theme.md_inline_code_fg)
                            .bg(theme.md_inline_code_bg),
                    ));
                }
            }

            // ===== List =====
            Event::Start(Tag::List(start)) => {
                state.flush_line();
                state.list_depth += 1;
                state.ordered_index = start;
            }
            Event::End(TagEnd::List(_)) => {
                state.flush_line();
                state.list_depth = state.list_depth.saturating_sub(1);
                state.ordered_index = None;
            }
            Event::Start(Tag::Item) => {
                state.flush_line();
                let indent = "  ".repeat(state.list_depth);
                let bullet = if let Some(ref mut idx) = state.ordered_index {
                    let s = format!("{}{}. ", indent, idx);
                    *idx += 1;
                    s
                } else {
                    format!("{}• ", indent)
                };
                state.current_spans.push(Span::styled(
                    bullet,
                    Style::default().fg(theme.md_list_bullet),
                ));
            }
            Event::End(TagEnd::Item) => {
                state.flush_line();
            }
            Event::TaskListMarker(checked) => {
                // 替换 Start(Item) 插入的 • 子弹为复选框符号
                if let Some(last) = state.current_spans.last_mut() {
                    let indent: String = last.content.chars().take_while(|c| *c == ' ').collect();
                    let (symbol, style) = if checked {
                        (
                            format!("{}● ", indent),
                            Style::default()
                                .fg(ratatui::style::Color::LightGreen)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        (
                            format!("{}○ ", indent),
                            Style::default().fg(theme.md_list_bullet),
                        )
                    };
                    *last = Span::styled(symbol, style);
                }
            }

            // ===== Paragraph =====
            Event::Start(Tag::Paragraph)
                if !state.lines.is_empty()
                    && !state.in_code_block
                    && state.heading_level.is_none() =>
            {
                let last_empty = state
                    .lines
                    .last()
                    .map(|l| l.spans.is_empty())
                    .unwrap_or(false);
                if !last_empty {
                    state.lines.push(Line::from(""));
                }
            }
            Event::End(TagEnd::Paragraph) => {
                state.flush_line();
            }

            // ===== Block Quote =====
            Event::Start(Tag::BlockQuote(_)) => {
                state.flush_line();
                state.lines.push(Line::from(""));
                state.in_blockquote = true;
                state.style_stack.push(
                    Style::default()
                        .fg(theme.md_blockquote_text)
                        .bg(theme.md_blockquote_bg),
                );
                state.current_spans.push(Span::styled(
                    "| ".to_string(),
                    Style::default()
                        .fg(theme.md_blockquote_bar)
                        .bg(theme.md_blockquote_bg)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                state.flush_line();
                state.in_blockquote = false;
                state.style_stack.pop();
                state.lines.push(Line::from(""));
            }

            // ===== Text (delegated to text.rs) =====
            Event::Text(text) => {
                state.handle_text_event(&text);
            }

            // ===== Soft / Hard Break =====
            Event::SoftBreak => {
                if state.in_table {
                    state.handle_soft_break_in_table();
                } else {
                    state.current_spans.push(Span::raw(" "));
                }
            }
            Event::HardBreak => {
                if state.in_table {
                    state.handle_hard_break_in_table();
                } else {
                    state.flush_line();
                }
            }

            // ===== Rule =====
            Event::Rule => {
                state.flush_line();
                state.lines.push(Line::from(Span::styled(
                    "─".repeat(content_width),
                    Style::default().fg(theme.md_rule),
                )));
            }

            // ===== Table (delegated to table.rs) =====
            Event::Start(Tag::Table(alignments)) => {
                state.handle_table_start(alignments);
            }
            Event::End(TagEnd::Table) => {
                state.handle_table_end();
            }
            Event::Start(Tag::TableHead) => {
                state.handle_table_head_start();
            }
            Event::End(TagEnd::TableHead) => {
                state.handle_table_head_end();
            }
            Event::Start(Tag::TableRow) => {
                state.handle_table_row_start();
            }
            Event::End(TagEnd::TableRow) => {
                state.handle_table_row_end();
            }
            Event::Start(Tag::TableCell) => {
                state.handle_table_cell_start();
            }
            Event::End(TagEnd::TableCell) => {
                state.handle_table_cell_end();
            }

            // ===== Image =====
            Event::Start(Tag::Image { dest_url, .. }) => {
                state.flush_line();
                state.image_url = Some(dest_url.to_string());
                state.image_alt.clear();
            }
            Event::End(TagEnd::Image) => {
                if let Some(url) = state.image_url.take() {
                    let placeholder_height = 16u16;
                    let marker = format!("\x00IMG:{}:{}", placeholder_height, url);
                    // 图片标记行（供渲染层识别，渲染时覆盖为图片）
                    state
                        .lines
                        .push(Line::from(Span::styled(marker, Style::default())));
                    // 占位空行（预留渲染空间）
                    for _ in 1..placeholder_height {
                        state.lines.push(Line::from(Span::raw("")));
                    }
                    // 图片路径标注行
                    let caption = format!("({})", url);
                    state.lines.push(Line::from(Span::styled(
                        caption,
                        Style::default()
                            .fg(ratatui::style::Color::DarkGray)
                            .add_modifier(Modifier::DIM),
                    )));
                }
                state.image_alt.clear();
            }

            _ => {}
        }
    }

    // 刷新最后一行
    if !state.current_spans.is_empty() {
        state.lines.push(Line::from(state.current_spans));
    }

    // 如果解析结果为空，至少返回原始文本
    if state.lines.is_empty() {
        let wrapped = wrap_text(md, content_width);
        for wl in wrapped {
            state
                .lines
                .push(Line::from(Span::styled(wl, state.base_style)));
        }
    }

    state.lines
}
