//! 工具结果渲染共享工具函数

use crate::command::chat::render::cache::bubble::bordered_line;
use crate::command::chat::render::theme::Theme;
use crate::util::text::wrap_text;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

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
pub(crate) fn parse_file_path_from_json(json: &str) -> Option<String> {
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
pub(crate) fn infer_lang_from_path(path: &str) -> &'static str {
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
    let max_display = crate::command::chat::constants::AGENT_RESULT_MAX_LINES;
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
