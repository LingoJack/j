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
use crate::command::chat::render::theme::{ToolCategoryColor, ToolStatusColor};
use crate::command::chat::tools::classification::{
    ToolCategory, ToolStatus, get_result_summary_for_tool,
};
use crate::command::chat::tools::tool_names;
use crate::markdown::highlight::highlight_code_line;
use crate::tui::editor_core::EditorTheme;
use crate::util::text::{
    line_number_continuation_prefix, line_number_prefix_width, wrap_text, wrap_text_with_prefix,
};

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
        let lang = tool_args
            .and_then(parse_file_path_from_json)
            .map(|p| infer_lang_from_path(&p))
            .unwrap_or("");
        let editor_theme = EditorTheme::from(theme);

        let all_lines: Vec<&str> = clean.lines().take(NORMAL_RESULT_MAX_LINES).collect();
        for line in all_lines {
            if let Some((prefix_w, cont_prefix)) = line_number_continuation_prefix(line) {
                // 分离行号前缀和代码内容
                let prefix_str: String = line.chars().take(prefix_w).collect();
                let code_content: String = line.chars().skip(prefix_w).collect();
                // 续行
                for (i, wrapped) in wrap_text_with_prefix(&code_content, content_w, &cont_prefix)
                    .into_iter()
                    .enumerate()
                {
                    let mut spans = vec![Span::styled("    ", Style::default().fg(theme.text_dim))];
                    if i == 0 {
                        // 首行：行号前缀
                        spans.push(Span::styled(
                            prefix_str.clone(),
                            Style::default().fg(theme.text_dim),
                        ));
                    } else {
                        // 续行：缩进对齐
                        spans.push(Span::styled(
                            cont_prefix.clone(),
                            Style::default().fg(theme.text_dim),
                        ));
                    }
                    if lang.is_empty() {
                        spans.push(Span::styled(wrapped, Style::default().fg(theme.text_dim)));
                    } else {
                        spans.extend(highlight_code_line(&wrapped, lang, &editor_theme));
                    }
                    lines.push(Line::from(spans));
                }
            } else {
                // 无行号前缀的行（如空行）
                for wrapped in wrap_text(line, content_w) {
                    if lang.is_empty() {
                        lines.push(Line::from(Span::styled(
                            format!("    {}", wrapped),
                            Style::default().fg(theme.text_dim),
                        )));
                    } else {
                        let mut spans =
                            vec![Span::styled("    ", Style::default().fg(theme.text_dim))];
                        spans.extend(highlight_code_line(&wrapped, lang, &editor_theme));
                        lines.push(Line::from(spans));
                    }
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

/// 从 tool_args JSON 中提取 file_path 字段
fn parse_file_path_from_json(json: &str) -> Option<String> {
    // 简单解析：查找 "file_path": "..." 或 "path": "..."
    let patterns = ["\"file_path\"", "\"path\""];
    for pattern in patterns {
        if let Some(idx) = json.find(pattern) {
            let rest = &json[idx + pattern.len()..];
            // 跳过可能的空白和冒号
            let rest = rest.trim_start_matches([' ', ':', '\t']);
            // 提取引号内的字符串
            if let Some(stripped) = rest.strip_prefix('"') {
                let end = stripped.find('"')?;
                return Some(stripped[..end].to_string());
            }
        }
    }
    None
}

/// 根据文件扩展名推断语法高亮语言
fn infer_lang_from_path(path: &str) -> &'static str {
    let path_lower = path.to_lowercase();
    // 先检查完整文件名（如 Makefile）
    if path_lower.ends_with("makefile") || path_lower.ends_with("justfile") {
        return "makefile";
    }
    if path_lower.ends_with("dockerfile") {
        return "dockerfile";
    }
    // 扩展名映射
    let ext = path_lower.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "rust",
        "go" => "go",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "tsx" => "tsx",
        "jsx" => "jsx",
        "java" => "java",
        "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" | "hh" => "cpp",
        "sh" | "bash" | "zsh" => "bash",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "md" | "markdown" => "markdown",
        "sql" => "sql",
        "css" | "scss" | "sass" => "css",
        "html" | "htm" => "html",
        "xml" => "xml",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "rb" => "ruby",
        "php" => "php",
        "lua" => "lua",
        "vim" => "vim",
        "dockerfile" => "dockerfile",
        "makefile" => "makefile",
        _ => "",
    }
}

/// Glob 结果：树形文件列表
fn render_glob_result(
    content: &str,
    _content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    // Glob 输出格式：
    //   找到 N 个匹配文件:
    //   (空行)
    //   path/to/file1
    //   path/to/file2
    //   ...

    let mut all_paths: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();

    // 跳过第一行统计信息（如 "找到 23 个匹配文件:"）
    if !all_paths.is_empty() && all_paths[0].contains("找到") && all_paths[0].contains("匹配") {
        all_paths.remove(0);
    }

    if all_paths.is_empty() {
        lines.push(Line::from(Span::styled(
            "    (无匹配文件)",
            Style::default().fg(theme.text_dim),
        )));
        return;
    }

    let total = all_paths.len();
    let max_display = NORMAL_RESULT_MAX_LINES.min(30);
    let display_paths = &all_paths[..total.min(max_display)];

    // 收集所有出现的目录（用于区分文件和目录）
    let mut dirs = std::collections::HashSet::new();
    for path in display_paths {
        let mut p = *path;
        while let Some(slash_idx) = p.rfind('/') {
            dirs.insert(&p[..slash_idx]);
            p = &p[..slash_idx];
        }
    }

    // 构建树形结构：提取公共前缀，按层级显示
    // 先找出最短的公共父目录
    let common_prefix = find_common_prefix(display_paths);

    for path in display_paths {
        // 相对于公共前缀显示
        let display_path = if common_prefix.is_empty() {
            *path
        } else if path.starts_with(&common_prefix) {
            &path[common_prefix.len()..]
        } else {
            *path
        };

        let indent = display_path.matches('/').count() * 2 + 4;
        let indent_str = " ".repeat(indent);
        let name = path.rsplit('/').next().unwrap_or(path);

        if dirs.contains(path) {
            // 目录：高亮 + 斜杠后缀
            lines.push(Line::from(vec![
                Span::styled(indent_str, Style::default()),
                Span::styled(
                    format!("{}/", name),
                    Style::default()
                        .fg(theme.config_title)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            // 文件：普通颜色
            lines.push(Line::from(vec![
                Span::styled(indent_str, Style::default()),
                Span::styled(name.to_string(), Style::default().fg(theme.text_normal)),
            ]));
        }
    }

    if total > max_display {
        lines.push(Line::from(Span::styled(
            format!("    ... (共 {} 个文件，显示前 {} 个)", total, max_display),
            Style::default().fg(theme.text_dim),
        )));
    } else if total > 1 {
        lines.push(Line::from(Span::styled(
            format!("    (共 {} 个文件)", total),
            Style::default().fg(theme.text_dim),
        )));
    }
}

/// 找出路径列表的最短公共前缀（不含文件名）
fn find_common_prefix(paths: &[&str]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let first = paths[0];
    let first_dir = first.rfind('/').map(|i| &first[..i + 1]).unwrap_or("");

    for path in paths {
        let mut common_len = 0;
        for (c1, c2) in first_dir.chars().zip(path.chars()) {
            if c1 == c2 {
                common_len += c1.len_utf8();
            } else {
                break;
            }
        }
        if common_len == 0 {
            return String::new();
        }
        // 确保 common_len 处停在 '/' 处
        let candidate = &first_dir[..common_len];
        if !candidate.ends_with('/') {
            if let Some(slash) = candidate.rfind('/') {
                return candidate[..slash + 1].to_string();
            }
            return String::new();
        }
    }
    first_dir.to_string()
}

/// Grep 结果：结构化搜索结果
///
/// 实际 Grep 工具输出格式（content 模式）：
/// ```text
/// 找到 5 个匹配:
///
/// src/command/chat/ui/chat.rs:123:let theme = Theme::load();
/// src/command/chat/ui/chat.rs:456:fn draw_help(t: &Theme) {
/// ```
///
/// files_with_matches 模式：
/// ```text
/// 找到 3 个匹配文件:
///
/// src/command/chat/ui/chat.rs
/// src/markdown/render.rs
/// ```
///
/// count 模式：
/// ```text
/// 共 10 处匹配:
///
/// src/command/chat/ui/chat.rs:5
/// src/markdown/render.rs:3
/// ```
fn render_grep_result(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let mut raw_lines: Vec<&str> = content.lines().collect();

    // 跳过第一行统计信息（如 "找到 5 个匹配:" 或 "共 10 处匹配:"）
    let header = raw_lines.first();
    if header.is_some_and(|h| h.contains("找到") || h.contains("共")) {
        raw_lines.remove(0);
    }

    // 过滤空行
    let all_lines: Vec<&str> = raw_lines.into_iter().filter(|l| !l.is_empty()).collect();

    if all_lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "    (无匹配)",
            Style::default().fg(theme.text_dim),
        )));
        return;
    }

    // 检测格式：尝试按 path:line:content 解析
    let is_content_format = all_lines
        .first()
        .is_some_and(|l| parse_grep_content_line(l).is_some());

    let is_count_format = all_lines.first().is_some_and(|l| {
        l.rfind(':')
            .is_some_and(|idx| l[idx + 1..].parse::<usize>().is_ok())
    });

    if is_content_format {
        render_grep_content_format(&all_lines, content_w, lines, theme);
    } else if is_count_format {
        render_grep_count_format(&all_lines, lines, theme);
    } else {
        // files_with_matches 格式 — 文件列表
        render_grep_files_format(&all_lines, lines, theme);
    }
}

/// 解析 path:lineno:content 格式行（匹配行，`:` 分隔）
/// 返回 (file_path, line_number, content)
fn parse_grep_content_line(line: &str) -> Option<(&str, usize, &str)> {
    let mut search_from = 0;
    while let Some(colon_idx) = line[search_from..].find(':') {
        let abs_idx = search_from + colon_idx;
        let after_colon = &line[abs_idx + 1..];
        if let Some(second_colon) = after_colon.find(':') {
            let line_num_str = &after_colon[..second_colon];
            if let Ok(line_num) = line_num_str.parse::<usize>() {
                let file_path = &line[..abs_idx];
                let content = &after_colon[second_colon + 1..];
                return Some((file_path, line_num, content));
            }
        }
        search_from = abs_idx + 1;
    }
    None
}

/// 解析上下文行格式（path-lineno:content，`-` 分隔路径和行号）
/// 返回 (file_path, line_number, content)
fn parse_grep_context_line(line: &str) -> Option<(&str, usize, &str)> {
    let mut search_from = 0;
    while let Some(dash_idx) = line[search_from..].find('-') {
        let abs_idx = search_from + dash_idx;
        let after_dash = &line[abs_idx + 1..];
        if let Some(colon_idx) = after_dash.find(':') {
            let line_num_str = &after_dash[..colon_idx];
            if let Ok(line_num) = line_num_str.parse::<usize>() {
                let file_path = &line[..abs_idx];
                let content = &after_dash[colon_idx + 1..];
                return Some((file_path, line_num, content));
            }
        }
        search_from = abs_idx + 1;
    }
    None
}

/// 渲染 content 模式的 Grep 结果
fn render_grep_content_format(
    all_lines: &[&str],
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let max_display = NORMAL_RESULT_MAX_LINES.min(50);
    let mut current_file: Option<&str> = None;
    let mut match_count = 0usize;
    let mut file_count = 0usize;
    let mut displayed = 0usize;

    for line in all_lines {
        if displayed >= max_display {
            break;
        }

        // 尝试匹配行格式: path:lineno:content
        if let Some((file_path, line_num, content)) = parse_grep_content_line(line) {
            if current_file != Some(file_path) {
                current_file = Some(file_path);
                file_count += 1;
                displayed += 1;
                lines.push(render_file_path_line(file_path, theme));
            }

            match_count += 1;
            displayed += 1;

            let wrapped = wrap_text(content, content_w.saturating_sub(14));
            for (i, w) in wrapped.iter().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled("      ", Style::default()),
                        Span::styled(
                            format!("{:>4} │ ", line_num),
                            Style::default().fg(theme.text_dim),
                        ),
                        Span::styled(w.to_string(), Style::default().fg(theme.text_normal)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("      ", Style::default()),
                        Span::styled("     │ ", Style::default().fg(theme.text_dim)),
                        Span::styled(w.to_string(), Style::default().fg(theme.text_normal)),
                    ]));
                }
            }
        }
        // 尝试上下文行格式: path-lineno:content
        else if let Some((file_path, line_num, content)) = parse_grep_context_line(line) {
            if current_file != Some(file_path) {
                current_file = Some(file_path);
                file_count += 1;
                displayed += 1;
                lines.push(render_file_path_line(file_path, theme));
            }

            displayed += 1;

            // 上下文行用 text_dim
            let wrapped = wrap_text(content, content_w.saturating_sub(14));
            for (i, w) in wrapped.iter().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled("      ", Style::default()),
                        Span::styled(
                            format!("{:>4} │ ", line_num),
                            Style::default().fg(theme.text_dim),
                        ),
                        Span::styled(w.to_string(), Style::default().fg(theme.text_dim)),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled("      ", Style::default()),
                        Span::styled("     │ ", Style::default().fg(theme.text_dim)),
                        Span::styled(w.to_string(), Style::default().fg(theme.text_dim)),
                    ]));
                }
            }
        }
    }

    push_grep_summary(match_count, file_count, lines, theme);

    if all_lines.len() > max_display {
        lines.push(Line::from(Span::styled(
            "    ... (结果已截断)".to_string(),
            Style::default().fg(theme.text_dim),
        )));
    }
}

