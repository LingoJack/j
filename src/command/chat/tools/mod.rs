mod ask;
mod browser;
mod file;
pub(crate) mod html_extract;
mod new_task;
mod shell;
mod skill_tool;
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
        queued_tasks: Arc<Mutex<Vec<String>>>,
    ) -> Self {
        let mut registry = Self {
            tools: vec![
                Box::new(shell::ShellTool),
                Box::new(file::ReadFileTool),
                Box::new(file::WriteFileTool),
                Box::new(file::EditFileTool),
                Box::new(web_fetch::WebFetchTool),
                Box::new(web_search::WebSearchTool),
                Box::new(browser::BrowserTool),
                Box::new(ask::AskTool { ask_tx }),
                Box::new(new_task::NewTaskTool { queued_tasks }),
            ],
        };

        // 如果有 skills，注册统一的 LoadSkillTool
        if !skills.is_empty() {
            registry.register(Box::new(self::skill_tool::LoadSkillTool { skills }));
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

    /// 构建工具摘要列表，用于系统提示词的 {{.tools}} 占位符（JSON 数组格式，含参数 schema）
    pub fn build_tools_summary(&self) -> String {
        let items: Vec<serde_json::Value> = self
            .tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name(),
                    "description": t.description(),
                    "parameters": t.parameters_schema()
                })
            })
            .collect();
        serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".to_string())
    }

    /// 生成 async-openai 的 ChatCompletionTools 列表
    pub fn to_openai_tools(&self) -> Vec<ChatCompletionTools> {
        self.tools
            .iter()
            .map(|t| {
                ChatCompletionTools::Function(ChatCompletionTool {
                    function: FunctionObject {
                        name: t.name().to_string(),
                        description: Some(t.description().to_string()),
                        parameters: Some(t.parameters_schema()),
                        strict: None,
                    },
                })
            })
            .collect()
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
