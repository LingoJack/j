mod table;
#[cfg(test)]
mod tests;
mod text;

use crate::markdown::ir::{
    Block, BlockKind, Inline, ListData, ListItem, ParsedDocument, SourceRange, TableData,
};
use crate::util::text::normalize_terminal_text;
use pulldown_cmark::{Event, Tag, TagEnd};

// ---------------------------------------------------------------------------
// ParseContext — IR accumulation state for the markdown event loop
// ---------------------------------------------------------------------------

/// Inline 容器类型，用于 inline 嵌套栈
#[derive(Debug, Clone)]
enum InlineContainer {
    Strong,
    Emphasis,
    Strikethrough,
    Link { url: String },
}

/// 解析上下文：累积 IR 节点
struct ParseContext {
    /// 已解析的 block 列表
    blocks: Vec<Block>,

    /// 当前 event 的源码范围（由 offset_iter 填充）
    current_source: SourceRange,

    // --- Inline 累积 ---
    /// 当前 block 级别的 inline 容器（paragraph / heading content / list item）
    current_inlines: Vec<Inline>,
    /// 嵌套栈：记录当前 inline 容器类型
    inline_stack: Vec<InlineContainer>,
    /// 嵌套子容器：每层开始一个新的 Vec<Inline>
    inline_children_stack: Vec<Vec<Inline>>,

    // --- Code block ---
    in_code_block: bool,
    code_block_content: String,
    code_block_lang: String,

    // --- List ---
    list_depth: usize,
    ordered_index: Option<u64>,
    list_ordered: bool,
    list_start_index: Option<u64>,
    list_items: Vec<ListItem>,
    in_list_item: bool,
    list_item_checked: Option<bool>,

    // --- Heading ---
    heading_level: Option<u8>,

    // --- Block quote ---
    blockquote_stack: Vec<Vec<Block>>,

    // --- Image ---
    image_url: Option<String>,
    image_alt: String,

    // --- Table ---
    in_table: bool,
    table_rows: Vec<Vec<Vec<Inline>>>,
    current_row: Vec<Vec<Inline>>,
    current_cell_inlines: Vec<Inline>,
    table_alignments: Vec<pulldown_cmark::Alignment>,
    /// 表格单元格内的 inline 栈
    table_inline_stack: Vec<InlineContainer>,
    table_inline_children_stack: Vec<Vec<Inline>>,
}

impl ParseContext {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            current_source: SourceRange::default(),
            current_inlines: Vec::new(),
            inline_stack: Vec::new(),
            inline_children_stack: Vec::new(),
            in_code_block: false,
            code_block_content: String::new(),
            code_block_lang: String::new(),
            list_depth: 0,
            ordered_index: None,
            list_ordered: false,
            list_start_index: None,
            list_items: Vec::new(),
            in_list_item: false,
            list_item_checked: None,
            heading_level: None,
            blockquote_stack: Vec::new(),
            image_url: None,
            image_alt: String::new(),
            in_table: false,
            table_rows: Vec::new(),
            current_row: Vec::new(),
            current_cell_inlines: Vec::new(),
            table_alignments: Vec::new(),
            table_inline_stack: Vec::new(),
            table_inline_children_stack: Vec::new(),
        }
    }

    /// 获取当前 inline 容器（可能在外层或 inline_children_stack 的最内层）
    fn current_inline_target(&mut self) -> &mut Vec<Inline> {
        if self.in_table {
            return self.table_inline_target();
        }
        if let Some(children) = self.inline_children_stack.last_mut() {
            children
        } else if self.in_list_item {
            // 列表项内容追加到最后一个 item
            if let Some(item) = self.list_items.last_mut() {
                &mut item.content
            } else {
                &mut self.current_inlines
            }
        } else {
            &mut self.current_inlines
        }
    }

    /// 表格内 inline 目标
    fn table_inline_target(&mut self) -> &mut Vec<Inline> {
        if let Some(children) = self.table_inline_children_stack.last_mut() {
            children
        } else {
            &mut self.current_cell_inlines
        }
    }

    /// 将当前 inline 容器 flush 为一个 block（Paragraph）
    fn flush_paragraph(&mut self) {
        if self.current_inlines.is_empty() {
            return;
        }
        let inlines = std::mem::take(&mut self.current_inlines);
        let block = Block {
            source: self.current_source,
            kind: BlockKind::Paragraph(inlines),
        };
        self.push_block(block);
    }

    /// Push block：如果在 blockquote 内则追加到 blockquote 栈顶，否则追加到 blocks
    fn push_block(&mut self, block: Block) {
        if let Some(bq_blocks) = self.blockquote_stack.last_mut() {
            bq_blocks.push(block);
        } else {
            self.blocks.push(block);
        }
    }

    /// Push inline 到当前目标容器
    fn push_inline(&mut self, inline: Inline) {
        self.current_inline_target().push(inline);
    }
}

