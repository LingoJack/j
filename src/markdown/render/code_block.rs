use crate::markdown::highlight::highlight_code_line;
use crate::markdown::theme::MdStyle;
use crate::util::text::{display_width, wrap_text};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// 渲染围栏代码块（撑满可用宽度 + 自动折行 + 前后空行）
pub fn render_code_block(
    lang: &str,
    code: &str,
    content_width: usize,
    theme: &dyn MdStyle,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    // 最小宽度保证
    let total_width = content_width.max(10);

    // 代码内容可用宽度 = total_width - 4（│ + 空格 + 内容 + 空格 + │）
    let code_inner_w = total_width.saturating_sub(4);

    // 前导空行
    lines.push(Line::from(""));

    // 开始围栏：┌─ lang ──────┐
    let (left_part, left_width) = if lang.is_empty() {
        ("┌─".to_string(), 2)
    } else {
        let s = format!("┌─ {} ─", lang);
        let w = display_width(&s);
        (s, w)
    };

    let dash_count = total_width.saturating_sub(left_width + 1).max(1);

    let top_border_style = Style::default().fg(theme.text_dim()).bg(theme.bg_primary());

    lines.push(Line::from(Span::styled(
        format!("{}{}┐", left_part, "─".repeat(dash_count)),
        top_border_style,
    )));

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

            // 空格 padding
            spans_vec.push(Span::styled(" ", Style::default().bg(theme.bg_primary())));

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

            // 右侧空格 padding
            spans_vec.push(Span::styled(" ", Style::default().bg(theme.bg_primary())));

            // 右侧边框 │
            spans_vec.push(Span::styled(
                "│",
                Style::default().fg(theme.text_dim()).bg(theme.bg_primary()),
            ));

            lines.push(Line::from(spans_vec));
        }
    }

    // 结束围栏：└─────────────┘
    let bottom_dash_count = total_width.saturating_sub(2).max(1);
    let bottom_border = format!("└{}┘", "─".repeat(bottom_dash_count));

    lines.push(Line::from(Span::styled(bottom_border, top_border_style)));

    // 后导空行
    lines.push(Line::from(""));

    lines
}
