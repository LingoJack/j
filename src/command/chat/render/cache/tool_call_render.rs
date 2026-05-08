//! 工具调用请求渲染：展开/折叠模式、各类工具专用渲染

use crate::command::chat::constants::{AGENT_CALL_PROMPT_MAX_LINES, TOOL_ARG_PREVIEW_MAX_CHARS};
use crate::command::chat::storage::ToolCallItem;
use crate::command::chat::tools::classification::{ToolCategory, format_json_value};
use crate::command::chat::tools::tool_names;
use crate::util::text::{sanitize_single_line_text, wrap_text};
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
            let display_name = sanitize_single_line_text(&display_name);
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
                // 尝试专用渲染，失败则回退到通用 JSON 渲染
                if !render_specialized_tool_call(
                    &tc.name,
                    &tc.arguments,
                    bubble_max_width,
                    content_w,
                    lines,
                    theme,
                ) {
                    // 通用回退
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&tc.arguments)
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
                let desc_text = sanitize_single_line_text(&desc_parts.join("  "));
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
                let desc_text = sanitize_single_line_text(&desc_parts.join("  "));
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
                let desc = sanitize_single_line_text(&desc);
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
                let args_preview = sanitize_single_line_text(&args_preview);

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
            let key = sanitize_single_line_text(key);
            let value_display = sanitize_single_line_text(&value_display);

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

/// 从工具调用参数 JSON 中提取描述信息（用于折叠模式显示）
/// - Bash/Shell：提取 description 字段
/// - Read/Write/Edit/Glob/Grep：提取 path 或 file_path 字段
/// - Agent/Teammate：提取 description / role 字段
/// - Task：action + title
/// - TaskOutput：task_id
/// - WebSearch：query
/// - WebFetch：url
/// - Ask：question 文本
/// - TodoWrite/TodoRead：操作摘要
/// - Compact：focus
/// - EnterPlanMode：description
/// - LoadSkill：skill name
/// - RegisterHook：action + event
/// - SendMessage：to + message 预览
/// - EnterWorktree/ExitWorktree：name
/// - WorkDone：summary
/// - IgnoreMessage：静默标记
pub(crate) fn extract_tool_description_from_args(
    tool_name: &str,
    arguments: &str,
) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;

    match tool_name {
        // ── 文件操作 ──
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

        // ── Agent / Teammate ──
        tool_names::AGENT => parsed
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tool_names::TEAMMATE => {
            let name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let role = parsed.get("role").and_then(|v| v.as_str()).unwrap_or(name);
            Some(role.to_string())
        }

        // ── Task（任务管理）──
        tool_names::TASK => {
            let action = parsed
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("task");
            let title = parsed.get("title").and_then(|v| v.as_str());
            match title {
                Some(t) => Some(format!("{}: {}", action, t)),
                None => Some(action.to_string()),
            }
        }

        // ── TaskOutput（后台任务输出）──
        tool_names::TASK_OUTPUT => {
            let task_id = parsed
                .get("task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            Some(format!("获取任务 {} 输出", task_id))
        }

        // ── 网络 ──
        tool_names::WEB_SEARCH => parsed
            .get("query")
            .and_then(|v| v.as_str())
            .map(|s| format!("搜索: {}", s)),
        tool_names::WEB_FETCH => parsed
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tool_names::BROWSER => parsed
            .get("url")
            .or_else(|| parsed.get("action"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),

        // ── Ask（用户提问）──
        tool_names::ASK => {
            // questions 是数组，取第一个问题的 question 字段
            if let Some(questions) = parsed.get("questions").and_then(|v| v.as_array())
                && let Some(first) = questions.first()
            {
                return first
                    .get("question")
                    .and_then(|v| v.as_str())
                    .map(|s| truncate_str(s, TOOL_ARG_PREVIEW_MAX_CHARS));
            }
            None
        }

        // ── Todo ──
        tool_names::TODO_WRITE => {
            if let Some(todos) = parsed.get("todos").and_then(|v| v.as_array()) {
                let count = todos.len();
                Some(format!("更新 {} 项待办", count))
            } else {
                Some("更新待办".to_string())
            }
        }
        tool_names::TODO_READ => Some("读取待办列表".to_string()),

        // ── Compact（对话压缩）──
        tool_names::COMPACT => {
            let focus = parsed.get("focus").and_then(|v| v.as_str());
            match focus {
                Some(f) => Some(format!("压缩对话 (focus: {})", f)),
                None => Some("压缩对话".to_string()),
            }
        }

        // ── Plan ──
        tool_names::ENTER_PLAN_MODE => parsed
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| format!("进入计划模式: {}", s))
            .or_else(|| Some("进入计划模式".to_string())),
        tool_names::EXIT_PLAN_MODE => Some("提交计划审批".to_string()),

        // ── LoadSkill ──
        tool_names::LOAD_SKILL => parsed
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| format!("加载技能: {}", s)),

        // ── RegisterHook ──
        tool_names::REGISTER_HOOK => {
            let action = parsed
                .get("action")
                .and_then(|v| v.as_str())
                .unwrap_or("register");
            let event = parsed.get("event").and_then(|v| v.as_str());
            match event {
                Some(e) => Some(format!("{} 钩子: {}", action, e)),
                None => Some(format!("{} 钩子", action)),
            }
        }

        // ── SendMessage ──
        tool_names::SEND_MESSAGE => {
            let to = parsed.get("to").and_then(|v| v.as_str());
            let msg = parsed
                .get("message")
                .and_then(|v| v.as_str())
                .map(|s| truncate_str(s, 40));
            match (to, msg) {
                (Some(t), Some(m)) => Some(format!("→ {} {}", t, m)),
                (Some(t), None) => Some(format!("→ {}", t)),
                (None, Some(m)) => Some(format!("广播: {}", m)),
                (None, None) => Some("发送消息".to_string()),
            }
        }

        // ── Worktree ──
        tool_names::ENTER_WORKTREE => parsed
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| format!("进入工作树: {}", s))
            .or_else(|| Some("进入工作树".to_string())),
        tool_names::EXIT_WORKTREE => Some("退出工作树".to_string()),

        // ── WorkDone ──
        tool_names::WORK_DONE => parsed
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| truncate_str(s, TOOL_ARG_PREVIEW_MAX_CHARS))
            .or_else(|| Some("工作完成".to_string())),

        // ── IgnoreMessage ──
        tool_names::IGNORE_MESSAGE => Some("忽略消息".to_string()),

        // ── ComputerUse ──
        #[cfg(target_os = "macos")]
        tool_names::COMPUTER_USE => parsed
            .get("action")
            .and_then(|v| v.as_str())
            .map(|s| format!("计算机操作: {}", s)),

        _ => None,
    }
}

