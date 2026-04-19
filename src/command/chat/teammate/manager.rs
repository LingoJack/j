use crate::command::chat::storage::{ChatMessage, TeammateSnapshotPersist};
use crate::util::log::write_info_log;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio_util::sync::CancellationToken;

// ========== Teammate 状态枚举 ==========

/// Teammate 的细粒度运行状态
#[derive(Clone, Debug, PartialEq)]
pub enum TeammateStatus {
    /// 刚创建，尚未开始
    Initializing,
    /// 正在调用 LLM 或执行工具
    Working,
    /// 空闲轮询等待新消息
    WaitingForMessage,
    /// 正常完成
    Completed,
    /// 被取消
    Cancelled,
    /// 出错
    Error(String),
}

/// Teammate 状态的可序列化版本（用于 session 持久化）
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TeammateStatusPersist {
    Initializing,
    Working,
    WaitingForMessage,
    Completed,
    Cancelled,
    Error(String),
}

impl From<TeammateStatus> for TeammateStatusPersist {
    fn from(status: TeammateStatus) -> Self {
        match status {
            TeammateStatus::Initializing => Self::Initializing,
            TeammateStatus::Working => Self::Working,
            TeammateStatus::WaitingForMessage => Self::WaitingForMessage,
            TeammateStatus::Completed => Self::Completed,
            TeammateStatus::Cancelled => Self::Cancelled,
            TeammateStatus::Error(e) => Self::Error(e),
        }
    }
}

impl From<TeammateStatusPersist> for TeammateStatus {
    fn from(status: TeammateStatusPersist) -> Self {
        match status {
            TeammateStatusPersist::Initializing => Self::Initializing,
            TeammateStatusPersist::Working => Self::Working,
            TeammateStatusPersist::WaitingForMessage => Self::WaitingForMessage,
            TeammateStatusPersist::Completed => Self::Completed,
            TeammateStatusPersist::Cancelled => Self::Cancelled,
            TeammateStatusPersist::Error(e) => Self::Error(e),
        }
    }
}

impl TeammateStatus {
    /// 状态符号（极简风格）
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Initializing => "◐",
            Self::Working => "●",
            Self::WaitingForMessage => "○",
            Self::Completed => "✓",
            Self::Cancelled => "✗",
            Self::Error(_) => "✗",
        }
    }

    /// 状态文字
    pub fn label(&self) -> &'static str {
        match self {
            Self::Initializing => "初始化",
            Self::Working => "工作中",
            Self::WaitingForMessage => "等待中",
            Self::Completed => "已完成",
            Self::Cancelled => "已取消",
            Self::Error(_) => "错误",
        }
    }

    /// 是否为终态（不会再变化）
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Error(_))
    }
}

/// Teammate 状态快照（供 UI 渲染用，无锁）
#[derive(Clone, Debug)]
pub struct TeammateSnapshot {
    pub name: String,
    pub role: String,
    pub status: TeammateStatus,
    pub current_tool: Option<String>,
    pub tool_calls_count: usize,
}

// ========== Thread-local Agent Identity ==========

