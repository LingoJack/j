use super::super::render::{display_width, wrap_text};
use super::super::theme::Theme;
use super::highlight::highlight_code_line;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

pub fn markdown_to_lines(md: &str, max_width: usize, theme: &Theme) -> Vec<Line<'static>> {
    use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

    // 内容区宽度 = max_width - 2（左侧 "  " 缩进由外层负责）
    let content_width = max_width.saturating_sub(2);

    // 预处理：修复 **"text"** 加粗不生效的问题。
    // CommonMark 规范规定：左侧分隔符 ** 后面是标点（如 " U+201C）且前面是字母（如中文字符）时，
    // 不被识别为有效的加粗开始标记。
    // 解决方案：在 ** 与中文引号之间插入零宽空格（U+200B），使 ** 后面不再紧跟标点，
    // 从而满足 CommonMark 规范。零宽空格在终端中不可见，不影响显示。
    let md_owned;
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

    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(md, options);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default().fg(theme.text_normal)];
    let mut in_code_block = false;
    let mut code_block_content = String::new();
    let mut code_block_lang = String::new();
    let mut list_depth: usize = 0;
    let mut ordered_index: Option<u64> = None;
    let mut heading_level: Option<u8> = None;
    let mut in_blockquote = false;
    // 链接相关状态
    let mut link_url: Option<String> = None;
    // 表格相关状态
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    let mut table_alignments: Vec<pulldown_cmark::Alignment> = Vec::new();

    let base_style = Style::default().fg(theme.text_normal);

    let flush_line = |current_spans: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>| {
        if !current_spans.is_empty() {
            lines.push(Line::from(std::mem::take(current_spans)));
        }
    };

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_line(&mut current_spans, &mut lines);
                heading_level = Some(level as u8);
                if !lines.is_empty() {
                    lines.push(Line::from(""));
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
                style_stack.push(heading_style);
            }
            Event::End(TagEnd::Heading(level)) => {
                flush_line(&mut current_spans, &mut lines);
                if (level as u8) <= 2 {
                    let sep_char = if (level as u8) == 1 { "━" } else { "─" };
                    lines.push(Line::from(Span::styled(
                        sep_char.repeat(content_width),
                        Style::default().fg(theme.md_heading_sep),
                    )));
                }
                style_stack.pop();
                heading_level = None;
            }
            Event::Start(Tag::Strong) => {
                let current = *style_stack.last().unwrap_or(&base_style);
                style_stack.push(current.add_modifier(Modifier::BOLD).fg(theme.text_bold));
            }
            Event::End(TagEnd::Strong) => {
                style_stack.pop();
            }
            Event::Start(Tag::Emphasis) => {
                let current = *style_stack.last().unwrap_or(&base_style);
                style_stack.push(current.add_modifier(Modifier::ITALIC));
            }
            Event::End(TagEnd::Emphasis) => {
                style_stack.pop();
            }
            Event::Start(Tag::Strikethrough) => {
                let current = *style_stack.last().unwrap_or(&base_style);
                style_stack.push(current.add_modifier(Modifier::CROSSED_OUT));
            }
            Event::End(TagEnd::Strikethrough) => {
                style_stack.pop();
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                let link_style = Style::default()
                    .fg(theme.md_link)
                    .add_modifier(Modifier::UNDERLINED);
                style_stack.push(link_style);
                link_url = Some(dest_url.to_string());
            }
            Event::End(TagEnd::Link) => {
                // 如果链接文本和 URL 不同，在文本后追加显示 URL
                if let Some(url) = link_url.take() {
                    let text_content: String = current_spans
                        .iter()
                        .rev()
                        .take_while(|s| s.style.fg == Some(theme.md_link))
                        .map(|s| s.content.to_string())
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                    if !text_content.is_empty() && text_content != url {
                        current_spans.push(Span::styled(
                            format!(" ({})", url),
                            Style::default()
                                .fg(theme.md_link)
                                .add_modifier(Modifier::DIM),
                        ));
                    }
                }
                style_stack.pop();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_line(&mut current_spans, &mut lines);
                in_code_block = true;
                code_block_content.clear();
                code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                let label = if code_block_lang.is_empty() {
                    " code ".to_string()
                } else {
                    format!(" {} ", code_block_lang)
                };
                let label_w = display_width(&label);
                let border_fill = content_width.saturating_sub(2 + label_w);
                let top_border = format!("┌─{}{}", label, "─".repeat(border_fill));
                lines.push(Line::from(Span::styled(
                    top_border,
                    Style::default().fg(theme.code_border),
                )));
            }
            Event::End(TagEnd::CodeBlock) => {
                let code_inner_w = content_width.saturating_sub(4);
                let code_content_expanded = code_block_content.replace('\t', "    ");
                for code_line in code_content_expanded.lines() {
                    let wrapped = wrap_text(code_line, code_inner_w);
                    for wl in wrapped {
                        let highlighted = highlight_code_line(&wl, &code_block_lang, theme);
                        let text_w: usize =
                            highlighted.iter().map(|s| display_width(&s.content)).sum();
                        let fill = code_inner_w.saturating_sub(text_w);
                        let mut spans_vec = Vec::new();
                        spans_vec.push(Span::styled("│ ", Style::default().fg(theme.code_border)));
                        for hs in highlighted {
                            spans_vec.push(Span::styled(
                                hs.content.to_string(),
                                hs.style.bg(theme.code_bg),
                            ));
                        }
                        spans_vec.push(Span::styled(
                            format!("{} │", " ".repeat(fill)),
                            Style::default().fg(theme.code_border).bg(theme.code_bg),
                        ));
                        lines.push(Line::from(spans_vec));
                    }
                }
                let bottom_border = format!("└{}", "─".repeat(content_width.saturating_sub(1)));
                lines.push(Line::from(Span::styled(
                    bottom_border,
                    Style::default().fg(theme.code_border),
                )));
                in_code_block = false;
                code_block_content.clear();
                code_block_lang.clear();
            }
            Event::Code(text) => {
                if in_table {
                    current_cell.push('`');
                    current_cell.push_str(&text);
                    current_cell.push('`');
                } else {
                    let code_str = format!(" {} ", text);
                    let code_w = display_width(&code_str);
                    let effective_prefix_w = if in_blockquote { 2 } else { 0 };
                    let full_line_w = content_width.saturating_sub(effective_prefix_w);
                    let existing_w: usize = current_spans
                        .iter()
                        .map(|s| display_width(&s.content))
                        .sum();
                    if existing_w + code_w > full_line_w && !current_spans.is_empty() {
                        flush_line(&mut current_spans, &mut lines);
                        if in_blockquote {
                            current_spans.push(Span::styled(
                                "| ".to_string(),
                                Style::default().fg(theme.md_blockquote_bar),
                            ));
                        }
                    }
                    current_spans.push(Span::styled(
                        code_str,
                        Style::default()
                            .fg(theme.md_inline_code_fg)
                            .bg(theme.md_inline_code_bg),
                    ));
                }
            }
            Event::Start(Tag::List(start)) => {
                flush_line(&mut current_spans, &mut lines);
                list_depth += 1;
                ordered_index = start;
            }
            Event::End(TagEnd::List(_)) => {
                flush_line(&mut current_spans, &mut lines);
                list_depth = list_depth.saturating_sub(1);
                ordered_index = None;
            }
            Event::Start(Tag::Item) => {
                flush_line(&mut current_spans, &mut lines);
                let indent = "  ".repeat(list_depth);
                let bullet = if let Some(ref mut idx) = ordered_index {
                    let s = format!("{}{}. ", indent, idx);
                    *idx += 1;
                    s
                } else {
                    format!("{}• ", indent)
                };
                current_spans.push(Span::styled(
                    bullet,
                    Style::default().fg(theme.md_list_bullet),
                ));
            }
            Event::End(TagEnd::Item) => {
                flush_line(&mut current_spans, &mut lines);
            }
            Event::Start(Tag::Paragraph) => {
                if !lines.is_empty() && !in_code_block && heading_level.is_none() {
                    let last_empty = lines.last().map(|l| l.spans.is_empty()).unwrap_or(false);
                    if !last_empty {
                        lines.push(Line::from(""));
                    }
                }
            }
            Event::End(TagEnd::Paragraph) => {
                flush_line(&mut current_spans, &mut lines);
            }
            Event::Start(Tag::BlockQuote(_)) => {
                flush_line(&mut current_spans, &mut lines);
                in_blockquote = true;
                style_stack.push(Style::default().fg(theme.md_blockquote_text));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                flush_line(&mut current_spans, &mut lines);
                in_blockquote = false;
                style_stack.pop();
            }
            Event::Text(text) => {
                if in_code_block {
                    code_block_content.push_str(&text);
                } else if in_table {
                    current_cell.push_str(&text);
                } else {
                    let style = *style_stack.last().unwrap_or(&base_style);
                    let text_str = text.to_string().replace('\u{200B}', "");

                    if let Some(level) = heading_level {
                        let (prefix, prefix_style) = match level {
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
                                "▸ ",
                                Style::default()
                                    .fg(theme.md_h3)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            _ => (
                                "▹ ",
                                Style::default()
                                    .fg(theme.md_h4)
                                    .add_modifier(Modifier::BOLD),
                            ),
                        };
                        current_spans.push(Span::styled(prefix.to_string(), prefix_style));
                        heading_level = None;
                    }

                    let effective_prefix_w = if in_blockquote { 2 } else { 0 };
                    let full_line_w = content_width.saturating_sub(effective_prefix_w);

                    let existing_w: usize = current_spans
                        .iter()
                        .map(|s| display_width(&s.content))
                        .sum();

                    let wrap_w = full_line_w.saturating_sub(existing_w);

                    let min_useful_w = full_line_w / 4;
                    let wrap_w = if wrap_w < min_useful_w.max(4) && !current_spans.is_empty() {
                        flush_line(&mut current_spans, &mut lines);
                        if in_blockquote {
                            current_spans.push(Span::styled(
                                "| ".to_string(),
                                Style::default().fg(theme.md_blockquote_bar),
                            ));
                        }
                        full_line_w
                    } else {
                        wrap_w
                    };

                    let link_style = Style::default()
                        .fg(theme.md_link)
                        .add_modifier(Modifier::UNDERLINED);
                    let in_link = link_url.is_some();

                    // 先将文本拆分为带样式的片段（URL vs 普通文本），再逐片段 wrap
                    // 这样 URL 即使跨行也能保持高亮
                    let segments: Vec<Span<'static>> = if in_link {
                        // 已在 Tag::Link 内，整段使用链接样式
                        text_str
                            .split('\n')
                            .enumerate()
                            .flat_map(|(i, line)| {
                                let mut v = Vec::new();
                                if i > 0 {
                                    v.push(Span::raw("\n"));
                                }
                                v.push(Span::styled(line.to_string(), style));
                                v
                            })
                            .collect()
                    } else {
                        // 拆分 URL 片段，保留换行符作为独立片段
                        text_str
                            .split('\n')
                            .enumerate()
                            .flat_map(|(i, line)| {
                                let mut v: Vec<Span<'static>> = Vec::new();
                                if i > 0 {
                                    v.push(Span::raw("\n"));
                                }
                                v.extend(split_text_with_urls(line, style, link_style));
                                v
                            })
                            .collect()
                    };

                    // 逐片段处理：计算累计宽度，遇到超宽时 wrap 并换行
                    let mut cur_line_w = existing_w;
                    let mut first_seg = true;
                    for seg in &segments {
                        if seg.content.as_ref() == "\n" {
                            flush_line(&mut current_spans, &mut lines);
                            if in_blockquote {
                                current_spans.push(Span::styled(
                                    "| ".to_string(),
                                    Style::default().fg(theme.md_blockquote_bar),
                                ));
                                cur_line_w = 2;
                            } else {
                                cur_line_w = 0;
                            }
                            first_seg = false;
                            continue;
                        }
                        let seg_text = seg.content.to_string();
                        let seg_style = seg.style;
                        let seg_w = display_width(&seg_text);
                        let avail = if first_seg {
                            wrap_w
                        } else {
                            full_line_w.saturating_sub(cur_line_w)
                        };
                        first_seg = false;

                        if seg_w <= avail {
                            // 片段整体放得下，直接追加
                            current_spans.push(Span::styled(seg_text, seg_style));
                            cur_line_w += seg_w;
                        } else {
                            // 需要 wrap 这个片段
                            let first_wrap_w = avail;
                            let first_wrapped = wrap_text(&seg_text, first_wrap_w.max(1));
                            // 第一段放入当前行
                            current_spans.push(Span::styled(first_wrapped[0].clone(), seg_style));
                            if first_wrapped.len() > 1 {
                                let rest: String = first_wrapped[1..].join("");
                                flush_line(&mut current_spans, &mut lines);
                                if in_blockquote {
                                    current_spans.push(Span::styled(
                                        "| ".to_string(),
                                        Style::default().fg(theme.md_blockquote_bar),
                                    ));
                                }
                                let rest_wrapped = wrap_text(&rest, full_line_w.max(1));
                                for (j, wl) in rest_wrapped.iter().enumerate() {
                                    if j > 0 {
                                        flush_line(&mut current_spans, &mut lines);
                                        if in_blockquote {
                                            current_spans.push(Span::styled(
                                                "| ".to_string(),
                                                Style::default().fg(theme.md_blockquote_bar),
                                            ));
                                        }
                                    }
                                    current_spans.push(Span::styled(wl.clone(), seg_style));
                                }
                                cur_line_w =
                                    display_width(rest_wrapped.last().unwrap_or(&String::new()));
                                if in_blockquote {
                                    cur_line_w += 2;
                                }
                            } else {
                                cur_line_w = display_width(&first_wrapped[0]);
                                if in_blockquote {
                                    cur_line_w += 2;
                                }
                            }
                        }
                    }
                }
            }
            Event::SoftBreak => {
                if in_table {
                    current_cell.push(' ');
                } else {
                    current_spans.push(Span::raw(" "));
                }
            }
            Event::HardBreak => {
                if in_table {
                    current_cell.push(' ');
                } else {
                    flush_line(&mut current_spans, &mut lines);
                }
            }
            Event::Rule => {
                flush_line(&mut current_spans, &mut lines);
                lines.push(Line::from(Span::styled(
                    "─".repeat(content_width),
                    Style::default().fg(theme.md_rule),
                )));
            }
            // ===== 表格支持 =====
            Event::Start(Tag::Table(alignments)) => {
                flush_line(&mut current_spans, &mut lines);
                in_table = true;
                table_rows.clear();
                table_alignments = alignments;
            }
            Event::End(TagEnd::Table) => {
                flush_line(&mut current_spans, &mut lines);
                in_table = false;

                if !table_rows.is_empty() {
                    let num_cols = table_rows.iter().map(|r| r.len()).max().unwrap_or(0);
                    if num_cols > 0 {
                        let mut col_widths: Vec<usize> = vec![0; num_cols];
                        for row in &table_rows {
                            for (i, cell) in row.iter().enumerate() {
                                let w = display_width(cell);
                                if w > col_widths[i] {
                                    col_widths[i] = w;
                                }
                            }
                        }

                        let sep_w = num_cols + 1;
                        let pad_w = num_cols * 2;
                        let avail = content_width.saturating_sub(sep_w + pad_w);
                        let max_col_w = avail * 2 / 3;
                        for cw in col_widths.iter_mut() {
                            if *cw > max_col_w {
                                *cw = max_col_w;
                            }
                        }
                        let total_col_w: usize = col_widths.iter().sum();
                        if total_col_w > avail && total_col_w > 0 {
                            let mut remaining = avail;
                            for (i, cw) in col_widths.iter_mut().enumerate() {
                                if i == num_cols - 1 {
                                    *cw = remaining.max(1);
                                } else {
                                    *cw = ((*cw) * avail / total_col_w).max(1);
                                    remaining = remaining.saturating_sub(*cw);
                                }
                            }
                        }

                        let table_style = Style::default().fg(theme.table_body);
                        let header_style = Style::default()
                            .fg(theme.table_header)
                            .add_modifier(Modifier::BOLD);
                        let border_style = Style::default().fg(theme.table_border);

                        let total_col_w_final: usize = col_widths.iter().sum();
                        let table_row_w = sep_w + pad_w + total_col_w_final;
                        let table_right_pad = content_width.saturating_sub(table_row_w);

                        // 渲染顶边框 ┌─┬─┐
                        let mut top = String::from("┌");
                        for (i, cw) in col_widths.iter().enumerate() {
                            top.push_str(&"─".repeat(cw + 2));
                            if i < num_cols - 1 {
                                top.push('┬');
                            }
                        }
                        top.push('┐');
                        let mut top_spans = vec![Span::styled(top, border_style)];
                        if table_right_pad > 0 {
                            top_spans.push(Span::raw(" ".repeat(table_right_pad)));
                        }
                        lines.push(Line::from(top_spans));

                        for (row_idx, row) in table_rows.iter().enumerate() {
                            let mut row_spans: Vec<Span> = Vec::new();
                            row_spans.push(Span::styled("│", border_style));
                            for (i, cw) in col_widths.iter().enumerate() {
                                let cell_text = row.get(i).map(|s| s.as_str()).unwrap_or("");
                                let cell_w = display_width(cell_text);
                                let text = if cell_w > *cw {
                                    let mut t = String::new();
                                    let mut w = 0;
                                    for ch in cell_text.chars() {
                                        use super::super::render::char_width;
                                        let chw = char_width(ch);
                                        if w + chw > *cw {
                                            break;
                                        }
                                        t.push(ch);
                                        w += chw;
                                    }
                                    let fill = cw.saturating_sub(w);
                                    format!(" {}{} ", t, " ".repeat(fill))
                                } else {
                                    let fill = cw.saturating_sub(cell_w);
                                    let align = table_alignments
                                        .get(i)
                                        .copied()
                                        .unwrap_or(pulldown_cmark::Alignment::None);
                                    match align {
                                        pulldown_cmark::Alignment::Center => {
                                            let left = fill / 2;
                                            let right = fill - left;
                                            format!(
                                                " {}{}{} ",
                                                " ".repeat(left),
                                                cell_text,
                                                " ".repeat(right)
                                            )
                                        }
                                        pulldown_cmark::Alignment::Right => {
                                            format!(" {}{} ", " ".repeat(fill), cell_text)
                                        }
                                        _ => format!(" {}{} ", cell_text, " ".repeat(fill)),
                                    }
                                };
                                let style = if row_idx == 0 {
                                    header_style
                                } else {
                                    table_style
                                };
                                row_spans.push(Span::styled(text, style));
                                row_spans.push(Span::styled("│", border_style));
                            }
                            if table_right_pad > 0 {
                                row_spans.push(Span::raw(" ".repeat(table_right_pad)));
                            }
                            lines.push(Line::from(row_spans));

                            if row_idx == 0 {
                                let mut sep = String::from("├");
                                for (i, cw) in col_widths.iter().enumerate() {
                                    sep.push_str(&"─".repeat(cw + 2));
                                    if i < num_cols - 1 {
                                        sep.push('┼');
                                    }
                                }
                                sep.push('┤');
                                let mut sep_spans = vec![Span::styled(sep, border_style)];
                                if table_right_pad > 0 {
                                    sep_spans.push(Span::raw(" ".repeat(table_right_pad)));
                                }
                                lines.push(Line::from(sep_spans));
                            }
                        }

                        // 底边框 └─┴─┘
                        let mut bottom = String::from("└");
                        for (i, cw) in col_widths.iter().enumerate() {
                            bottom.push_str(&"─".repeat(cw + 2));
                            if i < num_cols - 1 {
                                bottom.push('┴');
                            }
                        }
                        bottom.push('┘');
                        let mut bottom_spans = vec![Span::styled(bottom, border_style)];
                        if table_right_pad > 0 {
                            bottom_spans.push(Span::raw(" ".repeat(table_right_pad)));
                        }
                        lines.push(Line::from(bottom_spans));
                    }
                }
                table_rows.clear();
                table_alignments.clear();
            }
            Event::Start(Tag::TableHead) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                table_rows.push(current_row.clone());
                current_row.clear();
            }
            Event::Start(Tag::TableRow) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                table_rows.push(current_row.clone());
                current_row.clear();
            }
            Event::Start(Tag::TableCell) => {
                current_cell.clear();
            }
            Event::End(TagEnd::TableCell) => {
                current_row.push(current_cell.clone());
                current_cell.clear();
            }
            _ => {}
        }
    }

    // 刷新最后一行
    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    // 如果解析结果为空，至少返回原始文本
    if lines.is_empty() {
        let wrapped = wrap_text(md, content_width);
        for wl in wrapped {
            lines.push(Line::from(Span::styled(wl, base_style)));
        }
    }

    lines
}

