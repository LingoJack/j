//! 工具结果渲染：diff 着色、Bash 输出、Todo 列表、Agent 嵌套结果

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::command::chat::constants::{
    AGENT_RESULT_MAX_LINES, BASH_OUTPUT_MAX_LINES, ERROR_RESULT_MAX_LINES, NORMAL_RESULT_MAX_LINES,
};
use crate::command::chat::render::cache::bubble::bordered_line;
use crate::command::chat::render::cache::msg_render::agent_name_color;
use crate::command::chat::render::cache::{ContentContext, TOOL_RESULT_DISPLAY_MAX_LINES};
use crate::command::chat::render::theme::Theme;
use crate::command::chat::tools::classification::{
    ToolCategory, ToolStatus, get_result_summary_for_tool,
};
use crate::command::chat::tools::tool_names;
use crate::util::text::wrap_text;

/// render_tool_result_msg 的只读参数（lines 作为输出单独传）
pub struct ToolResultRenderParams<'a> {
    pub sender_name: Option<&'a str>,
    pub content: &'a str,
    pub label: &'a str,
    pub tool_args: Option<&'a str>,
    pub bubble_max_width: usize,
    pub theme: &'a Theme,
    pub expand: bool,
}

/// 渲染工具执行结果消息：展开时完整内容，折叠时只显示标签
pub fn render_tool_result_msg(params: &ToolResultRenderParams, lines: &mut Vec<Line<'static>>) {
    // 解构出局部变量，保持函数体不变
    let sender_name = params.sender_name;
    let content = params.content;
    let label = params.label;
    let tool_args = params.tool_args;
    let bubble_max_width = params.bubble_max_width;
    let theme = params.theme;
    let expand = params.expand;
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

    // 与前一条消息（tool_call）之间留一行间距
    lines.push(Line::from(""));

    // 解析 label，格式为 "工具名..." 或 "工具名[id]..."
    let (tool_name, is_error) = parse_tool_label(label);
    let category = ToolCategory::from_name(&tool_name);
    let tool_color = category.color(theme);
    // tool_result 统一使用 🔧 图标，与 tool_call_request 的分类图标区分
    let icon = "🔧";

    let status = if is_error {
        ToolStatus::Failed
    } else {
        ToolStatus::Success
    };
    let status_icon = status.icon();
    let status_color = status.color(theme);

    // 获取结果摘要
    let summary = get_result_summary_for_tool(content, is_error, &tool_name, tool_args);

    // 第一行：sender_prefix + 图标 + 工具名 + 状态 + 摘要
    let mut spans = sender_prefix_spans;
    spans.push(Span::styled(icon, Style::default().fg(tool_color)));
    spans.push(Span::styled(" ", Style::default()));
    spans.push(Span::styled(
        tool_name.clone(),
        Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(" ", Style::default()));
    spans.push(Span::styled(status_icon, Style::default().fg(status_color)));
    spans.push(Span::styled(" ", Style::default()));
    spans.push(Span::styled(summary, Style::default().fg(theme.text_dim)));
    lines.push(Line::from(spans));

    // Todo 工具特殊处理：折叠模式也显示 todo 列表
    let is_todo_tool = tool_name == "TodoRead" || tool_name == "TodoWrite";

    if (!expand && !is_todo_tool) || content.is_empty() {
        return;
    }

    // 展开模式：缩进显示内容
    let clean = crate::util::text::sanitize_tool_output(content);
    let content_w = bubble_max_width.saturating_sub(6);

    // 错误结果特殊处理
    if is_error {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(
                "Error:",
                Style::default()
                    .fg(theme.toast_error_border)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        let error_lines: Vec<&str> = clean.lines().take(ERROR_RESULT_MAX_LINES).collect();
        for line in error_lines {
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("      {}", wrapped),
                    Style::default().fg(theme.toast_error_border),
                )));
            }
        }

        let total_lines = clean.lines().count();
        let max_err_lines = ERROR_RESULT_MAX_LINES;
        if total_lines > max_err_lines {
            lines.push(Line::from(Span::styled(
                format!(
                    "    ... (共 {} 行，显示前 {} 行)",
                    total_lines, max_err_lines
                ),
                Style::default().fg(theme.text_dim),
            )));
        }
    } else if clean.contains("```diff\n") {
        // Diff 块特殊渲染
        render_diff_content(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::AGENT
        || tool_name == tool_names::TEAMMATE
        || tool_name == tool_names::COMPACT
        || tool_name == tool_names::LOAD_SKILL
        || tool_name == tool_names::ENTER_PLAN_MODE
        || tool_name == tool_names::EXIT_PLAN_MODE
    {
        // Agent/Compact/LoadSkill/Plan 结果边框显示
        render_agent_result_nested(&clean, bubble_max_width, lines, theme);
    } else if tool_name == tool_names::BASH {
        // Bash 结果：命令行高亮 + 输出
        render_bash_result(
            &clean,
            tool_args,
            &mut ContentContext {
                content_w,
                lines,
                theme,
                expand: false,
            },
        );
    } else if tool_name == tool_names::TODO_READ || tool_name == tool_names::TODO_WRITE {
        // TodoRead/TodoWrite 结果：折叠和展开都显示 todo 列表
        render_todo_result(
            content,
            &mut ContentContext {
                content_w,
                lines,
                theme,
                expand,
            },
        );
    } else {
        // 正常结果
        let all_lines: Vec<&str> = clean.lines().take(NORMAL_RESULT_MAX_LINES).collect();
        for line in all_lines {
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_dim),
                )));
            }
        }

        let total_lines = clean.lines().count();
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
}

