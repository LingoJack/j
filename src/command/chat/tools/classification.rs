use crate::command::chat::constants::{
    CLASSIFY_SIZE_THRESHOLD_BYTES, CLASSIFY_SIZE_THRESHOLD_CHARS, CLASSIFY_TITLE_TRUNCATE_LEN,
    CLASSIFY_TRUNCATE_LEN,
};
use crate::command::chat::theme::Theme;
use ratatui::style::Color;

use super::tool_names;

/// 工具类型分类
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolCategory {
    /// 文件操作类 (Read, Write, Edit, Glob)
    File,
    /// 搜索类 (Grep)
    Search,
    /// 执行类 (Bash, Task, TaskOutput)
    Execute,
    /// 网络类 (WebFetch, WebSearch)
    Network,
    /// 计划类 (EnterPlanMode, ExitPlanMode)
    Plan,
    /// 代理类 (Agent)
    Agent,
    /// 其他类
    Other,
}

impl ToolCategory {
    /// 根据工具名称判断分类
    pub fn from_name(name: &str) -> Self {
        match name {
            tool_names::READ | tool_names::WRITE | tool_names::EDIT | tool_names::GLOB => {
                Self::File
            }
            tool_names::GREP => Self::Search,
            tool_names::BASH | tool_names::TASK | tool_names::TASK_OUTPUT => Self::Execute,
            tool_names::WEB_FETCH | tool_names::WEB_SEARCH | tool_names::BROWSER => Self::Network,
            tool_names::ENTER_PLAN_MODE | tool_names::EXIT_PLAN_MODE => Self::Plan,
            tool_names::AGENT => Self::Agent,
            _ => Self::Other,
        }
    }

    /// 获取工具图标
    pub fn icon(&self) -> &'static str {
        match self {
            Self::File => "📄",
            Self::Search => "🔍",
            Self::Execute => "⚡",
            Self::Network => "🌐",
            Self::Plan => "📋",
            Self::Agent => "🤖",
            Self::Other => "🔧",
        }
    }

    /// 获取工具颜色（从主题）
    pub fn color(&self, theme: &Theme) -> Color {
        match self {
            Self::File => theme.label_user,       // 蓝色系
            Self::Search => theme.label_ai,       // 绿色系
            Self::Execute => theme.title_loading, // 黄/橙色系
            Self::Network => theme.config_title,  // 青色系
            Self::Plan => theme.label_ai,         // 绿色系
            Self::Agent => theme.title_loading,   // 黄/橙色系
            Self::Other => theme.text_dim,        // 灰色
        }
    }
}

/// 工具执行状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolStatus {
    /// 等待确认
    Pending,
    /// 执行中
    #[allow(dead_code)]
    Running,
    /// 成功完成
    Success,
    /// 失败
    Failed,
    /// 被拒绝
    #[allow(dead_code)]
    Rejected,
}

impl ToolStatus {
    /// 状态图标
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pending => "⏳",
            Self::Running => "⏱",
            Self::Success => "✓",
            Self::Failed => "✗",
            Self::Rejected => "⊘",
        }
    }

    /// 状态颜色
    pub fn color(&self, theme: &Theme) -> Color {
        match self {
            Self::Pending => theme.title_loading,
            Self::Running => theme.title_loading,
            Self::Success => theme.label_ai,
            Self::Failed => theme.toast_error_border,
            Self::Rejected => theme.tool_confirm_border,
        }
    }

    /// 状态文字
    #[allow(dead_code)]
    pub fn text(&self) -> &'static str {
        match self {
            Self::Pending => "等待确认",
            Self::Running => "执行中",
            Self::Success => "成功",
            Self::Failed => "失败",
            Self::Rejected => "已拒绝",
        }
    }
}

