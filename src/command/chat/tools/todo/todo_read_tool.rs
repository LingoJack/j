use super::super::{Tool, ToolResult};
use super::todo_manager::TodoManager;
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};

pub struct TodoReadTool {
    pub manager: Arc<TodoManager>,
}

impl TodoReadTool {
    pub const NAME: &'static str = "TodoRead";
}

impl Tool for TodoReadTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        "Read and list all current todo items. Returns the full todo list with id, content, and status for each item. Use this to check progress or review the current state of your task list."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn execute(&self, _arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let items = self.manager.list_todos();
        if items.is_empty() {
            return ToolResult {
                output: "No todo items found. Use TodoWrite to create new items.".to_string(),
                is_error: false,
                images: vec![],
            };
        }
        ToolResult {
            output: serde_json::to_string_pretty(&items).unwrap_or_default(),
            is_error: false,
            images: vec![],
        }
    }
}
