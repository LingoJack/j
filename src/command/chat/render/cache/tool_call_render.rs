//! 工具调用请求渲染：展开/折叠模式、各类工具专用渲染

use crate::command::chat::constants::{AGENT_CALL_PROMPT_MAX_LINES, TOOL_ARG_PREVIEW_MAX_CHARS};
use crate::command::chat::storage::ToolCallItem;
use crate::command::chat::tools::classification::{ToolCategory, format_json_value};
use crate::command::chat::tools::tool_names;
use crate::util::text::wrap_text;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::RenderContext;
use super::bubble::bordered_line;
use super::msg_render::agent_name_color;
use crate::command::chat::render::theme::Theme;

// ──────────────────────────────────────────────────────────────
// 1. render_tool_call_request_msg (pub fn)
// ──────────────────────────────────────────────────────────────

pub fn render_tool_call_request_msg(
    sender_name: Option<&str>,
    tool_calls: &[ToolCallItem],
    ctx: &mut RenderContext<'_>,
) {
    let lines = &mut *ctx.lines;
    let theme = ctx.theme;
    let bubble_max_width = ctx.bubble_max_width;
    let expand = ctx.expand;
    let content_w = bubble_max_width.saturating_sub(6);

    // 与前一条消息之间留一行间距
    lines.push(Line::from(""));

    for (i, tc) in tool_calls.iter().enumerate() {
        // 多个 tool_call 之间留一行间距
        if i > 0 {
            lines.push(Line::from(""));
        }
        let category = ToolCategory::from_name(&tc.name);
        let icon = category.icon();
        let tool_color = category.color(theme);

        // 构建首行前缀：有 sender_name 时 "  name · "，否则 "  "
        let sender_prefix_spans: Vec<Span<'static>> = if let Some(name) = sender_name {
            let label_color = agent_name_color(name);
            vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    name.to_string(),
                    Style::default()
                        .fg(label_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" · ", Style::default().fg(theme.text_dim)),
            ]
        } else {
            vec![Span::styled("  ", Style::default())]
        };

        if expand {
            // 展开模式：图标 + 工具名 + description（若有）+ 状态（第一行）
            let tool_desc = extract_tool_description_from_args(&tc.name, &tc.arguments);
            let display_name = if let Some(ref desc) = tool_desc {
                format!("{} - {}", tc.name, desc)
            } else {
                tc.name.clone()
            };
            let mut spans = sender_prefix_spans.clone();
            spans.push(Span::styled(icon, Style::default().fg(tool_color)));
            spans.push(Span::styled(" ", Style::default()));
            spans.push(Span::styled(
                display_name,
                Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::from(spans));

            // 参数详情
            if !tc.arguments.is_empty() {
                if matches!(tc.name.as_str(), tool_names::BASH) {
                    // Bash/Shell 工具使用专用渲染：显示命令 + 附加信息
                    if let Some(bash_args) = extract_bash_args(&tc.arguments) {
                        render_bash_call_request_expanded(
                            &bash_args,
                            bubble_max_width,
                            lines,
                            theme,
                        );
                    } else if let Ok(json_value) =
                        serde_json::from_str::<serde_json::Value>(&tc.arguments)
                    {
                        render_json_params_enhanced(&json_value, content_w, lines, theme);
                    }
                // Agent 工具使用专用渲染：边框 + prompt + 元信息
                } else if tc.name.as_str() == tool_names::AGENT {
                    if let Some(agent_args) = extract_agent_args(&tc.arguments) {
                        render_agent_call_request_expanded(
                            &agent_args,
                            bubble_max_width,
                            lines,
                            theme,
                        );
                    }
                // Teammate 工具使用专用渲染：边框 + name/role + prompt
                } else if tc.name.as_str() == tool_names::TEAMMATE {
                    if let Some(tm_args) = extract_teammate_args(&tc.arguments) {
                        render_teammate_call_request_expanded(
                            &tm_args,
                            bubble_max_width,
                            lines,
                            theme,
                        );
                    }
                // ExitPlanMode 工具使用专用渲染：边框显示
                } else if matches!(tc.name.as_str(), tool_names::EXIT_PLAN_MODE) {
                    render_exit_plan_mode_request(bubble_max_width, lines, theme);
                } else if let Ok(json_value) =
                    serde_json::from_str::<serde_json::Value>(&tc.arguments)
                {
                    render_json_params_enhanced(&json_value, content_w, lines, theme);
                } else {
                    // 非 JSON 参数，普通折行显示
                    for line in wrap_text(&tc.arguments, content_w) {
                        lines.push(Line::from(vec![
                            Span::styled("    ", Style::default()),
                            Span::styled(line, Style::default().fg(theme.text_dim)),
                        ]));
                    }
                }
            }
        } else {
            // 折叠模式：图标 + 工具名 + description（若有）或参数预览

            // Agent 工具专用折叠渲染：显示 [background] + description
            if tc.name.as_str() == tool_names::AGENT
                && let Some(agent_args) = extract_agent_args(&tc.arguments)
            {
                let mut desc_parts: Vec<String> = Vec::new();
                if agent_args.run_in_background {
                    desc_parts.push("[background]".to_string());
                }
                if let Some(ref desc) = agent_args.description {
                    desc_parts.push(desc.clone());
                }
                if desc_parts.is_empty() {
                    let first_line = agent_args.prompt.lines().next().unwrap_or("");
                    let cw: String = first_line
                        .chars()
                        .take(TOOL_ARG_PREVIEW_MAX_CHARS)
                        .collect();
                    let preview = if first_line.chars().count() > TOOL_ARG_PREVIEW_MAX_CHARS {
                        format!("{}...", cw)
                    } else {
                        cw
                    };
                    desc_parts.push(preview);
                }
                let desc_text = desc_parts.join("  ");
                let mut spans = sender_prefix_spans.clone();
                spans.push(Span::styled(icon, Style::default().fg(tool_color)));
                spans.push(Span::styled(" ", Style::default()));
                spans.push(Span::styled(
                    tc.name.clone(),
                    Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    format!("  {}", desc_text),
                    Style::default().fg(theme.text_dim),
                ));
                lines.push(Line::from(spans));
                continue;
            }

            // Teammate 工具专用折叠渲染：显示 name(role) + prompt 预览
            if tc.name.as_str() == tool_names::TEAMMATE
                && let Some(tm_args) = extract_teammate_args(&tc.arguments)
            {
                let mut desc_parts: Vec<String> = Vec::new();
                if tm_args.worktree {
                    desc_parts.push("[worktree]".to_string());
                }
                desc_parts.push(format!("{}({})", tm_args.name, tm_args.role));
                let first_line = tm_args.prompt.lines().next().unwrap_or("");
                let cw: String = first_line
                    .chars()
                    .take(TOOL_ARG_PREVIEW_MAX_CHARS)
                    .collect();
                let preview = if first_line.chars().count() > TOOL_ARG_PREVIEW_MAX_CHARS {
                    format!("{}...", cw)
                } else {
                    cw
                };
                desc_parts.push(preview);
                let desc_text = desc_parts.join("  ");
                let mut spans = sender_prefix_spans.clone();
                spans.push(Span::styled(icon, Style::default().fg(tool_color)));
                spans.push(Span::styled(" ", Style::default()));
                spans.push(Span::styled(
                    tc.name.clone(),
                    Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    format!("  {}", desc_text),
                    Style::default().fg(theme.text_dim),
                ));
                lines.push(Line::from(spans));
                continue;
            }

            let tool_desc = extract_tool_description_from_args(&tc.name, &tc.arguments);

            if let Some(desc) = tool_desc {
                // 有 description 时优先展示，替代 raw arguments
                let mut spans = sender_prefix_spans.clone();
                spans.push(Span::styled(icon, Style::default().fg(tool_color)));
                spans.push(Span::styled(" ", Style::default()));
                spans.push(Span::styled(
                    tc.name.clone(),
                    Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    format!("  {}", desc),
                    Style::default().fg(theme.text_dim),
                ));
                lines.push(Line::from(spans));
            } else {
                // 无 description，保留原有的参数预览逻辑
                let total_len = tc.arguments.chars().count();
                let truncated = total_len > TOOL_ARG_PREVIEW_MAX_CHARS;

                // 检测 JSON 开括号类型，用于截断时添加闭合括号
                let closing_bracket = if truncated {
                    tc.arguments.chars().next().and_then(|c| match c {
                        '{' => Some('}'),
                        '[' => Some(']'),
                        _ => None,
                    })
                } else {
                    None
                };

                // 如果需要闭合括号，预留 4 字符给 "...}" 或 "...]"
                let max_preview = TOOL_ARG_PREVIEW_MAX_CHARS;
                let preview_len = if closing_bracket.is_some() {
                    max_preview - 4
                } else {
                    max_preview
                };

                let args_preview: String = tc.arguments.chars().take(preview_len).collect();

                let suffix = if truncated {
                    if let Some(bracket) = closing_bracket {
                        format!("...{}", bracket)
                    } else {
                        "…".to_string()
                    }
                } else {
                    "".to_string()
                };

                let mut spans = sender_prefix_spans.clone();
                spans.push(Span::styled(icon, Style::default().fg(tool_color)));
                spans.push(Span::styled(" ", Style::default()));
                spans.push(Span::styled(
                    tc.name.clone(),
                    Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
                ));
                if !args_preview.is_empty() {
                    spans.push(Span::styled(
                        format!(" {}{}", args_preview, suffix),
                        Style::default().fg(theme.text_dim),
                    ));
                }
                lines.push(Line::from(spans));
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────
// 2. render_json_params_enhanced
// ──────────────────────────────────────────────────────────────

/// 渲染 JSON 参数（增强版）
pub(crate) fn render_json_params_enhanced(
    json: &serde_json::Value,
    max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    if let Some(obj) = json.as_object() {
        for (key, value) in obj {
            let value_str = format_json_value(value);
            let max_val_chars = max_width.saturating_sub(key.chars().count() + 7);

            let value_display = if value_str.chars().count() > max_val_chars {
                let truncated: String = value_str.chars().take(max_val_chars).collect();
                format!("{}…", truncated)
            } else {
                value_str
            };

            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(format!("{}:", key), Style::default().fg(theme.text_dim)),
                Span::styled(" ", Style::default()),
                Span::styled(value_display, Style::default().fg(theme.text_normal)),
            ]));
        }
    } else {
        // 非 JSON 对象，直接显示
        let value_str = format_json_value(json);
        for line in wrap_text(&value_str, max_width) {
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(line, Style::default().fg(theme.text_normal)),
            ]));
        }
    }
}

