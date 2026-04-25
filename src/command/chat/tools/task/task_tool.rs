use super::task_manager::TaskManager;
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::{Arc, atomic::AtomicBool};

/// TaskTool 参数
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct TaskParams {
    /// Operation to perform: create, get, list, or update
    action: String,
    /// A brief, actionable title for the task (required for create)
    #[serde(default)]
    title: Option<String>,
    /// Detailed description of what needs to be done
    #[serde(default)]
    description: Option<String>,
    /// Paths to task documents containing full details about the task
    #[serde(default)]
    task_doc_paths: Option<Vec<String>>,
    /// List of task IDs that must complete before this task can start (for create)
    #[serde(default)]
    blocked_by: Option<Vec<u64>>,
    /// The ID of the task to retrieve or update (required for get/update)
    #[serde(default)]
    task_id: Option<u64>,
    /// When true (list only), return only tasks that are pending and have no unresolved blockers
    #[serde(default)]
    ready: bool,
    /// New status for the task (for update)
    #[serde(default)]
    status: Option<String>,
    /// Person or agent responsible for the task (for update)
    #[serde(default)]
    #[allow(dead_code)]
    owner: Option<String>,
    /// Task IDs to add as blockers of the current task (for update)
    #[serde(default)]
    #[allow(dead_code)]
    add_blocked_by: Option<Vec<u64>>,
}

/// 任务管理工具，支持任务的创建、查询、列表和更新
#[derive(Debug)]
pub struct TaskTool {
    /// 任务管理器实例
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
        schema_to_tool_params::<TaskParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: TaskParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        // We also parse as Value for update_task which takes a Value
        let parsed: Value = serde_json::from_str(arguments).unwrap_or_default();

        match params.action.as_str() {
            "create" => self.execute_create(&params),
            "get" => self.execute_get(&params),
            "list" => self.execute_list(&params),
            "update" => self.execute_update(&params, &parsed),
            other => ToolResult {
                output: format!(
                    "Unknown action: '{}'. Must be one of: create, get, list, update",
                    other
                ),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }
}

impl TaskTool {
    fn execute_create(&self, params: &TaskParams) -> ToolResult {
        let title = match params.title.as_deref() {
            Some(s) => s,
            None => {
                return ToolResult {
                    output: "title is required for action=create".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let description = params.description.as_deref().unwrap_or("");
        let blocked_by = params.blocked_by.clone().unwrap_or_default();
        let task_doc_paths = params.task_doc_paths.clone().unwrap_or_default();

        match self
            .manager
            .create_task(title, description, blocked_by, task_doc_paths)
        {
            Ok(task) => ToolResult {
                output: serde_json::to_string_pretty(&task).unwrap_or_default(),
                is_error: false,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
            Err(e) => ToolResult {
                output: e.to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    fn execute_get(&self, params: &TaskParams) -> ToolResult {
        let task_id = match params.task_id {
            Some(id) => id,
            None => {
                return ToolResult {
                    output: "taskId is required for action=get".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        match self.manager.get_task(task_id) {
            Ok(task) => ToolResult {
                output: serde_json::to_string_pretty(&task).unwrap_or_default(),
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

    fn execute_list(&self, params: &TaskParams) -> ToolResult {
        let tasks = if params.ready {
            self.manager.list_ready_tasks()
        } else {
            self.manager.list_tasks()
        };

        if tasks.is_empty() {
            return ToolResult {
                output: if params.ready {
                    "No ready tasks found (all tasks are either blocked, in progress, or completed)"
                        .to_string()
                } else {
                    "No tasks exist".to_string()
                },
                is_error: false,
                images: vec![],
                plan_decision: PlanDecision::None,
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
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }

    fn execute_update(&self, params: &TaskParams, parsed: &Value) -> ToolResult {
        let task_id = match params.task_id {
            Some(id) => id,
            None => {
                return ToolResult {
                    output: "taskId is required for action=update".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let status = params.status.as_deref().unwrap_or_default();

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
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    }
                } else {
                    ToolResult {
                        output: format!(
                            "Update task successfully. updated task detail: {}",
                            serde_json::to_string_pretty(&task).unwrap_or_default()
                        ),
                        is_error: false,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    }
                }
            }
            Err(e) => ToolResult {
                output: e,
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }
}
