use crate::markdown::highlight::highlight_code_line;
use crate::markdown::theme::MdStyle;
use crate::theme::current_border_style;
use crate::util::text::{display_width, wrap_text};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// 代码块右侧内边距（字符数），防止竖线紧贴屏幕右边缘
const CODE_BLOCK_RIGHT_PADDING: usize = 2;
/// 代码块内容左右内边距（字符数），代码与竖线 │ 之间的空白
const CODE_BLOCK_INNER_PADDING_H: usize = 2;
/// 代码块内容上下内边距（行数），代码与上下边框之间的空白行数
const CODE_BLOCK_INNER_PADDING_V: usize = 1;

/// 渲染围栏代码块（撑满可用宽度 + 自动折行 + 前后空行）
pub fn render_code_block(
    lang: &str,
    code: &str,
    content_width: usize,
    theme: &dyn MdStyle,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    let border_style = current_border_style();

    // 最小宽度保证（减去右侧 padding）
    let total_width = content_width
        .saturating_sub(CODE_BLOCK_RIGHT_PADDING)
        .max(10);

    // 代码内容可用宽度 = total_width - 2(边框) - 2*INNER_PADDING_H
    let code_inner_w = total_width.saturating_sub(2 + 2 * CODE_BLOCK_INNER_PADDING_H);

    // 前导空行
    lines.push(Line::from(""));

    // 开始围栏：╭─ lang ──────╮ 或 ┌─ lang ──────┐
    let (left_part, left_width) = if lang.is_empty() {
        (format!("{}─", border_style.top_left()), 2)
    } else {
        let s = format!("{}─ {} ─", border_style.top_left(), lang);
        let w = display_width(&s);
        (s, w)
    };

    let dash_count = total_width.saturating_sub(left_width + 1).max(1);

    let top_border_style = Style::default().fg(theme.text_dim()).bg(theme.bg_primary());

    lines.push(Line::from(Span::styled(
        format!(
            "{}{}{}",
            left_part,
            "─".repeat(dash_count),
            border_style.top_right()
        ),
        top_border_style,
    )));

    // 上内边距空行
    for _ in 0..CODE_BLOCK_INNER_PADDING_V {
        lines.push(empty_content_line(code_inner_w, theme));
    }

    // 渲染代码内容行（自动折行）
    let code_content_expanded = code.replace('\t', "    ");
    for code_line in code_content_expanded.lines() {
        let wrapped = if code_inner_w > 0 {
            wrap_text(code_line, code_inner_w)
        } else {
            vec![code_line.to_string()]
        };
        for wl in wrapped {
            let editor_theme = theme.code_syntax_theme();
            let highlighted = highlight_code_line(&wl, lang, &editor_theme);
            let text_w: usize = highlighted.iter().map(|s| display_width(&s.content)).sum();
            let fill = code_inner_w.saturating_sub(text_w);

            let mut spans_vec = Vec::new();

            // 边框 │
            spans_vec.push(Span::styled(
                "│",
                Style::default().fg(theme.text_dim()).bg(theme.bg_primary()),
            ));

            // 左侧内边距空格
            spans_vec.push(Span::styled(
                " ".repeat(CODE_BLOCK_INNER_PADDING_H),
                Style::default().bg(theme.bg_primary()),
            ));

            // 代码内容（背景使用 bg_primary）
            for hs in highlighted {
                spans_vec.push(Span::styled(
                    hs.content.to_string(),
                    hs.style.bg(theme.bg_primary()),
                ));
            }

            // 右侧填充空格
            spans_vec.push(Span::styled(
                " ".repeat(fill),
                Style::default().bg(theme.bg_primary()),
            ));

            // 右侧内边距空格
            spans_vec.push(Span::styled(
                " ".repeat(CODE_BLOCK_INNER_PADDING_H),
                Style::default().bg(theme.bg_primary()),
            ));

            // 右侧边框 │
            spans_vec.push(Span::styled(
                "│",
                Style::default().fg(theme.text_dim()).bg(theme.bg_primary()),
            ));

            // 右侧 padding 空格
            spans_vec.push(Span::styled(
                " ".repeat(CODE_BLOCK_RIGHT_PADDING),
                Style::default().bg(theme.bg_primary()),
            ));

            lines.push(Line::from(spans_vec));
        }
    }

    // 下内边距空行
    for _ in 0..CODE_BLOCK_INNER_PADDING_V {
        lines.push(empty_content_line(code_inner_w, theme));
    }

    // 结束围栏：╰─────────────╯ 或 └─────────────┘
    let bottom_dash_count = total_width.saturating_sub(2).max(1);
    let bottom_border = format!(
        "{}{}{}",
        border_style.bottom_left(),
        "─".repeat(bottom_dash_count),
        border_style.bottom_right()
    );

    lines.push(Line::from(Span::styled(bottom_border, top_border_style)));

    // 后导空行
    lines.push(Line::from(""));

    lines
}

/// 生成一行只有边框和内边距空白的空行（用于上下内边距）
fn empty_content_line(code_inner_w: usize, theme: &dyn MdStyle) -> Line<'static> {
    let bg = theme.bg_primary();
    let mut spans_vec = Vec::new();

    // 左边框 │
    spans_vec.push(Span::styled(
        "│",
        Style::default().fg(theme.text_dim()).bg(bg),
    ));

    // 内容区域全空格
    let inner_total = code_inner_w + 2 * CODE_BLOCK_INNER_PADDING_H;
    spans_vec.push(Span::styled(
        " ".repeat(inner_total),
        Style::default().bg(bg),
    ));

    // 右边框 │
    spans_vec.push(Span::styled(
        "│",
        Style::default().fg(theme.text_dim()).bg(bg),
    ));

    // 右侧 padding
    spans_vec.push(Span::styled(
        " ".repeat(CODE_BLOCK_RIGHT_PADDING),
        Style::default().bg(bg),
    ));

    Line::from(spans_vec)
}