// ---------------------------------------------------------------------------
// Table separator normalization (unchanged from previous implementation)
// ---------------------------------------------------------------------------

/// 检测 markdown 文本中是否存在列数不足的表格分隔行。
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
    let inner = trimmed.trim_matches('|').trim();
    !inner.is_empty()
        && inner
            .chars()
            .all(|c| c == '-' || c == ':' || c == ' ' || c == '|')
}

/// 统计以 `|` 分隔的行的列数。
fn count_pipe_cells(line: &str) -> usize {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return 0;
    }
    let segments: Vec<&str> = trimmed.split('|').collect();
    let count = segments.len().saturating_sub(1);
    if count > 0 && segments.last().is_some_and(|s| s.trim().is_empty()) {
        count - 1
    } else {
        count
    }
}

// ---------------------------------------------------------------------------
// Byte offset → line number mapping
// ---------------------------------------------------------------------------

/// 构建行起始字节偏移表
fn build_line_offsets(text: &str) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(64);
    offsets.push(0);
    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// 将字节偏移转换为行号（0-based）
fn byte_to_line(byte: usize, line_offsets: &[usize]) -> usize {
    match line_offsets.binary_search(&(byte + 1)) {
        Ok(idx) => idx,
        Err(idx) => idx.saturating_sub(1),
    }
}

// ---------------------------------------------------------------------------
// Public API: parse_markdown
// ---------------------------------------------------------------------------

