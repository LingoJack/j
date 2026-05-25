//! Bash/Shell 工具调用渲染

use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::command::chat::render::theme::Theme;

/// Bash 工具参数结构
pub(crate) struct BashArgs {
    pub command: Option<String>,
    pub timeout: Option<u64>,
    pub run_in_background: bool,
    pub cwd: Option<String>,
}

/// 从 Bash 工具的 arguments JSON 中提取参数
pub(crate) fn extract_bash_args(arguments: &str) -> Option<BashArgs> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;

    Some(BashArgs {
        command: parsed
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        timeout: parsed.get("timeout").and_then(|v| v.as_u64()),
        run_in_background: parsed
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        cwd: parsed
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}

/// 渲染 Bash 工具调用请求的展开模式
pub(crate) fn render_bash_call_request_expanded(
    args: &BashArgs,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let content_w = bubble_max_width.saturating_sub(6);

    // 渲染命令行（$ 前缀）
    if let Some(ref cmd) = args.command {
        let cmd_with_prefix = format!("$ {}", cmd);
        for line in crate::util::text::wrap_text(&cmd_with_prefix, content_w) {
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(line, Style::default().fg(theme.text_normal)),
            ]));
        }
    }

    // 渲染附加信息行（后台运行、超时、工作目录）
    let mut meta_parts: Vec<String> = Vec::new();

    if args.run_in_background {
        meta_parts.push("[background]".to_string());
    }

    if let Some(timeout) = args.timeout {
        meta_parts.push(format!("timeout: {}s", timeout));
    }

    if let Some(ref cwd) = args.cwd {
        meta_parts.push(format!("cwd: {}", cwd));
    }

    if !meta_parts.is_empty() {
        let meta_line = meta_parts.join("  ");
        for line in crate::util::text::wrap_text(&meta_line, content_w) {
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(line, Style::default().fg(theme.text_dim)),
            ]));
        }
    }
}
