/// 子 agent/teammate 权限请求队列
///
/// 当子 agent 需要执行需要确认的工具（Write、Edit、Bash 等）
/// 且未被 .jcli/permissions.yaml 预先允许时，把请求推入此队列并阻塞
/// 等待主 TUI 用户批准或拒绝。
///
/// 设计约束：
/// - 子 agent 线程调用 `request_blocking`，阻塞最长 60 秒
/// - 主 TUI 循环 poll `pop_pending`，展示对话框，用户 y/n 后调用 `resolve`
/// - session 取消时调用 `deny_all` 唤醒所有阻塞线程
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// 发起权限请求的 agent 类型
#[derive(Clone, Debug, PartialEq)]
pub enum AgentType {
    /// Teammate agent（name 为 teammate 名称，如 "Backend"）
    Teammate,
    /// SubAgent（name 为 sub_id，如 "sub_0001"）
    SubAgent,
}

// NOTE: Cannot derive Debug - contains Condvar which does not implement Debug
/// 单条待决权限请求（共享给 TUI 和 agent 线程）
pub struct PendingAgentPerm {
    /// 发起请求的 agent 类型
    pub agent_type: AgentType,
    /// 发起请求的 agent 名称（teammate 名 / sub_id）
    pub name: String,
    /// 工具名称（"Write"/"Edit"/"Bash"）
    pub tool_name: String,
    /// 工具调用的 JSON 参数
    #[allow(dead_code)]
    pub arguments: String,
    /// 工具自身生成的人读确认提示
    pub confirm_msg: String,
    /// 决策通知（None=未决, Some(true)=允许, Some(false)=拒绝）
    decision: Arc<(Mutex<Option<bool>>, Condvar)>,
}

impl PendingAgentPerm {
    pub fn new(
        agent_type: AgentType,
        name: String,
        tool_name: String,
        arguments: String,
        confirm_msg: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            agent_type,
            name,
            tool_name,
            arguments,
            confirm_msg,
            decision: Arc::new((Mutex::new(None), Condvar::new())),
        })
    }

    /// 权限请求标题：按 agent 类型区分显示
    pub fn title(&self) -> String {
        match &self.agent_type {
            AgentType::Teammate => format!(" 权限请求 [{}] ", self.name),
            AgentType::SubAgent => format!(" SubAgent 权限请求 [{}] ", self.name),
        }
    }

    /// 子 agent 线程调用：阻塞等待决策，超时返回 false（拒绝）
    pub fn wait_for_decision(&self, timeout_secs: u64) -> bool {
        let (lock, cvar) = &*self.decision;
        let guard = lock.lock().unwrap_or_else(|e| e.into_inner());
        let (guard, _timed_out) = cvar
            .wait_timeout_while(guard, Duration::from_secs(timeout_secs), |d| d.is_none())
            .unwrap_or_else(|e| e.into_inner());
        guard.unwrap_or(false)
    }

    /// TUI 线程调用：设置决策并唤醒等待的 agent 线程
    pub fn resolve(&self, approved: bool) {
        let (lock, cvar) = &*self.decision;
        let mut d = lock.lock().unwrap_or_else(|e| e.into_inner());
        *d = Some(approved);
        cvar.notify_one();
    }

    /// 是否已有决策（用于防止重复显示已处理的请求）
    #[allow(dead_code)]
    pub fn is_decided(&self) -> bool {
        self.decision
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }
}

/// 权限请求队列（主 TUI 和所有 agent 线程共享同一个 Arc 实例）
pub struct PermissionQueue {
    pending: Mutex<VecDeque<Arc<PendingAgentPerm>>>,
}

impl Default for PermissionQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionQueue {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(VecDeque::new()),
        }
    }

    /// agent 线程调用：把请求加入队列并阻塞等待（最长 60 秒）。
    /// 返回 true 表示用户批准，false 表示拒绝或超时。
    pub fn request_blocking(&self, req: Arc<PendingAgentPerm>) -> bool {
        {
            let mut q = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            q.push_back(Arc::clone(&req));
        }
        req.wait_for_decision(60)
    }

    /// TUI 循环调用：取出下一个待决请求（非阻塞）
    pub fn pop_pending(&self) -> Option<Arc<PendingAgentPerm>> {
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop_front()
    }

    /// session 取消时调用：拒绝所有挂起的请求，唤醒所有等待线程
    pub fn deny_all(&self) {
        let mut q = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        for req in q.drain(..) {
            req.resolve(false);
        }
    }
}
