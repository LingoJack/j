//! 线程级 Agent 身份与工作目录隔离。
//!
//! 主 Agent、SubAgent、Teammate 各自运行在独立线程中，
//! 通过 thread_local 记录当前线程所属的 agent 名称/类型以及工作目录覆盖。

use crate::command::chat::permission::queue::AgentType;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

// ========== Thread-local Agent Identity ==========

thread_local! {
    /// 当前线程所属的 agent 名称（主 agent 为 "Main"，teammate 为其名称，subagent 为 sub_id）
    static CURRENT_AGENT_NAME: RefCell<String> = RefCell::new("Main".to_string());
    /// 当前线程所属的 agent 类型（默认 Main；teammate 设置为 Teammate，subagent 设置为 SubAgent）
    static CURRENT_AGENT_TYPE: RefCell<AgentType> = const { RefCell::new(AgentType::Main) };
    /// 当前线程的工作目录覆盖（worktree 模式下指向 worktree 路径）
    static THREAD_CWD: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// 设置当前线程的 agent 名称（在 teammate/subagent agent loop 启动时调用）
pub fn set_current_agent_name(name: &str) {
    CURRENT_AGENT_NAME.with(|cell| {
        *cell.borrow_mut() = name.to_string();
    });
}

/// 设置当前线程的 agent 类型
pub fn set_current_agent_type(agent_type: AgentType) {
    CURRENT_AGENT_TYPE.with(|cell| {
        *cell.borrow_mut() = agent_type;
    });
}

/// 获取当前线程的 agent 名称
pub fn current_agent_name() -> String {
    CURRENT_AGENT_NAME.with(|cell| cell.borrow().clone())
}

/// 获取当前线程的 agent 类型
pub fn current_agent_type() -> AgentType {
    CURRENT_AGENT_TYPE.with(|cell| cell.borrow().clone())
}

// ========== Thread-local CWD (worktree 隔离) ==========

/// 设置当前线程的工作目录（进入 worktree 时调用）
pub fn set_thread_cwd(path: &Path) {
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
