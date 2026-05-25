//! 工具结果渲染：diff 着色、Bash 输出、Todo 列表、Agent 嵌套结果

pub mod bash;
pub mod file_tools;
pub mod glob;
pub mod grep;
pub mod message_tools;
pub mod read;
pub mod shared;
pub mod task;
pub mod task_output;
pub mod todo;
pub mod web_fetch;
pub mod web_search;

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::command::chat::constants::{ERROR_RESULT_MAX_LINES, NORMAL_RESULT_MAX_LINES};
use crate::command::chat::render::cache::msg_render::agent_name_color;
use crate::command::chat::render::cache::{ContentContext, TOOL_RESULT_DISPLAY_MAX_LINES};
use crate::command::chat::render::theme::Theme;
use crate::command::chat::render::theme::{ToolCategoryColor, ToolStatusColor};
use crate::command::chat::tools::classification::{
    ToolCategory, ToolStatus, get_result_summary_for_tool,
};
use crate::command::chat::tools::tool_names;
use crate::util::text::{
    line_number_continuation_prefix, line_number_prefix_width, wrap_text, wrap_text_with_prefix,
};

use bash::render_bash_result;
use file_tools::render_write_edit_result;
use glob::render_glob_result;
use grep::render_grep_result;
use message_tools::render_send_message_result;
use read::render_read_result;
use shared::{parse_tool_label, render_agent_result_nested, render_diff_content};
use task::render_task_result;
use task_output::render_task_output_result;
use todo::render_todo_result;
use web_fetch::render_web_fetch_result;
use web_search::render_web_search_result;

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
    } else if tool_name == tool_names::SHELL {
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
            &clean,
            &mut ContentContext {
                content_w,
                lines,
                theme,
                expand,
            },
        );
    } else if tool_name == tool_names::READ {
        // Read 工具结果：带行号的代码，支持语法高亮
        render_read_result(&clean, tool_args, content_w, lines, theme);
    } else if tool_name == tool_names::GLOB {
        // Glob 工具结果：树形文件列表
        render_glob_result(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::GREP {
        // Grep 工具结果：结构化搜索结果
        render_grep_result(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::WEB_SEARCH {
        // WebSearch 工具结果：结构化搜索结果
        render_web_search_result(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::WEB_FETCH {
        // WebFetch 工具结果：内容预览（可能含 Markdown）
        render_web_fetch_result(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::TASK {
        // Task 工具结果：任务列表
        render_task_result(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::WRITE || tool_name == tool_names::EDIT {
        // Write/Edit 工具结果：文件路径高亮
        render_write_edit_result(&clean, tool_args, content_w, lines, theme);
    } else if tool_name == tool_names::TASK_OUTPUT {
        // TaskOutput 工具结果：结构化任务输出
        render_task_output_result(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::SEND_MESSAGE {
        // SendMessage 工具结果：发送确认
        render_send_message_result(&clean, tool_args, lines, theme);
    } else {
        // 正常结果
        let all_lines: Vec<&str> = clean.lines().take(NORMAL_RESULT_MAX_LINES).collect();
        // 预扫描：判断是否大部分行都带行号前缀（如 Read 工具输出）
        let numbered_count = all_lines
            .iter()
            .filter(|l| line_number_prefix_width(l).is_some())
            .count();
        let mostly_numbered = !all_lines.is_empty() && numbered_count * 2 >= all_lines.len();

        for line in all_lines {
            if mostly_numbered {
                // 带行号的文本：续行保留 │ 符号，与内容列对齐
                let (_, cont_prefix) =
                    line_number_continuation_prefix(line).unwrap_or_else(|| (0, String::new()));
                for wrapped in wrap_text_with_prefix(line, content_w, &cont_prefix) {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", wrapped),
                        Style::default().fg(theme.text_dim),
                    )));
                }
            } else {
                // 普通文本：标准 wrap
                for wrapped in wrap_text(line, content_w) {
                    lines.push(Line::from(Span::styled(
                        format!("    {}", wrapped),
                        Style::default().fg(theme.text_dim),
                    )));
                }
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
