use crate::markdown::highlight::highlight_code_line;
use crate::markdown::theme::MdStyle;
use crate::util::text::{display_width, wrap_text};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// 渲染围栏代码块
pub fn render_code_block(
    lang: &str,
    code: &str,
    content_width: usize,
    theme: &dyn MdStyle,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    let label = if lang.is_empty() {
        " code ".to_string()
    } else {
        format!(" {} ", lang)
    };
    let label_w = display_width(&label);
    let border_fill = content_width.saturating_sub(3 + label_w);
    let top_border = format!("┌─{}{}┐", label, "─".repeat(border_fill));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(theme.code_border()).bg(theme.code_bg()),
    )));

    let code_inner_w = content_width.saturating_sub(4);
    let code_content_expanded = code.replace('\t', "    ");
    for code_line in code_content_expanded.lines() {
        let wrapped = wrap_text(code_line, code_inner_w);
        for wl in wrapped {
            let editor_theme = theme.code_syntax_theme();
            let highlighted = highlight_code_line(&wl, lang, &editor_theme);
            let text_w: usize = highlighted.iter().map(|s| display_width(&s.content)).sum();
            let fill = code_inner_w.saturating_sub(text_w);
            let mut spans_vec = Vec::new();
            spans_vec.push(Span::styled(
                "│ ".to_string(),
                Style::default().fg(theme.code_border()).bg(theme.code_bg()),
            ));
            for hs in highlighted {
                spans_vec.push(Span::styled(
                    hs.content.to_string(),
                    hs.style.bg(theme.code_bg()),
                ));
            }
            spans_vec.push(Span::styled(
                format!("{} │", " ".repeat(fill)),
                Style::default().fg(theme.code_border()).bg(theme.code_bg()),
            ));
            lines.push(Line::from(spans_vec));
        }
    }

    let bottom_border = format!("└{}┘", "─".repeat(content_width.saturating_sub(2)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(theme.code_border()).bg(theme.code_bg()),
    )));

    lines
}
