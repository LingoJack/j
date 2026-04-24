/// 派生 Agent 权限请求队列
///
/// 当派生 Agent（SubAgent / Teammate）需要执行需要确认的工具（Write、Edit、Bash 等）
/// 且未被 .jcli/permissions.yaml 预先允许时，把请求推入此队列并阻塞
/// 等待主 TUI 用户批准或拒绝。
///
/// 设计约束：
/// - 派生 Agent 线程调用 `request_blocking`，阻塞最长 60 秒
/// - 主 TUI 循环 poll `pop_pending`，展示对话框，用户 y/n 后调用 `resolve`
/// - session 取消时调用 `deny_all` 唤醒所有阻塞线程
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// 发起权限请求的 agent 类型
#[derive(Clone, Debug, PartialEq)]
pub enum AgentType {
    /// 主 Agent（拥有 TUI，直接与用户交互；当前不会进入权限队列，但作为默认值预留）
    Main,
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
        confirm_msg: String,
    ) -> Arc<Self> {
        Arc::new(Self {
            agent_type,
            name,
            tool_name,
            confirm_msg,
            decision: Arc::new((Mutex::new(None), Condvar::new())),
        })
    }

    /// 权限请求标题：按 agent 类型区分显示
    pub fn title(&self) -> String {
        match &self.agent_type {
            AgentType::Main => " 权限请求 [Main] ".to_string(),
            AgentType::Teammate => format!(" 权限请求 [{}] ", self.name),
            AgentType::SubAgent => format!(" SubAgent 权限请求 [{}] ", self.name),
        }
    }

    /// 派生 Agent 线程调用：阻塞等待决策，超时返回 false（拒绝）
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

    /// 派生 Agent 线程调用：把请求加入队列并阻塞等待（最长 60 秒）。
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    // ════════════════════════════════════════════════════════════════
    // 回归测试：PendingAgentPerm 生命周期
    // 如果以下测试失败，说明权限请求的创建/决策/等待链路被破坏
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn agent_type_title_format() {
        let main_perm = PendingAgentPerm::new(
            AgentType::Main,
            "Main".into(),
            "Bash".into(),
            "run script".into(),
        );
        assert_eq!(main_perm.title(), " 权限请求 [Main] ");

        let teammate_perm = PendingAgentPerm::new(
            AgentType::Teammate,
            "Backend".into(),
            "Write".into(),
            "write file".into(),
        );
        assert_eq!(teammate_perm.title(), " 权限请求 [Backend] ");

        let sub_perm = PendingAgentPerm::new(
            AgentType::SubAgent,
            "sub_0001".into(),
            "Edit".into(),
            "edit file".into(),
        );
        assert_eq!(sub_perm.title(), " SubAgent 权限请求 [sub_0001] ");
    }

    #[test]
    fn pending_perm_resolve_approved() {
        let perm = PendingAgentPerm::new(
            AgentType::Teammate,
            "Backend".into(),
            "Bash".into(),
            "run tests".into(),
        );
        // 在另一个线程中等待
        let perm_clone = Arc::clone(&perm);
        let handle = thread::spawn(move || perm_clone.wait_for_decision(5));

        // 主线程批准
        perm.resolve(true);

        let result = handle.join().expect("线程不应 panic");
        assert!(result, "resolve(true) 后 wait_for_decision 应返回 true");
    }

    #[test]
    fn pending_perm_resolve_denied() {
        let perm = PendingAgentPerm::new(
            AgentType::SubAgent,
            "sub_0001".into(),
            "Write".into(),
            "write file".into(),
        );
        let perm_clone = Arc::clone(&perm);
        let handle = thread::spawn(move || perm_clone.wait_for_decision(5));

        perm.resolve(false);

        let result = handle.join().expect("线程不应 panic");
        assert!(!result, "resolve(false) 后 wait_for_decision 应返回 false");
    }

    #[test]
    fn pending_perm_timeout_returns_false() {
        let perm = PendingAgentPerm::new(
            AgentType::Teammate,
            "Frontend".into(),
            "Edit".into(),
            "edit file".into(),
        );
        // 不 resolve，等待极短超时
        let result = perm.wait_for_decision(1);
        assert!(!result, "超时未 resolve 时应返回 false");
    }

    #[test]
    fn pending_perm_agent_type_equality() {
        assert_eq!(AgentType::Main, AgentType::Main);
        assert_eq!(AgentType::Teammate, AgentType::Teammate);
        assert_eq!(AgentType::SubAgent, AgentType::SubAgent);
        assert_ne!(AgentType::Main, AgentType::Teammate);
        assert_ne!(AgentType::Teammate, AgentType::SubAgent);
    }

    #[test]
    fn pending_perm_fields_preserved() {
        let perm = PendingAgentPerm::new(
            AgentType::Teammate,
            "Backend".into(),
            "Bash".into(),
            "run script".into(),
        );
        assert_eq!(perm.agent_type, AgentType::Teammate);
        assert_eq!(perm.name, "Backend");
        assert_eq!(perm.tool_name, "Bash");
        assert_eq!(perm.confirm_msg, "run script");
    }

    // ════════════════════════════════════════════════════════════════
    // 回归测试：PermissionQueue 队列行为
    // ════════════════════════════════════════════════════════════════

    #[test]
    fn queue_empty_pop_returns_none() {
        let queue = PermissionQueue::new();
        assert!(queue.pop_pending().is_none(), "空队列 pop 应返回 None");
    }

    #[test]
    fn queue_request_and_pop_fifo() {
        let queue = PermissionQueue::new();

        let req1 = PendingAgentPerm::new(
            AgentType::Teammate,
            "Backend".into(),
            "Bash".into(),
            "first".into(),
        );
        let req2 = PendingAgentPerm::new(
            AgentType::SubAgent,
            "sub_0001".into(),
            "Write".into(),
            "second".into(),
        );

        // 模拟 push_back（直接操作内部 pending）
        // 由于 request_blocking 会阻塞，我们直接通过内部机制测试
        // 这里用 Arc clone 来模拟入队
        let mut q = queue.pending.lock().unwrap_or_else(|e| e.into_inner());
        q.push_back(Arc::clone(&req1));
        q.push_back(Arc::clone(&req2));
        drop(q);

        // FIFO 顺序
        let popped1 = queue.pop_pending();
        assert!(popped1.is_some(), "第一次 pop 应有结果");
        assert_eq!(popped1.unwrap().confirm_msg, "first");

        let popped2 = queue.pop_pending();
        assert!(popped2.is_some(), "第二次 pop 应有结果");
        assert_eq!(popped2.unwrap().confirm_msg, "second");

        assert!(queue.pop_pending().is_none(), "第三次 pop 应为 None");
    }

    #[test]
    fn queue_deny_all_resolves_false() {
        let queue = PermissionQueue::new();

        let req1 = PendingAgentPerm::new(
            AgentType::Teammate,
            "Frontend".into(),
            "Edit".into(),
            "edit css".into(),
        );
        let req2 = PendingAgentPerm::new(
            AgentType::Teammate,
            "Backend".into(),
            "Bash".into(),
            "run build".into(),
        );

        // 入队
        {
            let mut q = queue.pending.lock().unwrap_or_else(|e| e.into_inner());
            q.push_back(Arc::clone(&req1));
            q.push_back(Arc::clone(&req2));
        }

        // deny_all 唤醒所有
        queue.deny_all();

        // 验证所有请求被拒绝
        // 短暂等待确保线程同步
        thread::sleep(Duration::from_millis(50));
        assert!(!req1.wait_for_decision(0), "deny_all 后 req1 应被拒绝");
        assert!(!req2.wait_for_decision(0), "deny_all 后 req2 应被拒绝");

        // 队列已清空
        assert!(queue.pop_pending().is_none(), "deny_all 后队列应为空");
    }

    #[test]
    fn queue_default_is_new() {
        let queue = PermissionQueue::default();
        assert!(queue.pop_pending().is_none(), "default 队列应为空");
    }
}