/// 渲染包含 diff 块的工具结果内容
pub(crate) fn render_diff_content(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let mut in_diff = false;
    for line in content.lines() {
        if line.starts_with("```diff") {
            in_diff = true;
            continue;
        }
        if in_diff && line.starts_with("```") {
            in_diff = false;
            continue;
        }
        if in_diff {
            let color = if line.starts_with("- ")
                || line.starts_with('-') && !line.starts_with("---")
            {
                theme.diff_del
            } else if line.starts_with("+ ") || line.starts_with('+') && !line.starts_with("+++") {
                theme.diff_add
            } else if line.starts_with("@@ ") {
                theme.diff_header
            } else {
                theme.text_dim
            };
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(color),
                )));
            }
        } else {
            // diff 块外的文本正常渲染
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_dim),
                )));
            }
        }
    }
}

/// 渲染 Agent 工具结果（嵌套缩进显示）
pub(crate) fn render_agent_result_nested(
    content: &str,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let all_lines: Vec<&str> = content.lines().collect();
    let total = all_lines.len();
    let max_display = AGENT_RESULT_MAX_LINES;
    let display_lines = &all_lines[..total.min(max_display)];

    let border_color = theme.text_dim;
    let result_bg = theme.bg_primary;
    // bordered_line: 左 "  │ " (4) + 右 " │" (2) = 6 开销
    let content_w = bubble_max_width.saturating_sub(6);

    // 顶边框
    let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(border_color).bg(result_bg),
    )));

    // 内容行
    for line in display_lines.iter() {
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

    // 底边框
    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color).bg(result_bg),
    )));
}