thread_local! {
    /// 当前线程所属的 agent 名称（主 agent 为 "Main"，teammate 为其名称）
    static CURRENT_AGENT_NAME: RefCell<String> = RefCell::new("Main".to_string());
    /// 当前线程的工作目录覆盖（worktree 模式下指向 worktree 路径）
    static THREAD_CWD: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// 设置当前线程的 agent 名称（在 teammate agent loop 启动时调用）
pub fn set_current_agent_name(name: &str) {
    CURRENT_AGENT_NAME.with(|cell| {
        *cell.borrow_mut() = name.to_string();
    });
}

/// 获取当前线程的 agent 名称
pub fn current_agent_name() -> String {
    CURRENT_AGENT_NAME.with(|cell| cell.borrow().clone())
}

// ========== Thread-local CWD (worktree 隔离) ==========

/// 设置当前线程的工作目录（进入 worktree 时调用）
pub fn set_thread_cwd(path: &std::path::Path) {
    THREAD_CWD.with(|cell| {
        *cell.borrow_mut() = Some(path.to_path_buf());
    });
}

/// 获取当前线程的工作目录覆盖（None 表示未进入 worktree）
pub fn thread_cwd() -> Option<PathBuf> {
    THREAD_CWD.with(|cell| cell.borrow().clone())
}

/// 清除当前线程的工作目录覆盖
pub fn clear_thread_cwd() {
    THREAD_CWD.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

// ========== 全局文件编辑锁 ==========

/// 全局文件编辑锁（所有 agent 共享，进程级单例）
static GLOBAL_FILE_LOCKS: OnceLock<Mutex<HashMap<PathBuf, String>>> = OnceLock::new();

fn global_file_locks() -> &'static Mutex<HashMap<PathBuf, String>> {
    GLOBAL_FILE_LOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 尝试获取全局文件编辑锁
/// 返回 Ok(FileLockGuard) 成功，Err(holder_name) 表示被其他 agent 持有
pub fn acquire_global_file_lock(
    path: &std::path::Path,
    agent_name: &str,
) -> Result<FileLockGuard, String> {
    let canonical = path.to_path_buf();
    let mut map = global_file_locks()
        .lock()
        .map_err(|_| "file_locks mutex poisoned".to_string())?;

    if let Some(holder) = map.get(&canonical)
        && holder != agent_name
    {
        return Err(holder.clone());
    }

    map.insert(canonical.clone(), agent_name.to_string());
    Ok(FileLockGuard {
        path: canonical,
        agent_name: agent_name.to_string(),
    })
}

// ========== TeammateHandle ==========

// NOTE: Cannot derive Debug - contains JoinHandle<()> and CancellationToken which do not implement Debug
/// 单个 Teammate 的句柄（持有其 agent loop 的引用和通道）
#[allow(dead_code)]
pub struct TeammateHandle {
    /// Teammate 名称（如 "Frontend", "Backend"）
    pub name: String,
    /// 角色描述（如 "React frontend developer"）
    pub role: String,
    /// Teammate 的 pending_user_messages（广播消息注入到这里）
    pub pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// Teammate 的流式内容缓冲区
    pub streaming_content: Arc<Mutex<String>>,
    /// 取消令牌
    pub cancel_token: CancellationToken,
    /// 是否正在运行
    pub is_running: Arc<AtomicBool>,
    /// agent loop 线程句柄
    pub thread_handle: Option<std::thread::JoinHandle<()>>,
    /// Teammate 当前 system prompt 快照（由 agent loop 在启动时写入，供 /dump 读取）
    pub system_prompt_snapshot: Arc<Mutex<String>>,
    /// Teammate 当前 messages 快照（由 agent loop 每轮同步，供 /dump 读取）
    pub messages_snapshot: Arc<Mutex<Vec<ChatMessage>>>,
    /// 细粒度运行状态
    pub status: Arc<Mutex<TeammateStatus>>,
    /// 累计工具调用次数
    pub tool_calls_count: Arc<AtomicUsize>,
    /// 当前正在执行的工具名（None 表示未在执行工具）
    pub current_tool: Arc<Mutex<Option<String>>>,
    /// 唤醒标志：@自己 或来自 Main 时 set。
    /// 未 WorkDone 时，任何 pending 消息都会唤醒 teammate；
    /// WorkDone 后，只有 @self 才能重新激活（清除 work_done）。
    pub wake_flag: Arc<AtomicBool>,
    /// WorkDone 终态标志：WorkDone 工具调用后 set，teammate_loop 读到后立即进入 Completed。
    pub work_done: Arc<AtomicBool>,
}

#[allow(dead_code)]
impl TeammateHandle {
    /// 检查 teammate 是否仍在运行
    pub fn running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// 取消 teammate 的 agent loop
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }
}

// ========== FileLockGuard ==========

/// RAII 文件锁守卫：Drop 时自动释放锁
pub struct FileLockGuard {
    path: PathBuf,
    agent_name: String,
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        if let Ok(mut map) = global_file_locks().lock()
            && map.get(&self.path).map(|s| s.as_str()) == Some(self.agent_name.as_str())
        {
            map.remove(&self.path);
        }
    }
}

// ========== TeammateManager ==========

// NOTE: Cannot derive Debug - contains TeammateHandle which has JoinHandle and CancellationToken
/// Teammate 管理器：管理所有 teammate 实例、消息广播
#[allow(dead_code)]
pub struct TeammateManager {
    /// 所有 teammate 的句柄（key = name）
    pub teammates: HashMap<String, TeammateHandle>,
    /// Teammate → Main agent LLM 上下文通道（broadcast 时注入，drain_pending_user_messages 消费）
    pub pending_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// Agent/Teammate → UI 显示通道（teammate 消息也要写入以在 TUI 显示）
    pub ui_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// 从 session 恢复的 teammate 快照（只读展示，无活跃线程）
    recovered_teammates: HashMap<String, TeammateSnapshotPersist>,
}

