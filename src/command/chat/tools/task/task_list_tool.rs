use std::sync::{Arc, atomic::AtomicBool};

use serde_json::{Value, json};

use crate::command::chat::tools::{Tool, ToolResult, task::task_manager::TaskManager};

pub struct TaskListTool {
    pub manager: Arc<TaskManager>,
}

impl TaskListTool {
    pub const NAME: &'static str = "TaskList";
}

impl Tool for TaskListTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        List all tasks with summary information (ID, title, status, dependencies).
        Use the optional `ready` filter to show only actionable tasks (pending with no unresolved blockers).
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "ready": {
                    "type": "boolean",
                    "description": "When true, return only tasks that are pending and have no unresolved blockers (ready to work on)"
                }
            }
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let ready = serde_json::from_str::<Value>(arguments)
            .ok()
            .and_then(|v| v.get("ready").and_then(|r| r.as_bool()))
            .unwrap_or(false);

        let tasks = if ready {
            self.manager.list_ready_tasks()
        } else {
            self.manager.list_tasks()
        };

        if tasks.is_empty() {
            return ToolResult {
                output: if ready {
                    "No ready tasks found (all tasks are either blocked, in progress, or completed)"
                        .to_string()
                } else {
                    "No tasks exist".to_string()
                },
                is_error: false,
            };
        }

        let summary: Vec<Value> = tasks
            .iter()
            .map(|t| {
                json!({
                    "taskId": t.task_id,
                    "title": t.title,
                    "status": t.status,
                    "blockedBy": t.blocked_by,
                })
            })
            .collect();

        ToolResult {
            output: serde_json::to_string_pretty(&summary).unwrap_or_default(),
            is_error: false,
        }
    }
}
