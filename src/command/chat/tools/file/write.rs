use crate::command::chat::tools::{Tool, ToolResult, expand_tilde};
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};

/// 写入文件的工具
pub struct WriteFileTool;

impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "Write"
    }

    fn description(&self) -> &str {
        "Write content to a specified file. Overwrites if the file exists; auto-creates directories if they don't exist."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path to write (absolute or relative to current working directory)"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let v = match serde_json::from_str::<Value>(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        let path = match v.get("path").and_then(|c| c.as_str()) {
            Some(p) => expand_tilde(p),
            None => {
                return ToolResult {
                    output: "参数缺少 path 字段".to_string(),
                    is_error: true,
                };
            }
        };

        let content = match v.get("content").and_then(|c| c.as_str()) {
            Some(c) => c.to_string(),
            None => {
                return ToolResult {
                    output: "参数缺少 content 字段".to_string(),
                    is_error: true,
                };
            }
        };

        // 自动创建父目录
        let file_path = std::path::Path::new(&path);
        if let Some(parent) = file_path.parent()
            && !parent.exists()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            return ToolResult {
                output: format!("创建目录失败: {}", e),
                is_error: true,
            };
        }

        match std::fs::write(&path, &content) {
            Ok(_) => ToolResult {
                output: format!("已写入文件: {} ({} 字节)", path, content.len()),
                is_error: false,
            },
            Err(e) => ToolResult {
                output: format!("写入文件失败: {}", e),
                is_error: true,
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        let path = serde_json::from_str::<Value>(arguments)
            .ok()
            .and_then(|v| v.get("path").and_then(|c| c.as_str()).map(expand_tilde))
            .unwrap_or_else(|| "未知路径".to_string());
        format!("即将写入文件: {}", path)
    }
}