/// 将文本拆分为普通文本和 URL 片段，对 URL 应用链接样式
fn split_text_with_urls<'a>(text: &str, normal_style: Style, link_style: Style) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // 查找 URL 起始位置
        let url_start = remaining
            .find("https://")
            .or_else(|| remaining.find("http://"));

        match url_start {
            Some(start) => {
                // 添加 URL 之前的普通文本
                if start > 0 {
                    spans.push(Span::styled(remaining[..start].to_string(), normal_style));
                }
                // 找到 URL 结束位置：遇到空格、中文字符或特殊分隔符即停止
                let url_part = &remaining[start..];
                let url_end = url_part
                    .char_indices()
                    .find(|(i, c)| {
                        // 跳过开头的 http:// 或 https://
                        if *i < 8 {
                            return false;
                        }
                        c.is_whitespace()
                            || *c == '>'
                            || *c == ')'
                            || *c == ']'
                            // 中文字符和中文标点表示 URL 结束
                            || ('\u{4E00}'..='\u{9FFF}').contains(c) // CJK 汉字
                            || ('\u{3000}'..='\u{303F}').contains(c) // CJK 标点
                            || ('\u{FF00}'..='\u{FFEF}').contains(c) // 全角字符
                            || matches!(*c, '，' | '。' | '；' | '：' | '！' | '？' | '、' | '\u{201C}' | '\u{201D}' | '\u{2018}' | '\u{2019}')
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(url_part.len());
                // 去掉 URL 末尾的 ASCII 标点符号
                let url = url_part[..url_end].trim_end_matches(['.', ',', ';', ':', '!', '?']);
                let url_len = url.len();
                spans.push(Span::styled(url.to_string(), link_style));
                // URL 末尾被 trim 掉的标点作为普通文本
                if url_len < url_end {
                    spans.push(Span::styled(
                        url_part[url_len..url_end].to_string(),
                        normal_style,
                    ));
                }
                remaining = &remaining[start + url_end..];
            }
            None => {
                spans.push(Span::styled(remaining.to_string(), normal_style));
                break;
            }
        }
    }

    spans
}
