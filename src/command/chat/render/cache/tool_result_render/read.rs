//! Read 工具结果渲染：带行号的代码，支持语法高亮

use crate::command::chat::constants::NORMAL_RESULT_MAX_LINES;
use crate::command::chat::render::cache::TOOL_RESULT_DISPLAY_MAX_LINES;
use crate::command::chat::render::theme::Theme;
use crate::markdown::highlight::highlight_code_line;
use crate::tui::editor_core::EditorTheme;
use crate::util::text::{line_number_continuation_prefix, wrap_text, wrap_text_with_prefix};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// Read 工具结果渲染：带行号的代码，支持语法高亮
pub(crate) fn render_read_result(
    content: &str,
    tool_args: Option<&str>,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    use super::shared::{infer_lang_from_path, parse_file_path_from_json};

    let lang = tool_args
        .and_then(parse_file_path_from_json)
        .map(|p| infer_lang_from_path(&p))
        .unwrap_or("");
    let editor_theme = EditorTheme::from(theme);

    let all_lines: Vec<&str> = content.lines().take(NORMAL_RESULT_MAX_LINES).collect();
    for line in all_lines {
        if let Some((prefix_w, cont_prefix)) = line_number_continuation_prefix(line) {
            // 分离行号前缀和代码内容
            let prefix_str: String = line.chars().take(prefix_w).collect();
            let code_content: String = line.chars().skip(prefix_w).collect();
            // 续行
            for (i, wrapped) in wrap_text_with_prefix(&code_content, content_w, &cont_prefix)
                .into_iter()
                .enumerate()
            {
                let mut spans = vec![Span::styled("    ", Style::default().fg(theme.text_dim))];
                if i == 0 {
                    // 首行：行号前缀
                    spans.push(Span::styled(
                        prefix_str.clone(),
                        Style::default().fg(theme.text_dim),
                    ));
                } else {
                    // 续行：缩进对齐
                    spans.push(Span::styled(
                        cont_prefix.clone(),
                        Style::default().fg(theme.text_dim),
                    ));
                }
                if lang.is_empty() {
                    spans.push(Span::styled(wrapped, Style::default().fg(theme.text_dim)));
                } else {
                    spans.extend(highlight_code_line(&wrapped, lang, &editor_theme));
                }
                lines.push(Line::from(spans));
            }
        } else {
            // 无行号前缀的行（如空行）
            for wrapped in wrap_text(line, content_w) {
                if lang.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", wrapped),
                        Style::default().fg(theme.text_dim),
                    )));
                } else {
                    let mut spans = vec![Span::styled("    ", Style::default().fg(theme.text_dim))];
                    spans.extend(highlight_code_line(&wrapped, lang, &editor_theme));
                    lines.push(Line::from(spans));
                }
            }
        }
    }

    let total_lines = content.lines().count();
    if total_lines > TOOL_RESULT_DISPLAY_MAX_LINES {
        lines.push(Line::from(Span::styled(
            format!(
                "    ... (共 {} 行，显示前 {} 行)",
                total_lines, TOOL_RESULT_DISPLAY_MAX_LINES
            ),
            Style::default().fg(theme.text_dim),
        )));
    }
}
