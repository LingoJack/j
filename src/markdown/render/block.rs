use crate::markdown::ir::{Block, BlockKind, Inline, ListData};
use crate::markdown::theme::MdStyle;
use crate::util::text::display_width;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::RenderContext;
use super::code_block::render_code_block;
use super::inline::render_inlines;
use super::table::render_table;
use super::wrap::wrap_spans_with_prefix;

/// 渲染单个 block 元素
pub fn render_block(block: &Block, ctx: &RenderContext) -> Vec<Line<'static>> {
    match &block.kind {
        BlockKind::Paragraph(inlines) => render_paragraph(inlines, ctx),
        BlockKind::Heading { level, content } => render_heading(*level, content, ctx),
        BlockKind::CodeBlock { lang, code, fenced } => {
            if *fenced {
                render_code_block(lang, code, ctx.width, ctx.theme)
            } else {
                // 缩进代码块（4 空格）退化为普通段落渲染：实际项目里它常常是
                // 列表项的缩进续行，被切片喂给解析器后才被识别成代码块——画框
                // 反而误导。editor_core 的 BlockCache 也是同样的策略。
                render_indented_code_as_paragraph(code, ctx)
            }
        }
        BlockKind::Table(data) => render_table(data, &data.alignments, ctx.width, ctx.theme),
        BlockKind::List(data) => render_list(data, ctx, 0),
        BlockKind::BlockQuote(blocks) => render_blockquote(blocks, ctx),
        BlockKind::Rule => render_rule(ctx),
    }
}

/// 缩进代码块降级渲染：按行输出原始文本，不画围栏框。
fn render_indented_code_as_paragraph(code: &str, ctx: &RenderContext) -> Vec<Line<'static>> {
    let style = Style::default().fg(ctx.theme.text_normal());
    let mut lines: Vec<Line<'static>> = Vec::new();
    for raw in code.lines() {
        let spans = vec![Span::styled(raw.to_string(), style)];
        // 按宽度折行，保持与普通段落一致
        let wrapped = wrap_spans_with_prefix(spans, ctx.width, Vec::new(), Vec::new());
        if wrapped.is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.extend(wrapped);
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines
}

/// 渲染段落
fn render_paragraph(inlines: &[Inline], ctx: &RenderContext) -> Vec<Line<'static>> {
    if inlines.is_empty() {
        return Vec::new();
    }

    let base_style = Style::default().fg(ctx.theme.text_normal());
    let spans = render_inlines(inlines, base_style, ctx.theme);
    wrap_spans_with_prefix(spans, ctx.width, Vec::new(), Vec::new())
}

/// 渲染标题
fn render_heading(level: u8, content: &[Inline], ctx: &RenderContext) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    let heading_style = match level {
        1 => Style::default()
            .fg(ctx.theme.md_h1())
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        2 => Style::default()
            .fg(ctx.theme.md_h2())
            .add_modifier(Modifier::BOLD),
        3 => Style::default()
            .fg(ctx.theme.md_h3())
            .add_modifier(Modifier::BOLD),
        _ => Style::default()
            .fg(ctx.theme.md_h4())
            .add_modifier(Modifier::BOLD),
    };

    let (prefix, prefix_style) = match level {
        1 => (
            "◆ ",
            Style::default()
                .fg(ctx.theme.md_h1())
                .add_modifier(Modifier::BOLD),
        ),
        2 => (
            "◇ ",
            Style::default()
                .fg(ctx.theme.md_h2())
                .add_modifier(Modifier::BOLD),
        ),
        3 => (
            "〈",
            Style::default()
                .fg(ctx.theme.md_h3())
                .add_modifier(Modifier::BOLD),
        ),
        _ => (
            "› ",
            Style::default()
                .fg(ctx.theme.md_h4())
                .add_modifier(Modifier::BOLD),
        ),
    };

    let prefix_width = display_width(prefix);
    let mut content_spans = render_inlines(content, heading_style, ctx.theme);

    // H3 添加文艺风格后缀
    if level == 3 {
        content_spans.push(Span::styled(
            "〉".to_string(),
            Style::default()
                .fg(ctx.theme.md_h3())
                .add_modifier(Modifier::BOLD),
        ));
    }

    lines.extend(wrap_spans_with_prefix(
        content_spans,
        ctx.width,
        vec![Span::styled(prefix.to_string(), prefix_style)],
        vec![Span::raw(" ".repeat(prefix_width))],
    ));

    // H1/H2 显示分隔线
    if level <= 2 {
        let sep_char = if level == 1 { "━" } else { "─" };
        lines.push(Line::from(Span::styled(
            sep_char.repeat(ctx.width),
            Style::default().fg(ctx.theme.md_heading_sep()),
        )));
    }

    lines
}

