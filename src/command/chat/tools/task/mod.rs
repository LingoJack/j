mod entity;
mod task_manager;
mod task_tool;

pub use entity::AgentTask;
pub use task_manager::TaskManager;
pub use task_tool::TaskTool;

use std::sync::Arc;

/// 构建任务摘要，用于系统提示词的 {{.tasks}} 占位符
/// 列出所有未完成的任务（pending + in_progress），以及任务间的依赖关系
pub fn build_tasks_summary(task_manager: &Arc<TaskManager>) -> String {
    let tasks = task_manager.list_tasks();
    let active: Vec<&AgentTask> = tasks
        .iter()
        .filter(|t| t.status == "pending" || t.status == "in_progress")
        .collect();
    if active.is_empty() {
        return String::new();
    }
    let mut md = String::from("## Active Tasks\n\n");
    for t in &active {
        let status_tag = if t.status == "in_progress" {
            "[in_progress]"
        } else {
            "[pending]"
        };
        let owner_tag = if t.owner.is_empty() {
            String::new()
        } else {
            format!(" (owner: {})", t.owner)
        };
        let blocked_tag = if t.blocked_by.is_empty() {
            String::new()
        } else {
            let ids: Vec<String> = t.blocked_by.iter().map(|id| id.to_string()).collect();
            format!(" (blocked by: {})", ids.join(", "))
        };
        md.push_str(&format!(
            "- #{} {}{}{}: {}\n",
            t.task_id, status_tag, owner_tag, blocked_tag, t.title
        ));
    }
    md.trim_end().to_string()
}
