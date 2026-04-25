use super::todo_manager::TodoManager;
use crate::command::chat::tools::{PlanDecision, Tool, ToolResult, schema_to_tool_params};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, atomic::AtomicBool};

/// TodoReadTool 参数（无参数）
#[derive(Deserialize, JsonSchema)]
struct TodoReadParams {}

/// 待办事项读取工具，用于查看当前所有待办项的状态
#[derive(Debug)]
pub struct TodoReadTool {
    /// 待办事项管理器实例
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
        schema_to_tool_params::<TodoReadParams>()
    }

    fn execute(&self, _arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let items = self.manager.list_todos();
        if items.is_empty() {
            return ToolResult {
                output: "No todo items found. Use TodoWrite to create new items.".to_string(),
                is_error: false,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }
        ToolResult {
            output: serde_json::to_string_pretty(&items).unwrap_or_default(),
            is_error: false,
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }
}
