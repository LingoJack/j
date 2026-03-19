use super::{Tool, ToolResult};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicBool};

// ========== AgentTask ==========

/// 持久化任务数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: u64,
    pub subject: String,
    pub description: String,
    pub status: String, // "pending" | "in_progress" | "completed"
    #[serde(default)]
    pub blocked_by: Vec<u64>,
    #[serde(default)]
    pub blocks: Vec<u64>,
    #[serde(default)]
    pub owner: String,
}

// ========== TaskManager ==========

/// 任务管理器，负责 CRUD 操作和持久化
pub struct TaskManager {
    tasks_dir: PathBuf,
}

impl TaskManager {
    pub fn new() -> Self {
        let data_dir = crate::config::YamlConfig::data_dir();
        let tasks_dir = data_dir.join("agent").join("tasks");
        let _ = fs::create_dir_all(&tasks_dir);
        Self { tasks_dir }
    }

    /// 生成下一个任务 ID（基于已有文件的最大 ID + 1）
    fn next_id(&self) -> u64 {
        let mut max_id: u64 = 0;
        if let Ok(entries) = fs::read_dir(&self.tasks_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                // 格式: task_{id}.json
                if let Some(rest) = name.strip_prefix("task_") {
                    if let Some(id_str) = rest.strip_suffix(".json") {
                        if let Ok(id) = id_str.parse::<u64>() {
                            max_id = max_id.max(id);
                        }
                    }
                }
            }
        }
        max_id + 1
    }

    fn task_path(&self, id: u64) -> PathBuf {
        self.tasks_dir.join(format!("task_{}.json", id))
    }

    pub fn create_task(&self, subject: &str, description: &str) -> Result<AgentTask, String> {
        let id = self.next_id();
        let task = AgentTask {
            id,
            subject: subject.to_string(),
            description: description.to_string(),
            status: "pending".to_string(),
            blocked_by: vec![],
            blocks: vec![],
            owner: String::new(),
        };
        self.save_task(&task)?;
        Ok(task)
    }

    pub fn get_task(&self, id: u64) -> Result<AgentTask, String> {
        let path = self.task_path(id);
        if !path.exists() {
            return Err(format!("任务 {} 不存在", id));
        }
        let data = fs::read_to_string(&path).map_err(|e| format!("读取任务失败: {}", e))?;
        serde_json::from_str(&data).map_err(|e| format!("解析任务失败: {}", e))
    }

    pub fn list_tasks(&self) -> Vec<AgentTask> {
        let mut tasks = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.tasks_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json") {
                    if let Ok(data) = fs::read_to_string(&path) {
                        if let Ok(task) = serde_json::from_str::<AgentTask>(&data) {
                            tasks.push(task);
                        }
                    }
                }
            }
        }
        tasks.sort_by_key(|t| t.id);
        tasks
    }

    pub fn update_task(&self, id: u64, updates: &Value) -> Result<AgentTask, String> {
        let mut task = self.get_task(id)?;

        if let Some(status) = updates.get("status").and_then(|s| s.as_str()) {
            match status {
                "deleted" => {
                    // 删除任务文件
                    let path = self.task_path(id);
                    let _ = fs::remove_file(&path);
                    // 清理其他任务中对该任务的引用
                    self.clean_references(id);
                    task.status = "deleted".to_string();
                    return Ok(task);
                }
                "pending" | "in_progress" | "completed" => {
                    task.status = status.to_string();
                    // 完成时自动清理其他任务的 blocked_by
                    if status == "completed" {
                        self.clean_references(id);
                    }
                }
                _ => return Err(format!("无效的状态: {}", status)),
            }
        }

        if let Some(subject) = updates.get("subject").and_then(|s| s.as_str()) {
            task.subject = subject.to_string();
        }
        if let Some(description) = updates.get("description").and_then(|s| s.as_str()) {
            task.description = description.to_string();
        }
        if let Some(owner) = updates.get("owner").and_then(|s| s.as_str()) {
            task.owner = owner.to_string();
        }

        // 添加依赖关系
        if let Some(add_blocked_by) = updates.get("addBlockedBy").and_then(|v| v.as_array()) {
            for id_val in add_blocked_by {
                if let Some(dep_id) = id_val.as_u64() {
                    if !task.blocked_by.contains(&dep_id) {
                        task.blocked_by.push(dep_id);
                    }
                }
            }
        }
        if let Some(add_blocks) = updates.get("addBlocks").and_then(|v| v.as_array()) {
            for id_val in add_blocks {
                if let Some(dep_id) = id_val.as_u64() {
                    if !task.blocks.contains(&dep_id) {
                        task.blocks.push(dep_id);
                    }
                    // 同时更新目标任务的 blocked_by
                    if let Ok(mut target) = self.get_task(dep_id) {
                        if !target.blocked_by.contains(&id) {
                            target.blocked_by.push(id);
                            let _ = self.save_task(&target);
                        }
                    }
                }
            }
        }

        self.save_task(&task)?;
        Ok(task)
    }

    fn save_task(&self, task: &AgentTask) -> Result<(), String> {
        let path = self.task_path(task.id);
        let data =
            serde_json::to_string_pretty(task).map_err(|e| format!("序列化任务失败: {}", e))?;
        fs::write(&path, data).map_err(|e| format!("写入任务失败: {}", e))
    }

    /// 当任务完成或删除时，从所有其他任务的 blocked_by 中移除该 ID
    fn clean_references(&self, completed_id: u64) {
        let tasks = self.list_tasks();
        for mut task in tasks {
            if task.blocked_by.contains(&completed_id) {
                task.blocked_by.retain(|&id| id != completed_id);
                let _ = self.save_task(&task);
            }
            if task.blocks.contains(&completed_id) {
                task.blocks.retain(|&id| id != completed_id);
                let _ = self.save_task(&task);
            }
        }
    }
}

