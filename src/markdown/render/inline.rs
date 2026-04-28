use crate::markdown::ir::Inline;
use crate::markdown::theme::MdStyle;
use crate::util::text::display_width;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

/// 将 Inline 元素列表渲染为 Span 列表，应用主题着色
pub fn render_inlines(
    inlines: &[Inline],
    base_style: Style,
    theme: &dyn MdStyle,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for inline in inlines {
        render_inline(inline, base_style, theme, &mut spans);
    }
    spans
}

fn render_inline(
    inline: &Inline,
    base_style: Style,
    theme: &dyn MdStyle,
    out: &mut Vec<Span<'static>>,
) {
    match inline {
        Inline::Text(text) => {
            let text_clean = text.replace('\u{200B}', "");
            // 拆分 URL 与普通文本
            let link_style = Style::default()
                .fg(theme.md_link())
                .add_modifier(Modifier::UNDERLINED);
            let segments = split_text_with_urls(&text_clean, base_style, link_style);
            out.extend(segments);
        }
        Inline::Strong(children) => {
            let style = base_style
                .add_modifier(Modifier::BOLD)
                .fg(theme.text_bold());
            for child in children {
                render_inline(child, style, theme, out);
            }
        }
        Inline::Emphasis(children) => {
            let style = base_style.add_modifier(Modifier::ITALIC);
            for child in children {
                render_inline(child, style, theme, out);
            }
        }
        Inline::Strikethrough(children) => {
            let style = base_style.add_modifier(Modifier::CROSSED_OUT);
            for child in children {
                render_inline(child, style, theme, out);
            }
        }
        Inline::Code(text) => {
            let code_str = format!(" {} ", text);
            out.push(Span::styled(
                code_str,
                Style::default()
                    .fg(theme.md_inline_code_fg())
                    .bg(theme.md_inline_code_bg()),
            ));
        }
        Inline::Link { text, url } => {
            let link_style = Style::default()
                .fg(theme.md_link())
                .add_modifier(Modifier::UNDERLINED);
            for child in text {
                render_inline(child, link_style, theme, out);
            }
            // 如果链接文本和 URL 不同，在文本后追加显示 URL
            let text_content = collect_inline_text(text);
            if !text_content.is_empty() && text_content != *url {
                out.push(Span::styled(
                    format!(" ({})", url),
                    Style::default()
                        .fg(theme.md_link())
                        .add_modifier(Modifier::DIM),
                ));
            }
        }
        Inline::SoftBreak => {
            out.push(Span::raw(" "));
        }
        Inline::HardBreak => {
            // 硬换行在 inline 层用换行符标记，上层 render_block 需要处理
            out.push(Span::raw("\n"));
        }
    }
}

/// 收集 inline 元素中的纯文本内容（用于 URL 对比）
fn collect_inline_text(inlines: &[Inline]) -> String {
    let mut result = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(s) => result.push_str(s),
            Inline::Code(s) => result.push_str(s),
            Inline::Strong(children)
            | Inline::Emphasis(children)
            | Inline::Strikethrough(children) => result.push_str(&collect_inline_text(children)),
            Inline::Link { text, .. } => result.push_str(&collect_inline_text(text)),
            Inline::SoftBreak => result.push(' '),
            Inline::HardBreak => result.push('\n'),
        }
    }
    result
}

/// 计算 inline 元素的显示宽度
pub fn inline_display_width(inlines: &[Inline]) -> usize {
    inlines
        .iter()
        .map(|i| match i {
            Inline::Text(s) => display_width(s),
            Inline::Code(s) => display_width(&format!(" {} ", s)),
            Inline::Strong(children)
            | Inline::Emphasis(children)
            | Inline::Strikethrough(children) => inline_display_width(children),
            Inline::SoftBreak => 1,
            Inline::HardBreak => 0,
            Inline::Link { text, .. } => inline_display_width(text),
        })
        .sum()
}

// ---------------------------------------------------------------------------
// URL splitting (migrated from parser/text.rs)
// ---------------------------------------------------------------------------

/// 将文本拆分为普通文本和 URL 片段，对 URL 应用链接样式
pub(crate) fn split_text_with_urls(
    text: &str,
    normal_style: Style,
    link_style: Style,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        let url_start = remaining
            .find("https://")
            .or_else(|| remaining.find("http://"));

        match url_start {
            Some(start) => {
                if start > 0 {
                    spans.push(Span::styled(remaining[..start].to_string(), normal_style));
                }
                let url_part = &remaining[start..];
                let url_end = url_part
                    .char_indices()
                    .find(|(i, c)| {
                        if *i < 8 {
                            return false;
                        }
                        c.is_whitespace()
                            || *c == '>'
                            || *c == ')'
                            || *c == ']'
                            || ('\u{4E00}'..='\u{9FFF}').contains(c)
                            || ('\u{3000}'..='\u{303F}').contains(c)
                            || ('\u{FF00}'..='\u{FFEF}').contains(c)
                            || matches!(
                                *c,
                                '，' | '。'
                                    | '；'
                                    | '：'
                                    | '！'
                                    | '？'
                                    | '、'
                                    | '\u{201C}'
                                    | '\u{201D}'
                                    | '\u{2018}'
                                    | '\u{2019}'
                            )
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(url_part.len());
                let url = url_part[..url_end].trim_end_matches(['.', ',', ';', ':', '!', '?']);
                let url_len = url.len();
                spans.push(Span::styled(url.to_string(), link_style));
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
