//! Todo 工具结果渲染

use crate::command::chat::render::cache::ContentContext;
use crate::util::text::wrap_text;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// 渲染 TodoRead/TodoWrite 工具结果（实心点/空心点样式）
/// expand=true 时额外显示完成/未完成条数统计
pub(crate) fn render_todo_result(content: &str, ctx: &mut ContentContext<'_>) {
    let lines = &mut *ctx.lines;
    let theme = ctx.theme;
    let content_w = ctx.content_w;
    let expand = ctx.expand;
    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(content) {
        // 展开模式：先显示统计信息
        if expand {
            let total = items.len();
            let completed = items
                .iter()
                .filter(|i| i.get("status").and_then(|s| s.as_str()) == Some("completed"))
                .count();
            let pending = total.saturating_sub(completed);

            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    format!("完成 {} / 未完成 {}", completed, pending),
                    Style::default().fg(theme.text_dim),
                ),
            ]));
            lines.push(Line::from(""));
        }

        // 列出每个 todo 项
        for item in &items {
            let status = item
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("pending");
            let text = item
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("(empty)");

            // 实心点 ● 表示已完成/进行中，空心点 ○ 表示未开始
            let (dot, color) = match status {
                "completed" => ("●", theme.label_ai),        // 绿色实心点
                "in_progress" => ("◉", theme.title_loading), // 黄色双圈实心点
                "cancelled" => ("◌", theme.text_dim),        // 灰色空心虚圈
                _ => ("○", Color::Yellow),                   // pending: 黄色空心点
            };

            let text_style = if status == "completed" {
                Style::default()
                    .fg(theme.text_dim)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else {
                Style::default().fg(theme.text_white)
            };
            let max_w = content_w.saturating_sub(10); // "    ● " prefix
            for (i, wrapped) in wrap_text(text, max_w).iter().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(dot, Style::default().fg(color)),
                        Span::styled(" ", Style::default()),
                        Span::styled(wrapped.clone(), text_style),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("      ", Style::default()),
                        Span::styled(wrapped.clone(), text_style),
                    ]));
                }
            }
        }
    } else {
        // 非 JSON，回退到普通显示
        let all_lines: Vec<&str> = content.lines().take(100).collect();
        for line in all_lines {
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_dim),
                )));
            }
        }
    }
}