/// 纯解析：将 Markdown 文本解析为 IR 文档结构。
/// 不依赖终端宽度或主题，输出与渲染无关的中间表示。
pub fn parse_markdown(md: &str, max_width: usize) -> ParsedDocument {
    // 预处理：tab/carriage return 清洗
    let normalized_md;
    let md = if md.contains('\t') || md.contains('\r') {
        normalized_md = normalize_terminal_text(md);
        normalized_md.as_str()
    } else {
        md
    };

    // 预处理：中文引号与加粗标记的零宽空格
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

    // 预处理：表格分隔行修复（需要 max_width 仅用于判断逻辑，此处保留接口）
    let separator_fixed;
    let md = if needs_table_separator_fix(md) {
        separator_fixed = normalize_table_separators(md);
        md_owned = separator_fixed;
        &md_owned as &str
    } else {
        md
    };

    // 使用 max_width 避免未使用警告（表格分隔行修复可能间接用到）
    let _ = max_width;

    // 构建行偏移表（基于预处理后的文本）
    let line_offsets = build_line_offsets(md);

    let options = pulldown_cmark::Options::ENABLE_STRIKETHROUGH
        | pulldown_cmark::Options::ENABLE_TABLES
        | pulldown_cmark::Options::ENABLE_TASKLISTS;
    let parser = pulldown_cmark::Parser::new_ext(md, options);

    let mut ctx = ParseContext::new();

    for (event, range) in parser.into_offset_iter() {
        // 计算当前 event 的源码行范围
        let source = if !range.is_empty() {
            SourceRange {
                start_line: byte_to_line(range.start, &line_offsets),
                end_line: byte_to_line(range.end.saturating_sub(1), &line_offsets),
            }
        } else {
            SourceRange::default()
        };
        ctx.current_source = source;
        match event {
            // ===== Heading =====
            Event::Start(Tag::Heading { level, .. }) => {
                ctx.flush_paragraph();
                ctx.heading_level = Some(level as u8);
            }
            Event::End(TagEnd::Heading(level)) => {
                let content = std::mem::take(&mut ctx.current_inlines);
                let block = Block {
                    source: ctx.current_source,
                    kind: BlockKind::Heading {
                        level: level as u8,
                        content,
                    },
                };
                ctx.push_block(block);
                ctx.heading_level = None;
            }

            // ===== Strong / Emphasis / Strikethrough =====
            Event::Start(Tag::Strong) => {
                ctx.inline_stack.push(InlineContainer::Strong);
                ctx.inline_children_stack.push(Vec::new());
            }
            Event::End(TagEnd::Strong) => {
                ctx.inline_stack.pop();
                let children = ctx.inline_children_stack.pop().unwrap_or_default();
                ctx.push_inline(Inline::Strong(children));
            }
            Event::Start(Tag::Emphasis) => {
                ctx.inline_stack.push(InlineContainer::Emphasis);
                ctx.inline_children_stack.push(Vec::new());
            }
            Event::End(TagEnd::Emphasis) => {
                ctx.inline_stack.pop();
                let children = ctx.inline_children_stack.pop().unwrap_or_default();
                ctx.push_inline(Inline::Emphasis(children));
            }
            Event::Start(Tag::Strikethrough) => {
                ctx.inline_stack.push(InlineContainer::Strikethrough);
                ctx.inline_children_stack.push(Vec::new());
            }
            Event::End(TagEnd::Strikethrough) => {
                ctx.inline_stack.pop();
                let children = ctx.inline_children_stack.pop().unwrap_or_default();
                ctx.push_inline(Inline::Strikethrough(children));
            }

            // ===== Link =====
            Event::Start(Tag::Link { dest_url, .. }) => {
                ctx.inline_stack.push(InlineContainer::Link {
                    url: dest_url.to_string(),
                });
                ctx.inline_children_stack.push(Vec::new());
            }
            Event::End(TagEnd::Link) => {
                let container = ctx.inline_stack.pop();
                let children = ctx.inline_children_stack.pop().unwrap_or_default();
                if let Some(InlineContainer::Link { url }) = container {
                    ctx.push_inline(Inline::Link {
                        text: children,
                        url,
                    });
                }
            }

            // ===== Code Block =====
            Event::Start(Tag::CodeBlock(kind)) => {
                ctx.flush_paragraph();
                ctx.in_code_block = true;
                ctx.code_block_content.clear();
                ctx.code_block_lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
            }
            Event::End(TagEnd::CodeBlock) => {
                let block = Block {
                    source: ctx.current_source,
                    kind: BlockKind::CodeBlock {
                        lang: std::mem::take(&mut ctx.code_block_lang),
                        code: std::mem::take(&mut ctx.code_block_content),
                    },
                };
                ctx.push_block(block);
                ctx.in_code_block = false;
            }

            // ===== Inline Code =====
            Event::Code(text) => {
                if ctx.in_table {
                    ctx.table_handle_code(&text);
                } else {
                    ctx.push_inline(Inline::Code(text.to_string()));
                }
            }

            // ===== List =====
            Event::Start(Tag::List(start)) => {
                ctx.flush_paragraph();
                ctx.list_depth += 1;
                ctx.ordered_index = start;
                ctx.list_ordered = start.is_some();
                ctx.list_start_index = start;
                if ctx.list_items.is_empty() {
                    ctx.list_items = Vec::new();
                }
            }
            Event::End(TagEnd::List(_)) => {
                ctx.flush_paragraph();
                // 收集当前 list items 为一个 List block
                if ctx.list_depth == 1 && !ctx.list_items.is_empty() {
                    let items = std::mem::take(&mut ctx.list_items);
                    let block = Block {
                        source: ctx.current_source,
                        kind: BlockKind::List(ListData {
                            ordered: ctx.list_ordered,
                            start_index: ctx.list_start_index,
                            items,
                        }),
                    };
                    ctx.push_block(block);
                }
                ctx.list_depth = ctx.list_depth.saturating_sub(1);
                ctx.ordered_index = None;
            }
            Event::Start(Tag::Item) => {
                ctx.in_list_item = true;
                ctx.list_item_checked = None;
            }
            Event::End(TagEnd::Item) => {
                // flush current_inlines 到 list item
                let content = std::mem::take(&mut ctx.current_inlines);
                ctx.list_items.push(ListItem {
                    checked: ctx.list_item_checked,
                    content,
                });
                ctx.in_list_item = false;
                ctx.list_item_checked = None;
            }
            Event::TaskListMarker(checked) => {
                ctx.list_item_checked = Some(checked);
            }

            // ===== Paragraph =====
            Event::Start(Tag::Paragraph) => {
                // paragraph 开始时无需特殊处理
            }
            Event::End(TagEnd::Paragraph) => {
                ctx.flush_paragraph();
            }

            // ===== Block Quote =====
            Event::Start(Tag::BlockQuote(_)) => {
                ctx.flush_paragraph();
                ctx.blockquote_stack.push(Vec::new());
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                ctx.flush_paragraph();
                let inner_blocks = ctx.blockquote_stack.pop().unwrap_or_default();
                let block = Block {
                    source: ctx.current_source,
                    kind: BlockKind::BlockQuote(inner_blocks),
                };
                ctx.push_block(block);
            }

            // ===== Text =====
            Event::Text(text) => {
                if ctx.image_url.is_some() {
                    ctx.image_alt.push_str(&text);
                } else if ctx.in_code_block {
                    ctx.code_block_content.push_str(&text);
                } else if ctx.in_table {
                    ctx.table_handle_text(&text);
                } else {
                    ctx.push_inline(Inline::Text(text.to_string()));
                }
            }

            // ===== Soft / Hard Break =====
            Event::SoftBreak => {
                if ctx.in_table {
                    ctx.current_cell_inlines.push(Inline::SoftBreak);
                } else {
                    ctx.push_inline(Inline::SoftBreak);
                }
            }
            Event::HardBreak => {
                if ctx.in_table {
                    ctx.current_cell_inlines.push(Inline::HardBreak);
                } else {
                    ctx.push_inline(Inline::HardBreak);
                }
            }

            // ===== Rule =====
            Event::Rule => {
                ctx.flush_paragraph();
                ctx.push_block(Block {
                    source: ctx.current_source,
                    kind: BlockKind::Rule,
                });
            }

            // ===== Table =====
            Event::Start(Tag::Table(alignments)) => {
                ctx.flush_paragraph();
                ctx.in_table = true;
                ctx.table_rows.clear();
                ctx.table_alignments = alignments;
            }
            Event::End(TagEnd::Table) => {
                ctx.flush_paragraph();
                ctx.in_table = false;
                let data = TableData {
                    alignments: std::mem::take(&mut ctx.table_alignments),
                    rows: std::mem::take(&mut ctx.table_rows),
                };
                ctx.push_block(Block {
                    source: ctx.current_source,
                    kind: BlockKind::Table(data),
                });
            }
            Event::Start(Tag::TableHead) => {
                ctx.current_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                let row = std::mem::take(&mut ctx.current_row);
                ctx.table_rows.push(row);
            }
            Event::Start(Tag::TableRow) => {
                ctx.current_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                let row = std::mem::take(&mut ctx.current_row);
                ctx.table_rows.push(row);
            }
            Event::Start(Tag::TableCell) => {
                ctx.current_cell_inlines.clear();
                ctx.table_inline_stack.clear();
                ctx.table_inline_children_stack.clear();
            }
            Event::End(TagEnd::TableCell) => {
                // flush table inline stack
                ctx.table_flush_inline_stack();
                let cell = std::mem::take(&mut ctx.current_cell_inlines);
                ctx.current_row.push(cell);
            }

            // ===== Image =====
            Event::Start(Tag::Image { dest_url, .. }) => {
                ctx.flush_paragraph();
                ctx.image_url = Some(dest_url.to_string());
                ctx.image_alt.clear();
            }
            Event::End(TagEnd::Image) => {
                // 图片在当前 IR 中作为 Paragraph 处理（带特殊 marker）
                // 后续 Step 再添加 Image block kind
                if let Some(_url) = ctx.image_url.take() {
                    // 暂时忽略图片的 IR 处理，渲染时图片仍由外部机制处理
                }
                ctx.image_alt.clear();
            }

            _ => {}
        }
    }

    // flush 残余 inline
    ctx.flush_paragraph();

    ParsedDocument {
        blocks: ctx.blocks,
        line_to_block: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// ParseContext table helpers
// ---------------------------------------------------------------------------

impl ParseContext {
    fn table_handle_text(&mut self, text: &str) {
        self.table_flush_inline_stack();
        self.current_cell_inlines
            .push(Inline::Text(text.to_string()));
    }

    fn table_handle_code(&mut self, text: &str) {
        self.table_flush_inline_stack();
        self.current_cell_inlines
            .push(Inline::Code(text.to_string()));
    }

    /// Flush inline children stack into current cell
    fn table_flush_inline_stack(&mut self) {
        // 如果有未关闭的 inline 栈，将 children 合并回 cell
        while let Some(children) = self.table_inline_children_stack.pop() {
            let container = self.table_inline_stack.pop();
            match container {
                Some(InlineContainer::Strong) => {
                    self.current_cell_inlines.push(Inline::Strong(children));
                }
                Some(InlineContainer::Emphasis) => {
                    self.current_cell_inlines.push(Inline::Emphasis(children));
                }
                Some(InlineContainer::Strikethrough) => {
                    self.current_cell_inlines
                        .push(Inline::Strikethrough(children));
                }
                Some(InlineContainer::Link { url }) => {
                    self.current_cell_inlines.push(Inline::Link {
                        text: children,
                        url,
                    });
                }
                None => {
                    self.current_cell_inlines.extend(children);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public API: markdown_to_lines (Facade — unchanged signature)
// ---------------------------------------------------------------------------

use crate::markdown::render::render_document_wrapped;
use crate::markdown::theme::MdStyle;
use ratatui::text::Line;

/// 将 Markdown 文本渲染为 TUI 可显示的 `Line` 列表，应用主题着色和自动换行。
///
/// Facade 函数：内部调用 parse + render。
pub fn markdown_to_lines(md: &str, max_width: usize, theme: &dyn MdStyle) -> Vec<Line<'static>> {
    let content_width = max_width.saturating_sub(2);
    let doc = parse_markdown(md, content_width);
    render_document_wrapped(&doc, theme, content_width)
}

/// 从源码行切片中解析表格为 `TableData`。
///
/// 接收 editor 提供的表格行（含 `|` 分隔的行），利用 pulldown-cmark 解析为 IR，
/// 提取第一个 Table block 的 `TableData`。
pub fn parse_table_from_source(table_lines: &[&str]) -> Option<TableData> {
    use crate::markdown::ir::BlockKind;

    let md: String = table_lines.to_vec().join("\n");
    let doc = parse_markdown(&md, usize::MAX);

    for block in &doc.blocks {
        if let BlockKind::Table(data) = &block.kind {
            return Some(data.clone());
        }
    }

    None
}
