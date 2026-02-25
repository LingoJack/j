use super::render::{char_width, display_width, wrap_text};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub fn markdown_to_lines(md: &str, max_width: usize) -> Vec<Line<'static>> {
    use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

    // 内容区宽度 = max_width - 2（左侧 "  " 缩进由外层负责）
    let content_width = max_width.saturating_sub(2);

    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(md, options);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default().fg(Color::Rgb(220, 220, 230))];
    let mut in_code_block = false;
    let mut code_block_content = String::new();
    let mut code_block_lang = String::new();
    let mut list_depth: usize = 0;
    let mut ordered_index: Option<u64> = None;
    let mut heading_level: Option<u8> = None;
    // 跟踪是否在引用块中
    let mut in_blockquote = false;
    // 表格相关状态
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new(); // 收集所有行（含表头）
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    let mut table_alignments: Vec<pulldown_cmark::Alignment> = Vec::new();

    let base_style = Style::default().fg(Color::Rgb(220, 220, 230));

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
                // 根据标题级别使用不同的颜色
                let heading_style = match level as u8 {
                    1 => Style::default()
                        .fg(Color::Rgb(100, 180, 255))
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    2 => Style::default()
                        .fg(Color::Rgb(130, 190, 255))
                        .add_modifier(Modifier::BOLD),
                    3 => Style::default()
                        .fg(Color::Rgb(160, 200, 255))
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default()
                        .fg(Color::Rgb(180, 210, 255))
                        .add_modifier(Modifier::BOLD),
                };
                style_stack.push(heading_style);
            }
            Event::End(TagEnd::Heading(level)) => {
                flush_line(&mut current_spans, &mut lines);
                // h1/h2 下方加分隔线（完整填充 content_width）
                if (level as u8) <= 2 {
                    let sep_char = if (level as u8) == 1 { "━" } else { "─" };
                    lines.push(Line::from(Span::styled(
                        sep_char.repeat(content_width),
                        Style::default().fg(Color::Rgb(60, 70, 100)),
                    )));
                }
                style_stack.pop();
                heading_level = None;
            }
            Event::Start(Tag::Strong) => {
                let current = *style_stack.last().unwrap_or(&base_style);
                style_stack.push(
                    current
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Rgb(240, 210, 170)),
                );
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
                // 代码块上方边框（自适应宽度）
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
                    Style::default().fg(Color::Rgb(80, 90, 110)),
                )));
            }
            Event::End(TagEnd::CodeBlock) => {
                // 渲染代码块内容（带语法高亮）
                let code_inner_w = content_width.saturating_sub(4); // "│ " 前缀 + 右侧 " │" 后缀占4
                // 将 tab 替换为 4 空格（tab 的 unicode-width 为 0，但终端显示为多列，导致宽度计算错乱）
                let code_content_expanded = code_block_content.replace('\t', "    ");
                for code_line in code_content_expanded.lines() {
                    let wrapped = wrap_text(code_line, code_inner_w);
                    for wl in wrapped {
                        let highlighted = highlight_code_line(&wl, &code_block_lang);
                        let text_w: usize =
                            highlighted.iter().map(|s| display_width(&s.content)).sum();
                        let fill = code_inner_w.saturating_sub(text_w);
                        let mut spans_vec = Vec::new();
                        spans_vec.push(Span::styled(
                            "│ ",
                            Style::default().fg(Color::Rgb(80, 90, 110)),
                        ));
                        for hs in highlighted {
                            spans_vec.push(Span::styled(
                                hs.content.to_string(),
                                hs.style.bg(Color::Rgb(30, 30, 42)),
                            ));
                        }
                        spans_vec.push(Span::styled(
                            format!("{} │", " ".repeat(fill)),
                            Style::default()
                                .fg(Color::Rgb(80, 90, 110))
                                .bg(Color::Rgb(30, 30, 42)),
                        ));
                        lines.push(Line::from(spans_vec));
                    }
                }
                let bottom_border = format!("└{}", "─".repeat(content_width.saturating_sub(1)));
                lines.push(Line::from(Span::styled(
                    bottom_border,
                    Style::default().fg(Color::Rgb(80, 90, 110)),
                )));
                in_code_block = false;
                code_block_content.clear();
                code_block_lang.clear();
            }
            Event::Code(text) => {
                if in_table {
                    // 表格中的行内代码也收集到当前单元格
                    current_cell.push('`');
                    current_cell.push_str(&text);
                    current_cell.push('`');
                } else {
                    // 行内代码：检查行宽，放不下则先换行
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
                                Style::default().fg(Color::Rgb(80, 100, 140)),
                            ));
                        }
                    }
                    current_spans.push(Span::styled(
                        code_str,
                        Style::default()
                            .fg(Color::Rgb(230, 190, 120))
                            .bg(Color::Rgb(45, 45, 60)),
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
                    Style::default().fg(Color::Rgb(100, 160, 255)),
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
                style_stack.push(Style::default().fg(Color::Rgb(150, 160, 180)));
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
                    // 表格中的文本收集到当前单元格
                    current_cell.push_str(&text);
                } else {
                    let style = *style_stack.last().unwrap_or(&base_style);
                    let text_str = text.to_string();

                    // 标题：添加可视化符号前缀代替 # 标记
                    if let Some(level) = heading_level {
                        let (prefix, prefix_style) = match level {
                            1 => (
                                ">> ",
                                Style::default()
                                    .fg(Color::Rgb(100, 180, 255))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            2 => (
                                ">> ",
                                Style::default()
                                    .fg(Color::Rgb(130, 190, 255))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            3 => (
                                "> ",
                                Style::default()
                                    .fg(Color::Rgb(160, 200, 255))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            _ => (
                                "> ",
                                Style::default()
                                    .fg(Color::Rgb(180, 210, 255))
                                    .add_modifier(Modifier::BOLD),
                            ),
                        };
                        current_spans.push(Span::styled(prefix.to_string(), prefix_style));
                        heading_level = None; // 只加一次前缀
                    }

                    // 引用块：加左侧竖线
                    let effective_prefix_w = if in_blockquote { 2 } else { 0 }; // "| " 宽度
                    let full_line_w = content_width.saturating_sub(effective_prefix_w);

                    // 计算 current_spans 已有的显示宽度
                    let existing_w: usize = current_spans
                        .iter()
                        .map(|s| display_width(&s.content))
                        .sum();

                    // 剩余可用宽度
                    let wrap_w = full_line_w.saturating_sub(existing_w);

                    // 如果剩余宽度太小（不足整行的 1/4），先 flush 当前行再换行，
                    // 避免文字被挤到极窄的空间导致竖排
                    let min_useful_w = full_line_w / 4;
                    let wrap_w = if wrap_w < min_useful_w.max(4) && !current_spans.is_empty() {
                        flush_line(&mut current_spans, &mut lines);
                        if in_blockquote {
                            current_spans.push(Span::styled(
                                "| ".to_string(),
                                Style::default().fg(Color::Rgb(80, 100, 140)),
                            ));
                        }
                        // flush 后使用完整行宽
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
                                    Style::default().fg(Color::Rgb(80, 100, 140)),
                                ));
                            }
                        }
                        if !line.is_empty() {
                            // 第一行使用减去已有 span 宽度的 wrap_w，后续行使用完整 content_width
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
                                            Style::default().fg(Color::Rgb(80, 100, 140)),
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
                    Style::default().fg(Color::Rgb(70, 75, 90)),
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
                // 表格结束：计算列宽，渲染完整表格
                flush_line(&mut current_spans, &mut lines);
                in_table = false;

                if !table_rows.is_empty() {
                    let num_cols = table_rows.iter().map(|r| r.len()).max().unwrap_or(0);
                    if num_cols > 0 {
                        // 计算每列最大宽度
                        let mut col_widths: Vec<usize> = vec![0; num_cols];
                        for row in &table_rows {
                            for (i, cell) in row.iter().enumerate() {
                                let w = display_width(cell);
                                if w > col_widths[i] {
                                    col_widths[i] = w;
                                }
                            }
                        }

                        // 限制总宽度不超过 content_width，等比缩放
                        let sep_w = num_cols + 1; // 竖线占用
                        let pad_w = num_cols * 2; // 每列左右各1空格
                        let avail = content_width.saturating_sub(sep_w + pad_w);
                        // 单列最大宽度限制（避免一列过宽）
                        let max_col_w = avail * 2 / 3;
                        for cw in col_widths.iter_mut() {
                            if *cw > max_col_w {
                                *cw = max_col_w;
                            }
                        }
                        let total_col_w: usize = col_widths.iter().sum();
                        if total_col_w > avail && total_col_w > 0 {
                            // 等比缩放
                            let mut remaining = avail;
                            for (i, cw) in col_widths.iter_mut().enumerate() {
                                if i == num_cols - 1 {
                                    // 最后一列取剩余宽度，避免取整误差
                                    *cw = remaining.max(1);
                                } else {
                                    *cw = ((*cw) * avail / total_col_w).max(1);
                                    remaining = remaining.saturating_sub(*cw);
                                }
                            }
                        }

                        let table_style = Style::default().fg(Color::Rgb(180, 180, 200));
                        let header_style = Style::default()
                            .fg(Color::Rgb(120, 180, 255))
                            .add_modifier(Modifier::BOLD);
                        let border_style = Style::default().fg(Color::Rgb(60, 70, 100));

                        // 表格行的实际字符宽度（用空格字符计算，不依赖 Box Drawing 字符宽度）
                        // table_row_w = 竖线数(num_cols+1) + 每列(cw+2) = sep_w + pad_w + total_col_w
                        let total_col_w_final: usize = col_widths.iter().sum();
                        let table_row_w = sep_w + pad_w + total_col_w_final;
                        // 表格行右侧需要补充的空格数，使整行宽度等于 content_width
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
                        // 补充右侧空格，使宽度对齐 content_width
                        let mut top_spans = vec![Span::styled(top, border_style)];
                        if table_right_pad > 0 {
                            top_spans.push(Span::raw(" ".repeat(table_right_pad)));
                        }
                        lines.push(Line::from(top_spans));

                        for (row_idx, row) in table_rows.iter().enumerate() {
                            // 数据行 │ cell │ cell │
                            let mut row_spans: Vec<Span> = Vec::new();
                            row_spans.push(Span::styled("│", border_style));
                            for (i, cw) in col_widths.iter().enumerate() {
                                let cell_text = row.get(i).map(|s| s.as_str()).unwrap_or("");
                                let cell_w = display_width(cell_text);
                                let text = if cell_w > *cw {
                                    // 截断
                                    let mut t = String::new();
                                    let mut w = 0;
                                    for ch in cell_text.chars() {
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
                                    // 根据对齐方式填充
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
                                        _ => {
                                            format!(" {}{} ", cell_text, " ".repeat(fill))
                                        }
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
                            // 补充右侧空格，使宽度对齐 content_width
                            if table_right_pad > 0 {
                                row_spans.push(Span::raw(" ".repeat(table_right_pad)));
                            }
                            lines.push(Line::from(row_spans));

                            // 表头行后加分隔线 ├─┼─┤
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

/// 简单的代码语法高亮（无需外部依赖）
/// 根据语言类型对常见关键字、字符串、注释、数字进行着色
pub fn highlight_code_line<'a>(line: &'a str, lang: &str) -> Vec<Span<'static>> {
    let lang_lower = lang.to_lowercase();
    // Rust 使用多组词汇分别高亮
    // keywords: 控制流/定义关键字 → 紫色
    // primitive_types: 原始类型 → 青绿色
    // 其他类型名（大写开头）自动通过 type_style 高亮 → 暖黄色
    // 宏调用（word!）通过 macro_style 高亮 → 淡蓝色
    let keywords: &[&str] = match lang_lower.as_str() {
        "rust" | "rs" => &[
            // 控制流/定义关键字（紫色）
            "fn", "let", "mut", "pub", "use", "mod", "struct", "enum", "impl", "trait", "for",
            "while", "loop", "if", "else", "match", "return", "self", "Self", "where", "async",
            "await", "move", "ref", "type", "const", "static", "crate", "super", "as", "in",
            "true", "false", "unsafe", "extern", "dyn", "abstract", "become", "box", "do", "final",
            "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "union", "break",
            "continue",
        ],
        "python" | "py" => &[
            "def", "class", "return", "if", "elif", "else", "for", "while", "import", "from", "as",
            "with", "try", "except", "finally", "raise", "pass", "break", "continue", "yield",
            "lambda", "and", "or", "not", "in", "is", "True", "False", "None", "global",
            "nonlocal", "assert", "del", "async", "await", "self", "print",
        ],
        "javascript" | "js" | "typescript" | "ts" | "jsx" | "tsx" => &[
            "function",
            "const",
            "let",
            "var",
            "return",
            "if",
            "else",
            "for",
            "while",
            "class",
            "new",
            "this",
            "import",
            "export",
            "from",
            "default",
            "async",
            "await",
            "try",
            "catch",
            "finally",
            "throw",
            "typeof",
            "instanceof",
            "true",
            "false",
            "null",
            "undefined",
            "of",
            "in",
            "switch",
            "case",
        ],
        "go" | "golang" => &[
            "func",
            "package",
            "import",
            "return",
            "if",
            "else",
            "for",
            "range",
            "struct",
            "interface",
            "type",
            "var",
            "const",
            "defer",
            "go",
            "chan",
            "select",
            "case",
            "switch",
            "default",
            "break",
            "continue",
            "map",
            "true",
            "false",
            "nil",
            "make",
            "append",
            "len",
            "cap",
        ],
        "java" | "kotlin" | "kt" => &[
            "public",
            "private",
            "protected",
            "class",
            "interface",
            "extends",
            "implements",
            "return",
            "if",
            "else",
            "for",
            "while",
            "new",
            "this",
            "import",
            "package",
            "static",
            "final",
            "void",
            "int",
            "String",
            "boolean",
            "true",
            "false",
            "null",
            "try",
            "catch",
            "throw",
            "throws",
            "fun",
            "val",
            "var",
            "when",
            "object",
            "companion",
        ],
        "sh" | "bash" | "zsh" | "shell" => &[
            "if",
            "then",
            "else",
            "elif",
            "fi",
            "for",
            "while",
            "do",
            "done",
            "case",
            "esac",
            "function",
            "return",
            "exit",
            "echo",
            "export",
            "local",
            "readonly",
            "set",
            "unset",
            "shift",
            "source",
            "in",
            "true",
            "false",
            "read",
            "declare",
            "typeset",
            "trap",
            "eval",
            "exec",
            "test",
            "select",
            "until",
            "break",
            "continue",
            "printf",
            // Go 命令
            "go",
            "build",
            "run",
            "test",
            "fmt",
            "vet",
            "mod",
            "get",
            "install",
            "clean",
            "doc",
            "list",
            "version",
            "env",
            "generate",
            "tool",
            "proxy",
            "GOPATH",
            "GOROOT",
            "GOBIN",
            "GOMODCACHE",
            "GOPROXY",
            "GOSUMDB",
            // Cargo 命令
            "cargo",
            "new",
            "init",
            "add",
            "remove",
            "update",
            "check",
            "clippy",
            "rustfmt",
            "rustc",
            "rustup",
            "publish",
            "install",
            "uninstall",
            "search",
            "tree",
            "locate_project",
            "metadata",
            "audit",
            "watch",
            "expand",
        ],
        "c" | "cpp" | "c++" | "h" | "hpp" => &[
            "int",
            "char",
            "float",
            "double",
            "void",
            "long",
            "short",
            "unsigned",
            "signed",
            "const",
            "static",
            "extern",
            "struct",
            "union",
            "enum",
            "typedef",
            "sizeof",
            "return",
            "if",
            "else",
            "for",
            "while",
            "do",
            "switch",
            "case",
            "break",
            "continue",
            "default",
            "goto",
            "auto",
            "register",
            "volatile",
            "class",
            "public",
            "private",
            "protected",
            "virtual",
            "override",
            "template",
            "namespace",
            "using",
            "new",
            "delete",
            "try",
            "catch",
            "throw",
            "nullptr",
            "true",
            "false",
            "this",
            "include",
            "define",
            "ifdef",
            "ifndef",
            "endif",
        ],
        "sql" => &[
            "SELECT",
            "FROM",
            "WHERE",
            "INSERT",
            "UPDATE",
            "DELETE",
            "CREATE",
            "DROP",
            "ALTER",
            "TABLE",
            "INDEX",
            "INTO",
            "VALUES",
            "SET",
            "AND",
            "OR",
            "NOT",
            "NULL",
            "JOIN",
            "LEFT",
            "RIGHT",
            "INNER",
            "OUTER",
            "ON",
            "GROUP",
            "BY",
            "ORDER",
            "ASC",
            "DESC",
            "HAVING",
            "LIMIT",
            "OFFSET",
            "UNION",
            "AS",
            "DISTINCT",
            "COUNT",
            "SUM",
            "AVG",
            "MIN",
            "MAX",
            "LIKE",
            "IN",
            "BETWEEN",
            "EXISTS",
            "CASE",
            "WHEN",
            "THEN",
            "ELSE",
            "END",
            "BEGIN",
            "COMMIT",
            "ROLLBACK",
            "PRIMARY",
            "KEY",
            "FOREIGN",
            "REFERENCES",
            "select",
            "from",
            "where",
            "insert",
            "update",
            "delete",
            "create",
            "drop",
            "alter",
            "table",
            "index",
            "into",
            "values",
            "set",
            "and",
            "or",
            "not",
            "null",
            "join",
            "left",
            "right",
            "inner",
            "outer",
            "on",
            "group",
            "by",
            "order",
            "asc",
            "desc",
            "having",
            "limit",
            "offset",
            "union",
            "as",
            "distinct",
            "count",
            "sum",
            "avg",
            "min",
            "max",
            "like",
            "in",
            "between",
            "exists",
            "case",
            "when",
            "then",
            "else",
            "end",
            "begin",
            "commit",
            "rollback",
            "primary",
            "key",
            "foreign",
            "references",
        ],
        "yaml" | "yml" => &["true", "false", "null", "yes", "no", "on", "off"],
        "toml" => &[
            "true",
            "false",
            "true",
            "false",
            // Cargo.toml 常用
            "name",
            "version",
            "edition",
            "authors",
            "dependencies",
            "dev-dependencies",
            "build-dependencies",
            "features",
            "workspace",
            "members",
            "exclude",
            "include",
            "path",
            "git",
            "branch",
            "tag",
            "rev",
            "package",
            "lib",
            "bin",
            "example",
            "test",
            "bench",
            "doc",
            "profile",
            "release",
            "debug",
            "opt-level",
            "lto",
            "codegen-units",
            "panic",
            "strip",
            "default",
            "features",
            "optional",
            // 常见配置项
            "repository",
            "homepage",
            "documentation",
            "license",
            "license-file",
            "keywords",
            "categories",
            "readme",
            "description",
            "resolver",
        ],
        "css" | "scss" | "less" => &[
            "color",
            "background",
            "border",
            "margin",
            "padding",
            "display",
            "position",
            "width",
            "height",
            "font",
            "text",
            "flex",
            "grid",
            "align",
            "justify",
            "important",
            "none",
            "auto",
            "inherit",
            "initial",
            "unset",
        ],
        "dockerfile" | "docker" => &[
            "FROM",
            "RUN",
            "CMD",
            "LABEL",
            "EXPOSE",
            "ENV",
            "ADD",
            "COPY",
            "ENTRYPOINT",
            "VOLUME",
            "USER",
            "WORKDIR",
            "ARG",
            "ONBUILD",
            "STOPSIGNAL",
            "HEALTHCHECK",
            "SHELL",
            "AS",
        ],
        "ruby" | "rb" => &[
            "def", "end", "class", "module", "if", "elsif", "else", "unless", "while", "until",
            "for", "do", "begin", "rescue", "ensure", "raise", "return", "yield", "require",
            "include", "attr", "self", "true", "false", "nil", "puts", "print",
        ],
        _ => &[
            "fn", "function", "def", "class", "return", "if", "else", "for", "while", "import",
            "export", "const", "let", "var", "true", "false", "null", "nil", "None", "self",
            "this",
        ],
    };

    // 原始/内建类型列表（青绿色）
    let primitive_types: &[&str] = match lang_lower.as_str() {
        "rust" | "rs" => &[
            "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
            "f32", "f64", "bool", "char", "str",
        ],
        "go" | "golang" => &[
            // Go 内建类型
            "int",
            "int8",
            "int16",
            "int32",
            "int64",
            "uint",
            "uint8",
            "uint16",
            "uint32",
            "uint64",
            "uintptr",
            "float32",
            "float64",
            "complex64",
            "complex128",
            "bool",
            "byte",
            "rune",
            "string",
            "error",
            "any",
        ],
        _ => &[],
    };

    // Go 语言显式类型名列表（暖黄色，因为 Go 的大写开头不代表类型）
    let go_type_names: &[&str] = match lang_lower.as_str() {
        "go" | "golang" => &[
            // 常见标准库类型和接口
            "Reader",
            "Writer",
            "Closer",
            "ReadWriter",
            "ReadCloser",
            "WriteCloser",
            "ReadWriteCloser",
            "Seeker",
            "Context",
            "Error",
            "Stringer",
            "Mutex",
            "RWMutex",
            "WaitGroup",
            "Once",
            "Pool",
            "Map",
            "Duration",
            "Time",
            "Timer",
            "Ticker",
            "Buffer",
            "Builder",
            "Request",
            "Response",
            "ResponseWriter",
            "Handler",
            "HandlerFunc",
            "Server",
            "Client",
            "Transport",
            "File",
            "FileInfo",
            "FileMode",
            "Decoder",
            "Encoder",
            "Marshaler",
            "Unmarshaler",
            "Logger",
            "Flag",
            "Regexp",
            "Conn",
            "Listener",
            "Addr",
            "Scanner",
            "Token",
            "Type",
            "Value",
            "Kind",
            "Cmd",
            "Signal",
        ],
        _ => &[],
    };

    let comment_prefix = match lang_lower.as_str() {
        "python" | "py" | "sh" | "bash" | "zsh" | "shell" | "ruby" | "rb" | "yaml" | "yml"
        | "toml" | "dockerfile" | "docker" => "#",
        "sql" => "--",
        "css" | "scss" | "less" => "/*",
        _ => "//",
    };

    // ===== 代码高亮配色方案（基于 One Dark Pro）=====
    // 默认代码颜色 - 银灰色
    let code_style = Style::default().fg(Color::Rgb(171, 178, 191));
    // 关键字颜色 - 紫色（保留 One Dark 经典紫）
    let kw_style = Style::default().fg(Color::Rgb(198, 120, 221));
    // 字符串颜色 - 柔和绿色
    let str_style = Style::default().fg(Color::Rgb(152, 195, 121));
    // 注释颜色 - 深灰蓝色 + 斜体
    let comment_style = Style::default()
        .fg(Color::Rgb(92, 99, 112))
        .add_modifier(Modifier::ITALIC);
    // 数字颜色 - 橙黄色
    let num_style = Style::default().fg(Color::Rgb(209, 154, 102));
    // 类型/大写开头标识符 - 暖黄色
    let type_style = Style::default().fg(Color::Rgb(229, 192, 123));
    // 原始类型（i32, u64, bool 等）- 青绿色
    let primitive_style = Style::default().fg(Color::Rgb(86, 182, 194));

    let trimmed = line.trim_start();

    // 注释行
    if trimmed.starts_with(comment_prefix) {
        return vec![Span::styled(line.to_string(), comment_style)];
    }

    // 逐词解析
    let mut spans = Vec::new();
    let mut chars = line.chars().peekable();
    let mut buf = String::new();

    while let Some(&ch) = chars.peek() {
        // 双引号字符串（支持 \ 转义）
        if ch == '"' {
            if !buf.is_empty() {
                spans.extend(colorize_tokens(
                    &buf,
                    keywords,
                    primitive_types,
                    go_type_names,
                    code_style,
                    kw_style,
                    num_style,
                    type_style,
                    primitive_style,
                    &lang_lower,
                ));
                buf.clear();
            }
            let mut s = String::new();
            s.push(ch);
            chars.next();
            let mut escaped = false;
            while let Some(&c) = chars.peek() {
                s.push(c);
                chars.next();
                if escaped {
                    escaped = false;
                    continue;
                }
                if c == '\\' {
                    escaped = true;
                    continue;
                }
                if c == '"' {
                    break;
                }
            }
            spans.push(Span::styled(s, str_style));
            continue;
        }
        // 反引号字符串（不支持转义，遇到配对反引号结束）
        // Go 原始字符串、JS 模板字符串、Rust 无此语义但兼容处理
        if ch == '`' {
            if !buf.is_empty() {
                spans.extend(colorize_tokens(
                    &buf,
                    keywords,
                    primitive_types,
                    go_type_names,
                    code_style,
                    kw_style,
                    num_style,
                    type_style,
                    primitive_style,
                    &lang_lower,
                ));
                buf.clear();
            }
            let mut s = String::new();
            s.push(ch);
            chars.next();
            while let Some(&c) = chars.peek() {
                s.push(c);
                chars.next();
                if c == '`' {
                    break; // 反引号不支持转义，遇到配对直接结束
                }
            }
            spans.push(Span::styled(s, str_style));
            continue;
        }
        // Rust 生命周期参数 ('a, 'static 等) vs 字符字面量 ('x')
        if ch == '\'' && matches!(lang_lower.as_str(), "rust" | "rs") {
            if !buf.is_empty() {
                spans.extend(colorize_tokens(
                    &buf,
                    keywords,
                    primitive_types,
                    go_type_names,
                    code_style,
                    kw_style,
                    num_style,
                    type_style,
                    primitive_style,
                    &lang_lower,
                ));
                buf.clear();
            }
            let mut s = String::new();
            s.push(ch);
            chars.next();
            let mut is_lifetime = false;
            // 收集后续字符来判断是生命周期还是字符字面量
            while let Some(&c) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    s.push(c);
                    chars.next();
                } else if c == '\'' && s.len() == 2 {
                    // 'x' 形式 - 字符字面量
                    s.push(c);
                    chars.next();
                    break;
                } else {
                    // 'name 形式 - 生命周期参数
                    is_lifetime = true;
                    break;
                }
            }
            if is_lifetime || (s.len() > 1 && !s.ends_with('\'')) {
                // 生命周期参数 - 使用浅橙色（与关键字紫色区分）
                let lifetime_style = Style::default().fg(Color::Rgb(229, 192, 123));
                spans.push(Span::styled(s, lifetime_style));
            } else {
                // 字符字面量
                spans.push(Span::styled(s, str_style));
            }
            continue;
        }
        // 其他语言的字符串（包含单引号）
        if ch == '\'' && !matches!(lang_lower.as_str(), "rust" | "rs") {
            // 先刷新 buf
            if !buf.is_empty() {
                spans.extend(colorize_tokens(
                    &buf,
                    keywords,
                    primitive_types,
                    go_type_names,
                    code_style,
                    kw_style,
                    num_style,
                    type_style,
                    primitive_style,
                    &lang_lower,
                ));
                buf.clear();
            }
            let mut s = String::new();
            s.push(ch);
            chars.next();
            let mut escaped = false;
            while let Some(&c) = chars.peek() {
                s.push(c);
                chars.next();
                if escaped {
                    escaped = false;
                    continue;
                }
                if c == '\\' {
                    escaped = true;
                    continue;
                }
                if c == '\'' {
                    break;
                }
            }
            spans.push(Span::styled(s, str_style));
            continue;
        }
        // Rust 属性 (#[...] 或 #![...])
        if ch == '#' && matches!(lang_lower.as_str(), "rust" | "rs") {
            let mut lookahead = chars.clone();
            if let Some(next) = lookahead.next() {
                if next == '[' {
                    if !buf.is_empty() {
                        spans.extend(colorize_tokens(
                            &buf,
                            keywords,
                            primitive_types,
                            go_type_names,
                            code_style,
                            kw_style,
                            num_style,
                            type_style,
                            primitive_style,
                            &lang_lower,
                        ));
                        buf.clear();
                    }
                    let mut attr = String::new();
                    attr.push(ch);
                    chars.next();
                    let mut depth = 0;
                    while let Some(&c) = chars.peek() {
                        attr.push(c);
                        chars.next();
                        if c == '[' {
                            depth += 1;
                        } else if c == ']' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                    // 属性使用青绿色（与关键字紫色区分）
                    let attr_style = Style::default().fg(Color::Rgb(86, 182, 194));
                    spans.push(Span::styled(attr, attr_style));
                    continue;
                }
            }
        }
        // Shell 变量 ($VAR, ${VAR}, $1 等)
        if ch == '$'
            && matches!(
                lang_lower.as_str(),
                "sh" | "bash" | "zsh" | "shell" | "dockerfile" | "docker"
            )
        {
            if !buf.is_empty() {
                spans.extend(colorize_tokens(
                    &buf,
                    keywords,
                    primitive_types,
                    go_type_names,
                    code_style,
                    kw_style,
                    num_style,
                    type_style,
                    primitive_style,
                    &lang_lower,
                ));
                buf.clear();
            }
            // Shell 变量使用青色（与属性统一风格）
            let var_style = Style::default().fg(Color::Rgb(86, 182, 194));
            let mut var = String::new();
            var.push(ch);
            chars.next();
            if let Some(&next_ch) = chars.peek() {
                if next_ch == '{' {
                    // ${VAR}
                    var.push(next_ch);
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        var.push(c);
                        chars.next();
                        if c == '}' {
                            break;
                        }
                    }
                } else if next_ch == '(' {
                    // $(cmd)
                    var.push(next_ch);
                    chars.next();
                    let mut depth = 1;
                    while let Some(&c) = chars.peek() {
                        var.push(c);
                        chars.next();
                        if c == '(' {
                            depth += 1;
                        }
                        if c == ')' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                } else if next_ch.is_alphanumeric()
                    || next_ch == '_'
                    || next_ch == '@'
                    || next_ch == '#'
                    || next_ch == '?'
                    || next_ch == '!'
                {
                    // $VAR, $1, $@, $#, $? 等
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            var.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
            }
            spans.push(Span::styled(var, var_style));
            continue;
        }
        // 行内注释检测
        // 注意：chars.clone().collect() 包含当前 peek 的 ch，所以 rest 已经以 ch 开头
        if ch == '/' || ch == '#' || ch == '-' {
            let rest: String = chars.clone().collect();
            if rest.starts_with(comment_prefix) {
                if !buf.is_empty() {
                    spans.extend(colorize_tokens(
                        &buf,
                        keywords,
                        primitive_types,
                        go_type_names,
                        code_style,
                        kw_style,
                        num_style,
                        type_style,
                        primitive_style,
                        &lang_lower,
                    ));
                    buf.clear();
                }
                // 消耗掉所有剩余字符（包括当前 ch）
                while chars.peek().is_some() {
                    chars.next();
                }
                spans.push(Span::styled(rest, comment_style));
                break;
            }
        }
        buf.push(ch);
        chars.next();
    }

    if !buf.is_empty() {
        spans.extend(colorize_tokens(
            &buf,
            keywords,
            primitive_types,
            go_type_names,
            code_style,
            kw_style,
            num_style,
            type_style,
            primitive_style,
            &lang_lower,
        ));
    }

    if spans.is_empty() {
        spans.push(Span::styled(line.to_string(), code_style));
    }

    spans
}

/// 将文本按照 word boundary 拆分并对关键字、数字、类型名、原始类型着色
pub fn colorize_tokens<'a>(
    text: &str,
    keywords: &[&str],
    primitive_types: &[&str],
    go_type_names: &[&str],
    default_style: Style,
    kw_style: Style,
    num_style: Style,
    type_style: Style,
    primitive_style: Style,
    lang: &str,
) -> Vec<Span<'static>> {
    // 宏调用样式（Rust 专用）- 淡蓝色，与属性青绿色区分
    let macro_style = Style::default().fg(Color::Rgb(97, 175, 239));

    let mut spans = Vec::new();
    let mut current_word = String::new();
    let mut current_non_word = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_alphanumeric() || ch == '_' {
            if !current_non_word.is_empty() {
                spans.push(Span::styled(current_non_word.clone(), default_style));
                current_non_word.clear();
            }
            current_word.push(ch);
        } else {
            // Rust 宏调用高亮：word! 或 word!()
            if ch == '!' && matches!(lang, "rust" | "rs") && !current_word.is_empty() {
                // 检查后面是否跟着 ( 或 { 或 [
                let is_macro = chars
                    .peek()
                    .map(|&c| c == '(' || c == '{' || c == '[' || c.is_whitespace())
                    .unwrap_or(true);
                if is_macro {
                    // 将当前 word 作为宏名高亮
                    spans.push(Span::styled(current_word.clone(), macro_style));
                    current_word.clear();
                    spans.push(Span::styled("!".to_string(), macro_style));
                    continue;
                }
            }
            if !current_word.is_empty() {
                let style = classify_word(
                    &current_word,
                    keywords,
                    primitive_types,
                    go_type_names,
                    kw_style,
                    primitive_style,
                    num_style,
                    type_style,
                    default_style,
                    lang,
                );
                spans.push(Span::styled(current_word.clone(), style));
                current_word.clear();
            }
            current_non_word.push(ch);
        }
    }

    // 刷新剩余
    if !current_non_word.is_empty() {
        spans.push(Span::styled(current_non_word, default_style));
    }
    if !current_word.is_empty() {
        let style = classify_word(
            &current_word,
            keywords,
            primitive_types,
            go_type_names,
            kw_style,
            primitive_style,
            num_style,
            type_style,
            default_style,
            lang,
        );
        spans.push(Span::styled(current_word, style));
    }

    spans
}

/// 根据语言规则判断一个 word 应该使用哪种颜色样式
pub fn classify_word(
    word: &str,
    keywords: &[&str],
    primitive_types: &[&str],
    go_type_names: &[&str],
    kw_style: Style,
    primitive_style: Style,
    num_style: Style,
    type_style: Style,
    default_style: Style,
    lang: &str,
) -> Style {
    if keywords.contains(&word) {
        kw_style
    } else if primitive_types.contains(&word) {
        primitive_style
    } else if word
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        num_style
    } else if matches!(lang, "go" | "golang") {
        // Go 语言：大写开头不代表类型，只有显式列表中的才高亮为类型色
        if go_type_names.contains(&word) {
            type_style
        } else {
            default_style
        }
    } else if word
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        // 其他语言：大写开头 → 类型名高亮
        type_style
    } else {
        default_style
    }
}
