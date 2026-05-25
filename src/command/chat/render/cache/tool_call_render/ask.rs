//! Ask 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::render_kv_line;

/// Ask 工具展开渲染
pub(crate) fn render_ask_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(questions) = parsed.get("questions").and_then(|v| v.as_array()) {
        for (i, q) in questions.iter().enumerate() {
            let question_text = q.get("question").and_then(|v| v.as_str()).unwrap_or("?");
            let header = q
                .get("header")
                .and_then(|v| v.as_str())
                .unwrap_or("question");

            // 问题标签
            let label = if questions.len() > 1 {
                format!("Q{} [{}]", i + 1, header)
            } else {
                header.to_string()
            };
            render_kv_line(&label, question_text, content_w, lines, theme);

            // 选项预览
            if let Some(options) = q.get("options").and_then(|v| v.as_array()) {
                let opts_preview: Vec<String> = options
                    .iter()
                    .filter_map(|o| o.get("label").and_then(|l| l.as_str()).map(String::from))
                    .collect();
                if !opts_preview.is_empty() {
                    render_kv_line(
                        "options",
                        &opts_preview.join(" / "),
                        content_w,
                        lines,
                        theme,
                    );
                }
            }
        }
    }

    true
}