/// 渲染 count 模式的 Grep 结果
fn render_grep_count_format(all_lines: &[&str], lines: &mut Vec<Line<'static>>, theme: &Theme) {
    let mut total_count = 0usize;
    let mut file_count = 0usize;

    for line in all_lines {
        if let Some(last_colon) = line.rfind(':') {
            let file_path = &line[..last_colon];
            let count_str = &line[last_colon + 1..];
            if let Ok(count) = count_str.parse::<usize>() {
                total_count += count;
                file_count += 1;
                lines.push(Line::from(vec![
                    Span::styled("    ", Style::default()),
                    Span::styled(
                        file_path.to_string(),
                        Style::default().fg(theme.config_title),
                    ),
                    Span::styled(" — ".to_string(), Style::default().fg(theme.text_dim)),
                    Span::styled(
                        format!("{} 处", count),
                        Style::default().fg(theme.text_normal),
                    ),
                ]));
            }
        }
    }

    push_grep_summary(total_count, file_count, lines, theme);
}

/// 渲染 files_with_matches 模式的 Grep 结果
fn render_grep_files_format(all_lines: &[&str], lines: &mut Vec<Line<'static>>, theme: &Theme) {
    for line in all_lines {
        lines.push(render_file_path_line(line, theme));
    }

    if all_lines.len() > 1 {
        lines.push(Line::from(Span::styled(
            format!("    (共 {} 个文件)", all_lines.len()),
            Style::default().fg(theme.text_dim),
        )));
    }
}