/// 渲染列表（递归，支持嵌套）
/// `depth`: 嵌套层级，0 为最外层；每一级缩进 2 空格
fn render_list(data: &ListData, ctx: &RenderContext, depth: usize) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let base_style = Style::default().fg(ctx.theme.text_normal());
    let indent_str = "  ".repeat(depth);
    let child_indent_str = "  ".repeat(depth + 1);

    for (idx, item) in data.items.iter().enumerate() {
        let bullet = if data.ordered {
            let num = data
                .start_index
                .map(|s| s + idx as u64)
                .unwrap_or(idx as u64 + 1);
            format!("{}{}. ", indent_str, num)
        } else {
            format!(
                "{}{} ",
                indent_str,
                task_list_marker(item.checked, ctx.theme)
            )
        };

        let bullet_style = Style::default().fg(ctx.theme.md_list_bullet());

        // item 自身 inline 为空且没有 children 时仍输出一行空 bullet，
        // 但更常见的是有 inline 或 children；这里只要任一非空即输出 bullet 行
        let has_inline = !item.content.is_empty();
        let has_children = !item.children.is_empty();
        if has_inline || !has_children {
            let bullet_width = display_width(&bullet);
            let content_spans = render_inlines(&item.content, base_style, ctx.theme);
            let wrapped_lines = wrap_spans_with_prefix(
                content_spans,
                ctx.width,
                vec![Span::styled(bullet, bullet_style)],
                vec![Span::raw(" ".repeat(bullet_width))],
            );
            lines.extend(wrapped_lines);
        } else {
            // 没有 inline 只有 children：仍然输出 bullet 作为视觉锚点
            lines.push(Line::from(Span::styled(bullet, bullet_style)));
        }

        // 递归渲染 children
        for child in &item.children {
            let child_lines = match &child.kind {
                BlockKind::List(sub) => render_list(sub, ctx, depth + 1),
                _ => {
                    // 其他 block：用 render_block 渲染后统一加缩进
                    let rendered = render_block(child, ctx);
                    rendered
                        .into_iter()
                        .map(|line| prepend_indent(line, &child_indent_str))
                        .collect()
                }
            };
            lines.extend(child_lines);
        }
    }

    lines
}
/// 在 Line 的开头插入一段缩进 span
fn prepend_indent(line: Line<'static>, indent: &str) -> Line<'static> {
    if indent.is_empty() {
        return line;
    }
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(line.spans.len() + 1);
    spans.push(Span::raw(indent.to_string()));
    spans.extend(line.spans);
    Line::from(spans)
}

/// 获取 task list 标记符号
fn task_list_marker(checked: Option<bool>, _theme: &dyn MdStyle) -> String {
    match checked {
        Some(true) => "●".to_string(),
        Some(false) => "○".to_string(),
        None => "•".to_string(),
    }
}

/// 渲染引用块（与 thinking block 风格一致：竖线 + bg_primary 背景）
fn render_blockquote(blocks: &[Block], ctx: &RenderContext) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    // 背景色使用 bg_primary（与 thinking block 一致，融入主背景）
    let bg_color = ctx.theme.bg_primary();
    let bar_style = Style::default()
        .fg(ctx.theme.md_blockquote_bar())
        .bg(bg_color)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default()
        .fg(ctx.theme.md_blockquote_text())
        .bg(bg_color);

    // 前导空行
    lines.push(Line::from(""));

    for block in blocks {
        let inner_lines = render_block(block, ctx);
        for inner_line in inner_lines {
            let mut line_spans: Vec<Span<'static>> = Vec::new();
            line_spans.push(Span::styled("  ".to_string(), text_style));
            line_spans.push(Span::styled("| ".to_string(), bar_style));
            for span in inner_line.spans {
                line_spans.push(Span::styled(
                    span.content.to_string(),
                    span.style.bg(bg_color),
                ));
            }
            lines.push(Line::from(line_spans));
        }
    }

    // 后导空行
    lines.push(Line::from(""));

    lines
}

/// 渲染水平分隔线
fn render_rule(ctx: &RenderContext) -> Vec<Line<'static>> {
    vec![Line::from(Span::styled(
        "─".repeat(ctx.width),
        Style::default().fg(ctx.theme.md_rule()),
    ))]
}

/// 渲染图片占位符（当前 IR 不支持 Image block，暂不实现）
#[allow(dead_code)]
fn render_image_placeholder(
    url: &str,
    _alt: &str,
    height: u16,
    _ctx: &RenderContext,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let marker = format!("\x00IMG:{}:{}", height, url);
    lines.push(Line::from(Span::styled(marker, Style::default())));
    for _ in 1..height {
        lines.push(Line::from(Span::raw("")));
    }
    let caption = format!("({})", url);
    lines.push(Line::from(Span::styled(
        caption,
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM),
    )));
    lines
}
