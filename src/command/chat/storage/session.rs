use super::config::agent_data_dir;
use super::types::{ChatMessage, ChatSession, MessageRole, SessionEvent};
use crate::command::chat::constants::MESSAGE_PREVIEW_MAX_LEN;
use crate::error;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 获取 sessions 目录: ~/.jdata/agent/data/sessions/
pub fn sessions_dir() -> PathBuf {
    let dir = agent_data_dir().join("sessions");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 获取单个 session 的 JSONL 文件路径（兼容别名，指向新布局主文件）
pub fn session_file_path(session_id: &str) -> PathBuf {
    SessionPaths::new(session_id).transcript()
}

/// Session 目录布局抽象。
///
/// 布局：`sessions/<id>/transcript.jsonl`。
#[derive(Debug)]
pub struct SessionPaths {
    dir: PathBuf,
}

impl SessionPaths {
    pub fn new(session_id: &str) -> Self {
        let dir = sessions_dir().join(session_id);
        Self { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// 主数据文件：`sessions/<id>/transcript.jsonl`
    pub fn transcript(&self) -> PathBuf {
        self.dir.join("transcript.jsonl")
    }

    /// 元数据文件：`sessions/<id>/session.json`
    pub fn meta_file(&self) -> PathBuf {
        self.dir.join("session.json")
    }

    /// compact 快照目录：`sessions/<id>/.transcripts/`
    pub fn transcripts_dir(&self) -> PathBuf {
        self.dir.join(".transcripts")
    }

    /// Teammate 状态文件：`sessions/<id>/teammates.json`
    pub fn teammates_file(&self) -> PathBuf {
        self.dir.join("teammates.json")
    }

    /// Teammate 独立目录根：`sessions/<id>/teammates/`
    pub fn teammates_dir(&self) -> PathBuf {
        self.dir.join("teammates")
    }

    /// 单个 teammate 的独立子目录：`sessions/<id>/teammates/<sanitized_name>/`
    pub fn teammate_dir(&self, sanitized_name: &str) -> PathBuf {
        self.teammates_dir().join(sanitized_name)
    }

    /// 单个 teammate 的 transcript JSONL 路径：`sessions/<id>/teammates/<sanitized_name>/transcript.jsonl`
    pub fn teammate_transcript(&self, sanitized_name: &str) -> PathBuf {
        self.teammate_dir(sanitized_name).join("transcript.jsonl")
    }

    /// 单个 teammate 的 todo 文件路径：`sessions/<id>/teammates/<sanitized_name>/todos.json`
    pub fn teammate_todos_file(&self, sanitized_name: &str) -> PathBuf {
        self.teammate_dir(sanitized_name).join("todos.json")
    }

    /// SubAgent 状态文件：`sessions/<id>/subagents.json`
    pub fn subagents_file(&self) -> PathBuf {
        self.dir.join("subagents.json")
    }

    /// SubAgent 独立目录根：`sessions/<id>/subagents/`
    pub fn subagents_dir(&self) -> PathBuf {
        self.dir.join("subagents")
    }

    /// 单个 subagent 的独立子目录：`sessions/<id>/subagents/<sub_id>/`
    pub fn subagent_dir(&self, sub_id: &str) -> PathBuf {
        self.subagents_dir().join(sub_id)
    }

    /// 单个 subagent 的 transcript JSONL 路径：`sessions/<id>/subagents/<sub_id>/transcript.jsonl`
    pub fn subagent_transcript(&self, sub_id: &str) -> PathBuf {
        self.subagent_dir(sub_id).join("transcript.jsonl")
    }

    /// 单个 subagent 的 todo 文件路径：`sessions/<id>/subagents/<sub_id>/todos.json`
    pub fn subagent_todos_file(&self, sub_id: &str) -> PathBuf {
        self.subagent_dir(sub_id).join("todos.json")
    }

    /// Task 状态文件：`sessions/<id>/tasks.json`
    pub fn tasks_file(&self) -> PathBuf {
        self.dir.join("tasks.json")
    }

    /// Todo 状态文件：`sessions/<id>/todos.json`
    pub fn todos_file(&self) -> PathBuf {
        self.dir.join("todos.json")
    }

    /// Plan 状态文件：`sessions/<id>/plan.json`
    pub fn plan_file(&self) -> PathBuf {
        self.dir.join("plan.json")
    }

    /// InvokedSkills 状态文件：`sessions/<id>/skills.json`
    pub fn skills_file(&self) -> PathBuf {
        self.dir.join("skills.json")
    }

    /// Session Hook 状态文件：`sessions/<id>/hooks.json`
    pub fn hooks_file(&self) -> PathBuf {
        self.dir.join("hooks.json")
    }

    /// Sandbox 状态文件：`sessions/<id>/sandbox.json`
    pub fn sandbox_file(&self) -> PathBuf {
        self.dir.join("sandbox.json")
    }

    pub fn ensure_dir(&self) -> std::io::Result<()> {
        fs::create_dir_all(&self.dir)
    }
}

/// 追加一个事件到 session JSONL 文件（append-only，POSIX 下原子安全）
///
/// 同时增量更新 `session.json` 元数据。
pub fn append_session_event(session_id: &str, event: &SessionEvent) -> bool {
    let paths = SessionPaths::new(session_id);
    if paths.ensure_dir().is_err() {
        return false;
    }
    let path = paths.transcript();
    let ok = match serde_json::to_string(event) {
        Ok(line) => match fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut file) => writeln!(file, "{}", line).is_ok(),
            Err(_) => false,
        },
        Err(_) => false,
    };
    if ok {
        update_session_meta_on_event(session_id, event);
    }
    ok
}

/// 增量更新 session.json 元数据（追加事件后调用）
fn update_session_meta_on_event(session_id: &str, event: &SessionEvent) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut meta = load_session_meta_file(session_id).unwrap_or_else(|| SessionMetaFile {
        id: session_id.to_string(),
        title: String::new(),
        message_count: 0,
        created_at: now,
        updated_at: now,
        model: None,
        auto_approve: false,
    });
    meta.updated_at = now;
    match event {
        SessionEvent::Msg { message: msg, .. } => {
            meta.message_count += 1;
            if meta.title.is_empty() && msg.role == MessageRole::User && !msg.content.is_empty() {
                meta.title = msg.content.chars().take(MESSAGE_PREVIEW_MAX_LEN).collect();
            }
        }
        SessionEvent::Clear => {
            meta.message_count = 0;
        }
        SessionEvent::Restore { messages } => {
            meta.message_count = messages.len();
            if meta.title.is_empty()
                && let Some(first_user) = messages
                    .iter()
                    .find(|m| m.role == MessageRole::User && !m.content.is_empty())
            {
                meta.title = first_user
                    .content
                    .chars()
                    .take(MESSAGE_PREVIEW_MAX_LEN)
                    .collect();
            }
        }
    }
    let _ = save_session_meta_file(&meta);
}