/// 渲染文件路径行（带图标）
fn render_file_path_line(file_path: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled(
            file_path.to_string(),
            Style::default()
                .fg(theme.config_title)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

/// 添加 Grep 统计行
fn push_grep_summary(
    match_count: usize,
    file_count: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    if match_count == 0 && file_count == 0 {
        return;
    }
    let summary = if file_count > 1 {
        format!("    (共 {} 处匹配，{} 个文件)", match_count, file_count)
    } else {
        format!("    (共 {} 处匹配)", match_count)
    };
    lines.push(Line::from(Span::styled(
        summary,
        Style::default().fg(theme.text_dim),
    )));
}

/// WebSearch 结果：结构化搜索结果
///
/// 输入格式（由 format_search_results 生成）：
/// ```
/// 搜索: query
///
/// 1. Title
///    URL
///    highlight text
///
/// 2. Title
///    URL
/// ```
fn render_web_search_result(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let mut result_count = 0usize;
    let mut iter = content.lines().peekable();

    // 跳过 "搜索: query" 首行
    if iter
        .peek()
        .is_some_and(|l| l.starts_with("搜索:") || l.starts_with("搜索："))
    {
        iter.next();
    }

    while let Some(line) = iter.next() {
        if line.is_empty() {
            continue;
        }

        // 检测序号行："1. Title"
        if let Some(rest) = line
            .trim_start()
            .strip_prefix(|c: char| c.is_ascii_digit())
            .and_then(|r| r.strip_prefix(". "))
        {
            result_count += 1;
            // 标题行
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    format!("{}. ", result_count),
                    Style::default()
                        .fg(theme.config_title)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    rest.to_string(),
                    Style::default()
                        .fg(theme.text_normal)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            // 后续行：URL + 摘要
            let mut sub_lines = 0usize;
            while let Some(next) = iter.peek() {
                if next.is_empty()
                    || next
                        .trim_start()
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_digit() && next.contains(". "))
                {
                    break;
                }
                let sub_line = iter.next().unwrap_or("");
                sub_lines += 1;
                if sub_lines == 1 {
                    // URL 行
                    lines.push(Line::from(Span::styled(
                        format!("       {}", sub_line),
                        Style::default().fg(theme.text_dim),
                    )));
                } else {
                    // 摘要行
                    for wrapped in wrap_text(sub_line, content_w.saturating_sub(7)) {
                        lines.push(Line::from(Span::styled(
                            format!("       {}", wrapped),
                            Style::default().fg(theme.text_dim),
                        )));
                    }
                }
            }
            // 结果之间加空行
            lines.push(Line::from(""));
            continue;
        }

        // 非序号行，普通折行
        for wrapped in wrap_text(line, content_w.saturating_sub(4)) {
            lines.push(Line::from(Span::styled(
                format!("    {}", wrapped),
                Style::default().fg(theme.text_dim),
            )));
        }
    }

    // 统计行
    if result_count > 1 {
        lines.push(Line::from(Span::styled(
            format!("    (共 {} 个结果)", result_count),
            Style::default().fg(theme.text_dim),
        )));
    }
}

/// WebFetch 结果：内容预览
///
/// 自动检测是否为 Markdown 内容：
/// - 包含 # 标题、- 列表等标记时用 markdown_to_lines 渲染
/// - 否则纯文本折行显示
fn render_web_fetch_result(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    // 检测是否为 Markdown 内容
    let has_markdown = content.lines().take(20).any(|l| {
        l.starts_with("# ")
            || l.starts_with("## ")
            || l.starts_with("- ")
            || l.starts_with("* ")
            || l.starts_with("> ")
            || l.starts_with("```")
            || l.starts_with("| ")
            || l.starts_with("1. ")
    });

    if has_markdown {
        // 用 IR 渲染器渲染 Markdown
        use crate::markdown::parser::markdown_to_lines;
        let md_lines = markdown_to_lines(content, content_w, theme);
        for md_line in md_lines {
            // 添加 4 空格缩进
            let mut spans = vec![Span::styled("    ", Style::default())];
            spans.extend(md_line.spans);
            lines.push(Line::from(spans));
        }
    } else {
        // 纯文本折行
        let all_lines: Vec<&str> = content.lines().take(NORMAL_RESULT_MAX_LINES).collect();
        for line in all_lines {
            for wrapped in wrap_text(line, content_w.saturating_sub(4)) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_normal),
                )));
            }
        }
        let total = content.lines().count();
        if total > NORMAL_RESULT_MAX_LINES {
            lines.push(Line::from(Span::styled(
                format!(
                    "    ... (共 {} 行，显示前 {} 行)",
                    total, NORMAL_RESULT_MAX_LINES
                ),
                Style::default().fg(theme.text_dim),
            )));
        }
    }
}