/// 将字符串截断到最大字符数，超出时加 "…"
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}…", truncated)
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

// ──────────────────────────────────────────────────────────────
// 8. render_specialized_tool_call — 展开模式专用渲染分发
// ──────────────────────────────────────────────────────────────

/// 根据工具名称分发到专用展开渲染，返回 true 表示已渲染
fn render_specialized_tool_call(
    tool_name: &str,
    arguments: &str,
    bubble_max_width: usize,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    match tool_name {
        tool_names::BASH => {
            if let Some(bash_args) = extract_bash_args(arguments) {
                render_bash_call_request_expanded(&bash_args, bubble_max_width, lines, theme);
                return true;
            }
            false
        }
        tool_names::AGENT => {
            if let Some(agent_args) = extract_agent_args(arguments) {
                render_agent_call_request_expanded(&agent_args, bubble_max_width, lines, theme);
                return true;
            }
            false
        }
        tool_names::TEAMMATE => {
            if let Some(tm_args) = extract_teammate_args(arguments) {
                render_teammate_call_request_expanded(&tm_args, bubble_max_width, lines, theme);
                return true;
            }
            false
        }
        tool_names::EXIT_PLAN_MODE => {
            render_exit_plan_mode_request(bubble_max_width, lines, theme);
            true
        }
        tool_names::TASK => render_task_call_request_expanded(arguments, content_w, lines, theme),
        tool_names::TASK_OUTPUT => {
            render_task_output_call_request_expanded(arguments, content_w, lines, theme)
        }
        tool_names::WEB_SEARCH => {
            render_web_search_call_request_expanded(arguments, content_w, lines, theme)
        }
        tool_names::WEB_FETCH => {
            render_web_fetch_call_request_expanded(arguments, content_w, lines, theme)
        }
        tool_names::BROWSER => {
            render_browser_call_request_expanded(arguments, content_w, lines, theme)
        }
        tool_names::ASK => render_ask_call_request_expanded(arguments, content_w, lines, theme),
        tool_names::TODO_WRITE => {
            render_todo_write_call_request_expanded(arguments, content_w, lines, theme)
        }
        tool_names::TODO_READ => render_todo_read_call_request_expanded(content_w, lines, theme),
        tool_names::COMPACT => {
            render_compact_call_request_expanded(arguments, content_w, lines, theme)
        }
        tool_names::ENTER_PLAN_MODE => {
            render_enter_plan_mode_call_request_expanded(arguments, content_w, lines, theme)
        }
        tool_names::LOAD_SKILL => {
            render_load_skill_call_request_expanded(arguments, content_w, lines, theme)
        }
        tool_names::REGISTER_HOOK => {
            render_register_hook_call_request_expanded(arguments, content_w, lines, theme)
        }
        tool_names::SEND_MESSAGE => {
            render_send_message_call_request_expanded(arguments, content_w, lines, theme)
        }
        tool_names::ENTER_WORKTREE | tool_names::EXIT_WORKTREE => {
            render_worktree_call_request_expanded(tool_name, arguments, content_w, lines, theme)
        }
        tool_names::WORK_DONE => {
            render_work_done_call_request_expanded(arguments, content_w, lines, theme)
        }
        tool_names::IGNORE_MESSAGE => {
            render_ignore_message_call_request_expanded(content_w, lines, theme)
        }
        #[cfg(target_os = "macos")]
        tool_names::COMPUTER_USE => {
            render_computer_use_call_request_expanded(arguments, content_w, lines, theme)
        }
        _ => false,
    }
}

