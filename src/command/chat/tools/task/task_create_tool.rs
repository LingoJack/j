use super::super::{Tool, ToolResult};
use super::task_manager::TaskManager;
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};

pub struct TaskCreateTool {
    pub manager: Arc<TaskManager>,
}

impl Tool for TaskCreateTool {
    fn name(&self) -> &str {
        "TaskCreate"
    }

    fn description(&self) -> &str {
        r#"
        Create a new task to break down complex requirements into smaller, trackable units of work.
        Supports optional dependency parameters to define task ordering at creation time.
        Note that the task should be self-contained and actionable based on the provided title and description and task documents given, as it will be assigned to an agent for execution.
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "A brief, actionable title for the task"
                },
                "description": {
                    "type": "string",
                    "description": "Detailed description of what needs to be done"
                },
                "taskDocPaths": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "description": "Location of the task document, which records full details about the task"
                    },
                },
                "blockedBy": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "description": "List of task IDs that must complete before this task can start"
                },
                "blocks": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "description": "List of task IDs that this task blocks (cannot start until this task completes)"
                }
            },
            "required": ["title"]
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
        let title = match parsed.get("title").and_then(|s| s.as_str()) {
            Some(s) => s,
            None => {
                return ToolResult {
                    output: "title is required".to_string(),
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

        let blocks: Vec<u64> = parsed
            .get("blocks")
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
            .create_task(title, description, blocked_by, blocks, task_doc_paths)
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
}
