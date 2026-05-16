//! 单行 Markdown 渲染子模块

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::util::text::char_width;

use super::MarkdownRenderer;

impl MarkdownRenderer {
    /// 渲染单行 Markdown（带行号）
    pub(super) fn render_single_line_with_number(
        &self,
        line: &str,
        line_idx: usize,
        max_width: usize,
    ) -> Line<'static> {
        let line_num = self.format_line_number(line_idx);

        // 应用水平滚动（使用迭代器避免 Vec<char> 分配）
        let visible_line: String = line.chars().skip(self.horizontal_scroll).collect();

        let trimmed = visible_line.trim_start();
        let indent_chars = visible_line.chars().count() - trimmed.chars().count();
        let indent_width: usize = visible_line
            .chars()
            .take(indent_chars)
            .map(char_width)
            .sum();
        let indent = " ".repeat(indent_width);

        // 标题
        if let Some(stripped) = trimmed.strip_prefix("# ") {
            let text = stripped.trim();
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("◆ ", self.style_bold(self.theme.md_h1)),
            ];
            // heading 内容应用 md_h1 颜色 + 加粗
            let heading_style = Style::default()
                .fg(self.theme.md_h1)
                .add_modifier(Modifier::BOLD);
            for span in rendered {
                spans.push(Span::styled(span.content, span.style.patch(heading_style)));
            }
            return Line::from(spans);
        }
        if let Some(stripped) = trimmed.strip_prefix("## ") {
            let text = stripped.trim();
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("◇ ", self.style_bold(self.theme.md_h2)),
            ];
            // heading 内容应用 md_h2 颜色 + 加粗
            let heading_style = Style::default()
                .fg(self.theme.md_h2)
                .add_modifier(Modifier::BOLD);
            for span in rendered {
                spans.push(Span::styled(span.content, span.style.patch(heading_style)));
            }
            return Line::from(spans);
        }
        if let Some(stripped) = trimmed.strip_prefix("### ") {
            let text = stripped.trim();
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("〈 ", self.style_bold(self.theme.md_h3)),
            ];
            // heading 内容应用 md_h3 颜色 + 加粗
            let heading_style = Style::default()
                .fg(self.theme.md_h3)
                .add_modifier(Modifier::BOLD);
            for span in rendered {
                spans.push(Span::styled(span.content, span.style.patch(heading_style)));
            }
            spans.push(Span::styled(" 〉", self.style_bold(self.theme.md_h3)));
            return Line::from(spans);
        }
        if let Some(stripped) = trimmed.strip_prefix("#### ") {
            let text = stripped.trim();
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("› ", self.style_bold(self.theme.md_h4)),
            ];
            // heading 内容应用 md_h4 颜色 + 加粗
            let heading_style = Style::default()
                .fg(self.theme.md_h4)
                .add_modifier(Modifier::BOLD);
            for span in rendered {
                spans.push(Span::styled(span.content, span.style.patch(heading_style)));
            }
            spans.push(Span::styled(" ", self.style(self.theme.text_normal)));
            return Line::from(spans);
        }

        // 水平线
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            let width = max_width.saturating_sub(indent_width).min(40);
            return Line::from(vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("─".repeat(width), self.style(self.theme.text_dim)),
            ]);
        }

        // 任务列表（必须在无序列表之前检查，因为 `- [ ]` 也以 `- ` 开头）
        if let Some(stripped) = trimmed.strip_prefix("- [ ]") {
            let text = stripped.trim();
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("○ ", self.style(self.theme.text_dim)),
            ];
            spans.extend(rendered);
            return Line::from(spans);
        }
        if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
            let text = trimmed[5..].trim();
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("● ", self.style(self.theme.md_list_bullet)),
            ];
            spans.extend(rendered);
            return Line::from(spans);
        }

        // 无序列表
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let text = &trimmed[2..];
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("• ", self.style(self.theme.text_normal)),
            ];
            spans.extend(rendered);
            return Line::from(spans);
        }

        // 有序列表
        if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit())
            && let Some(num_end) = rest.find(['.', ')'])
            && (rest.get(num_end..num_end + 2) == Some(". ")
                || rest.get(num_end..num_end + 2) == Some(") "))
        {
            let num_str = &trimmed[..rest.len() - rest.len() + num_end + 1];
            let text = &rest[num_end + 2..];
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled(
                    format!("{} ", num_str),
                    self.style_bold(self.theme.md_list_bullet),
                ),
            ];
            spans.extend(rendered);
            return Line::from(spans);
        }

        // 引用块（与 thinking block 风格一致：| 竖线 + bg_primary 背景）
        if trimmed.starts_with('>') {
            let mut level = 0;
            let mut rest = trimmed;
            while rest.starts_with('>') {
                level += 1;
                rest = rest[1..].trim_start();
            }
            let text = rest;

            // 背景色使用 bg_primary（与 thinking block 一致，融入主背景）
            let bg_color = self.theme.bg_primary;
            let bar_color = self.theme.md_blockquote_bar;
            let text_color = self.theme.md_blockquote_text;

            // 引用块竖线: 每级一个 | ，加粗显示
            let bar: String = (0..level).map(|_| "|").collect::<Vec<_>>().join("");
            let bar_style = Style::default()
                .fg(bar_color)
                .bg(bg_color)
                .add_modifier(Modifier::BOLD);

            // 引用块文字: 使用主题专用颜色 + bg_primary 背景
            let text_style = Style::default().fg(text_color).bg(bg_color);

            // 渲染行内元素，然后覆盖基础文字颜色和背景
            let rendered = self.render_inline(text);
            let styled_rendered: Vec<Span<'static>> = rendered
                .into_iter()
                .map(|span| {
                    // 保留行内代码等特殊样式，只覆盖基础文字颜色
                    let has_special_bg =
                        span.style.bg.is_some_and(|bg| bg != self.theme.bg_primary);
                    if has_special_bg {
                        span // 保留行内代码等自带背景的样式
                    } else {
                        Span::styled(
                            span.content,
                            span.style.fg.map_or(text_style, |fg| {
                                // 对于使用 text_normal 的普通文本，替换为 md_blockquote_text
                                if fg == self.theme.text_normal {
                                    text_style
                                } else {
                                    // 加粗/斜体等保留原前景色，只加 bg_primary 背景
                                    Style::default().fg(fg).bg(bg_color)
                                }
                            }),
                        )
                    }
                })
                .collect();

            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("  ", Style::default()),
                Span::styled(format!("{} ", bar), bar_style),
            ];
            spans.extend(styled_rendered);
            return Line::from(spans);
        }

        // 普通文本
        let rendered = self.render_inline(trimmed);
        let mut spans = vec![
            Span::styled(line_num, self.style(Color::DarkGray)),
            Span::styled(indent, self.style(self.theme.text_normal)),
        ];
        spans.extend(rendered);
        Line::from(spans)
    }
}
