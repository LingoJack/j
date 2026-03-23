mod ask;
pub mod background;
mod browser;
pub mod compact;
mod file;
mod grep;
pub mod hook;
mod shell;
mod skill;
pub mod task;
mod web_fetch;
mod web_search;

use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObject};
use serde_json::Value;
use std::sync::{Arc, Mutex, atomic::AtomicBool, mpsc};

// ========== ToolResult ==========

/// 工具执行结果
pub struct ToolResult {
    /// 返回给 LLM 的内容
    pub output: String,
    /// 是否执行出错
    pub is_error: bool,
}

// ========== Tool trait ==========

/// 工具 trait
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    /// 执行工具（同步）；cancelled 为取消信号，支持提前终止
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult;
    /// 是否需要用户确认（shell 命令需要，文件读取不需要）
    fn requires_confirmation(&self) -> bool {
        false
    }
    /// 生成确认提示文字（供 TUI 展示）
    fn confirmation_message(&self, arguments: &str) -> String {
        format!("调用工具 {} 参数: {}", self.name(), arguments)
    }
}

// ========== ToolRegistry ==========

/// 工具注册表
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    /// 创建注册表（包含内置工具，以及当 skills 非空时注册 load_skill）
    pub fn new(
        skills: Vec<crate::command::chat::skill::Skill>,
        ask_tx: mpsc::Sender<crate::command::chat::app::AskRequest>,
        background_manager: Arc<background::BackgroundManager>,
        task_manager: Arc<task::TaskManager>,
        hook_manager: Arc<Mutex<crate::command::chat::hook::HookManager>>,
    ) -> Self {
        let mut registry = Self {
            tools: vec![
                Box::new(shell::ShellTool),
                Box::new(file::ReadFileTool),
                Box::new(file::WriteFileTool),
                Box::new(file::EditFileTool),
                Box::new(file::GlobTool),
                Box::new(grep::GrepTool),
                Box::new(web_fetch::WebFetchTool),
                Box::new(web_search::WebSearchTool),
                Box::new(browser::BrowserTool),
                Box::new(ask::AskTool { ask_tx }),
                // 后台任务工具
                Box::new(background::BackgroundRunTool {
                    manager: Arc::clone(&background_manager),
                }),
                Box::new(background::CheckBackgroundTool {
                    manager: Arc::clone(&background_manager),
                }),
                // 任务管理工具
                Box::new(task::TaskCreateTool {
                    manager: Arc::clone(&task_manager),
                }),
                Box::new(task::TaskUpdateTool {
                    manager: Arc::clone(&task_manager),
                }),
                Box::new(task::TaskListTool {
                    manager: Arc::clone(&task_manager),
                }),
                Box::new(task::TaskGetTool {
                    manager: Arc::clone(&task_manager),
                }),
                // Context compact 工具
                Box::new(compact::CompactTool),
                // Hook 管理工具
                Box::new(hook::RegisterHookTool { hook_manager }),
            ],
        };

        // 如果有 skills，注册统一的 LoadSkillTool
        if !skills.is_empty() {
            registry.register(Box::new(self::skill::LoadSkillTool { skills }));
        }

        registry
    }

    /// 注册一个工具
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.push(tool);
    }

    /// 按名称获取工具
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    /// 按名称执行工具，返回结果（可在任何线程调用，ToolRegistry: Send + Sync）
    pub fn execute(&self, name: &str, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        match self.get(name) {
            Some(tool) => tool.execute(arguments, cancelled),
            None => ToolResult {
                output: format!("未知工具: {}", name),
                is_error: true,
            },
        }
    }

    /// 构建工具摘要列表，用于系统提示词的 {{.tools}} 占位符（Markdown 格式）
    /// 当 disabled 非空时，过滤掉其中列出的工具
    pub fn build_tools_summary(&self, disabled: &[String]) -> String {
        let mut md = String::new();
        for t in self
            .tools
            .iter()
            .filter(|t| !disabled.iter().any(|d| d == t.name()))
        {
            let name = t.name();
            md.push_str(&format!("<{}>\n", name));
            md.push_str(&format!("description:\n{}\n", t.description().trim()));
            md.push_str(&json_schema_to_xml_params(&t.parameters_schema()));
            md.push_str(&format!("<{}/>\n\n", name));
        }
        md.trim_end().to_string()
    }

    /// 生成过滤后的 ChatCompletionTools 列表（排除 disabled 中的工具）
    pub fn to_openai_tools_filtered(&self, disabled: &[String]) -> Vec<ChatCompletionTools> {
        self.tools
            .iter()
            .filter(|t| !disabled.iter().any(|d| d == t.name()))
            .map(|t| {
                ChatCompletionTools::Function(ChatCompletionTool {
                    function: FunctionObject {
                        name: t.name().to_string(),
                        description: Some(t.description().trim().to_string()),
                        parameters: Some(t.parameters_schema()),
                        strict: None,
                    },
                })
            })
            .collect()
    }

    /// 返回所有注册的工具名称（供 UI 使用）
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name()).collect()
    }
}

// ========== Helper functions ==========

/// 展开路径中的 ~ 为用户 home 目录
pub fn expand_tilde(path: &str) -> String {
    if path == "~" {
        std::env::var("HOME").unwrap_or_else(|_| "~".to_string())
    } else if let Some(rest) = path.strip_prefix("~/") {
        match std::env::var("HOME") {
            Ok(home) => format!("{}/{}", home, rest),
            Err(_) => path.to_string(),
        }
    } else {
        path.to_string()
    }
}

/// 将 JSON Schema 转为 Markdown 参数列表
fn json_schema_to_xml_params(schema: &Value) -> String {
    let properties = match schema.get("properties").and_then(|p| p.as_object()) {
        Some(p) => p,
        None => return String::new(),
    };
    let required: Vec<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut md = String::from("parameter schema:\n");
    for (name, prop) in properties {
        let type_str = prop
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("string");
        let desc = prop
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");
        let req = if required.contains(&name.as_str()) {
            ", required"
        } else {
            ""
        };
        md.push_str(&format!("- `{}` ({}{}) — {}\n", name, type_str, req, desc));
    }
    md
}

/// 简单的危险命令过滤
pub fn is_dangerous_command(cmd: &str) -> bool {
    let dangerous_patterns = [
        "rm -rf /",
        "rm -rf /*",
        "mkfs",
        "dd if=",
        ":(){:|:&};:",
        "chmod -R 777 /",
        "chown -R",
        "> /dev/sda",
        "wget -O- | sh",
        "curl | sh",
        "alias",
        "curl | bash",
    ];
    let cmd_lower = cmd.to_lowercase();
    for pat in &dangerous_patterns {
        if cmd_lower.contains(pat) {
            return true;
        }
    }
    false
}
