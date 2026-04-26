//! 光标视觉行渲染子模块

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::util::text::display_width;

use crate::tui::editor_core::search::SearchState;
use crate::tui::editor_core::wrap_engine::VisualLine;

use super::MarkdownRenderer;

/// 光标视觉行渲染所需的上下文参数
pub(super) struct CursorLineContext<'a> {
    pub(super) line_num_str: &'a str,
    pub(super) line_num_style: Style,
    pub(super) cursor_col: Option<usize>,
    pub(super) search: &'a SearchState,
    pub(super) code_block_max_width: Option<usize>,
    pub(super) is_last_vl: bool,
}

impl MarkdownRenderer {
    /// 渲染光标行的视觉行（源码 + 光标高亮）
    pub(super) fn render_cursor_visual_line(
        &self,
        text: String,
        vl: &VisualLine,
        ctx: &CursorLineContext<'_>,
    ) -> Line<'static> {
        let in_code_block = ctx.code_block_max_width.is_some();

        // 代码块内的光标行：文本用 code_bg，行号保持 bg_input
        let (text_style, line_num_bg) = if in_code_block {
            (
                Style::default()
                    .fg(self.theme.text_normal)
                    .bg(self.theme.code_bg),
                self.theme.bg_input,
            )
        } else {
            (
                self.style_input(self.theme.text_normal),
                self.theme.bg_input,
            )
        };

        let effective_line_num_style = ctx.line_num_style.bg(line_num_bg);
        let mut spans = vec![Span::styled(
            ctx.line_num_str.to_string(),
            effective_line_num_style,
        )];

        // 代码块光标行：添加左边框
        if in_code_block {
            spans.push(Span::styled("│", self.style_code(self.theme.text_dim)));
            spans.push(Span::styled(" ", Style::default().bg(self.theme.code_bg)));
        }

        // 计算显示宽度（在 text 被消费之前）
        let text_display_width = display_width(&text);

        // 搜索高亮 + 光标叠加：当光标行同时启用搜索时，
        // 使用搜索高亮作为基础，再在光标位置叠加光标块
        if ctx.search.is_searching() && ctx.search.match_count() > 0 {
            let highlight_spans =
                ctx.search
                    .highlight_line(vl.logical_line, &text, &self.theme, vl.start_col);
            let cursor_style = Style::default()
                .fg(self.theme.cursor_fg)
                .bg(self.theme.cursor_bg)
                .add_modifier(Modifier::BOLD);

            if let Some(col) = ctx.cursor_col {
                let cursor_in_this_vl = if col == vl.end_col {
                    ctx.is_last_vl
                } else {
                    col >= vl.start_col && col < vl.end_col
                };
                if cursor_in_this_vl {
                    let char_idx_at_cursor = col.saturating_sub(vl.start_col);
                    spans.extend(Self::overlay_cursor_on_spans(
                        highlight_spans,
                        char_idx_at_cursor,
                        cursor_style,
                    ));
                } else {
                    spans.extend(highlight_spans);
                }
            } else {
                spans.extend(highlight_spans);
            }
            return Line::from(spans).patch_style(Style::default().bg(line_num_bg));
        }

        // 处理光标位置
        if let Some(col) = ctx.cursor_col {
            // 判断光标是否在当前视觉行范围内
            // 当 col == vl.end_col 时：
            //   - 如果是最后一个视觉行，光标在行尾，属于当前视觉行
            //   - 如果不是最后一个视觉行，光标属于下一个视觉行（end_col == next start_col）
            let cursor_in_this_vl = if col == vl.end_col {
                ctx.is_last_vl
            } else {
                col >= vl.start_col && col < vl.end_col
            };

            if cursor_in_this_vl {
                // 光标在当前视觉行内
                let chars: Vec<char> = text.chars().collect();
                let char_idx_at_cursor = col.saturating_sub(vl.start_col);

                if char_idx_at_cursor > 0 {
                    let before: String = chars.iter().take(char_idx_at_cursor).collect();
                    spans.push(Span::styled(before, text_style));
                }

                let cursor_style = Style::default()
                    .fg(self.theme.cursor_fg)
                    .bg(self.theme.cursor_bg)
                    .add_modifier(Modifier::BOLD);

                if char_idx_at_cursor < chars.len() {
                    spans.push(Span::styled(
                        chars[char_idx_at_cursor].to_string(),
                        cursor_style,
                    ));
                    if char_idx_at_cursor + 1 < chars.len() {
                        let after: String = chars.iter().skip(char_idx_at_cursor + 1).collect();
                        spans.push(Span::styled(after, text_style));
                    }
                } else {
                    // 光标在行尾，用空格显示背景色，与字上光标一致
                    spans.push(Span::styled(" ", cursor_style));
                }
            } else {
                // 光标不在当前视觉行，正常渲染文本
                spans.push(Span::styled(text, text_style));
            }
        } else {
            // 无光标信息（不应该发生，但作为 fallback）
            spans.push(Span::styled(text, text_style));
        }

        // 代码块光标行：添加右边框 + 填充
        if let Some(max_width) = ctx.code_block_max_width {
            let fill_width = max_width.saturating_sub(text_display_width);
            spans.push(Span::styled(
                " ".repeat(fill_width),
                Style::default().bg(self.theme.code_bg),
            ));
            spans.push(Span::styled(" ", Style::default().bg(self.theme.code_bg)));
            spans.push(Span::styled("│", self.style_code(self.theme.text_dim)));
        }

        Line::from(spans)
    }
}
