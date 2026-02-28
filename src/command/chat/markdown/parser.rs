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
    // 表格相关状态
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    let mut table_alignments: Vec<pulldown_cmark::Alignment> = Vec::new();

    let base_style = Style::default().fg(theme.text_normal);

    let flush_line = |current_spans: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>| {
        if !current_spans.is_empty() {
            lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
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

                    for (i, line) in text_str.split('\n').enumerate() {
                        if i > 0 {
                            flush_line(&mut current_spans, &mut lines);
                            if in_blockquote {
                                current_spans.push(Span::styled(
                                    "| ".to_string(),
                                    Style::default().fg(theme.md_blockquote_bar),
                                ));
                            }
                        }
                        if !line.is_empty() {
                            let effective_wrap = if i == 0 {
                                wrap_w
                            } else {
                                content_width.saturating_sub(effective_prefix_w)
                            };
                            let wrapped = wrap_text(line, effective_wrap);
                            for (j, wl) in wrapped.iter().enumerate() {
                                if j > 0 {
                                    flush_line(&mut current_spans, &mut lines);
                                    if in_blockquote {
                                        current_spans.push(Span::styled(
                                            "| ".to_string(),
                                            Style::default().fg(theme.md_blockquote_bar),
                                        ));
                                    }
                                }
                                current_spans.push(Span::styled(wl.clone(), style));
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