/// 查找最近修改的 session ID（用于 --continue）
pub fn find_latest_session_id() -> Option<String> {
    let dir = sessions_dir();
    let mut entries: Vec<(std::time::SystemTime, String)> = Vec::new();
    let read_dir = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return None,
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let transcript = path.join("transcript.jsonl");
        if let Ok(meta) = transcript.metadata()
            && let Ok(modified) = meta.modified()
        {
            entries.push((modified, id.to_string()));
        }
    }
    entries.sort_by(|a, b| b.0.cmp(&a.0));
    entries.into_iter().next().map(|(_, id)| id)
}

/// 从 JSONL 文件 replay 出 ChatSession（供 resume 等功能使用）
pub fn load_session(session_id: &str) -> ChatSession {
    let path = SessionPaths::new(session_id).transcript();
    if !path.exists() {
        return ChatSession::default();
    }
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return ChatSession::default(),
    };
    let mut messages: Vec<ChatMessage> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<SessionEvent>(line) {
            Ok(event) => match event {
                SessionEvent::Msg { message, .. } => messages.push(message),
                SessionEvent::Clear => messages.clear(),
                SessionEvent::Restore { messages: restored } => messages = restored,
            },
            Err(_) => {
                // 损坏行直接跳过，继续处理剩余行
            }
        }
    }

    // ★ 存量清理：移除历史遗留的孤立 tool_call / tool_result。
    //   检测到变化时追加一条 Restore 事件，让 jsonl 下次加载直接从干净快照出发，
    //   orphan 不再反复出现在 sanitize 日志里。
    if let Some(sanitized) = sanitize_loaded_messages(&messages) {
        let restore_event = SessionEvent::Restore {
            messages: sanitized.clone(),
        };
        append_session_event(session_id, &restore_event);
        messages = sanitized;
    }

    ChatSession { messages }
}

