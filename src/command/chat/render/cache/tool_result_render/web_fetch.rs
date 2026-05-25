//! WebFetch 工具结果渲染：内容预览

use crate::command::chat::constants::NORMAL_RESULT_MAX_LINES;
use crate::command::chat::render::theme::Theme;
use crate::util::text::wrap_text;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// WebFetch 结果：内容预览
///
/// 自动检测是否为 Markdown 内容：
/// - 包含 # 标题、- 列表等标记时用 markdown_to_lines 渲染
/// - 否则纯文本折行显示
pub(crate) fn render_web_fetch_result(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    // 检测是否为 Markdown 内容
    let has_markdown = content.lines().take(20).any(|l| {
        l.starts_with("# ")
            || l.starts_with("## ")
            || l.starts_with("- ")
            || l.starts_with("* ")
            || l.starts_with("> ")
            || l.starts_with("```")
            || l.starts_with("| ")
            || l.starts_with("1. ")
    });

    if has_markdown {
        // 用 IR 渲染器渲染 Markdown
        use crate::markdown::parser::markdown_to_lines;
        let md_lines = markdown_to_lines(content, content_w, theme);
        for md_line in md_lines {
            // 添加 4 空格缩进
            let mut spans = vec![Span::styled("    ", Style::default())];
            spans.extend(md_line.spans);
            lines.push(Line::from(spans));
        }
    } else {
        // 纯文本折行
        let all_lines: Vec<&str> = content.lines().take(NORMAL_RESULT_MAX_LINES).collect();
        for line in all_lines {
            for wrapped in wrap_text(line, content_w.saturating_sub(4)) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_normal),
                )));
            }
        }
        let total = content.lines().count();
        if total > NORMAL_RESULT_MAX_LINES {
            lines.push(Line::from(Span::styled(
                format!(
                    "    ... (共 {} 行，显示前 {} 行)",
                    total, NORMAL_RESULT_MAX_LINES
                ),
                Style::default().fg(theme.text_dim),
            )));
        }
    }
}