// ──────────────────────────────────────────────────────────────
// 3. extract_tool_description_from_args
// ──────────────────────────────────────────────────────────────

/// 从工具调用参数 JSON 中提取描述信息
/// - Bash/Shell：提取 description 字段
/// - Read/Write/Edit/Glob/Grep：提取 path 或 file_path 字段
/// - Agent/AgentTeam：提取 description 字段
pub(crate) fn extract_tool_description_from_args(
    tool_name: &str,
    arguments: &str,
) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;

    match tool_name {
        tool_names::BASH => parsed.get("description")?.as_str().map(|s| s.to_string()),
        tool_names::READ
        | tool_names::WRITE
        | tool_names::EDIT
        | tool_names::GLOB
        | tool_names::GREP => parsed
            .get("path")
            .or_else(|| parsed.get("file_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tool_names::AGENT => parsed
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tool_names::TEAMMATE => {
            let name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let role = parsed.get("role").and_then(|v| v.as_str()).unwrap_or(name);
            Some(role.to_string())
        }
        _ => None,
    }
}

// ──────────────────────────────────────────────────────────────
// 4. TeammateCallArgs + extract_teammate_args + render_teammate_call_request_expanded
// ──────────────────────────────────────────────────────────────

/// Teammate 工具参数结构（用于渲染）
pub(crate) struct TeammateCallArgs {
    pub name: String,
    pub role: String,
    pub prompt: String,
    pub worktree: bool,
}

/// 从 Teammate 工具的 arguments JSON 中提取参数
pub(crate) fn extract_teammate_args(arguments: &str) -> Option<TeammateCallArgs> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;
    Some(TeammateCallArgs {
        name: parsed.get("name")?.as_str()?.to_string(),
        role: parsed
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        prompt: parsed.get("prompt")?.as_str()?.to_string(),
        worktree: parsed
            .get("worktree")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

/// 渲染 Teammate 工具调用请求的展开模式（边框 + name/role + prompt + 元信息）
pub(crate) fn render_teammate_call_request_expanded(
    args: &TeammateCallArgs,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let border_color = theme.text_dim;
    let result_bg = theme.bg_primary;
    let content_w = bubble_max_width.saturating_sub(6);

    // 元信息行：name(role) [worktree]
    let mut meta_parts = vec![format!(
        "{}({})",
        args.name,
        if args.role.is_empty() {
            &args.name
        } else {
            &args.role
        }
    )];
    if args.worktree {
        meta_parts.push("[worktree]".to_string());
    }
    let meta_line = meta_parts.join("  ");
    for wrapped in wrap_text(&meta_line, content_w) {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default().bg(result_bg)),
            Span::styled(wrapped, Style::default().fg(theme.text_dim).bg(result_bg)),
        ]));
    }

    // Prompt 边框显示
    let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(border_color).bg(result_bg),
    )));

    let prompt_lines: Vec<&str> = args.prompt.lines().collect();
    let total = prompt_lines.len();
    let max_display = AGENT_CALL_PROMPT_MAX_LINES;
    let display_lines = &prompt_lines[..total.min(max_display)];

    for line in display_lines {
        for wrapped in wrap_text(line, content_w) {
            lines.push(bordered_line(
                vec![Span::styled(
                    wrapped,
                    Style::default().fg(theme.text_dim).bg(result_bg),
                )],
                bubble_max_width,
                border_color,
                result_bg,
            ));
        }
    }

    if total > max_display {
        lines.push(bordered_line(
            vec![Span::styled(
                format!("... (共 {} 行)", total),
                Style::default().fg(theme.text_dim).bg(result_bg),
            )],
            bubble_max_width,
            border_color,
            result_bg,
        ));
    }

    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color).bg(result_bg),
    )));
}

