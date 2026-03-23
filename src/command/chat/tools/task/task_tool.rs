use super::super::{Tool, ToolResult};
use super::task_manager::TaskManager;
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};

pub struct TaskTool {
    pub manager: Arc<TaskManager>,
}

impl TaskTool {
    pub const NAME: &'static str = "Task";
}

impl Tool for TaskTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Manage tasks (create / get / list / update). Use the `action` field to choose the operation.

        **action: "create"**
        Create a self-contained task. Tasks should be actionable based on the provided title,
        description, and task documents, as they will be assigned to an agent for execution.

        Do NOT use the Task tool for very small, single-step operations (use TodoWrite instead), such as:
        - Reading one known file path
        - Searching for a single class/function definition in a known file
        - Finding a simple, localized match in one or two files
        - Tasks that can be completed with a single read_file or search_file call

        Use "create" for tasks that require multiple steps, such as when you break down a complex
        task into multiple sub-tasks. Use blockedBy to specify dependencies between them.
        Required fields: title

        **action: "get"**
        Retrieve full details of a single task by its ID, including title, description, status,
        owner, and dependency information (blockedBy).
        Required fields: taskId

        **action: "list"**
        List all tasks with summary information (ID, title, status, dependencies).
        Use the optional `ready: true` filter to show only actionable tasks
        (pending with no unresolved blockers).

        **action: "update"**
        Update an existing task's status, title, description, owner, or dependencies.
        Status flow: pending → in_progress → completed. Use "deleted" to remove a task entirely.
        When a task is completed or deleted, it is automatically removed from other tasks' blockedBy lists.
        Required fields: taskId
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "get", "list", "update"],
                    "description": "Operation to perform: create, get, list, or update"
                },
                "title": {
                    "type": "string",
                    "description": "A brief, actionable title for the task (required for create)"
                },
                "description": {
                    "type": "string",
                    "description": "Detailed description of what needs to be done"
                },
                "taskDocPaths": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Paths to task documents containing full details about the task"
                },
                "blockedBy": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "description": "List of task IDs that must complete before this task can start (for create)"
                },
                "taskId": {
                    "type": "integer",
                    "description": "The ID of the task to retrieve or update (required for get/update)"
                },
                "ready": {
                    "type": "boolean",
                    "description": "When true (list only), return only tasks that are pending and have no unresolved blockers"
                },
                "status": {
                    "type": "string",
                    "enum": ["pending", "in_progress", "completed", "deleted"],
                    "description": "New status for the task (for update)"
                },
                "owner": {
                    "type": "string",
                    "description": "Person or agent responsible for the task (for update)"
                },
                "addBlockedBy": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "description": "Task IDs to add as blockers of the current task (for update)"
                }
            }
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

        let action = match parsed.get("action").and_then(|v| v.as_str()) {
            Some(a) => a,
            None => {
                return ToolResult {
                    output: "action is required (create, get, list, update)".to_string(),
                    is_error: true,
                };
            }
        };

        match action {
            "create" => self.execute_create(&parsed),
            "get" => self.execute_get(&parsed),
            "list" => self.execute_list(&parsed),
            "update" => self.execute_update(&parsed),
            other => ToolResult {
                output: format!(
                    "Unknown action: '{}'. Must be one of: create, get, list, update",
                    other
                ),
                is_error: true,
            },
        }
    }
}

impl TaskTool {
    fn execute_create(&self, parsed: &Value) -> ToolResult {
        let title = match parsed.get("title").and_then(|s| s.as_str()) {
            Some(s) => s,
            None => {
                return ToolResult {
                    output: "title is required for action=create".to_string(),
                    is_error: true,
                };
            }
        };

        let description = parsed
            .get("description")
            .and_then(|s| s.as_str())
            .unwrap_or("");

        let blocked_by: Vec<u64> = parsed
            .get("blockedBy")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
            .unwrap_or_default();

        let task_doc_paths: Vec<String> = parsed
            .get("taskDocPaths")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        match self
            .manager
            .create_task(title, description, blocked_by, task_doc_paths)
        {
            Ok(task) => ToolResult {
                output: serde_json::to_string_pretty(&task).unwrap_or_default(),
                is_error: false,
            },
            Err(e) => ToolResult {
                output: e.to_string(),
                is_error: true,
            },
        }
    }

    fn execute_get(&self, parsed: &Value) -> ToolResult {
        let task_id = match parsed.get("taskId").and_then(|v| v.as_u64()) {
            Some(id) => id,
            None => {
                return ToolResult {
                    output: "taskId is required for action=get".to_string(),
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

    fn execute_list(&self, parsed: &Value) -> ToolResult {
        let ready = parsed
            .get("ready")
            .and_then(|r| r.as_bool())
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

    fn execute_update(&self, parsed: &Value) -> ToolResult {
        let task_id = match parsed.get("taskId").and_then(|v| v.as_u64()) {
            Some(id) => id,
            None => {
                return ToolResult {
                    output: "taskId is required for action=update".to_string(),
                    is_error: true,
                };
            }
        };

        let status = parsed
            .get("status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        match self.manager.update_task(task_id, parsed) {
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
