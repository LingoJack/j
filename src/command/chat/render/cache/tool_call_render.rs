//! 工具调用请求渲染：展开/折叠模式、各类工具专用渲染

pub mod agent;
pub mod ask;
pub mod bash;
pub mod browser;
pub mod compact;
pub mod computer_use;
pub mod edit;
pub mod enter_plan_mode;
pub mod exit_plan_mode;
pub mod glob;
pub mod grep;
pub mod ignore_message;
pub mod load_skill;
pub mod read;
pub mod register_hook;
pub mod send_message;
pub mod shared;
pub mod task;
pub mod task_output;
pub mod teammate;
pub mod todo_read;
pub mod todo_write;
pub mod web_fetch;
pub mod web_search;
pub mod work_done;
pub mod worktree;
pub mod write;

use crate::command::chat::constants::TOOL_ARG_PREVIEW_MAX_CHARS;
use crate::command::chat::render::theme::ToolCategoryColor;
use crate::command::chat::storage::ToolCallItem;
use crate::command::chat::tools::classification::ToolCategory;
use crate::command::chat::tools::tool_names;
use crate::util::text::{sanitize_single_line_text, wrap_text};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::RenderContext;
use super::msg_render::agent_name_color;
use crate::command::chat::render::theme::Theme;

use agent::{extract_agent_args, render_agent_call_request_expanded};
use ask::render_ask_call_request_expanded;
use bash::extract_bash_args;
use browser::render_browser_call_request_expanded;
use compact::render_compact_call_request_expanded;
#[cfg(target_os = "macos")]
use computer_use::render_computer_use_call_request_expanded;
use edit::render_edit_call_request_expanded;
use enter_plan_mode::render_enter_plan_mode_call_request_expanded;
use exit_plan_mode::render_exit_plan_mode_request;
use glob::render_glob_call_request_expanded;
use grep::render_grep_call_request_expanded;
use ignore_message::render_ignore_message_call_request_expanded;
use load_skill::render_load_skill_call_request_expanded;
use read::render_read_call_request_expanded;
use register_hook::render_register_hook_call_request_expanded;
use send_message::render_send_message_call_request_expanded;
use shared::{extract_tool_description_from_args, render_json_params_enhanced};
use task::render_task_call_request_expanded;
use task_output::render_task_output_call_request_expanded;
use teammate::{extract_teammate_args, render_teammate_call_request_expanded};
use todo_read::render_todo_read_call_request_expanded;
use todo_write::render_todo_write_call_request_expanded;
use web_fetch::render_web_fetch_call_request_expanded;
use web_search::render_web_search_call_request_expanded;
use work_done::render_work_done_call_request_expanded;
use worktree::render_worktree_call_request_expanded;
use write::render_write_call_request_expanded;

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
// 2. render_specialized_tool_call — 展开模式专用渲染分发
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
        tool_names::SHELL => {
            if let Some(bash_args) = extract_bash_args(arguments) {
                bash::render_bash_call_request_expanded(&bash_args, bubble_max_width, lines, theme);
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
        tool_names::GLOB => render_glob_call_request_expanded(arguments, content_w, lines, theme),
        tool_names::GREP => render_grep_call_request_expanded(arguments, content_w, lines, theme),
        tool_names::READ => render_read_call_request_expanded(arguments, content_w, lines, theme),
        tool_names::WRITE => render_write_call_request_expanded(arguments, content_w, lines, theme),
        tool_names::EDIT => render_edit_call_request_expanded(arguments, content_w, lines, theme),
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
