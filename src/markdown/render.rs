mod block;
mod code_block;
pub mod inline;
pub mod table;

use crate::markdown::ir::ParsedDocument;
use crate::markdown::theme::MdStyle;
use ratatui::text::Line;

use self::block::render_block;

/// 渲染上下文，传递给各 block renderer
pub(crate) struct RenderContext<'a> {
    pub width: usize,
    pub theme: &'a dyn MdStyle,
}

/// 将解析后的文档渲染为 TUI 可显示的 `Line` 列表
pub fn render_document_wrapped(
    doc: &ParsedDocument,
    theme: &dyn MdStyle,
    width: usize,
) -> Vec<Line<'static>> {
    let ctx = RenderContext { width, theme };
    let mut lines: Vec<Line<'static>> = Vec::new();

    for block in &doc.blocks {
        let block_lines = render_block(block, &ctx);
        lines.extend(block_lines);
    }

    // 如果渲染结果为空，至少返回一行
    if lines.is_empty() {
        lines.push(Line::from(""));
    }

    lines
}