/// 渲染 Bash 工具结果（命令行高亮 + 输出）
pub(crate) fn render_bash_result(
    content: &str,
    tool_args: Option<&str>,
    ctx: &mut ContentContext<'_>,
) {
    let lines = &mut *ctx.lines;
    let theme = ctx.theme;
    let content_w = ctx.content_w;
    // 提取命令
    let command = tool_args
        .and_then(|args| serde_json::from_str::<serde_json::Value>(args).ok())
        .and_then(|v| {
            v.get("command")
                .and_then(|c| c.as_str().map(|s| s.to_string()))
        });

    if let Some(cmd) = command {
        // 命令行用高亮颜色显示
        let cmd_w = content_w.saturating_sub(6); // "    $ " 前缀
        for (i, cmd_line) in cmd.lines().enumerate() {
            let prefix = if i == 0 { "    $ " } else { "      " };
            for wrapped in wrap_text(cmd_line, cmd_w) {
                lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(theme.label_ai)),
                    Span::styled(
                        wrapped,
                        Style::default()
                            .fg(theme.text_white)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
        }
    }

    // 输出内容（灰色）
    let output_lines: Vec<&str> = content.lines().take(BASH_OUTPUT_MAX_LINES).collect();
    for line in &output_lines {
        for wrapped in wrap_text(line, content_w) {
            lines.push(Line::from(Span::styled(
                format!("    {}", wrapped),
                Style::default().fg(theme.text_dim),
            )));
        }
    }

    let total_lines = content.lines().count();
    if total_lines > 100 {
        lines.push(Line::from(Span::styled(
            format!("    ... (共 {} 行，显示前 100 行)", total_lines),
            Style::default().fg(theme.text_dim),
        )));
    }
}

/// 渲染 TodoRead/TodoWrite 工具结果（实心点/空心点样式）
/// expand=true 时额外显示完成/未完成条数统计
pub(crate) fn render_todo_result(content: &str, ctx: &mut ContentContext<'_>) {
    let lines = &mut *ctx.lines;
    let theme = ctx.theme;
    let content_w = ctx.content_w;
    let expand = ctx.expand;
    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(content) {
        // 展开模式：先显示统计信息
        if expand {
            let total = items.len();
            let completed = items
                .iter()
                .filter(|i| i.get("status").and_then(|s| s.as_str()) == Some("completed"))
                .count();
            let pending = total.saturating_sub(completed);

            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    format!("完成 {} / 未完成 {}", completed, pending),
                    Style::default().fg(theme.text_dim),
                ),
            ]));
            lines.push(Line::from(""));
        }

        // 列出每个 todo 项
        for item in &items {
            let status = item
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("pending");
            let text = item
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("(empty)");

            // 实心点 ● 表示已完成/进行中，空心点 ○ 表示未开始
            let (dot, color) = match status {
                "completed" => ("●", theme.label_ai),        // 绿色实心点
                "in_progress" => ("◉", theme.title_loading), // 黄色双圈实心点
                "cancelled" => ("◌", theme.text_dim),        // 灰色空心虚圈
                _ => ("○", Color::Yellow),                   // pending: 黄色空心点
            };

            let text_style = if status == "completed" {
                Style::default()
                    .fg(theme.text_dim)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else {
                Style::default().fg(theme.text_white)
            };
            let max_w = content_w.saturating_sub(10); // "    ● " prefix
            for (i, wrapped) in wrap_text(text, max_w).iter().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(dot, Style::default().fg(color)),
                        Span::styled(" ", Style::default()),
                        Span::styled(wrapped.clone(), text_style),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("      ", Style::default()),
                        Span::styled(wrapped.clone(), text_style),
                    ]));
                }
            }
        }
    } else {
        // 非 JSON，回退到普通显示
        let all_lines: Vec<&str> = content.lines().take(100).collect();
        for line in all_lines {
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_dim),
                )));
            }
        }
    }
}

/// 解析工具标签，提取工具名和错误状态
pub(crate) fn parse_tool_label(label: &str) -> (String, bool) {
    let is_error = label.contains("错误") || label.contains("失败") || label.contains("error");
    // 兼容旧格式 "工具 xxx" 和新格式直接工具名
    let tool_name = if label.starts_with("工具 ") {
        label
            .chars()
            .skip(3)
            .collect::<String>()
            .split(['.', ' '])
            .next()
            .unwrap_or(label)
            .to_string()
    } else {
        label.split(['.', ' ']).next().unwrap_or(label).to_string()
    };
    (tool_name, is_error)
}
