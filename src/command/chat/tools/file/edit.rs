use crate::command::chat::tools::{Tool, ToolResult, expand_tilde};
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};

/// 编辑文件的工具（基于字符串替换）
pub struct EditFileTool;

impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "Edit"
    }

    fn description(&self) -> &str {
        "Edit a file by exact string match and replace. old_string must match uniquely in the file and is replaced with new_string. If new_string is empty, the matched content is deleted."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "Original string to replace (must be unique in the file)"
                },
                "new_string": {
                    "type": "string",
                    "description": "Replacement string; empty string means delete"
                }
            },
            "required": ["path", "old_string", "new_string"]
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

        let old_string = match v.get("old_string").and_then(|c| c.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return ToolResult {
                    output: "参数缺少 old_string 字段".to_string(),
                    is_error: true,
                };
            }
        };

        let new_string = v
            .get("new_string")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        // 读取文件
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    output: format!("读取文件失败: {}", e),
                    is_error: true,
                };
            }
        };

        // 检查匹配次数
        let count = content.matches(&old_string).count();
        if count == 0 {
            return ToolResult {
                output: "未找到匹配的字符串".to_string(),
                is_error: true,
            };
        }
        if count > 1 {
            return ToolResult {
                output: format!(
                    "old_string 在文件中匹配了 {} 次，必须唯一匹配。请提供更多上下文使其唯一",
                    count
                ),
                is_error: true,
            };
        }

        // 执行替换
        let new_content = content.replacen(&old_string, &new_string, 1);
        match std::fs::write(&path, &new_content) {
            Ok(_) => ToolResult {
                output: format!("已编辑文件: {}", path),
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
        let v = serde_json::from_str::<Value>(arguments).ok();
        let path = v
            .as_ref()
            .and_then(|v| v.get("path").and_then(|c| c.as_str()).map(expand_tilde))
            .unwrap_or_else(|| "未知路径".to_string());
        let old = v
            .as_ref()
            .and_then(|v| v.get("old_string").and_then(|c| c.as_str()))
            .unwrap_or("");
        let first_line = old.lines().next().unwrap_or("");
        let has_more = old.lines().count() > 1;
        let preview = if has_more {
            format!("{}...", first_line)
        } else {
            first_line.to_string()
        };
        format!("即将编辑文件 {} (替换: \"{}\")", path, preview)
    }
}
