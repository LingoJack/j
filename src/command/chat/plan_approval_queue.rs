use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

/// 计划审批结果
#[derive(Debug, Clone, PartialEq)]
pub enum PlanDecision {
    /// 批准，继续上下文
    Approve,
    /// 批准并清空探索上下文
    ApproveAndClear,
    /// 拒绝，保留 plan mode 继续修改
    Reject,
}

/// 单个待审批计划请求
pub struct PendingPlanApproval {
    /// 计划名称（文件名 stem）
    pub plan_name: String,
    /// 计划内容（plan 文件全文）
    pub plan_content: String,
    /// (决策, 通知)
    decision: Arc<(Mutex<Option<PlanDecision>>, Condvar)>,
}

impl PendingPlanApproval {
    pub fn new(plan_name: String, plan_content: String) -> Arc<Self> {
        Arc::new(Self {
            plan_name,
            plan_content,
            decision: Arc::new((Mutex::new(None), Condvar::new())),
        })
    }

    /// 阻塞等待用户决策，超时（120s）自动拒绝
    pub fn wait_for_decision(&self, timeout_secs: u64) -> PlanDecision {
        let (lock, cvar) = &*self.decision;
        let guard = lock.lock().unwrap();
        let (mut guard, _timed_out) = cvar
            .wait_timeout_while(
                guard,
                std::time::Duration::from_secs(timeout_secs),
                |d| d.is_none(),
            )
            .unwrap();
        if guard.is_none() {
            *guard = Some(PlanDecision::Reject);
        }
        guard.clone().unwrap_or(PlanDecision::Reject)
    }

    /// 由 TUI 侧调用，设置决策并唤醒等待线程
    pub fn resolve(&self, decision: PlanDecision) {
        let (lock, cvar) = &*self.decision;
        if let Ok(mut guard) = lock.lock() {
            if guard.is_none() {
                *guard = Some(decision);
                cvar.notify_all();
            }
        }
    }

    pub fn is_decided(&self) -> bool {
        self.decision
            .0
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }
}

/// 计划审批队列（子 agent → TUI）
pub struct PlanApprovalQueue {
    pending: Mutex<VecDeque<Arc<PendingPlanApproval>>>,
}

impl Default for PlanApprovalQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanApprovalQueue {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(VecDeque::new()),
        }
    }

    /// 子 agent 调用：将计划加入队列并阻塞等待决策
    pub fn request_blocking(&self, req: Arc<PendingPlanApproval>) -> PlanDecision {
        {
            let mut q = self.pending.lock().unwrap();
            q.push_back(Arc::clone(&req));
        }
        req.wait_for_decision(120)
    }

    /// TUI 侧每帧轮询：取出队首待审批项（无则返回 None）
    pub fn pop_pending(&self) -> Option<Arc<PendingPlanApproval>> {
        self.pending.lock().ok()?.pop_front()
    }

    /// 取消所有待审批（CancelStream 时调用）
    pub fn reject_all(&self) {
        if let Ok(mut q) = self.pending.lock() {
            for req in q.drain(..) {
                req.resolve(PlanDecision::Reject);
            }
        }
    }
}
