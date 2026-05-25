//! 工具调用渲染共享工具函数

use crate::command::chat::constants::TOOL_ARG_PREVIEW_MAX_CHARS;
use crate::command::chat::render::theme::Theme;
use crate::command::chat::tools::classification::format_json_value;
use crate::util::text::{sanitize_single_line_text, wrap_text};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

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
        crate::command::chat::tools::tool_names::SHELL => {
            parsed.get("description")?.as_str().map(|s| s.to_string())
        }
        crate::command::chat::tools::tool_names::READ
        | crate::command::chat::tools::tool_names::WRITE
        | crate::command::chat::tools::tool_names::EDIT
        | crate::command::chat::tools::tool_names::GLOB
        | crate::command::chat::tools::tool_names::GREP => parsed
            .get("path")
            .or_else(|| parsed.get("file_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),

        // ── Agent / Teammate ──
        crate::command::chat::tools::tool_names::AGENT => parsed
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        crate::command::chat::tools::tool_names::TEAMMATE => {
            let name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let role = parsed.get("role").and_then(|v| v.as_str()).unwrap_or(name);
            Some(role.to_string())
        }

        // ── Task（任务管理）──
        crate::command::chat::tools::tool_names::TASK => {
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
        crate::command::chat::tools::tool_names::TASK_OUTPUT => {
            let task_id = parsed
                .get("task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            Some(format!("获取任务 {} 输出", task_id))
        }

        // ── 网络 ──
        crate::command::chat::tools::tool_names::WEB_SEARCH => parsed
            .get("query")
            .and_then(|v| v.as_str())
            .map(|s| format!("搜索: {}", s)),
        crate::command::chat::tools::tool_names::WEB_FETCH => parsed
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        crate::command::chat::tools::tool_names::BROWSER => parsed
            .get("url")
            .or_else(|| parsed.get("action"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),

        // ── Ask（用户提问）──
        crate::command::chat::tools::tool_names::ASK => {
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
        crate::command::chat::tools::tool_names::TODO_WRITE => {
            if let Some(todos) = parsed.get("todos").and_then(|v| v.as_array()) {
                let count = todos.len();
                Some(format!("更新 {} 项待办", count))
            } else {
                Some("更新待办".to_string())
            }
        }
        crate::command::chat::tools::tool_names::TODO_READ => Some("读取待办列表".to_string()),

        // ── Compact（对话压缩）──
        crate::command::chat::tools::tool_names::COMPACT => {
            let focus = parsed.get("focus").and_then(|v| v.as_str());
            match focus {
                Some(f) => Some(format!("压缩对话 (focus: {})", f)),
                None => Some("压缩对话".to_string()),
            }
        }

        // ── Plan ──
        crate::command::chat::tools::tool_names::ENTER_PLAN_MODE => parsed
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| format!("进入计划模式: {}", s))
            .or_else(|| Some("进入计划模式".to_string())),
        crate::command::chat::tools::tool_names::EXIT_PLAN_MODE => Some("提交计划审批".to_string()),

        // ── LoadSkill ──
        crate::command::chat::tools::tool_names::LOAD_SKILL => parsed
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| format!("加载技能: {}", s)),

        // ── RegisterHook ──
        crate::command::chat::tools::tool_names::REGISTER_HOOK => {
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
        crate::command::chat::tools::tool_names::SEND_MESSAGE => {
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
        crate::command::chat::tools::tool_names::ENTER_WORKTREE => parsed
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| format!("进入工作树: {}", s))
            .or_else(|| Some("进入工作树".to_string())),
        crate::command::chat::tools::tool_names::EXIT_WORKTREE => Some("退出工作树".to_string()),

        // ── WorkDone ──
        crate::command::chat::tools::tool_names::WORK_DONE => parsed
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| truncate_str(s, TOOL_ARG_PREVIEW_MAX_CHARS))
            .or_else(|| Some("工作完成".to_string())),

        // ── IgnoreMessage ──
        crate::command::chat::tools::tool_names::IGNORE_MESSAGE => Some("忽略消息".to_string()),

        // ── ComputerUse ──
        #[cfg(target_os = "macos")]
        crate::command::chat::tools::tool_names::COMPUTER_USE => parsed
            .get("action")
            .and_then(|v| v.as_str())
            .map(|s| format!("计算机操作: {}", s)),

        // ── LoadTool ──
        crate::command::chat::tools::tool_names::LOAD_TOOL => parsed
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| format!("加载工具: {}", s)),

        // ── Session ──
        crate::command::chat::tools::tool_names::SESSION => parsed
            .get("action")
            .and_then(|v| v.as_str())
            .map(|s| format!("会话: {}", s))
            .or_else(|| Some("会话操作".to_string())),

        _ => None,
    }
}

/// 将字符串截断到最大字符数，超出时加 "…"
pub(crate) fn truncate_str(s: &str, max_chars: usize) -> String {
    let s = sanitize_single_line_text(s);
    if s.chars().count() <= max_chars {
        s
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}…", truncated)
    }
}

/// 截断长路径（保留首尾）
pub(crate) fn truncate_path(path: &str, max_len: usize) -> String {
    if path.chars().count() > max_len {
        let first_part: String = path.chars().take(30).collect();
        let last_part: String = path.chars().rev().take(25).collect();
        format!(
            "{}...{}",
            first_part,
            last_part.chars().rev().collect::<String>()
        )
    } else {
        path.to_string()
    }
}

/// 摘要字符串内容（多行显示行数 + 首行预览，单行显示长度 + 截断预览）
pub(crate) fn summarize_string_content(s: &str, preview_len: usize) -> String {
    let line_count = s.lines().count();
    if line_count > 1 {
        // 多行：显示行数 + 首行预览
        let first_line = s.lines().next().unwrap_or("");
        let preview = if first_line.chars().count() > preview_len {
            format!(
                "{}...",
                first_line.chars().take(preview_len).collect::<String>()
            )
        } else {
            first_line.to_string()
        };
        format!("{} 行: \"{}\"", line_count, preview)
    } else {
        // 单行：显示长度 + 截断预览
        let char_count = s.chars().count();
        let preview: String = if char_count > preview_len {
            format!("{}...", s.chars().take(preview_len).collect::<String>())
        } else if char_count == 0 {
            String::from("(空)")
        } else {
            s.to_string()
        };
        if char_count > preview_len {
            format!("{} 字符: \"{}\"", char_count, preview)
        } else {
            format!("\"{}\"", preview)
        }
    }
}

/// 渲染键值对行
pub(crate) fn render_kv_line(
    key: &str,
    value: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let key = sanitize_single_line_text(key);
    let value = sanitize_single_line_text(value);
    let max_val_chars = content_w.saturating_sub(key.chars().count() + 7);
    let display = if value.chars().count() > max_val_chars {
        format!("{}…", truncate_str(&value, max_val_chars))
    } else {
        value
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
pub(crate) fn render_tag_line(
    tag: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let tag = sanitize_single_line_text(tag);
    for wrapped in wrap_text(&tag, content_w) {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(wrapped, Style::default().fg(theme.text_dim)),
        ]));
    }
}
