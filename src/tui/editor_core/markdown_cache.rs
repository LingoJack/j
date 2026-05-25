//! Markdown 解析与渲染缓存
//!
//! Editor 按 VisualLine 逐行渲染，但需要知道每个源码行所属的 Block
//! 以及该 Block 的渲染结果。`MarkdownCache` 缓存全文解析结果和
//! 按需渲染的 Block，避免每帧重复解析。
//!
//! TODO: Step 7 性能优化时启用。当前 editor 采用逐行渲染模式，
//! 未使用该缓存。如果未来需要 editor 预览模式或全文渲染优化，
//! 可以启用该缓存来提升性能。

#![allow(dead_code)]

use crate::markdown::ir::{Block, BlockKind, ParsedDocument, SourceRange};
use crate::markdown::render::render_document_wrapped;
use crate::markdown::theme::MdStyle;
use ratatui::text::Line;

/// 渲染后的 Block 结果
#[derive(Debug, Clone)]
pub struct RenderedBlock {
    /// 渲染输出的行（不含行号前缀）
    pub lines: Vec<Line<'static>>,
    /// 每个渲染行对应的源码行号（用于光标定位和续行判断）
    pub source_lines: Vec<usize>,
    /// Block 的源码行范围
    pub source_range: SourceRange,
}

/// Markdown 解析与渲染缓存
#[derive(Debug)]
pub struct MarkdownCache {
    /// 缓存版本号（每次 rebuild 递增）
    revision: u64,
    /// 解析后的文档结构
    doc: Option<ParsedDocument>,
    /// 源码行 -> Block 索引映射
    line_to_block: Vec<Option<usize>>,
    /// 已渲染的 Block（按需填充）
    rendered_blocks: Vec<Option<RenderedBlock>>,
    /// 上次渲染宽度（用于判断是否需要重新渲染）
    last_width: usize,
}

impl Default for MarkdownCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownCache {
    /// 创建空缓存
    pub fn new() -> Self {
        Self {
            revision: 0,
            doc: None,
            line_to_block: Vec::new(),
            rendered_blocks: Vec::new(),
            last_width: 0,
        }
    }

    /// 重建缓存（全文解析 + 构建 line_to_block 映射）
    pub fn rebuild(&mut self, text: &str, width: usize) {
        // 解析全文
        let doc = crate::markdown::parser::parse_markdown(text, width);

        // 构建 line_to_block 映射
        let line_count = text.lines().count();
        let mut line_to_block: Vec<Option<usize>> = vec![None; line_count];

        for (block_idx, block) in doc.blocks.iter().enumerate() {
            let start = block.source.start_line;
            let end = block.source.end_line;
            // 填充该 Block 覆盖的所有源码行
            for slot in line_to_block
                .iter_mut()
                .take(end.min(line_count.saturating_sub(1)) + 1)
                .skip(start)
            {
                *slot = Some(block_idx);
            }
        }

        // 清空旧的渲染缓存
        let rendered_blocks: Vec<Option<RenderedBlock>> = doc.blocks.iter().map(|_| None).collect();

        self.doc = Some(doc);
        self.line_to_block = line_to_block;
        self.rendered_blocks = rendered_blocks;
        self.last_width = width;
        self.revision += 1;
    }

    /// 获取缓存版本号
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// 获取源码行数
    pub fn line_count(&self) -> usize {
        self.line_to_block.len()
    }

    /// 获取指定源码行所属的 Block 索引
    pub fn get_block_for_line(&self, line_idx: usize) -> Option<usize> {
        self.line_to_block.get(line_idx).copied().flatten()
    }

