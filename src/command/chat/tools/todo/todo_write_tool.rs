use super::super::{Tool, ToolResult};
use super::entity::TodoItem;
use super::todo_manager::TodoManager;
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};

pub struct TodoWriteTool {
    pub manager: Arc<TodoManager>,
}

impl TodoWriteTool {
    pub const NAME: &'static str = "TodoWrite";
}

impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Create and manage a structured todo list to maintain state across long turns.
    
        CRITICAL RULES:
        1. Only ONE item can be 'in_progress' at any time; the system enforces this automatically.
        2. For updates, always use 'merge=true' and only provide the specific items being modified.
        3. Support batch updates: efficiently transition states by marking a task 'completed' and the next 'in_progress' in a single call.
        4. Use this to demonstrate progress and ensure complex requirements are not missed.
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "description": "Array of todo items. Each item has: id (string, optional for new items), content (string, the todo text), status (string, optional, defaults to 'pending')",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Item ID. Required when merge=true to update existing items. Auto-generated if omitted."
                            },
                            "content": {
                                "type": "string",
                                "description": "The todo item text"
                            },
                            "status": {
                                "type": "string",
                                "description": "Item status: pending, in_progress, completed, or cancelled",
                                "enum": ["pending", "in_progress", "completed", "cancelled"]
                            }
                        },
                        "required": ["content"]
                    }
                },
                "merge": {
                    "type": "boolean",
                    "description": "If false (default), replace the entire list. If true, only update/add the provided items by id."
                }
            },
            "required": ["todos"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let parsed: Value = match serde_json::from_str(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("Failed to parse arguments: {}", e),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        let todos_arr = match parsed.get("todos").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => {
                return ToolResult {
                    output: "todos (array) is required".to_string(),
                    is_error: true,
                    images: vec![],
                };
            }
        };

        let merge = parsed
            .get("merge")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let items: Vec<TodoItem> = todos_arr
            .iter()
            .map(|item| TodoItem {
                id: item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                content: item
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                status: item
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pending")
                    .to_string(),
            })
            .collect();

        match self.manager.write_todos(items, merge) {
            Ok(all_todos) => ToolResult {
                output: serde_json::to_string_pretty(&all_todos).unwrap_or_default(),
                is_error: false,
                images: vec![],
            },
            Err(e) => ToolResult {
                output: e,
                is_error: true,
                images: vec![],
            },
        }
    }
}
