//! LoadSkill 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::{render_kv_line, render_tag_line};

/// LoadSkill 工具展开渲染
pub(crate) fn render_load_skill_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let name = parsed
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    render_tag_line(&format!("加载技能: {}", name), content_w, lines, theme);

    if let Some(args) = parsed.get("arguments").and_then(|v| v.as_str()) {
        render_kv_line("arguments", args, content_w, lines, theme);
    }

    true
}