/// Task 结果：结构化任务列表
///
/// 输入格式：JSON 数组
/// ```json
/// [
///   { "taskId": "1", "title": "...", "status": "completed", "blockedBy": [] }
/// ]
/// ```
fn render_task_result(
    content: &str,
    _content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    use serde_json::Value;
    // 尝试解析 JSON
    let parsed: Option<Value> = serde_json::from_str(content).ok();
    if let Some(Value::Array(arr)) = parsed {
        if arr.is_empty() {
            lines.push(Line::from(Span::styled(
                "    (无任务)",
                Style::default().fg(theme.text_dim),
            )));
            return;
        }

        for item in &arr {
            let task_id = item.get("taskId").and_then(|v| v.as_str()).unwrap_or("?");
            let title = item
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("(无标题)");
            let status = item
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("pending");

            // 状态图标 + 颜色
            let (icon, color) = match status {
                "completed" => ("●", theme.label_ai),
                "in_progress" => ("◉", theme.title_loading),
                "pending" => ("○", theme.text_dim),
                "deleted" => ("✕", theme.toast_error_border),
                _ => ("·", theme.text_dim),
            };

            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(format!("{} ", icon), Style::default().fg(color)),
                Span::styled(
                    format!("#{} ", task_id),
                    Style::default().fg(theme.text_dim),
                ),
                Span::styled(
                    format!("[{}] ", status),
                    Style::default().fg(theme.text_dim),
                ),
                Span::styled(title.to_string(), Style::default().fg(theme.text_normal)),
            ]));
        }

        lines.push(Line::from(Span::styled(
            format!("    (共 {} 项任务)", arr.len()),
            Style::default().fg(theme.text_dim),
        )));
    } else {
        // 非 JSON 格式，普通渲染
        for line in content.lines().take(NORMAL_RESULT_MAX_LINES) {
            lines.push(Line::from(Span::styled(
                format!("    {}", line),
                Style::default().fg(theme.text_dim),
            )));
        }
    }
}

