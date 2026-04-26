//! 行内 Markdown 元素渲染子模块

use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

use super::MarkdownRenderer;

impl MarkdownRenderer {
    /// 渲染行内元素
    pub(super) fn render_inline(&self, text: &str) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let mut remaining = text;

        while !remaining.is_empty() {
            let code_pos = remaining.find('`');
            let img_pos = remaining.find("![");
            let bold_pos = remaining.find("**");
            let strike_pos = remaining.find("~~");
            let italic_pos = remaining.find('*');
            let link_pos = remaining.find('[');

            let min_pos = [
                code_pos, img_pos, bold_pos, strike_pos, italic_pos, link_pos,
            ]
            .iter()
            .filter_map(|&p| p)
            .min();

            let Some(pos) = min_pos else {
                spans.push(Span::styled(
                    remaining.to_string(),
                    self.style(self.theme.text_normal),
                ));
                break;
            };

            let is_img = img_pos == Some(pos);
            let is_code = code_pos == Some(pos) && !is_img;
            let is_bold = bold_pos == Some(pos);
            let is_strike = strike_pos == Some(pos);
            let is_link = link_pos == Some(pos) && !is_img;
            let is_italic = italic_pos == Some(pos) && !is_bold && !is_img;

            if pos > 0 {
                spans.push(Span::styled(
                    remaining[..pos].to_string(),
                    self.style(self.theme.text_normal),
                ));
            }

            remaining = &remaining[pos..];

            // 行内代码
            if is_code {
                remaining = &remaining[1..];
                if let Some(end) = remaining.find('`') {
                    spans.push(Span::styled(
                        remaining[..end].to_string(),
                        Style::default()
                            .fg(self.theme.md_inline_code_fg)
                            .bg(self.theme.md_inline_code_bg),
                    ));
                    remaining = &remaining[end + 1..];
                } else {
                    spans.push(Span::styled(
                        format!("`{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 图片
            else if is_img {
                remaining = &remaining[2..];
                if let Some(alt_end) = remaining.find("](") {
                    let alt = &remaining[..alt_end];
                    remaining = &remaining[alt_end + 2..];
                    if let Some(url_end) = remaining.find(')') {
                        spans.push(Span::styled(
                            format!("🖼 {}", alt),
                            Style::default()
                                .fg(self.theme.text_dim)
                                .add_modifier(Modifier::ITALIC),
                        ));
                        remaining = &remaining[url_end + 1..];
                    } else {
                        spans.push(Span::styled(
                            format!("![{}{}", alt, remaining),
                            self.style(self.theme.text_normal),
                        ));
                        break;
                    }
                } else {
                    spans.push(Span::styled(
                        format!("![{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 粗体
            else if is_bold {
                remaining = &remaining[2..];
                if let Some(end) = remaining.find("**") {
                    let inner = &remaining[..end];
                    let inner_spans = self.render_inline(inner);
                    for span in inner_spans {
                        spans.push(Span::styled(
                            span.content,
                            span.style.add_modifier(Modifier::BOLD),
                        ));
                    }
                    remaining = &remaining[end + 2..];
                } else {
                    spans.push(Span::styled(
                        format!("**{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 删除线
            else if is_strike {
                remaining = &remaining[2..];
                if let Some(end) = remaining.find("~~") {
                    let inner = &remaining[..end];
                    let inner_spans = self.render_inline(inner);
                    for span in inner_spans {
                        spans.push(Span::styled(
                            span.content,
                            span.style.add_modifier(Modifier::CROSSED_OUT),
                        ));
                    }
                    remaining = &remaining[end + 2..];
                } else {
                    spans.push(Span::styled(
                        format!("~~{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 斜体
            else if is_italic {
                remaining = &remaining[1..];
                if let Some(end) = remaining.find('*') {
                    let inner = &remaining[..end];
                    let inner_spans = self.render_inline(inner);
                    for span in inner_spans {
                        spans.push(Span::styled(
                            span.content,
                            span.style.add_modifier(Modifier::ITALIC),
                        ));
                    }
                    remaining = &remaining[end + 1..];
                } else {
                    spans.push(Span::styled(
                        format!("*{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 链接
            else if is_link {
                remaining = &remaining[1..];
                if let Some(text_end) = remaining.find("](") {
                    let link_text = &remaining[..text_end];
                    remaining = &remaining[text_end + 2..];
                    if let Some(url_end) = remaining.find(')') {
                        spans.push(Span::styled(
                            link_text.to_string(),
                            self.style(self.theme.md_link)
                                .add_modifier(Modifier::UNDERLINED),
                        ));
                        spans.push(Span::styled(
                            " ↗".to_string(),
                            self.style(self.theme.text_dim),
                        ));
                        remaining = &remaining[url_end + 1..];
                    } else {
                        spans.push(Span::styled(
                            format!("[{}{}", link_text, remaining),
                            self.style(self.theme.text_normal),
                        ));
                        break;
                    }
                } else {
                    spans.push(Span::styled(
                        format!("[{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
        }

        spans
    }
}
