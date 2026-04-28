use crate::markdown::ir::{Block, BlockKind, Inline, ListData};
use crate::markdown::theme::MdStyle;
use crate::util::text::{display_width, wrap_text};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::RenderContext;
use super::code_block::render_code_block;
use super::inline::render_inlines;
use super::table::render_table;

/// 渲染单个 block 元素
pub fn render_block(block: &Block, ctx: &RenderContext) -> Vec<Line<'static>> {
    match &block.kind {
        BlockKind::Paragraph(inlines) => render_paragraph(inlines, ctx),
        BlockKind::Heading { level, content } => render_heading(*level, content, ctx),
        BlockKind::CodeBlock { lang, code } => render_code_block(lang, code, ctx.width, ctx.theme),
        BlockKind::Table(data) => render_table(data, &data.alignments, ctx.width, ctx.theme),
        BlockKind::List(data) => render_list(data, ctx),
        BlockKind::BlockQuote(blocks) => render_blockquote(blocks, ctx),
        BlockKind::Rule => render_rule(ctx),
    }
}

/// 渲染段落
fn render_paragraph(inlines: &[Inline], ctx: &RenderContext) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    if inlines.is_empty() {
        return lines;
    }

    let base_style = Style::default().fg(ctx.theme.text_normal());
    let full_line_w = ctx.width;

    // 先渲染所有 inline 为 spans
    let spans = render_inlines(inlines, base_style, ctx.theme);

    // 按 \n 拆分 spans
    let mut segments_by_newline: Vec<Vec<Span<'static>>> = Vec::new();
    let mut current_seg: Vec<Span<'static>> = Vec::new();
    for span in spans {
        if span.content.as_ref() == "\n" {
            segments_by_newline.push(current_seg);
            current_seg = Vec::new();
        } else {
            current_seg.push(span);
        }
    }
    segments_by_newline.push(current_seg);

    // 对每个 segment 进行 wrap
    let mut cur_line_spans: Vec<Span<'static>> = Vec::new();
    let mut cur_line_w: usize = 0;

    for seg in segments_by_newline {
        if seg.is_empty() {
            // 空的 segment 表示 \n，flush 当前行
            if !cur_line_spans.is_empty() {
                lines.push(Line::from(std::mem::take(&mut cur_line_spans)));
                cur_line_w = 0;
            }
            continue;
        }

        let seg_w: usize = seg.iter().map(|s| display_width(&s.content)).sum();
        let avail = full_line_w.saturating_sub(cur_line_w);

        if seg_w <= avail {
            cur_line_spans.extend(seg);
            cur_line_w += seg_w;
        } else {
            // 需要 wrap 这个 segment
            let seg_text: String = seg.iter().map(|s| s.content.to_string()).collect();
            let first_wrap_w = avail.max(1);
            let first_wrapped = wrap_text(&seg_text, first_wrap_w);
            // 第一段放入当前行
            let first_style = seg.first().map(|s| s.style).unwrap_or(base_style);
            cur_line_spans.push(Span::styled(first_wrapped[0].clone(), first_style));
            if first_wrapped.len() > 1 {
                let rest: String = first_wrapped[1..].join("");
                lines.push(Line::from(std::mem::take(&mut cur_line_spans)));
                let rest_wrapped = wrap_text(&rest, full_line_w.max(1));
                for wl in rest_wrapped {
                    lines.push(Line::from(Span::styled(wl, first_style)));
                }
                cur_line_w = 0;
            } else {
                cur_line_w = display_width(&first_wrapped[0]);
            }
        }
    }

    // flush 最后一行
    if !cur_line_spans.is_empty() {
        lines.push(Line::from(cur_line_spans));
    }

    lines
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

    let mut content_spans = vec![Span::styled(prefix.to_string(), prefix_style)];
    content_spans.extend(render_inlines(content, heading_style, ctx.theme));

    // H3 添加文艺风格后缀
    if level == 3 {
        content_spans.push(Span::styled(
            "〉".to_string(),
            Style::default()
                .fg(ctx.theme.md_h3())
                .add_modifier(Modifier::BOLD),
        ));
    }

    lines.push(Line::from(content_spans));

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

/// 渲染列表
fn render_list(data: &ListData, ctx: &RenderContext) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let base_style = Style::default().fg(ctx.theme.text_normal());
    let list_depth = 0; // 当前 IR 不支持嵌套列表深度跟踪，暂用 0

    for (idx, item) in data.items.iter().enumerate() {
        let indent = "  ".repeat(list_depth);
        let bullet = if data.ordered {
            let num = data
                .start_index
                .map(|s| s + idx as u64)
                .unwrap_or(idx as u64 + 1);
            format!("{}{}. ", indent, num)
        } else {
            format!("{}{} ", indent, task_list_marker(item.checked, ctx.theme))
        };

        let bullet_style = Style::default().fg(ctx.theme.md_list_bullet());
        let mut line_spans = vec![Span::styled(bullet, bullet_style)];
        line_spans.extend(render_inlines(&item.content, base_style, ctx.theme));
        lines.push(Line::from(line_spans));
    }

    lines
}

/// 获取 task list 标记符号
fn task_list_marker(checked: Option<bool>, _theme: &dyn MdStyle) -> String {
    match checked {
        Some(true) => "●".to_string(),
        Some(false) => "○".to_string(),
        None => "•".to_string(),
    }
}

/// 渲染引用块
fn render_blockquote(blocks: &[Block], ctx: &RenderContext) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    // 前导空行
    lines.push(Line::from(""));

    let _blockquote_style = Style::default()
        .fg(ctx.theme.md_blockquote_text())
        .bg(ctx.theme.md_blockquote_bg());
    let bar_style = Style::default()
        .fg(ctx.theme.md_blockquote_bar())
        .bg(ctx.theme.md_blockquote_bg())
        .add_modifier(Modifier::BOLD);

    for block in blocks {
        let inner_lines = render_block(block, ctx);
        for inner_line in inner_lines {
            let mut line_spans: Vec<Span<'static>> = Vec::new();
            line_spans.push(Span::styled("| ".to_string(), bar_style));
            for span in inner_line.spans {
                line_spans.push(Span::styled(
                    span.content.to_string(),
                    span.style.bg(ctx.theme.md_blockquote_bg()),
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