/// Write/Edit 结果：文件路径高亮
fn render_write_edit_result(
    content: &str,
    tool_args: Option<&str>,
    _content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    // 从 tool_args 提取文件路径
    let file_path = tool_args.and_then(parse_file_path_from_json);

    if let Some(path) = file_path {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(
                path,
                Style::default()
                    .fg(theme.config_title)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" — ", Style::default().fg(theme.text_dim)),
            // content 通常是 "1 file written" 或类似
            Span::styled(
                content.lines().next().unwrap_or("").to_string(),
                Style::default().fg(theme.text_dim),
            ),
        ]));
    } else {
        // 无法提取路径，普通渲染
        for line in content.lines().take(NORMAL_RESULT_MAX_LINES) {
            lines.push(Line::from(Span::styled(
                format!("    {}", line),
                Style::default().fg(theme.text_dim),
            )));
        }
    }
}

/// SendMessage 结果：发送确认
fn render_send_message_result(
    content: &str,
    tool_args: Option<&str>,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    // 从 tool_args 提取目标
    let target = tool_args.and_then(|args| {
        let pattern = "\"to\"";
        if let Some(idx) = args.find(pattern) {
            let rest = &args[idx + pattern.len()..];
            let rest = rest.trim_start_matches([' ', ':', '\t']);
            if let Some(stripped) = rest.strip_prefix('"')
                && let Some(end) = stripped.find('"')
            {
                return Some(stripped[..end].to_string());
            }
        }
        None
    });

    // 从 tool_args 提取消息内容
    let message = tool_args.and_then(|args| {
        let pattern = "\"message\"";
        if let Some(idx) = args.find(pattern) {
            let rest = &args[idx + pattern.len()..];
            let rest = rest.trim_start_matches([' ', ':', '\t']);
            if let Some(stripped) = rest.strip_prefix('"')
                && let Some(end) = stripped.find('"')
            {
                return Some(stripped[..end].to_string());
            }
        }
        None
    });

    // 第一行：发送目标
    if let Some(t) = &target {
        lines.push(Line::from(vec![
            Span::styled("    -> ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("@{}", t),
                Style::default()
                    .fg(theme.config_title)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    // 消息预览
    if let Some(msg) = &message {
        for wrapped in wrap_text(msg, 80) {
            lines.push(Line::from(Span::styled(
                format!("       {}", wrapped),
                Style::default().fg(theme.text_dim),
            )));
        }
    } else {
        // 无消息预览，显示原始 content
        for line in content.lines().take(NORMAL_RESULT_MAX_LINES) {
            lines.push(Line::from(Span::styled(
                format!("    {}", line),
                Style::default().fg(theme.text_dim),
            )));
        }
    }
}
