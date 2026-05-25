//! WebSearch 工具结果渲染：结构化搜索结果

use crate::command::chat::render::theme::Theme;
use crate::util::text::wrap_text;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// WebSearch 结果：结构化搜索结果
///
/// 输入格式（由 format_search_results 生成）：
/// ```text
/// 搜索: query
///
/// 1. Title
///    URL
///    highlight text
///
/// 2. Title
///    URL
/// ```
pub(crate) fn render_web_search_result(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let mut result_count = 0usize;
    let mut iter = content.lines().peekable();

    // 跳过 "搜索: query" 首行
    if iter
        .peek()
        .is_some_and(|l| l.starts_with("搜索:") || l.starts_with("搜索："))
    {
        iter.next();
    }

    while let Some(line) = iter.next() {
        if line.is_empty() {
            continue;
        }

        // 检测序号行："1. Title"
        if let Some(rest) = line
            .trim_start()
            .strip_prefix(|c: char| c.is_ascii_digit())
            .and_then(|r| r.strip_prefix(". "))
        {
            result_count += 1;
            // 标题行
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    format!("{}. ", result_count),
                    Style::default()
                        .fg(theme.config_title)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    rest.to_string(),
                    Style::default()
                        .fg(theme.text_normal)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            // 后续行：URL + 摘要
            let mut sub_lines = 0usize;
            while let Some(next) = iter.peek() {
                if next.is_empty()
                    || next
                        .trim_start()
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_digit() && next.contains(". "))
                {
                    break;
                }
                let sub_line = iter.next().unwrap_or("");
                sub_lines += 1;
                if sub_lines == 1 {
                    // URL 行
                    lines.push(Line::from(Span::styled(
                        format!("       {}", sub_line),
                        Style::default().fg(theme.text_dim),
                    )));
                } else {
                    // 摘要行
                    for wrapped in wrap_text(sub_line, content_w.saturating_sub(7)) {
                        lines.push(Line::from(Span::styled(
                            format!("       {}", wrapped),
                            Style::default().fg(theme.text_dim),
                        )));
                    }
                }
            }
            // 结果之间加空行
            lines.push(Line::from(""));
            continue;
        }

        // 非序号行，普通折行
        for wrapped in wrap_text(line, content_w.saturating_sub(4)) {
            lines.push(Line::from(Span::styled(
                format!("    {}", wrapped),
                Style::default().fg(theme.text_dim),
            )));
        }
    }

    // 统计行
    if result_count > 1 {
        lines.push(Line::from(Span::styled(
            format!("    (共 {} 个结果)", result_count),
            Style::default().fg(theme.text_dim),
        )));
    }
}
