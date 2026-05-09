//! Plan Mode 状态管理与审批队列
//!
//! 包含：
//! - `PlanModeState`: Plan mode 全局状态（active + plan_file_path，受 Mutex 保护）
//! - `PlanApprovalQueue`: Teammate Plan 审批请求队列
//! - `PendingPlanApproval`: 单条审批请求（Condvar 阻塞等待）
//! - `PLAN_MODE_WHITELIST`: Plan mode 允许的工具白名单

use crate::app::types::PlanDecision;
use crate::tools::tool_names;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

/// Plan 审批等待超时（秒）
const PLAN_APPROVAL_TIMEOUT_SECS: u64 = 120;

// ========== Plan Approval Queue (Teammate → TUI) ==========

// NOTE: Cannot derive Debug - contains Condvar which does not implement Debug
/// 单条待决 Plan 审批请求（共享给 TUI 和 teammate 线程）
pub struct PendingPlanApproval {
    /// 发起请求的 agent 名称（"Frontend"/"Backend" 等）
    pub agent_name: String,
    /// Plan 文件内容
    pub plan_content: String,
    /// Plan 名称（plan-xxx.md 的 xxx）
    pub plan_name: String,
    /// 决策通知（None=未决, Some(PlanDecision)=已决）
    decision: Arc<(Mutex<Option<PlanDecision>>, Condvar)>,
}

impl PendingPlanApproval {
    /// 创建一条待决的 Plan 审批请求，返回 Arc 包装实例
    pub fn new(agent_name: String, plan_content: String, plan_name: String) -> Arc<Self> {
        Arc::new(Self {
            agent_name,
            plan_content,
            plan_name,
            decision: Arc::new((Mutex::new(None), Condvar::new())),
        })
    }

    /// Teammate 线程调用：阻塞等待决策，超时返回 Reject
    pub fn wait_for_decision(&self, timeout_secs: u64) -> PlanDecision {
        let (lock, cvar) = &*self.decision;
        let guard = lock.lock().unwrap_or_else(|e| e.into_inner());
        let (mut guard, _timed_out) = cvar
            .wait_timeout_while(guard, std::time::Duration::from_secs(timeout_secs), |d| {
                d.is_none()
            })
            .unwrap_or_else(|e| e.into_inner());
        if guard.is_none() {
            *guard = Some(PlanDecision::Reject);
        }
        guard.clone().unwrap_or(PlanDecision::Reject)
    }

    /// TUI 线程调用：设置决策并唤醒等待的 teammate 线程
    pub fn resolve(&self, decision: PlanDecision) {
        let (lock, cvar) = &*self.decision;
        let mut d = lock.lock().unwrap_or_else(|e| e.into_inner());
        *d = Some(decision);
        cvar.notify_one();
    }
}

/// Plan 审批请求队列（主 TUI 和所有 teammate 线程共享同一个 Arc 实例）
pub struct PlanApprovalQueue {
    pending: Mutex<VecDeque<Arc<PendingPlanApproval>>>,
}

impl Default for PlanApprovalQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanApprovalQueue {
    /// 创建新的 Plan 审批队列
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(VecDeque::new()),
        }
    }

    /// Teammate 线程调用：把请求加入队列并阻塞等待
    /// 返回 PlanDecision 表示用户决策
    pub fn request_blocking(&self, req: Arc<PendingPlanApproval>) -> PlanDecision {
        {
            let mut q = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            q.push_back(Arc::clone(&req));
        }
        req.wait_for_decision(PLAN_APPROVAL_TIMEOUT_SECS)
    }

    /// TUI 循环调用：取出下一个待决请求（非阻塞）
    pub fn pop_pending(&self) -> Option<Arc<PendingPlanApproval>> {
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop_front()
    }

    /// Session 取消时调用：拒绝所有挂起的请求，唤醒所有等待线程
    pub fn deny_all(&self) {
        let mut q = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        for req in q.drain(..) {
            req.resolve(PlanDecision::Reject);
        }
    }
}

// ========== Plan Mode State ==========

/// Plan mode 内部状态（受 Mutex 保护，保证原子性）
#[derive(Debug)]
struct PlanModeInner {
    active: bool,
    plan_file_path: Option<String>,
}

/// Plan Mode 全局状态（跨工具共享）
///
/// 使用单一 Mutex 保护 active + plan_file_path，避免以下并发问题：
/// - enter() 的 TOCTOU 竞态（先检查 is_active 再进入）
/// - exit() 不清理 plan_file_path
/// - is_active() 与 get_plan_file_path() 之间状态不一致
#[derive(Debug)]
pub struct PlanModeState {
    inner: Mutex<PlanModeInner>,
}

impl Default for PlanModeState {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanModeState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PlanModeInner {
                active: false,
                plan_file_path: None,
            }),
        }
    }

    /// 检查是否处于 plan mode
    pub fn is_active(&self) -> bool {
        self.inner.lock().map(|g| g.active).unwrap_or(false)
    }

    /// 进入 plan mode，同时设置 plan 文件路径
    /// 返回 Ok(()) 表示成功进入，Err(msg) 表示已在 plan mode
    pub fn enter(&self, path: impl Into<String>) -> Result<(), String> {
        let path = path.into();
        match self.inner.lock() {
            Ok(mut guard) => {
                if guard.active {
                    return Err("Already in plan mode. Use ExitPlanMode to exit.".to_string());
                }
                guard.active = true;
                guard.plan_file_path = Some(path);
                Ok(())
            }
            Err(e) => Err(format!("Lock poisoned: {}", e)),
        }
    }

    /// 退出 plan mode，保留 plan 文件（不删除）
    pub fn exit(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.active = false;
            guard.plan_file_path = None;
        }
    }

    /// 原子地检查是否 active 并获取 plan 文件路径
    /// 返回 (is_active, plan_file_path)
    pub fn get_state(&self) -> (bool, Option<String>) {
        match self.inner.lock() {
            Ok(guard) => (guard.active, guard.plan_file_path.clone()),
            Err(_) => (false, None),
        }
    }

    /// 获取 plan 文件路径（仅在 active 时有意义）
    pub fn get_plan_file_path(&self) -> Option<String> {
        self.inner.lock().ok()?.plan_file_path.clone()
    }
}

/// plan mode 下允许执行的工具白名单
///
/// 所有名称均引用 `tool_names` 常量，与各工具的 `NAME` 定义自动对齐。
pub const PLAN_MODE_WHITELIST: &[&str] = &[
    tool_names::READ,
    tool_names::GLOB,
    tool_names::GREP,
    tool_names::WEB_FETCH,
    tool_names::WEB_SEARCH,
    tool_names::ASK,
    tool_names::COMPACT,
    tool_names::TODO_READ,
    tool_names::TODO_WRITE,
    tool_names::TASK_OUTPUT,
    tool_names::TASK,
    tool_names::ENTER_PLAN_MODE,
    tool_names::EXIT_PLAN_MODE,
    tool_names::ENTER_WORKTREE,
    tool_names::EXIT_WORKTREE,
    tool_names::AGENT,    // plan mode 允许启动子 agent 做代码探索
    tool_names::TEAMMATE, // plan mode 允许创建 teammate
];

/// 检查工具是否在 plan mode 白名单中
pub fn is_allowed_in_plan_mode(tool_name: &str) -> bool {
    PLAN_MODE_WHITELIST.contains(&tool_name)
}
