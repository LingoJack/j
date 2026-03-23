use std::sync::{Arc, atomic::AtomicBool};

use serde_json::{Value, json};

use crate::command::chat::tools::{Tool, ToolResult, task::task_manager::TaskManager};

pub struct TaskUpdateTool {
    pub manager: Arc<TaskManager>,
}

impl Tool for TaskUpdateTool {
    fn name(&self) -> &str {
        "TaskUpdate"
    }

    fn description(&self) -> &str {
        r#"
        Update an existing task's status, title, description, owner, or dependencies.
        Status flow: pending → in_progress → completed. Use "deleted" to remove a task entirely.
        When a task is completed or deleted, it is automatically removed from other tasks' blockedBy lists.
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "taskId": {
                    "type": "integer",
                    "description": "The ID of the task to update"
                },
                "status": {
                    "type": "string",
                    "description": "New status: pending, in_progress, completed, or deleted",
                    "enum": ["pending", "in_progress", "completed", "deleted"]
                },
                "title": {
                    "type": "string",
                    "description": "New title for the task"
                },
                "description": {
                    "type": "string",
                    "description": "New description for the task"
                },
                "owner": {
                    "type": "string",
                    "description": "Person or agent responsible for the task"
                },
                "addBlockedBy": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "description": "Task IDs to add as blockers of the current task"
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

        let status = parsed
            .get("status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        match self.manager.update_task(task_id, &parsed) {
            Ok(task) => {
                if status == "completed" {
                    ToolResult {
                        output: format!(
                            "Update task successfully. Following tasks are ready: \n\n{}",
                            serde_json::to_string_pretty(&self.manager.list_ready_tasks())
                                .unwrap_or_default()
                        ),
                        is_error: false,
                    }
                } else {
                    ToolResult {
                        output: format!(
                            "Update task successfully. updated task detail: {}",
                            serde_json::to_string_pretty(&task).unwrap_or_default()
                        ),
                        is_error: false,
                    }
                }
            }
            Err(e) => ToolResult {
                output: e,
                is_error: true,
            },
        }
    }
}
