//! TaskOutput 工具调用渲染

use crate::command::chat::render::theme::Theme;
use ratatui::text::Line;

use super::shared::render_kv_line;

/// TaskOutput 工具展开渲染
pub(crate) fn render_task_output_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(task_id) = parsed.get("task_id").and_then(|v| v.as_str()) {
        render_kv_line("task_id", task_id, content_w, lines, theme);
    }

    // block
    if let Some(block) = parsed.get("block").and_then(|v| v.as_bool()) {
        render_kv_line(
            "block",
            if block {
                "true (等待完成)"
            } else {
                "false (非阻塞)"
            },
            content_w,
            lines,
            theme,
        );
    }

    // timeout
    if let Some(timeout) = parsed.get("timeout").and_then(|v| v.as_u64()) {
        render_kv_line(
            "timeout",
            &format!("{}ms", timeout),
            content_w,
            lines,
            theme,
        );
    }

    true
}