// ──────────────────────────────────────────────────────────────
// 9. 各工具专用展开渲染
// ──────────────────────────────────────────────────────────────

/// 渲染键值对行
fn render_kv_line(
    key: &str,
    value: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let max_val_chars = content_w.saturating_sub(key.chars().count() + 7);
    let display = if value.chars().count() > max_val_chars {
        format!("{}…", truncate_str(value, max_val_chars))
    } else {
        value.to_string()
    };
    for wrapped in wrap_text(&display, content_w) {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(format!("{}:", key), Style::default().fg(theme.text_dim)),
            Span::styled(" ", Style::default()),
            Span::styled(wrapped, Style::default().fg(theme.text_normal)),
        ]));
    }
}

/// 渲染标签行（如 `[background]`、`[worktree]` 等）
fn render_tag_line(tag: &str, content_w: usize, lines: &mut Vec<Line<'static>>, theme: &Theme) {
    for wrapped in wrap_text(tag, content_w) {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(wrapped, Style::default().fg(theme.text_dim)),
        ]));
    }
}

/// Task 工具展开渲染
fn render_task_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let action = parsed
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("task");

    // action 标签
    render_tag_line(&format!("[{}]", action), content_w, lines, theme);

    // title
    if let Some(title) = parsed.get("title").and_then(|v| v.as_str()) {
        render_kv_line("title", title, content_w, lines, theme);
    }

    // description
    if let Some(desc) = parsed.get("description").and_then(|v| v.as_str()) {
        render_kv_line("description", desc, content_w, lines, theme);
    }

    // taskId（update/get 时）
    if let Some(task_id) = parsed.get("taskId").and_then(|v| v.as_str()) {
        render_kv_line("taskId", task_id, content_w, lines, theme);
    }

    // status（update 时）
    if let Some(status) = parsed.get("status").and_then(|v| v.as_str()) {
        render_kv_line("status", status, content_w, lines, theme);
    }

    true
}

/// TaskOutput 工具展开渲染
fn render_task_output_call_request_expanded(
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

/// WebSearch 工具展开渲染
fn render_web_search_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(query) = parsed.get("query").and_then(|v| v.as_str()) {
        render_kv_line("query", query, content_w, lines, theme);
    }

    if let Some(count) = parsed.get("count").and_then(|v| v.as_u64()) {
        render_kv_line("count", &count.to_string(), content_w, lines, theme);
    }

    if let Some(search_type) = parsed.get("type").and_then(|v| v.as_str()) {
        render_kv_line("type", search_type, content_w, lines, theme);
    }

    true
}

/// WebFetch 工具展开渲染
fn render_web_fetch_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(url) = parsed.get("url").and_then(|v| v.as_str()) {
        render_kv_line("url", url, content_w, lines, theme);
    }

    if let Some(mode) = parsed.get("extract_mode").and_then(|v| v.as_str()) {
        render_kv_line("mode", mode, content_w, lines, theme);
    }

    true
}

