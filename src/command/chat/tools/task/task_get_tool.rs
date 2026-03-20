use super::super::{Tool, ToolResult};
use super::task_manager::TaskManager;
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};

pub struct TaskGetTool {
    pub manager: Arc<TaskManager>,
}

impl Tool for TaskGetTool {
    fn name(&self) -> &str {
        "TaskGet"
    }

    fn description(&self) -> &str {
        r#"
        Retrieve full details of a single task by its ID, including title, description, status, owner, and dependency information (blockedBy / blocks).
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "taskId": {
                    "type": "integer",
                    "description": "The ID of the task to retrieve"
                }
            },
            "required": ["taskId"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let parsed: Value = match serde_json::from_str(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("Failed to parse arguments: {}", e),
                    is_error: true,
                };
            }
        };

        let task_id = match parsed.get("taskId").and_then(|v| v.as_u64()) {
            Some(id) => id,
            None => {
                return ToolResult {
                    output: "taskId is required".to_string(),
                    is_error: true,
                };
            }
        };

        match self.manager.get_task(task_id) {
            Ok(task) => ToolResult {
                output: serde_json::to_string_pretty(&task).unwrap_or_default(),
                is_error: false,
            },
            Err(e) => ToolResult {
                output: e,
                is_error: true,
            },
        }
    }
}