// ──────────────────────────────────────────────────────────────
// 5. BashArgs + extract_bash_args + render_bash_call_request_expanded
// ──────────────────────────────────────────────────────────────

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

// ──────────────────────────────────────────────────────────────
// 6. AgentCallArgs + extract_agent_args + render_agent_call_request_expanded
// ──────────────────────────────────────────────────────────────

/// Agent 工具参数结构（用于渲染）
pub(crate) struct AgentCallArgs {
    pub prompt: String,
    pub description: Option<String>,
    pub run_in_background: bool,
}

/// 从 Agent 工具的 arguments JSON 中提取参数
pub(crate) fn extract_agent_args(arguments: &str) -> Option<AgentCallArgs> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;
    Some(AgentCallArgs {
        prompt: parsed.get("prompt")?.as_str()?.to_string(),
        description: parsed
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        run_in_background: parsed
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

/// 渲染 Agent 工具调用请求的展开模式（边框 + prompt + 元信息）
pub(crate) fn render_agent_call_request_expanded(
    args: &AgentCallArgs,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let border_color = theme.text_dim;
    let result_bg = theme.bg_primary;
    let content_w = bubble_max_width.saturating_sub(6);

    // 元信息行：[background] 标识
    if args.run_in_background {
        for wrapped in wrap_text("[background]", content_w) {
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default().bg(result_bg)),
                Span::styled(wrapped, Style::default().fg(theme.text_dim).bg(result_bg)),
            ]));
        }
    }

    // Prompt 边框显示（复用 render_agent_result_nested 的边框风格）
    let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(border_color).bg(result_bg),
    )));

    let prompt_lines: Vec<&str> = args.prompt.lines().collect();
    let total = prompt_lines.len();
    let max_display = AGENT_CALL_PROMPT_MAX_LINES;
    let display_lines = &prompt_lines[..total.min(max_display)];

    for line in display_lines {
        for wrapped in wrap_text(line, content_w) {
            lines.push(bordered_line(
                vec![Span::styled(
                    wrapped,
                    Style::default().fg(theme.text_dim).bg(result_bg),
                )],
                bubble_max_width,
                border_color,
                result_bg,
            ));
        }
    }

    // 截断提示
    if total > max_display {
        lines.push(bordered_line(
            vec![Span::styled(
                format!("... (共 {} 行)", total),
                Style::default().fg(theme.text_dim).bg(result_bg),
            )],
            bubble_max_width,
            border_color,
            result_bg,
        ));
    }

    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color).bg(result_bg),
    )));
}

// ──────────────────────────────────────────────────────────────
// 7. render_exit_plan_mode_request
// ──────────────────────────────────────────────────────────────

/// 渲染 ExitPlanMode 工具调用请求（边框显示）
pub(crate) fn render_exit_plan_mode_request(
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let border_color = theme.text_dim;
    let result_bg = theme.bg_primary;
    let content_w = bubble_max_width.saturating_sub(6);

    // 顶边框
    let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(border_color).bg(result_bg),
    )));

    // 内容：提交计划审批提示
    let hint = "提交计划审批，等待用户批准后退出计划模式";
    for wrapped in wrap_text(hint, content_w) {
        lines.push(bordered_line(
            vec![Span::styled(
                wrapped,
                Style::default().fg(theme.text_dim).bg(result_bg),
            )],
            bubble_max_width,
            border_color,
            result_bg,
        ));
    }

    // 底边框
    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color).bg(result_bg),
    )));
}
