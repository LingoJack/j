use crate::util::text::{chars_with_display_width, display_width};
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// 前缀感知的 span 折行。
///
/// 适用于 markdown 列表这类“首行有 bullet / 序号，续行需要对齐”的场景，
/// 也可用于普通段落（prefix 留空）。
pub(crate) fn wrap_spans_with_prefix(
    spans: Vec<Span<'static>>,
    max_width: usize,
    first_prefix: Vec<Span<'static>>,
    continuation_prefix: Vec<Span<'static>>,
) -> Vec<Line<'static>> {
    let first_prefix_width: usize = first_prefix.iter().map(|s| display_width(&s.content)).sum();
    let continuation_prefix_width: usize = continuation_prefix
        .iter()
        .map(|s| display_width(&s.content))
        .sum();
    let max_width = max_width
        .max(first_prefix_width + 2)
        .max(continuation_prefix_width + 2);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line = first_prefix;
    let mut current_width = first_prefix_width;
    let mut current_prefix_width = first_prefix_width;
    let mut current_style: Option<Style> = None;
    let mut current_text = String::new();

    for span in spans {
        let style = span.style;
        for (ch, ch_width) in chars_with_display_width(&span.content) {
            if ch == '\n' {
                flush_text_buffer(&mut current_line, &mut current_text, &mut current_style);
                lines.push(Line::from(std::mem::take(&mut current_line)));
                current_line = continuation_prefix.clone();
                current_width = continuation_prefix_width;
                current_prefix_width = continuation_prefix_width;
                current_style = None;
                continue;
            }

            if current_width + ch_width > max_width && current_width > current_prefix_width {
                flush_text_buffer(&mut current_line, &mut current_text, &mut current_style);
                lines.push(Line::from(std::mem::take(&mut current_line)));
                current_line = continuation_prefix.clone();
                current_width = continuation_prefix_width;
                current_prefix_width = continuation_prefix_width;
                current_style = None;
            }

            if current_style != Some(style) && !current_text.is_empty() {
                flush_text_buffer(&mut current_line, &mut current_text, &mut current_style);
            }
            current_style = Some(style);
            current_text.push(ch);
            current_width += ch_width;
        }
    }

    flush_text_buffer(&mut current_line, &mut current_text, &mut current_style);
    lines.push(Line::from(current_line));
    lines
}

fn flush_text_buffer(
    current_line: &mut Vec<Span<'static>>,
    current_text: &mut String,
    current_style: &mut Option<Style>,
) {
    if current_text.is_empty() {
        return;
    }
    current_line.push(Span::styled(
        std::mem::take(current_text),
        current_style.unwrap_or_default(),
    ));
}