/// 格式化 JSON 值为简短显示
pub fn format_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => {
            // 使用字符数而不是字节数来截断，避免 UTF-8 边界问题
            let char_count = s.chars().count();
            if char_count > CLASSIFY_TRUNCATE_LEN {
                let truncated: String = s.chars().take(CLASSIFY_TRUNCATE_LEN - 3).collect();
                format!("\"{}...\"", truncated)
            } else {
                format!("\"{}\"", s)
            }
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else {
                format!("[{} items]", arr.len())
            }
        }
        serde_json::Value::Object(obj) => {
            if obj.is_empty() {
                "{}".to_string()
            } else {
                let keys: Vec<&str> = obj.keys().take(3).map(|s| s.as_str()).collect();
                format!("{{{}}}", keys.join(", "))
            }
        }
    }
}

/// 获取工具特性化结果摘要
pub fn get_result_summary_for_tool(
    content: &str,
    is_error: bool,
    tool_name: &str,
    tool_args: Option<&str>,
) -> String {
    if is_error {
        return "失败".to_string();
    }

    if content.is_empty() {
        return "无输出".to_string();
    }

    // 工具特性化摘要
    match tool_name {
        tool_names::READ => get_read_summary(content, tool_args),
        tool_names::BASH => get_bash_summary(content, tool_args),
        tool_names::TODO_WRITE => get_todo_write_summary(content, tool_args),
        tool_names::TODO_READ => get_todo_read_summary(content),
        tool_names::TASK => get_task_summary(content, tool_args),
        _ => get_generic_summary(content),
    }
}

/// Read 工具摘要：显示文件路径和行数
fn get_read_summary(content: &str, tool_args: Option<&str>) -> String {
    let lines = content.lines().count();
    let file_path = tool_args
        .and_then(|args| serde_json::from_str::<serde_json::Value>(args).ok())
        .and_then(|v| {
            v.get("file_path")
                .and_then(|p| p.as_str().map(|s| s.to_string()))
        });

    if let Some(path) = file_path {
        // 只取文件名部分，避免过长
        let short = short_path(&path, 40);
        format!("{} ({} 行)", short, lines)
    } else {
        format!("{} 行", lines)
    }
}

/// Bash 工具摘要：显示命令预览
fn get_bash_summary(content: &str, tool_args: Option<&str>) -> String {
    let command = tool_args
        .and_then(|args| serde_json::from_str::<serde_json::Value>(args).ok())
        .and_then(|v| {
            v.get("command")
                .and_then(|c| c.as_str().map(|s| s.to_string()))
        });

    let lines = content.lines().count();
    let line_info = if lines > 1 {
        format!(" ({} 行输出)", lines)
    } else {
        String::new()
    };

    if let Some(cmd) = command {
        // 截取命令的第一行前 50 字符
        let first_line = cmd.lines().next().unwrap_or(&cmd);
        let short_cmd: String = first_line.chars().take(CLASSIFY_TRUNCATE_LEN).collect();
        let suffix = if first_line.chars().count() > CLASSIFY_TRUNCATE_LEN {
            "…"
        } else {
            ""
        };
        format!("{}{}{}", short_cmd, suffix, line_info)
    } else {
        format!("完成{}", line_info)
    }
}

