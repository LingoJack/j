use crate::command::chat::storage::ChatMessage;
use crate::util::log::write_info_log;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use tokio_util::sync::CancellationToken;

// ========== Thread-local Agent Identity ==========

thread_local! {
    /// 当前线程所属的 agent 名称（主 agent 为 "Main"，teammate 为其名称）
    static CURRENT_AGENT_NAME: RefCell<String> = RefCell::new("Main".to_string());
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

/// Teammate 管理器：管理所有 teammate 实例、消息广播
#[allow(dead_code)]
pub struct TeammateManager {
    /// 所有 teammate 的句柄（key = name）
    pub teammates: HashMap<String, TeammateHandle>,
    /// 主 agent 的 pending_user_messages（广播时也要注入）
    pub main_pending: Arc<Mutex<Vec<ChatMessage>>>,
    /// 主 agent 的 shared_messages（teammate 消息也要写入以在 TUI 显示）
    pub shared_messages: Arc<Mutex<Vec<ChatMessage>>>,
}

#[allow(dead_code)]
impl TeammateManager {
    /// 创建管理器
    pub fn new(
        main_pending: Arc<Mutex<Vec<ChatMessage>>>,
        shared_messages: Arc<Mutex<Vec<ChatMessage>>>,
    ) -> Self {
        Self {
            teammates: HashMap::new(),
            main_pending,
            shared_messages,
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
                &formatted[..formatted.len().min(100)]
            ),
        );

        // 注入到主 agent 的 pending（如果发送者不是主 agent）
        if from != "Main"
            && let Ok(mut pending) = self.main_pending.lock()
        {
            pending.push(ChatMessage::text("user", &formatted));
        }

        // 注入到所有其他 teammate 的 pending
        for (name, handle) in &self.teammates {
            if name == from {
                continue; // 不给自己发
            }
            if let Ok(mut pending) = handle.pending_user_messages.lock() {
                pending.push(ChatMessage::text("user", &formatted));
            }
        }

        // Teammate 发出的消息写入 shared_messages 以在 TUI 中显示
        // Main agent 的消息不需要（Main 的工具调用本身已通过 agent loop 显示）
        if from != "Main"
            && let Ok(mut shared) = self.shared_messages.lock()
        {
            shared.push(ChatMessage::text("assistant", &formatted));
        }
    }

    /// 获取团队成员摘要（供 system prompt 使用）
    pub fn team_summary(&self) -> String {
        if self.teammates.is_empty() {
            return String::new();
        }

        let mut summary = String::from("## Teammates\n\n当前团队成员:\n");
        summary.push_str("- Main (主协调者)\n");
        for (name, handle) in &self.teammates {
            let status = if handle.running() {
                "工作中"
            } else {
                "空闲"
            };
            summary.push_str(&format!("- {} ({}) [{}]\n", name, handle.role, status));
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
            main_pending: Arc::new(Mutex::new(Vec::new())),
            shared_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
}
