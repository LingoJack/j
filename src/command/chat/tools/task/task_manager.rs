use super::entity::AgentTask;
use crate::command::chat::permission::JcliConfig;
use crate::util::safe_lock;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

// ========== TaskManager ==========

/// 任务管理器，负责 CRUD 操作和持久化
pub struct TaskManager {
    tasks_dir: PathBuf,
    write_lock: Mutex<()>,
}

impl TaskManager {
    pub fn new() -> Self {
        // 优先使用 .jcli/tasks/，找不到则在 cwd 下创建 .jcli/tasks/
        let config_dir = JcliConfig::find_config_dir().or_else(JcliConfig::ensure_config_dir);
        let tasks_dir = match config_dir {
            Some(dir) => dir.join("tasks"),
            None => {
                // 极端 fallback：使用全局目录
                let data_dir = crate::config::YamlConfig::data_dir();
                data_dir.join("agent").join("data").join("tasks")
            }
        };
        let _ = fs::create_dir_all(&tasks_dir);
        Self {
            tasks_dir,
            write_lock: Mutex::new(()),
        }
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

    pub fn create_task(
        &self,
        title: &str,
        description: &str,
        blocked_by: Vec<u64>,
        task_doc_paths: Vec<String>,
    ) -> Result<AgentTask, String> {
        let _lock = safe_lock(&self.write_lock, "TaskManager::create_task");
        let task_id = self.next_id();
        let task = AgentTask {
            task_id,
            title: title.to_string(),
            description: description.to_string(),
            status: "pending".to_string(),
            blocked_by,
            owner: String::new(),
            task_doc_paths,
        };
        self.save_task(&task)?;

        Ok(task)
    }

    /// Return tasks that are pending and have no unresolved blockers (ready to work on)
    pub fn list_ready_tasks(&self) -> Vec<AgentTask> {
        self.list_tasks()
            .into_iter()
            .filter(|t| t.status == "pending" && t.blocked_by.is_empty())
            .collect()
    }

    pub fn get_task(&self, id: u64) -> Result<AgentTask, String> {
        let path = self.task_path(id);
        if !path.exists() {
            return Err(format!("Task {} does not exist", id));
        }
        let data = fs::read_to_string(&path).map_err(|e| format!("Failed to read task: {}", e))?;
        serde_json::from_str(&data).map_err(|e| format!("Failed to parse task: {}", e))
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
        tasks.sort_by_key(|t| t.task_id);
        tasks
    }

    pub fn update_task(&self, id: u64, updates: &Value) -> Result<AgentTask, String> {
        let _lock = safe_lock(&self.write_lock, "TaskManager::update_task");
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
                _ => return Err(format!("Invalid status: {}", status)),
            }
        }

        if let Some(title) = updates.get("title").and_then(|s| s.as_str()) {
            task.title = title.to_string();
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

        self.save_task(&task)?;
        Ok(task)
    }

    fn save_task(&self, task: &AgentTask) -> Result<(), String> {
        let path = self.task_path(task.task_id);
        let data = serde_json::to_string_pretty(task)
            .map_err(|e| format!("Failed to serialize task: {}", e))?;
        fs::write(&path, data).map_err(|e| format!("Failed to write task: {}", e))
    }

    /// 当任务完成或删除时，从所有其他任务的 blocked_by 中移除该 ID
    fn clean_references(&self, completed_id: u64) {
        let tasks = self.list_tasks();
        for mut task in tasks {
            if task.blocked_by.contains(&completed_id) {
                task.blocked_by.retain(|&id| id != completed_id);
                let _ = self.save_task(&task);
            }
        }
    }
}
