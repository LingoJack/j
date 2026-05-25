//! EnterPlanMode 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::{render_kv_line, render_tag_line};

/// EnterPlanMode 工具展开渲染
pub(crate) fn render_enter_plan_mode_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    render_tag_line("进入计划模式（只读模式）", content_w, lines, theme);

    if let Some(desc) = parsed.get("description").and_then(|v| v.as_str()) {
        render_kv_line("plan", desc, content_w, lines, theme);
    }

    true
}
