use super::ParserState;
use crate::util::text::{display_width, wrap_text};
use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

// ---------------------------------------------------------------------------
// ParserState text-related methods
// ---------------------------------------------------------------------------

impl<'a> ParserState<'a> {
    /// Handle `Event::Text(text)` — the most complex event with URL splitting and auto-wrapping.
    pub(crate) fn handle_text_event(&mut self, text: &str) {
        // 图片 alt 累积
        if self.image_url.is_some() {
            self.image_alt.push_str(text);
            return;
        }
        // 代码块内容累积
        if self.in_code_block {
            self.code_block_content.push_str(text);
            return;
        }
        // 表格单元格内容累积
        if self.in_table {
            self.current_cell.push_str(text);
            return;
        }

        // 普通文本：URL 拆分 + 自动换行
        let style = *self.style_stack.last().unwrap_or(&self.base_style);
        let text_str = text.replace('\u{200B}', "");

        let effective_prefix_w = if self.in_blockquote { 2 } else { 0 };
        let full_line_w = self.content_width.saturating_sub(effective_prefix_w);

        let existing_w: usize = self
            .current_spans
            .iter()
            .map(|s| display_width(&s.content))
            .sum();
        // existing_w 包含了 prefix span 的宽度，但 full_line_w 已排除了 prefix 空间，需扣除避免双重计算
        let content_w_on_line = existing_w.saturating_sub(effective_prefix_w);
        let wrap_w = full_line_w.saturating_sub(content_w_on_line);

        let min_useful_w = full_line_w / 4;
        let wrap_w = if wrap_w < min_useful_w.max(4) && !self.current_spans.is_empty() {
            self.flush_line();
            if self.in_blockquote {
                self.current_spans.push(Span::styled(
                    "| ".to_string(),
                    Style::default()
                        .fg(self.theme.md_blockquote_bar())
                        .bg(self.theme.md_blockquote_bg())
                        .add_modifier(Modifier::BOLD),
                ));
            }
            full_line_w
        } else {
            wrap_w
        };

        let link_style = Style::default()
            .fg(self.theme.md_link())
            .add_modifier(Modifier::UNDERLINED);
        let in_link = self.link_url.is_some();

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
        // cur_line_w 只追踪内容宽度（不含 prefix），因为 full_line_w 已排除 prefix
        let mut cur_line_w = content_w_on_line;
        let mut first_seg = true;
        for seg in &segments {
            if seg.content.as_ref() == "\n" {
                self.flush_line();
                if self.in_blockquote {
                    self.current_spans.push(Span::styled(
                        "| ".to_string(),
                        Style::default()
                            .fg(self.theme.md_blockquote_bar())
                            .bg(self.theme.md_blockquote_bg())
                            .add_modifier(Modifier::BOLD),
                    ));
                    cur_line_w = 0;
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
                self.current_spans.push(Span::styled(seg_text, seg_style));
                cur_line_w += seg_w;
            } else {
                // 需要 wrap 这个片段
                let first_wrap_w = avail;
                let first_wrapped = wrap_text(&seg_text, first_wrap_w.max(1));
                // 第一段放入当前行
                self.current_spans
                    .push(Span::styled(first_wrapped[0].clone(), seg_style));
                if first_wrapped.len() > 1 {
                    let rest: String = first_wrapped[1..].join("");
                    self.flush_line();
                    if self.in_blockquote {
                        self.current_spans.push(Span::styled(
                            "| ".to_string(),
                            Style::default()
                                .fg(self.theme.md_blockquote_bar())
                                .bg(self.theme.md_blockquote_bg())
                                .add_modifier(Modifier::BOLD),
                        ));
                    }
                    let rest_wrapped = wrap_text(&rest, full_line_w.max(1));
                    for (j, wl) in rest_wrapped.iter().enumerate() {
                        if j > 0 {
                            self.flush_line();
                            if self.in_blockquote {
                                self.current_spans.push(Span::styled(
                                    "| ".to_string(),
                                    Style::default()
                                        .fg(self.theme.md_blockquote_bar())
                                        .bg(self.theme.md_blockquote_bg())
                                        .add_modifier(Modifier::BOLD),
                                ));
                            }
                        }
                        self.current_spans.push(Span::styled(wl.clone(), seg_style));
                    }
                    cur_line_w = display_width(rest_wrapped.last().unwrap_or(&String::new()));
                } else {
                    cur_line_w = display_width(&first_wrapped[0]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Private helper: URL splitting
// ---------------------------------------------------------------------------

/// 将文本拆分为普通文本和 URL 片段，对 URL 应用链接样式
pub(crate) fn split_text_with_urls<'a>(
    text: &str,
    normal_style: Style,
    link_style: Style,
) -> Vec<Span<'a>> {
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
                            || matches!(
                                *c,
                                '，' | '。' | '；'
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
