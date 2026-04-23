use super::session::SessionPaths;
use super::types::ChatMessage;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Teammate 快照（可序列化，用于 session 持久化）
///
/// 消息历史存储于独立 transcript JSONL：`sessions/<id>/teammates/<sanitized_name>.jsonl`。
/// 本结构只保存元数据 + 未消费的 pending（供将来 RespawnTeammate 灌回）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeammateSnapshotPersist {
    pub name: String,
    pub role: String,
    pub prompt: String,
    pub worktree: bool,
    pub worktree_branch: Option<String>,
    pub inherit_permissions: bool,
    pub status: crate::command::chat::teammate::TeammateStatusPersist,
    /// 未被消费的广播消息（将来 Respawn 时灌回）
    #[serde(default)]
    pub pending_user_messages: Vec<ChatMessage>,
    pub tool_calls_count: usize,
    pub current_tool: Option<String>,
    pub work_done: bool,
}

/// 把任意名字转成文件系统安全的文件名片段（去除非法字符，限长）。
pub fn sanitize_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_whitespace() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            out.push('_');
        } else {
            out.push(c);
        }
    }
    // 限长 64，避免极端长名
    if out.chars().count() > 64 {
        out = out.chars().take(64).collect();
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

/// SubAgent 快照（只读历史，用于 session 恢复时展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentSnapshotPersist {
    pub id: String,
    pub description: String,
    pub mode: String,
    pub status: String,
    pub current_tool: Option<String>,
    pub tool_calls_count: usize,
    pub current_round: usize,
    pub started_at_epoch: u64,
    /// 独立 transcript 相对路径（相对于 session 根目录），例：`subagents/sub_0001/transcript.jsonl`
    #[serde(default)]
    pub transcript_file: String,
}

/// Plan 状态快照（可序列化，用于 session 持久化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStatePersist {
    pub active: bool,
    pub plan_file_path: Option<String>,
    pub plan_content: Option<String>,
}

/// Session Hook 注册快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHookPersist {
    pub event: crate::command::chat::infra::hook::HookEvent,
    pub definition: crate::command::chat::infra::hook::HookDef,
}

/// Sandbox 额外安全目录快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxStatePersist {
    pub extra_safe_dirs: Vec<PathBuf>,
}

// ========== Session 状态 Save/Load 通用辅助 ==========

fn save_session_json<T: Serialize + ?Sized>(path: &Path, data: &T) -> bool {
    match serde_json::to_string_pretty(data) {
        Ok(json) => fs::write(path, json).is_ok(),
        Err(_) => false,
    }
}

fn load_session_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

// ========== Session 状态专用 Save/Load ==========

/// 保存 Teammates 状态
pub fn save_teammates_state(session_id: &str, data: &[TeammateSnapshotPersist]) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.teammates_file(), data)
}

/// 加载 Teammates 状态
pub fn load_teammates_state(session_id: &str) -> Option<Vec<TeammateSnapshotPersist>> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.teammates_file())
}

/// 保存 SubAgents 状态
pub fn save_subagents_state(session_id: &str, data: &[SubAgentSnapshotPersist]) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.subagents_file(), data)
}

/// 加载 SubAgents 状态
#[allow(dead_code)]
pub fn load_subagents_state(session_id: &str) -> Option<Vec<SubAgentSnapshotPersist>> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.subagents_file())
}

/// 保存 Tasks 状态
pub fn save_tasks_state(
    session_id: &str,
    data: &[crate::command::chat::tools::task::AgentTask],
) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.tasks_file(), data)
}

/// 加载 Tasks 状态
pub fn load_tasks_state(
    session_id: &str,
) -> Option<Vec<crate::command::chat::tools::task::AgentTask>> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.tasks_file())
}

/// 保存 Todos 状态
pub fn save_todos_state(
    session_id: &str,
    data: &[crate::command::chat::tools::todo::TodoItem],
) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.todos_file(), data)
}

/// 加载 Todos 状态
pub fn load_todos_state(
    session_id: &str,
) -> Option<Vec<crate::command::chat::tools::todo::TodoItem>> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.todos_file())
}

/// 保存 Plan 状态
pub fn save_plan_state(session_id: &str, data: &PlanStatePersist) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.plan_file(), data)
}

/// 加载 Plan 状态
pub fn load_plan_state(session_id: &str) -> Option<PlanStatePersist> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.plan_file())
}

/// 保存 InvokedSkills 状态
pub fn save_skills_state(
    session_id: &str,
    data: &std::collections::HashMap<String, crate::command::chat::context::compact::InvokedSkill>,
) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.skills_file(), data)
}

/// 加载 InvokedSkills 状态
pub fn load_skills_state(
    session_id: &str,
) -> Option<std::collections::HashMap<String, crate::command::chat::context::compact::InvokedSkill>>
{
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.skills_file())
}

/// 保存 Session Hooks 状态
pub fn save_hooks_state(session_id: &str, data: &[SessionHookPersist]) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.hooks_file(), data)
}

/// 加载 Session Hooks 状态
pub fn load_hooks_state(session_id: &str) -> Option<Vec<SessionHookPersist>> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.hooks_file())
}

/// 保存 Sandbox 状态
pub fn save_sandbox_state(session_id: &str, data: &SandboxStatePersist) -> bool {
    let paths = SessionPaths::new(session_id);
    let _ = paths.ensure_dir();
    save_session_json(&paths.sandbox_file(), data)
}

/// 加载 Sandbox 状态
pub fn load_sandbox_state(session_id: &str) -> Option<SandboxStatePersist> {
    let paths = SessionPaths::new(session_id);
    load_session_json(&paths.sandbox_file())
}