// ========== TaskCreateTool ==========

pub struct TaskCreateTool {
    pub manager: Arc<TaskManager>,
}

impl Tool for TaskCreateTool {
    fn name(&self) -> &str {
        "TaskCreate"
    }

    fn description(&self) -> &str {
        r#"
        创建一个新任务，用于跟踪复杂多步骤工作的进度。
        适用场景：
        - 复杂的任务
        - 多步骤任务需要拆分和跟踪
        - 需要记录待办事项和依赖关系
        任务会持久化存储，重启后仍可查看。
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "subject": {
                    "type": "string",
                    "description": "任务标题，简短的祈使句（如 '修复登录页面的 bug'）"
                },
                "description": {
                    "type": "string",
                    "description": "任务的详细描述，包含上下文和完成标准"
                }
            },
            "required": ["subject", "description"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let parsed: Value = match serde_json::from_str(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        let subject = match parsed.get("subject").and_then(|s| s.as_str()) {
            Some(s) => s,
            None => {
                return ToolResult {
                    output: "缺少 subject 参数".to_string(),
                    is_error: true,
                };
            }
        };

        let description = parsed
            .get("description")
            .and_then(|s| s.as_str())
            .unwrap_or("");

        match self.manager.create_task(subject, description) {
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
}

// ========== TaskUpdateTool ==========

pub struct TaskUpdateTool {
    pub manager: Arc<TaskManager>,
}

impl Tool for TaskUpdateTool {
    fn name(&self) -> &str {
        "TaskUpdate"
    }

    fn description(&self) -> &str {
        r#"
        更新已有任务的状态、标题、描述或依赖关系。
        状态流转: pending → in_progress → completed。
        设置 status 为 "deleted" 可删除任务。
        完成任务时会自动清理其他任务的依赖引用。
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "taskId": {
                    "type": "integer",
                    "description": "要更新的任务 ID"
                },
                "status": {
                    "type": "string",
                    "description": "新状态: pending / in_progress / completed / deleted",
                    "enum": ["pending", "in_progress", "completed", "deleted"]
                },
                "subject": {
                    "type": "string",
                    "description": "新的任务标题"
                },
                "description": {
                    "type": "string",
                    "description": "新的任务描述"
                },
                "owner": {
                    "type": "string",
                    "description": "任务负责人"
                },
                "addBlockedBy": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "description": "添加阻塞当前任务的任务 ID 列表"
                },
                "addBlocks": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "description": "添加被当前任务阻塞的任务 ID 列表"
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
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        let task_id = match parsed.get("taskId").and_then(|v| v.as_u64()) {
            Some(id) => id,
            None => {
                return ToolResult {
                    output: "缺少 taskId 参数".to_string(),
                    is_error: true,
                };
            }
        };

        match self.manager.update_task(task_id, &parsed) {
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
}

// ========== TaskListTool ==========

pub struct TaskListTool {
    pub manager: Arc<TaskManager>,
}

impl Tool for TaskListTool {
    fn name(&self) -> &str {
        "TaskList"
    }

    fn description(&self) -> &str {
        r#"
        列出所有任务的摘要信息，包括 ID、标题、状态、负责人和依赖关系。
        用于查看整体进度和找到可执行的任务。
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn execute(&self, _arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let tasks = self.manager.list_tasks();
        if tasks.is_empty() {
            return ToolResult {
                output: "当前没有任务".to_string(),
                is_error: false,
            };
        }

        let summary: Vec<Value> = tasks
            .iter()
            .map(|t| {
                json!({
                    "id": t.id,
                    "subject": t.subject,
                    "status": t.status,
                    "owner": t.owner,
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

// ========== TaskGetTool ==========

pub struct TaskGetTool {
    pub manager: Arc<TaskManager>,
}

impl Tool for TaskGetTool {
    fn name(&self) -> &str {
        "TaskGet"
    }

    fn description(&self) -> &str {
        r#"
        获取单个任务的完整详情，包括描述、依赖关系等。
        用于在开始工作前了解任务的完整要求。
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "taskId": {
                    "type": "integer",
                    "description": "要查询的任务 ID"
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
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        let task_id = match parsed.get("taskId").and_then(|v| v.as_u64()) {
            Some(id) => id,
            None => {
                return ToolResult {
                    output: "缺少 taskId 参数".to_string(),
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
}
