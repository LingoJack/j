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
        Create a self-contained task.
        The task should be actionable based on the provided title and description and task documents given,
        as it will be assigned to an agent for execution.

        Do NOT use the Task tool for very small (Instead, use TodoWrite), single-step operations such as:
        - Reading one known file path
        - Searching for a single class/function definition in a known file
        - Finding a simple, localized match in one or two files
        - Tasks that can be completed with a single read_file or search_file call

        Use the Task tool for tasks that require multiple steps, such as:
        When you break down a complex task into multiple sub-tasks, use TaskCreate for each sub-task, and use the blockedBy/blocks fields to specify dependencies between them. 
        - requirements analysis
        - design
        - implementation

        Usage notes:
        - Launch multiple agents concurrently when beneficial by issuing several Task tool calls in one message.
        - Each agent invocation is stateless, so your prompt should clearly describe the task the subagent needs to accomplish, but should not expand the request into unnecessary details or steps.
        - The agent will return one final message; summarize the result for the user as needed.
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