#[allow(dead_code)]
impl TeammateManager {
    /// 创建管理器
    pub fn new(
        pending_messages: Arc<Mutex<Vec<ChatMessage>>>,
        ui_messages: Arc<Mutex<Vec<ChatMessage>>>,
    ) -> Self {
        Self {
            teammates: HashMap::new(),
            pending_messages,
            ui_messages,
            recovered_teammates: HashMap::new(),
        }
    }

    /// 广播消息到所有其他 agent 的 pending_user_messages
    ///
    /// - `from`: 发送者名称
    /// - `text`: 消息内容
    /// - `at_target`: 可选的 @目标（消息仍广播给所有人，但带 @前缀）
    ///
    /// 消息格式: `<FromAgent> @Target text` 或 `<FromAgent> text`
    /// 以 user 角色注入（和用户 append 消息走同一个 drain 机制）
    pub fn broadcast(&self, from: &str, text: &str, at_target: Option<&str>) {
        let formatted = if let Some(target) = at_target {
            format!("<{}> @{} {}", from, target, text)
        } else {
            format!("<{}> {}", from, text)
        };

        write_info_log(
            "TeammateManager",
            &format!(
                "broadcast from={}: {}",
                from,
                &formatted[..{
                    let mut b = formatted.len().min(100);
                    while b > 0 && !formatted.is_char_boundary(b) {
                        b -= 1;
                    }
                    b
                }]
            ),
        );

        // 注入到主 agent 的 pending（如果发送者不是主 agent）
        if from != "Main"
            && let Ok(mut pending) = self.pending_messages.lock()
        {
            pending.push(ChatMessage::text("user", &formatted));
        }

        // 注入到所有其他 teammate 的 pending
        // 唤醒语义：@self 或 from==Main 时 set wake_flag（用于 WorkDone 后重新激活判断）
        // 非 WorkDone 状态下，pending 有消息就唤醒，不依赖 wake_flag
        for (name, handle) in &self.teammates {
            if name == from {
                continue; // 不给自己发
            }
            if let Ok(mut pending) = handle.pending_user_messages.lock() {
                pending.push(ChatMessage::text("user", &formatted));
            }
            let should_wake = from == "Main" || at_target == Some(name.as_str());
            if should_wake {
                handle.wake_flag.store(true, Ordering::Relaxed);
            }
        }

        // Teammate 发出的消息写入 ui_messages 以在 TUI 中显示
        // Main agent 的消息不需要（Main 的工具调用本身已通过 agent loop 显示）
        if from != "Main"
            && let Ok(mut shared) = self.ui_messages.lock()
        {
            shared.push(ChatMessage::text("assistant", &formatted));
        }
    }

    /// 获取团队成员摘要（供 system prompt 使用）
    pub fn team_summary(&self) -> String {
        if self.teammates.is_empty() && self.recovered_teammates.is_empty() {
            return String::new();
        }

        let mut summary = String::from("## Teammates\n\n当前团队成员:\n");
        summary.push_str("- Main (主协调者)\n");
        for (name, handle) in &self.teammates {
            let status = handle
                .status
                .lock()
                .map(|s| format!("{} {}", s.icon(), s.label()))
                .unwrap_or_else(|_| {
                    if handle.running() {
                        "● 工作中".to_string()
                    } else {
                        "○ 空闲".to_string()
                    }
                });
            summary.push_str(&format!("- {} ({}) [{}]\n", name, handle.role, status));
        }
        // 展示从 session 恢复的 teammate（只读历史）
        for (name, snapshot) in &self.recovered_teammates {
            let status: TeammateStatus = snapshot.status.clone().into();
            summary.push_str(&format!(
                "- {} ({}) [{} 🔄session-recovery]\n",
                name,
                snapshot.role,
                status.label()
            ));
        }
        summary.push_str(
            "\n使用 SendMessage 工具向其他 agent 发送消息。可以用 @AgentName 指定目标。\n",
        );
        summary
    }

