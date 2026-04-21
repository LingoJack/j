use super::super::{PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params};
use super::entity::TodoItem;
use super::todo_manager::TodoManager;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, atomic::AtomicBool};

/// Todo 项参数
#[derive(Deserialize, JsonSchema)]
struct TodoItemParam {
    /// Item ID. Required when merge=true to update existing items. Auto-generated if omitted.
    #[serde(default)]
    id: Option<String>,
    /// The todo item text. Required for new items; optional when merge=true (omit to keep existing).
    #[serde(default)]
    content: String,
    /// Item status: pending, in_progress, completed, or cancelled
    #[serde(default = "default_status")]
    status: String,
}

fn default_status() -> String {
    "pending".to_string()
}

/// TodoWriteTool 参数
#[derive(Deserialize, JsonSchema)]
struct TodoWriteParams {
    /// Array of todo items
    todos: Vec<TodoItemParam>,
    /// If false (default), replace the entire list. If true, only update/add the provided items by id.
    #[serde(default)]
    merge: bool,
}

/// 待办事项写入工具，用于创建和管理结构化待办列表
#[derive(Debug)]
pub struct TodoWriteTool {
    /// 待办事项管理器实例
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
        schema_to_tool_params::<TodoWriteParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: TodoWriteParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        if !params.merge {
            let empty_indices: Vec<usize> = params
                .todos
                .iter()
                .enumerate()
                .filter(|(_, item)| item.content.is_empty())
                .map(|(i, _)| i + 1)
                .collect();
            if !empty_indices.is_empty() {
                return ToolResult {
                    output: format!(
                        "content is required for new items (merge=false). Missing in item(s): {}",
                        empty_indices
                            .iter()
                            .map(|i| i.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        }

        let items: Vec<TodoItem> = params
            .todos
            .iter()
            .map(|item| TodoItem {
                id: item.id.clone().unwrap_or_default(),
                content: item.content.clone(),
                status: item.status.clone(),
            })
            .collect();

        match self.manager.write_todos(items, params.merge) {
            Ok(all_todos) => ToolResult {
                output: serde_json::to_string_pretty(&all_todos).unwrap_or_default(),
                is_error: false,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
            Err(e) => ToolResult {
                output: e,
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }
}