/// 双向配对清理：
///   - 移除 tool_call_id 为空或在任何 assistant tool_calls 中找不到对应项的 tool result
///   - 移除 assistant tool_calls 中 id 为空或找不到对应 tool result 的条目；
///     tool_calls 被全部清空时置为 None（保留 content 文本）
///
/// 返回 `Some(sanitized)` 表示发生了变化，`None` 表示原样可用。
fn sanitize_loaded_messages(messages: &[ChatMessage]) -> Option<Vec<ChatMessage>> {
    let tool_result_ids: std::collections::HashSet<String> = messages
        .iter()
        .filter(|m| m.role == MessageRole::Tool)
        .filter_map(|m| m.tool_call_id.clone())
        .filter(|id| !id.is_empty())
        .collect();

    let assistant_tool_call_ids: std::collections::HashSet<String> = messages
        .iter()
        .filter(|m| m.role == MessageRole::Assistant)
        .flat_map(|m| m.tool_calls.as_deref().unwrap_or(&[]))
        .map(|tc| tc.id.clone())
        .filter(|id| !id.is_empty())
        .collect();

    let mut changed = false;
    let mut out: Vec<ChatMessage> = Vec::with_capacity(messages.len());
    for msg in messages {
        if msg.role == MessageRole::Tool {
            let id = msg.tool_call_id.as_deref().unwrap_or("");
            if id.is_empty() || !assistant_tool_call_ids.contains(id) {
                changed = true;
                continue;
            }
            out.push(msg.clone());
        } else if msg.role == MessageRole::Assistant {
            if let Some(ref tcs) = msg.tool_calls {
                let kept: Vec<_> = tcs
                    .iter()
                    .filter(|tc| !tc.id.is_empty() && tool_result_ids.contains(&tc.id))
                    .cloned()
                    .collect();
                if kept.len() != tcs.len() {
                    changed = true;
                    let mut new_msg = msg.clone();
                    new_msg.tool_calls = if kept.is_empty() { None } else { Some(kept) };
                    // 若 tool_calls 被清空且没有文本内容，整条消息也无意义——跳过
                    if new_msg.tool_calls.is_none() && new_msg.content.trim().is_empty() {
                        continue;
                    }
                    out.push(new_msg);
                } else {
                    out.push(msg.clone());
                }
            } else {
                out.push(msg.clone());
            }
        } else {
            out.push(msg.clone());
        }
    }

    if changed { Some(out) } else { None }
}

/// 从 JSONL 文件按出现顺序读取 `(ChatMessage, timestamp_ms)` 列表。
///
/// 供 teammate / subagent 等独立 transcript 的读取使用：保留时间戳、不做 Clear/Restore 处理。
pub fn read_transcript_with_timestamps(path: &Path) -> Vec<(ChatMessage, u64)> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<(ChatMessage, u64)> = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(SessionEvent::Msg {
            message,
            timestamp_ms,
        }) = serde_json::from_str::<SessionEvent>(line)
        {
            out.push((message, timestamp_ms));
        }
    }
    out
}

/// 向任意路径的 JSONL 文件 append 一条事件（append-only；用于 teammate/subagent 独立 transcript）。
pub fn append_event_to_path(path: &Path, event: &SessionEvent) -> bool {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let line = match serde_json::to_string(event) {
        Ok(s) => s,
        Err(_) => return false,
    };
    match fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut file) => writeln!(file, "{}", line).is_ok(),
        Err(_) => false,
    }
}

// ========== 会话元数据 ==========

/// session.json 元数据文件内容（持久化到 `sessions/<id>/session.json`）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetaFile {
    /// 会话 ID
    pub id: String,
    /// 会话标题（首条 user 消息截断）
    #[serde(default)]
    pub title: String,
    /// 消息计数
    pub message_count: usize,
    /// 创建时间戳（epoch seconds）
    pub created_at: u64,
    /// 最后更新时间戳（epoch seconds）
    pub updated_at: u64,
    /// 使用的模型名称
    #[serde(default)]
    pub model: Option<String>,
    /// 是否自动批准所有操作（bypass 模式）
    #[serde(default)]
    pub auto_approve: bool,
}

/// 会话元数据（用于会话列表展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    /// 会话标题（从 session.json 读取）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub message_count: usize,
    pub first_message_preview: Option<String>,
    pub updated_at: u64,
}

