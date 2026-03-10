use crate::command::chat::tools::{Tool, ToolResult};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex, atomic::AtomicBool};

// ========== NewTaskTool ==========

pub struct NewTaskTool {
    /// 共享的任务队列
    pub queued_tasks: Arc<Mutex<Vec<String>>>,
}

impl Tool for NewTaskTool {
    fn name(&self) -> &str {
        "new_task"
    }

    fn description(&self) -> &str {
        "创建一个新的后续任务。该任务会在当前对话轮次结束后自动开始执行。适用于需要分步骤完成的复杂任务。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "任务描述（将作为新的用户消息发送）"
                }
            },
            "required": ["task"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let parsed = serde_json::from_str::<Value>(arguments).ok();

        let task = parsed
            .as_ref()
            .and_then(|v| v.get("task").and_then(|t| t.as_str()))
            .unwrap_or("");

        if task.is_empty() {
            return ToolResult {
                output: "参数缺少 task 字段".to_string(),
                is_error: true,
            };
        }

        match self.queued_tasks.lock() {
            Ok(mut tasks) => {
                tasks.push(task.to_string());
                ToolResult {
                    output: format!("任务已创建，将在当前轮次结束后自动执行: {}", task),
                    is_error: false,
                }
            }
            Err(_) => ToolResult {
                output: "无法访问任务队列".to_string(),
                is_error: true,
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
