//! EnterWorktree / ExitWorktree 工具调用渲染

use crate::command::chat::render::theme::Theme;
use crate::command::chat::tools::tool_names;
use ratatui::text::Line;

use super::shared::{render_kv_line, render_tag_line};

/// EnterWorktree / ExitWorktree 工具展开渲染
pub(crate) fn render_worktree_call_request_expanded(
    tool_name: &str,
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if tool_name == tool_names::ENTER_WORKTREE {
        render_tag_line("进入隔离工作树", content_w, lines, theme);
        if let Some(name) = parsed.get("name").and_then(|v| v.as_str()) {
            render_kv_line("name", name, content_w, lines, theme);
        }
    } else {
        let action = parsed
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("keep");
        render_tag_line("退出工作树", content_w, lines, theme);
        render_kv_line("action", action, content_w, lines, theme);
    }

    true
}
