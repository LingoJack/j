use crate::command::chat::theme::Theme;
use ratatui::style::Color;

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
            "Read" | "Write" | "Edit" | "Glob" | "FileRead" | "FileWrite" | "FileEdit" => {
                Self::File
            }
            "Grep" | "GrepTool" => Self::Search,
            "Bash" | "Task" | "TaskOutput" | "TaskCreate" | "TaskUpdate" | "TaskGet" => {
                Self::Execute
            }
            "WebFetch" | "WebSearch" | "WebBrowser" => Self::Network,
            "EnterPlanMode" | "ExitPlanMode" => Self::Plan,
            "Agent" => Self::Agent,
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
            if char_count > 50 {
                let truncated: String = s.chars().take(47).collect();
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

/// 获取结果摘要
pub fn get_result_summary(content: &str, is_error: bool) -> String {
    if is_error {
        return "失败".to_string();
    }

    if content.is_empty() {
        return "无输出".to_string();
    }

    // 统计信息
    let lines = content.lines().count();
    let chars = content.chars().count();

    if lines > 1 {
        if chars > 1024 {
            format!("{} 行, {:.1}KB", lines, chars as f64 / 1024.0)
        } else {
            format!("{} 行, {} 字符", lines, chars)
        }
    } else if chars > 100 {
        format!("{:.1}KB", chars as f64 / 1024.0)
    } else {
        format!("{} 字符", chars)
    }
}

/// 格式化执行时间
#[allow(dead_code)]
pub fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{:.1}m", ms as f64 / 60000.0)
    }
}