/// Browser 工具展开渲染
fn render_browser_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(action) = parsed.get("action").and_then(|v| v.as_str()) {
        render_kv_line("action", action, content_w, lines, theme);
    }

    if let Some(url) = parsed.get("url").and_then(|v| v.as_str()) {
        render_kv_line("url", url, content_w, lines, theme);
    }

    true
}

/// Ask 工具展开渲染
fn render_ask_call_request_expanded(
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

/// TodoWrite 工具展开渲染
fn render_todo_write_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(todos) = parsed.get("todos").and_then(|v| v.as_array()) {
        render_tag_line(
            &format!("待办列表 ({} 项)", todos.len()),
            content_w,
            lines,
            theme,
        );
        for todo in todos {
            let content = todo.get("content").and_then(|v| v.as_str()).unwrap_or("?");
            let status = todo
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("pending");
            let bullet = match status {
                "completed" => "[x]",
                "in_progress" => "[~]",
                "cancelled" => "[-]",
                _ => "[ ]",
            };
            let line_text = format!("{} {}", bullet, content);
            let display = truncate_str(&line_text, content_w);
            lines.push(Line::from(vec![
                Span::styled("      ", Style::default()),
                Span::styled(display, Style::default().fg(theme.text_dim)),
            ]));
        }
    }

    true
}

/// TodoRead 工具展开渲染
fn render_todo_read_call_request_expanded(
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    render_tag_line("读取待办列表", content_w, lines, theme);
    true
}

/// Compact 工具展开渲染
fn render_compact_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    render_tag_line("压缩对话上下文", content_w, lines, theme);

    if let Some(focus) = parsed.get("focus").and_then(|v| v.as_str()) {
        render_kv_line("focus", focus, content_w, lines, theme);
    }

    true
}

/// EnterPlanMode 工具展开渲染
fn render_enter_plan_mode_call_request_expanded(
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

/// LoadSkill 工具展开渲染
fn render_load_skill_call_request_expanded(
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

/// RegisterHook 工具展开渲染
fn render_register_hook_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let action = parsed
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("register");
    render_tag_line(&format!("[{}]", action), content_w, lines, theme);

    if let Some(event) = parsed.get("event").and_then(|v| v.as_str()) {
        render_kv_line("event", event, content_w, lines, theme);
    }

    if let Some(hook_type) = parsed.get("type").and_then(|v| v.as_str()) {
        render_kv_line("type", hook_type, content_w, lines, theme);
    }

    if let Some(command) = parsed.get("command").and_then(|v| v.as_str()) {
        render_kv_line("command", command, content_w, lines, theme);
    }

    if let Some(prompt) = parsed.get("prompt").and_then(|v| v.as_str()) {
        render_kv_line(
            "prompt",
            &truncate_str(prompt, 100),
            content_w,
            lines,
            theme,
        );
    }

    true
}

/// SendMessage 工具展开渲染
fn render_send_message_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let to = parsed.get("to").and_then(|v| v.as_str());
    if let Some(target) = to {
        render_kv_line("to", &format!("@{}", target), content_w, lines, theme);
    } else {
        render_tag_line("广播消息", content_w, lines, theme);
    }

    if let Some(message) = parsed.get("message").and_then(|v| v.as_str()) {
        render_kv_line(
            "message",
            &truncate_str(message, 100),
            content_w,
            lines,
            theme,
        );
    }

    true
}

/// EnterWorktree / ExitWorktree 工具展开渲染
fn render_worktree_call_request_expanded(
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

/// WorkDone 工具展开渲染
fn render_work_done_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    render_tag_line("工作完成声明", content_w, lines, theme);

    if let Some(summary) = parsed.get("summary").and_then(|v| v.as_str()) {
        render_kv_line("summary", summary, content_w, lines, theme);
    }

    true
}

/// IgnoreMessage 工具展开渲染
fn render_ignore_message_call_request_expanded(
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    render_tag_line("忽略消息", content_w, lines, theme);
    true
}

/// ComputerUse 工具展开渲染
#[cfg(target_os = "macos")]
fn render_computer_use_call_request_expanded(
    arguments: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) -> bool {
    let parsed = match serde_json::from_str::<serde_json::Value>(arguments) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if let Some(action) = parsed.get("action").and_then(|v| v.as_str()) {
        render_kv_line("action", action, content_w, lines, theme);
    }

    if let Some(display_num) = parsed.get("display_number").and_then(|v| v.as_u64()) {
        render_kv_line("display", &display_num.to_string(), content_w, lines, theme);
    }

    true
}
