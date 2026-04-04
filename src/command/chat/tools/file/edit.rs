use crate::command::chat::tools::{
    Tool, ToolResult, expand_tilde, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, atomic::AtomicBool};

/// EditFileTool 参数
#[derive(Deserialize, JsonSchema)]
struct EditFileParams {
    /// File path to edit
    path: String,
    /// Original string to replace (must be unique in the file)
    old_string: String,
    /// Replacement string; empty string means delete
    #[serde(default)]
    new_string: String,
}

/// 编辑文件的工具（基于字符串替换）
pub struct EditFileTool;

impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "Edit"
    }

    fn description(&self) -> &str {
        r#"
        Performs exact string replacements in files.

        Usage:
        - You must use your Read tool at least once in the conversation before editing. Read the file first to understand its content
        - When editing text from Read tool output, ensure you preserve the exact indentation (tabs/spaces). The line number prefix format is: number + │. Everything after │ is the actual file content to match. Never include any part of the line number prefix in old_string or new_string
        - ALWAYS prefer editing existing files in the codebase. NEVER write new files unless explicitly required
        - Only use emojis if the user explicitly requests it
        - The edit will FAIL if old_string is not unique in the file. Either provide a larger string with more surrounding context to make it unique, or break the edit into smaller unique chunks
        - If new_string is empty, the matched content is deleted
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<EditFileParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: EditFileParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let path = expand_tilde(&params.path);

        // 读取文件
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    output: format!("读取文件失败: {}", e),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        // 检查匹配次数
        let count = content.matches(&params.old_string).count();
        if count == 0 {
            return ToolResult {
                output: "未找到匹配的字符串".to_string(),
                is_error: true,
                images: vec![],
            };
        }
        if count > 1 {
            return ToolResult {
                output: format!(
                    "old_string 在文件中匹配了 {} 次，必须唯一匹配。请提供更多上下文使其唯一",
                    count
                ),
                is_error: true,
                images: vec![],
            };
        }

        // 执行替换
        let new_content = content.replacen(&params.old_string, &params.new_string, 1);
        match std::fs::write(&path, &new_content) {
            Ok(_) => {
                // 构建含 diff 的输出
                let mut output = format!("已编辑文件: {}\n```diff\n", path);
                for line in params.old_string.lines() {
                    output.push_str(&format!("- {}\n", line));
                }
                for line in params.new_string.lines() {
                    output.push_str(&format!("+ {}\n", line));
                }
                output.push_str("```");
                ToolResult {
                    output,
                    is_error: false,
                    images: vec![],
                }
            }
            Err(e) => ToolResult {
                output: format!("写入文件失败: {}", e),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        if let Ok(params) = serde_json::from_str::<EditFileParams>(arguments) {
            let path = expand_tilde(&params.path);
            let first_line = params.old_string.lines().next().unwrap_or("");
            let has_more = params.old_string.lines().count() > 1;
            let preview = if has_more {
                format!("{}...", first_line)
            } else {
                first_line.to_string()
            };
            format!("即将编辑文件 {} (替换: \"{}\")", path, preview)
        } else {
            "即将编辑文件".to_string()
        }
    }
}
