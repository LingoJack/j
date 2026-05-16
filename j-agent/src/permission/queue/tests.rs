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
        "Shell".into(),
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
        "Shell".into(),
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
        "Shell".into(),
        "run script".into(),
    );
    assert_eq!(perm.agent_type, AgentType::Teammate);
    assert_eq!(perm.name, "Backend");
    assert_eq!(perm.tool_name, "Shell");
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
        "Shell".into(),
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
        "Shell".into(),
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