    /// 获取指定 Block 的渲染结果（懒渲染 + 缓存）
    pub fn get_rendered_block(
        &mut self,
        block_idx: usize,
        theme: &dyn MdStyle,
        width: usize,
    ) -> Option<&RenderedBlock> {
        let doc = self.doc.as_ref()?;
        let block = doc.blocks.get(block_idx)?;

        // 判断是否需要重新渲染（宽度变化）
        let needs_render =
            self.rendered_blocks.get(block_idx)?.is_none() || self.last_width != width;

        if needs_render {
            // 渲染该 Block
            let lines = render_single_block(block, theme, width);
            let source_range = block.source;

            // 构建渲染行 -> 源码行映射
            // 对于简单 Block（paragraph/heading/list/rule），源码行和渲染行大致 1:1
            // 对于复杂 Block（table/code_block），需要特殊处理
            let source_lines = build_rendered_line_source_mapping(block, &lines);

            let rendered = RenderedBlock {
                lines,
                source_lines,
                source_range,
            };

            self.rendered_blocks[block_idx] = Some(rendered);
            self.last_width = width;
        }

        self.rendered_blocks
            .get(block_idx)
            .and_then(|rb| rb.as_ref())
    }

    /// 获取指定源码行的渲染结果（从所属 Block 中提取）
    ///
    /// 返回 `(渲染行, 是否为该 Block 的首行)`。
    /// 首行用于判断是否需要显示 block 前缀（如 heading 的 ◆、list 的 •）。
    pub fn get_rendered_line_for_source(
        &mut self,
        line_idx: usize,
        theme: &dyn MdStyle,
        width: usize,
    ) -> Option<(Line<'static>, bool)> {
        let block_idx = self.get_block_for_line(line_idx)?;
        let rendered = self.get_rendered_block(block_idx, theme, width)?;

        // 找到该源码行对应的渲染行索引
        let render_idx = rendered.source_lines.iter().position(|&l| l == line_idx)?;

        let line = rendered.lines.get(render_idx)?.clone();
        let is_block_first_line = render_idx == 0;

        Some((line, is_block_first_line))
    }

    /// 清空缓存（buffer 完全更换时）
    pub fn clear(&mut self) {
        self.doc = None;
        self.line_to_block.clear();
        self.rendered_blocks.clear();
        self.revision += 1;
    }
}

/// 渲染单个 Block（不包含行号前缀）
fn render_single_block(block: &Block, theme: &dyn MdStyle, width: usize) -> Vec<Line<'static>> {
    // 构造只包含一个 Block 的临时文档
    let temp_doc = ParsedDocument {
        blocks: vec![block.clone()],
        line_to_block: vec![],
    };

    render_document_wrapped(&temp_doc, theme, width)
}

