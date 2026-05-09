use crate::constants::{
    CLASSIFY_SIZE_THRESHOLD_BYTES, CLASSIFY_SIZE_THRESHOLD_CHARS, CLASSIFY_TITLE_TRUNCATE_LEN,
    CLASSIFY_TRUNCATE_LEN, HOOK_LOG_DESC_MAX_LEN,
};

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
    /// 协作者类 (Teammate)
    Teammate,
    /// 压缩类 (Compact)
    Compact,
    /// 发送消息 (SendMessage)
    SendMessage,
    /// 忽略消息 (IgnoreMessage)
    IgnoreMessage,
    /// 工作完成 (WorkDone)
    WorkDone,
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
            tool_names::TEAMMATE => Self::Teammate,
            tool_names::COMPACT => Self::Compact,
            tool_names::SEND_MESSAGE => Self::SendMessage,
            tool_names::IGNORE_MESSAGE => Self::IgnoreMessage,
            tool_names::WORK_DONE => Self::WorkDone,
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
            Self::Teammate => "👥",
            Self::Compact => "📦",
            Self::SendMessage => "✉️",
            Self::IgnoreMessage => "💤",
            Self::WorkDone => "🚩",
            Self::Other => "🔧",
        }
    }
}

/// 工具执行状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolStatus {
    /// 成功完成
    Success,
    /// 失败
    Failed,
}

impl ToolStatus {
    /// 状态图标
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Success => "✓",
            Self::Failed => "✗",
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
        tool_names::AGENT => get_agent_summary(content, tool_args),
        tool_names::TEAMMATE => get_teammate_summary(content, tool_args),
        tool_names::COMPACT => get_compact_summary(content),
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

/// TodoWrite 工具摘要：显示操作描述
fn get_todo_write_summary(_content: &str, tool_args: Option<&str>) -> String {
    tool_args
        .and_then(|args| serde_json::from_str::<serde_json::Value>(args).ok())
        .map(|v| {
            let is_merge = v.get("merge").and_then(|m| m.as_bool()).unwrap_or(false);
            let count = v
                .get("todos")
                .and_then(|t| t.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            if is_merge {
                format!("更新 {} 项待办", count)
            } else {
                format!("写入 {} 项待办", count)
            }
        })
        .unwrap_or_else(|| "写入待办".to_string())
}

/// TodoRead 工具摘要：显示读取数量
fn get_todo_read_summary(content: &str) -> String {
    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(content) {
        format!("读取 {} 项待办", items.len())
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
/// Agent 工具摘要：提取 description + 首行输出
fn get_agent_summary(content: &str, tool_args: Option<&str>) -> String {
    let lines = content.lines().count();
    let desc = tool_args
        .and_then(|args| serde_json::from_str::<serde_json::Value>(args).ok())
        .and_then(|v| {
            v.get("description")
                .and_then(|d| d.as_str().map(|s| s.to_string()))
        });

    // 首行非空内容作为摘要
    let first_line = content.lines().find(|l| !l.trim().is_empty()).unwrap_or("");

    if let Some(d) = desc {
        let max_d: String = d.chars().take(20).collect();
        if first_line.is_empty() {
            max_d
        } else {
            let max_f: String = first_line.chars().take(40).collect();
            format!("{}: {}", max_d, max_f)
        }
    } else if first_line.is_empty() {
        format!("{} 行", lines)
    } else {
        let max_f: String = first_line.chars().take(50).collect();
        max_f
    }
}

/// Teammate 工具摘要：提取 name + 首行输出
fn get_teammate_summary(content: &str, tool_args: Option<&str>) -> String {
    let name = tool_args
        .and_then(|args| serde_json::from_str::<serde_json::Value>(args).ok())
        .and_then(|v| {
            v.get("name")
                .and_then(|n| n.as_str().map(|s| s.to_string()))
        });

    let first_line = content.lines().find(|l| !l.trim().is_empty()).unwrap_or("");

    if let Some(n) = name {
        if first_line.is_empty() {
            n
        } else {
            let max_f: String = first_line.chars().take(40).collect();
            format!("{}: {}", n, max_f)
        }
    } else if first_line.is_empty() {
        "完成".to_string()
    } else {
        let max_f: String = first_line.chars().take(50).collect();
        max_f
    }
}

/// Compact 工具摘要：提取压缩信息
fn get_compact_summary(content: &str) -> String {
    // 内容格式: "📦 上下文已压缩 (N 条消息 → 摘要, transcript: path)"
    // 直接取第一行作为摘要
    content
        .lines()
        .next()
        .map(|l| {
            let chars: String = l.chars().take(HOOK_LOG_DESC_MAX_LEN).collect();
            chars
        })
        .unwrap_or_else(|| "压缩完成".to_string())
}

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