/// 加载 session.json 元数据（不存在返回 None）
pub fn load_session_meta_file(session_id: &str) -> Option<SessionMetaFile> {
    let path = SessionPaths::new(session_id).meta_file();
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 保存 session.json 元数据
pub fn save_session_meta_file(meta: &SessionMetaFile) -> bool {
    let paths = SessionPaths::new(&meta.id);
    if paths.ensure_dir().is_err() {
        return false;
    }
    match serde_json::to_string_pretty(meta) {
        Ok(json) => fs::write(paths.meta_file(), json).is_ok(),
        Err(_) => false,
    }
}

/// 从 transcript.jsonl 逐行扫描生成元数据（懒生成 / 迁移用）
fn derive_session_meta_from_transcript(session_id: &str) -> Option<SessionMetaFile> {
    let paths = SessionPaths::new(session_id);
    let transcript = paths.transcript();
    let content = fs::read_to_string(&transcript).ok()?;

    let mut message_count: usize = 0;
    let mut first_user_preview: Option<String> = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<SessionEvent>(line) {
            match event {
                SessionEvent::Msg {
                    message: ref msg, ..
                } => {
                    message_count += 1;
                    if first_user_preview.is_none()
                        && msg.role == MessageRole::User
                        && !msg.content.is_empty()
                    {
                        first_user_preview =
                            Some(msg.content.chars().take(MESSAGE_PREVIEW_MAX_LEN).collect());
                    }
                }
                SessionEvent::Clear => {
                    message_count = 0;
                    first_user_preview = None;
                }
                SessionEvent::Restore { ref messages } => {
                    message_count = messages.len();
                    first_user_preview = messages
                        .iter()
                        .find(|m| m.role == MessageRole::User && !m.content.is_empty())
                        .map(|m| m.content.chars().take(MESSAGE_PREVIEW_MAX_LEN).collect());
                }
            }
        }
    }

    let updated_at = transcript
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Some(SessionMetaFile {
        id: session_id.to_string(),
        title: first_user_preview.clone().unwrap_or_default(),
        message_count,
        created_at: updated_at,
        updated_at,
        model: None,
        auto_approve: false,
    })
}

/// 列出所有会话的元数据，按更新时间倒序
///
/// 优先读 `session.json` 元数据文件（O(1)），不存在时 fallback 到逐行扫描
/// `transcript.jsonl` 并懒生成 `session.json`。
pub fn list_sessions() -> Vec<SessionMeta> {
    let dir = sessions_dir();
    let read_dir = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    let mut ids: Vec<String> = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if path.join("transcript.jsonl").exists() {
            ids.push(id.to_string());
        }
    }

    let mut sessions: Vec<SessionMeta> = Vec::new();
    for id in ids {
        // 优先读 session.json
        if let Some(meta_file) = load_session_meta_file(&id) {
            sessions.push(SessionMeta {
                id: meta_file.id,
                title: if meta_file.title.is_empty() {
                    None
                } else {
                    Some(meta_file.title)
                },
                message_count: meta_file.message_count,
                first_message_preview: None,
                updated_at: meta_file.updated_at,
            });
            continue;
        }

        // fallback：逐行扫描 transcript 并懒生成 session.json
        if let Some(derived) = derive_session_meta_from_transcript(&id) {
            let title = if derived.title.is_empty() {
                None
            } else {
                Some(derived.title.clone())
            };
            let preview_for_ui = title.clone();
            let _ = save_session_meta_file(&derived);
            sessions.push(SessionMeta {
                id: derived.id,
                title,
                message_count: derived.message_count,
                first_message_preview: preview_for_ui,
                updated_at: derived.updated_at,
            });
        }
    }
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    sessions
}

/// 生成会话 ID（时间戳微秒 + 进程 ID，无需外部依赖）
pub fn generate_session_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    let pid = std::process::id();
    format!("{:x}-{:x}", ts, pid)
}

/// 删除指定 session 目录
pub fn delete_session(session_id: &str) -> bool {
    let paths = SessionPaths::new(session_id);
    let dir = paths.dir().to_path_buf();
    if dir.exists()
        && let Err(e) = fs::remove_dir_all(&dir)
    {
        error!("✖️ 删除 session 目录失败: {}", e);
        return false;
    }
    true
}