/// 构建渲染行 -> 源码行映射
///
/// 对于简单 Block，渲染行与源码行大致 1:1。
/// 复杂 Block（表格、代码块）需要特殊处理。
fn build_rendered_line_source_mapping(
    block: &Block,
    rendered_lines: &[Line<'static>],
) -> Vec<usize> {
    let start_line = block.source.start_line;
    let end_line = block.source.end_line;

    match &block.kind {
        // Paragraph：源码行可能被 wrap 成多行，每行都映射到同一个源码行
        BlockKind::Paragraph(_) => {
            // 暂时假设 1:1，后续可根据 wrap 逻辑改进
            rendered_lines
                .iter()
                .enumerate()
                .map(|(i, _)| start_line + i.min(end_line - start_line))
                .collect()
        }

        // Heading：渲染为 1-2 行（内容行 + 分隔行）
        BlockKind::Heading { level, .. } => {
            if *level <= 2 {
                // H1/H2 有分隔行
                vec![start_line, start_line]
            } else {
                vec![start_line]
            }
        }

        // List：每个 item 可能 wrap 成多行
        BlockKind::List(_) => {
            // 暂时假设每个 item 1 行
            rendered_lines
                .iter()
                .enumerate()
                .map(|(i, _)| start_line + i.min(end_line - start_line))
                .collect()
        }

        // BlockQuote：内部 Block 递归处理
        BlockKind::BlockQuote(_) => rendered_lines
            .iter()
            .enumerate()
            .map(|(i, _)| start_line + i.min(end_line - start_line))
            .collect(),

        // Rule：1 行
        BlockKind::Rule => vec![start_line],

        // CodeBlock：顶框 + 内容行 + 底框
        BlockKind::CodeBlock { .. } => {
            let content_line_count = rendered_lines.len().saturating_sub(2); // 减去顶框和底框
            let mut mapping = vec![start_line]; // 顶框
            for i in 0..content_line_count {
                mapping.push(start_line + 1 + i.min(end_line - start_line - 1));
            }
            mapping.push(end_line); // 底框
            mapping
        }

        // Table：边框行 + 内容行
        BlockKind::Table(_data) => {
            // 所有渲染行映射到 start_line（表格整体属于起始行）
            rendered_lines.iter().map(|_| start_line).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::editor_core::theme::{BorderStyle, EditorTheme};

    fn test_theme() -> EditorTheme {
        // 使用 search/tests.rs 中的 test_theme 定义
        EditorTheme {
            bg_primary: ratatui::style::Color::Reset,
            bg_input: ratatui::style::Color::Reset,
            code_bg: ratatui::style::Color::DarkGray,
            cursor_fg: ratatui::style::Color::Black,
            cursor_bg: ratatui::style::Color::Cyan,
            text_normal: ratatui::style::Color::White,
            text_dim: ratatui::style::Color::DarkGray,
            text_bold: ratatui::style::Color::White,
            md_h1: ratatui::style::Color::Cyan,
            md_h2: ratatui::style::Color::Green,
            md_h3: ratatui::style::Color::Yellow,
            md_h4: ratatui::style::Color::Magenta,
            md_heading_sep: ratatui::style::Color::DarkGray,
            md_link: ratatui::style::Color::Blue,
            md_list_bullet: ratatui::style::Color::Yellow,
            md_blockquote_bar: ratatui::style::Color::Cyan,
            md_blockquote_bg: ratatui::style::Color::DarkGray,
            md_blockquote_text: ratatui::style::Color::Gray,
            md_inline_code_fg: ratatui::style::Color::Magenta,
            md_inline_code_bg: ratatui::style::Color::DarkGray,
            md_rule: ratatui::style::Color::DarkGray,
            code_border: ratatui::style::Color::DarkGray,
            code_border_style: BorderStyle::default(),
            table_header: ratatui::style::Color::White,
            table_body: ratatui::style::Color::White,
            code_default: ratatui::style::Color::White,
            code_keyword: ratatui::style::Color::Magenta,
            code_string: ratatui::style::Color::Green,
            code_comment: ratatui::style::Color::DarkGray,
            code_number: ratatui::style::Color::Yellow,
            code_type: ratatui::style::Color::Yellow,
            code_primitive: ratatui::style::Color::Cyan,
            code_macro: ratatui::style::Color::LightCyan,
            code_lifetime: ratatui::style::Color::LightMagenta,
            code_attribute: ratatui::style::Color::LightBlue,
            code_shell_var: ratatui::style::Color::LightCyan,
            label_ai: ratatui::style::Color::Green,
        }
    }

    #[test]
    fn cache_rebuild_basic() {
        let mut cache = MarkdownCache::new();
        let md = "# Hello\n\nThis is a paragraph.\n\n- Item 1\n- Item 2\n";
        cache.rebuild(md, 80);

        assert!(cache.doc.is_some());
        assert_eq!(cache.line_count(), 6); // 6 lines including empty lines
        assert!(cache.get_block_for_line(0).is_some()); // heading
        assert!(cache.get_block_for_line(2).is_some()); // paragraph
    }

    #[test]
    fn cache_get_rendered_line() {
        let mut cache = MarkdownCache::new();
        let md = "# Hello\n\nWorld\n";
        cache.rebuild(md, 80);

        let theme = test_theme();
        let result = cache.get_rendered_line_for_source(0, &theme, 80);
        assert!(result.is_some());

        let (line, is_first) = result.unwrap();
        assert!(is_first); // First line of heading block
        assert!(!line.spans.is_empty());
    }
}
