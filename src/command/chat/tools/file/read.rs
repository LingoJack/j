use crate::command::chat::tools::{Tool, ToolResult, expand_tilde};
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};

/// 读取文件的工具
pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "ReadFile"
    }

    fn description(&self) -> &str {
        "读取本地文件内容并返回（带行号）。支持通过 offset 和 limit 参数按行范围读取。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要读取的文件路径（绝对路径或相对于当前工作目录）"
                },
                "offset": {
                    "type": "integer",
                    "description": "从第几行开始读取（0-based，即 0 表示第 1 行），不传则从头开始"
                },
                "limit": {
                    "type": "integer",
                    "description": "读取多少行，不传则读到文件末尾"
                }
            },
            "required": ["path"]
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

        let offset = v.get("offset").and_then(|o| o.as_u64()).map(|o| o as usize);
        let limit = v.get("limit").and_then(|l| l.as_u64()).map(|l| l as usize);

        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                let total = lines.len();
                let start = offset.unwrap_or(0).min(total);
                let count = limit.unwrap_or(total - start).min(total - start);
                let selected: Vec<String> = lines[start..start + count]
                    .iter()
                    .enumerate()
                    .map(|(i, line)| format!("{:>4}│ {}", start + i + 1, line))
                    .collect();
                let mut result = selected.join("\n");

                if start + count < total {
                    result.push_str(&format!("\n...(还有 {} 行未显示)", total - start - count));
                }

                ToolResult {
                    output: result,
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
