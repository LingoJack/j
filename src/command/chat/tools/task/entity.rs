use serde::{Deserialize, Serialize};

/// 持久化任务数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub task_id: u64,
    pub title: String,
    pub description: String,
    pub status: String, // "pending" | "in_progress" | "completed"
    #[serde(default)]
    pub blocked_by: Vec<u64>,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub task_doc_paths: Vec<String>,
}
