use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObject};
use serde_json::{Value, json};

use super::skill::Skill;

/// 工具执行结果
pub struct ToolResult {
    /// 返回给 LLM 的内容
    pub output: String,
    /// 是否执行出错
    pub is_error: bool,
}

/// 工具 trait
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    /// 执行工具（同步）
    fn execute(&self, arguments: &str) -> ToolResult;
    /// 是否需要用户确认（shell 命令需要，文件读取不需要）
    fn requires_confirmation(&self) -> bool {
        false
    }
    /// 生成确认提示文字（供 TUI 展示）
    fn confirmation_message(&self, arguments: &str) -> String {
        format!("调用工具 {} 参数: {}", self.name(), arguments)
    }
}

// ========== run_shell ==========

/// 执行 shell 命令的工具
pub struct ShellTool;

/// 简单的危险命令过滤
fn is_dangerous_command(cmd: &str) -> bool {
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

impl Tool for ShellTool {
    fn name(&self) -> &str {
        "run_shell"
    }

    fn description(&self) -> &str {
        "在当前系统上执行 shell 命令，返回命令的 stdout 和 stderr 输出；注意每次调用 run_shell 都会创建一个新的进程，状态是不延续的"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "要执行的 shell 命令（在 bash 中执行）"
                }
            },
            "required": ["command"]
        })
    }

    fn execute(&self, arguments: &str) -> ToolResult {
        let command = match serde_json::from_str::<Value>(arguments) {
            Ok(v) => match v.get("command").and_then(|c| c.as_str()) {
                Some(cmd) => cmd.to_string(),
                None => {
                    return ToolResult {
                        output: "参数缺少 command 字段".to_string(),
                        is_error: true,
                    };
                }
            },
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        // 安全过滤
        if is_dangerous_command(&command) {
            return ToolResult {
                output: "该命令被安全策略拒绝执行".to_string(),
                is_error: true,
            };
        }

        match std::process::Command::new("bash")
            .arg("-c")
            .arg(&command)
            .output()
        {
            Ok(output) => {
                let mut result = String::new();
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if !stdout.is_empty() {
                    result.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !result.is_empty() {
                        result.push_str("\n[stderr]\n");
                    } else {
                        result.push_str("[stderr]\n");
                    }
                    result.push_str(&stderr);
                }

                if result.is_empty() {
                    result = "(无输出)".to_string();
                }

                // 截断到 4000 字节
                const MAX_BYTES: usize = 4000;
                let truncated = if result.len() > MAX_BYTES {
                    let mut end = MAX_BYTES;
                    while !result.is_char_boundary(end) {
                        end -= 1;
                    }
                    format!("{}\n...(输出已截断)", &result[..end])
                } else {
                    result
                };

                let is_error = !output.status.success();
                ToolResult {
                    output: truncated,
                    is_error,
                }
            }
            Err(e) => ToolResult {
                output: format!("执行失败: {}", e),
                is_error: true,
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        // 尝试解析 command 字段
        let cmd = serde_json::from_str::<Value>(arguments)
            .ok()
            .and_then(|v| {
                v.get("command")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| arguments.to_string());
        format!("即将执行: {}", cmd)
    }
}

// ========== read_file ==========

/// 读取文件的工具
pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "读取本地文件内容并返回。文件路径必须是绝对路径或相对于当前工作目录的路径。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要读取的文件路径"
                }
            },
            "required": ["path"]
        })
    }

    fn execute(&self, arguments: &str) -> ToolResult {
        let path = match serde_json::from_str::<Value>(arguments) {
            Ok(v) => match v.get("path").and_then(|c| c.as_str()) {
                Some(p) => p.to_string(),
                None => {
                    return ToolResult {
                        output: "参数缺少 path 字段".to_string(),
                        is_error: true,
                    };
                }
            },
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                // 截断到 8000 字节
                const MAX_BYTES: usize = 8000;
                let truncated = if content.len() > MAX_BYTES {
                    let mut end = MAX_BYTES;
                    while !content.is_char_boundary(end) {
                        end -= 1;
                    }
                    format!("{}\n...(文件内容已截断)", &content[..end])
                } else {
                    content
                };
                ToolResult {
                    output: truncated,
                    is_error: false,
                }
            }
            Err(e) => ToolResult {
                output: format!("读取文件失败: {}", e),
                is_error: true,
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ========== ToolRegistry ==========

/// 工具注册表
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    /// 创建注册表（包含 run_shell、read_file，以及当 skills 非空时注册 load_skill）
    pub fn new(skills: Vec<Skill>) -> Self {
        let mut registry = Self {
            tools: vec![Box::new(ShellTool), Box::new(ReadFileTool)],
        };

        // 如果有 skills，注册统一的 LoadSkillTool
        if !skills.is_empty() {
            registry.register(Box::new(super::skill::LoadSkillTool { skills }));
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

    /// 构建工具摘要列表，用于系统提示词的 {{.tools}} 占位符
    pub fn build_tools_summary(&self) -> String {
        self.tools
            .iter()
            .map(|t| format!("- **{}**: {}", t.name(), t.description()))
            .collect::<Vec<_>>()
            .join("\n")
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