    /// 获取所有 teammate 名称列表（包含 "Main"）
    pub fn all_names(&self) -> Vec<String> {
        let mut names = vec!["Main".to_string()];
        names.extend(self.teammates.keys().cloned());
        names
    }

    /// 获取所有 teammate 的状态快照（供 UI 渲染用，无锁拷贝）
    pub fn teammate_snapshots(&self) -> Vec<TeammateSnapshot> {
        self.teammates
            .iter()
            .map(|(name, handle)| {
                let status = handle
                    .status
                    .lock()
                    .map(|s| s.clone())
                    .unwrap_or(TeammateStatus::Initializing);
                let current_tool = handle.current_tool.lock().ok().and_then(|t| t.clone());
                let tool_calls_count = handle.tool_calls_count.load(Ordering::Relaxed);
                TeammateSnapshot {
                    name: name.clone(),
                    role: handle.role.clone(),
                    status,
                    current_tool,
                    tool_calls_count,
                }
            })
            .collect()
    }

    /// 停止指定 teammate
    pub fn stop_teammate(&mut self, name: &str) {
        if let Some(handle) = self.teammates.get(name) {
            handle.cancel();
            write_info_log("TeammateManager", &format!("stopped teammate: {}", name));
        }
    }

    /// 停止所有 teammates
    pub fn stop_all(&mut self) {
        for (name, handle) in &self.teammates {
            handle.cancel();
            write_info_log("TeammateManager", &format!("stopping teammate: {}", name));
        }
    }

    /// 清理已完成的 teammate（回收 thread handle）
    pub fn cleanup_finished(&mut self) {
        let finished: Vec<String> = self
            .teammates
            .iter()
            .filter(|(_, h)| {
                !h.running()
                    && h.thread_handle
                        .as_ref()
                        .map(|t| t.is_finished())
                        .unwrap_or(true)
            })
            .map(|(name, _)| name.clone())
            .collect();

        for name in finished {
            if let Some(mut handle) = self.teammates.remove(&name) {
                if let Some(th) = handle.thread_handle.take() {
                    let _ = th.join();
                }
                write_info_log("TeammateManager", &format!("cleaned up teammate: {}", name));
            }
        }
    }

    /// 注册一个 teammate（由 CreateTeammate 工具或 teammate_loop 调用）
    pub fn register_teammate(&mut self, handle: TeammateHandle) {
        write_info_log(
            "TeammateManager",
            &format!("registered teammate: {} ({})", handle.name, handle.role),
        );
        self.teammates.insert(handle.name.clone(), handle);
    }
}

impl Default for TeammateManager {
    fn default() -> Self {
        Self {
            teammates: HashMap::new(),
            pending_messages: Arc::new(Mutex::new(Vec::new())),
            ui_messages: Arc::new(Mutex::new(Vec::new())),
            recovered_teammates: HashMap::new(),
        }
    }
}

// ========== Recovered Teammates 方法 ==========

impl TeammateManager {
    /// 设置从 session 恢复的 teammate 快照
    pub fn set_recovered_teammates(&mut self, teammates: Vec<TeammateSnapshotPersist>) {
        self.recovered_teammates = teammates.into_iter().map(|t| (t.name.clone(), t)).collect();
    }

    /// 清除所有 recovered teammates
    pub fn clear_recovered_teammates(&mut self) {
        self.recovered_teammates.clear();
    }

    /// 获取 recovered teammates 的快照引用（用于 save 时合并 prompt 信息）
    pub fn recovered_teammates_snapshot(&self) -> HashMap<String, TeammateSnapshotPersist> {
        self.recovered_teammates.clone()
    }

    /// 获取 recovered teammate 的名称和角色列表（用于 UI 展示）
    #[allow(dead_code)]
    pub fn recovered_teammates_list(&self) -> Vec<(String, String, TeammateStatusPersist)> {
        self.recovered_teammates
            .iter()
            .map(|(name, t)| (name.clone(), t.role.clone(), t.status.clone()))
            .collect()
    }

    /// 获取指定名称的 recovered teammate（用于 RespawnTeammate）
    pub fn get_recovered_teammate(&self, name: &str) -> Option<TeammateSnapshotPersist> {
        self.recovered_teammates.get(name).cloned()
    }

    /// 移除一个 recovered teammate（respawn 成功后）
    pub fn remove_recovered_teammate(&mut self, name: &str) {
        self.recovered_teammates.remove(name);
    }
}