/// TodoWrite 工具摘要：从结果内容统计状态（与 TodoRead 一致）
fn get_todo_write_summary(content: &str, tool_args: Option<&str>) -> String {
    // TodoWrite 返回更新后的全部 todo 列表，用相同的统计逻辑
    let action_prefix = tool_args
        .and_then(|args| serde_json::from_str::<serde_json::Value>(args).ok())
        .map(|v| {
            let is_merge = v.get("merge").and_then(|m| m.as_bool()).unwrap_or(false);
            let count = v
                .get("todos")
                .and_then(|t| t.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            if is_merge {
                format!("更新 {} 项 → ", count)
            } else {
                format!("写入 {} 项 → ", count)
            }
        })
        .unwrap_or_default();

    let status_summary = get_todo_status_summary(content);
    format!("{}{}", action_prefix, status_summary)
}

/// TodoRead 工具摘要：解析 JSON 内容统计状态
fn get_todo_read_summary(content: &str) -> String {
    get_todo_status_summary(content)
}

/// 从 todo JSON 列表中统计状态摘要
fn get_todo_status_summary(content: &str) -> String {
    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(content) {
        let total = items.len();
        let completed = items
            .iter()
            .filter(|i| i.get("status").and_then(|s| s.as_str()) == Some("completed"))
            .count();
        let in_progress = items
            .iter()
            .filter(|i| i.get("status").and_then(|s| s.as_str()) == Some("in_progress"))
            .count();
        let pending = total.saturating_sub(completed + in_progress);

        let mut parts = Vec::new();
        if pending > 0 {
            parts.push(format!("⬜{}", pending));
        }
        if in_progress > 0 {
            parts.push(format!("[~]{}", in_progress));
        }
        if completed > 0 {
            parts.push(format!("☑️{}", completed));
        }
        format!("{} 项 ({})", total, parts.join(" "))
    } else {
        get_generic_summary(content)
    }
}

/// Task 工具摘要
fn get_task_summary(content: &str, tool_args: Option<&str>) -> String {
    let parsed = tool_args.and_then(|args| serde_json::from_str::<serde_json::Value>(args).ok());

    if let Some(ref v) = parsed {
        let action = v.get("action").and_then(|a| a.as_str()).unwrap_or("");
        match action {
            "create" => {
                let title = v
                    .get("title")
                    .and_then(|t| t.as_str())
                    .unwrap_or("untitled");
                let short: String = title.chars().take(CLASSIFY_TITLE_TRUNCATE_LEN).collect();
                format!("create: \"{}\"", short)
            }
            "list" => {
                // 从 content 中尝试统计任务数
                let count = content.lines().filter(|l| l.contains("\"id\"")).count();
                if count > 0 {
                    format!("list: {} 项任务", count)
                } else {
                    "list".to_string()
                }
            }
            "get" => {
                let task_id = v
                    .get("taskId")
                    .and_then(|t| t.as_u64())
                    .map(|id| format!("#{}", id))
                    .unwrap_or_default();
                format!("get {}", task_id)
            }
            "update" => {
                let task_id = v
                    .get("taskId")
                    .and_then(|t| t.as_u64())
                    .map(|id| format!("#{}", id))
                    .unwrap_or_default();
                let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("");
                if !status.is_empty() {
                    format!("update {} -> {}", task_id, status)
                } else {
                    format!("update {}", task_id)
                }
            }
            _ => get_generic_summary(content),
        }
    } else {
        get_generic_summary(content)
    }
}

/// 通用摘要（原有逻辑）
fn get_generic_summary(content: &str) -> String {
    let lines = content.lines().count();
    let chars = content.chars().count();

    if lines > 1 {
        if chars > CLASSIFY_SIZE_THRESHOLD_BYTES {
            format!("{} 行, {:.1}KB", lines, chars as f64 / 1024.0)
        } else {
            format!("{} 行, {} 字符", lines, chars)
        }
    } else if chars > CLASSIFY_SIZE_THRESHOLD_CHARS {
        format!("{:.1}KB", chars as f64 / 1024.0)
    } else {
        format!("{} 字符", chars)
    }
}

/// 截断路径，保留文件名和部分目录
fn short_path(path: &str, max_len: usize) -> String {
    if path.chars().count() <= max_len {
        return path.to_string();
    }
    // 取最后几个路径段
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 2 {
        let truncated: String = path.chars().take(max_len.saturating_sub(1)).collect();
        return format!("{}…", truncated);
    }
    // 保留最后 2-3 段
    let mut result = String::new();
    for i in (0..parts.len()).rev() {
        let candidate = parts[i..].join("/");
        if candidate.chars().count() + 2 > max_len {
            break;
        }
        result = candidate;
    }
    if result.is_empty() {
        result = parts.last().unwrap_or(&"").to_string();
    }
    format!("…/{}", result)
}
