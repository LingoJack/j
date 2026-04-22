use crate::command::chat::agent::thread_identity::current_agent_name;
use crate::command::chat::app::types::PlanDecision;
use crate::command::chat::app::{AskOption, AskQuestion, AskRequest};
use crate::command::chat::tools::{Tool, ToolResult, schema_to_tool_params};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, atomic::AtomicBool, mpsc};

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

    /// Teammate 线程调用：把请求加入队列并阻塞等待（最长 120 秒）
    /// 返回 PlanDecision 表示用户决策
    pub fn request_blocking(&self, req: Arc<PendingPlanApproval>) -> PlanDecision {
        {
            let mut q = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            q.push_back(Arc::clone(&req));
        }
        req.wait_for_decision(120)
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
/// TODO 此处不应该 magic value 应该引用现有的 constant 常量
pub const PLAN_MODE_WHITELIST: &[&str] = &[
    "Read",
    "Glob",
    "Grep",
    "WebFetch",
    "WebSearch",
    "Ask",
    "Compact",
    "TodoRead",
    "TodoWrite",
    "TaskOutput",
    "Task",
    "EnterPlanMode",
    "ExitPlanMode",
    "EnterWorktree",
    "ExitWorktree",
    "Agent",     // plan mode 允许启动子 agent 做代码探索
    "AgentTeam", // plan mode 允许批量创建 teammate
];

/// 检查工具是否在 plan mode 白名单中
pub fn is_allowed_in_plan_mode(tool_name: &str) -> bool {
    PLAN_MODE_WHITELIST.contains(&tool_name)
}

// ========== EnterPlanModeTool ==========

/// 将描述文本转为安全的文件名（只保留字母数字、中文、下划线、短横线）
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c > '\u{4e00}')
        .collect::<String>()
        .trim()
        .to_string()
}

/// EnterPlanMode 参数
#[derive(Deserialize, JsonSchema)]
struct EnterPlanModeParams {
    /// Short description used as the plan file name (e.g. "add-auth" becomes plan-add-auth.md)
    #[serde(default)]
    description: Option<String>,
}

/// 进入计划模式工具，用于在编写代码前探索代码库并设计实现方案
#[derive(Debug)]
pub struct EnterPlanModeTool {
    /// 计划模式的全局共享状态
    pub plan_state: Arc<PlanModeState>,
}

impl EnterPlanModeTool {
    pub const NAME: &'static str = "EnterPlanMode";
}

impl Tool for EnterPlanModeTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Enter plan mode to explore the codebase and design an implementation approach before writing code.
        In plan mode, only read-only tools (Read, Glob, Grep, WebFetch, WebSearch, Ask, etc.) are available.
        Write tools (Bash, Write, Edit, etc.) will be blocked until plan mode is exited.

        Use this proactively before starting non-trivial implementation tasks. Prefer using EnterPlanMode when ANY of these apply:
        - New feature implementation with architectural decisions
        - Multiple valid approaches exist and user should choose
        - Code modifications that affect existing behavior
        - Multi-file changes (touching more than 2-3 files)
        - Unclear requirements that need exploration first

        Do NOT use for: single-line fixes, typos, or purely research/exploration tasks.

        The `description` parameter is used as the plan file name (e.g. "add-auth" → plan-add-auth.md).
        If a plan file with the same name already exists, you will be warned so you can choose a different name.
        Plan files are preserved after exiting plan mode for future reference.
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<EnterPlanModeParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: EnterPlanModeParams =
            serde_json::from_str(arguments).unwrap_or(EnterPlanModeParams { description: None });
        let description = params
            .description
            .as_deref()
            .unwrap_or("implementation-plan");

        // 创建 plan 目录
        let plan_dir = crate::command::chat::permission::JcliConfig::ensure_config_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join(".jcli"));
        let plans_dir = plan_dir.join("plans");
        let _ = std::fs::create_dir_all(&plans_dir);

        // 基于描述生成文件名（如 plan-add-auth.md）
        let safe_name = sanitize_filename(description);
        let file_name = if safe_name.is_empty() {
            format!("plan-{}.md", std::process::id())
        } else {
            format!("plan-{}.md", safe_name)
        };
        let plan_file = plans_dir.join(&file_name);
        let plan_path = plan_file.display().to_string();

        // 检查同名文件是否已存在
        let mut warning = String::new();
        if plan_file.exists() {
            match std::fs::read_to_string(&plan_file) {
                Ok(existing) => {
                    // 提取已有 plan 的第一行标题作为摘要
                    let first_line = existing.lines().next().unwrap_or("");
                    warning = format!(
                        "⚠️ Plan file already exists: {} (content starts with: {})\n\
                         The existing file will be overwritten. Consider using a different description to avoid this.\n\n",
                        plan_path, first_line
                    );
                }
                Err(_) => {
                    warning = format!(
                        "⚠️ Plan file already exists: {}\n\
                         The existing file will be overwritten. Consider using a different description to avoid this.\n\n",
                        plan_path
                    );
                }
            }
        }

        // 写入初始模板
        let template = format!("# Plan: {}\n\n## Steps\n\n1. \n\n## Notes\n\n", description);
        let _ = std::fs::write(&plan_file, &template);

        // 原子性地进入 plan mode
        match self.plan_state.enter(plan_path.clone()) {
            Ok(()) => ToolResult {
                output: format!(
                    "{}Entered plan mode. Plan file: {}\n\
                     In plan mode, only read-only tools are available.\n\
                     Write your plan to the plan file, then use ExitPlanMode when ready for user approval.\n\
                     Plan files are preserved after exit for future reference.",
                    warning, plan_path
                ),
                is_error: false,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
            Err(msg) => ToolResult {
                output: msg,
                is_error: false,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ========== ExitPlanModeTool ==========

/// ExitPlanMode 参数
#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct ExitPlanModeParams {
    /// Optional list of prompt-based permissions needed to implement the plan
    #[serde(default)]
    #[serde(rename = "allowedPrompts")]
    allowed_prompts: Option<Vec<AllowedPrompt>>,
}

/// 计划实施所需的权限描述
#[derive(Deserialize, JsonSchema)]
#[allow(dead_code)]
struct AllowedPrompt {
    /// The tool this prompt applies to (e.g. 'Bash')
    #[serde(default)]
    tool: Option<String>,
    /// Semantic description of the action (e.g. 'run tests')
    #[serde(default)]
    prompt: Option<String>,
}

/// 退出计划模式工具，读取计划文件并提交用户审批
pub struct ExitPlanModeTool {
    /// 计划模式的全局共享状态
    pub plan_state: Arc<PlanModeState>,
    /// 用于向主线程发送提问请求的通道发送端
    pub ask_tx: mpsc::Sender<AskRequest>,
    /// Plan 审批队列（teammate 通过此队列路由审批请求到主 TUI）
    pub plan_approval_queue: Option<Arc<PlanApprovalQueue>>,
}

impl ExitPlanModeTool {
    pub const NAME: &'static str = "ExitPlanMode";
}

impl Tool for ExitPlanModeTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Exit plan mode and submit the plan for user approval.
        Reads the plan file and presents it to the user for review.
        If approved, plan mode is deactivated and write tools become available again.
        If rejected, plan mode remains active so you can revise the plan.
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<ExitPlanModeParams>()
    }

    fn execute(&self, _arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        if !self.plan_state.is_active() {
            return ToolResult {
                output: "Not in plan mode. Use EnterPlanMode first.".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        // 读取 plan 文件内容
        let plan_content = match self.plan_state.get_plan_file_path() {
            Some(path) => match std::fs::read_to_string(&path) {
                Ok(content) => content,
                Err(e) => {
                    return ToolResult {
                        output: format!("Failed to read plan file: {}", e),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
            },
            None => {
                return ToolResult {
                    output: "No plan file path set.".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        // 判断是否在 teammate 线程中（非 Main agent）
        let agent_name = current_agent_name();
        if agent_name != "Main" {
            // Teammate 模式：通过 PlanApprovalQueue 路由到主 TUI
            if let Some(ref queue) = self.plan_approval_queue {
                // 从 plan 文件路径提取计划名
                let plan_file_path = self.plan_state.get_plan_file_path().unwrap_or_default();
                let plan_name = std::path::Path::new(&plan_file_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.strip_prefix("plan-"))
                    .unwrap_or("Plan")
                    .to_string();

                let req = PendingPlanApproval::new(agent_name, plan_content.clone(), plan_name);
                let decision = queue.request_blocking(req);

                match decision {
                    PlanDecision::Approve => {
                        let plan_file_path = self.plan_state.get_plan_file_path();
                        self.plan_state.exit();
                        let preserved_msg = plan_file_path
                            .as_deref()
                            .map(|p| format!("\nPlan file preserved at: {}", p))
                            .unwrap_or_default();
                        ToolResult {
                            output: format!(
                                "Plan approved! Exited plan mode. You can now proceed with implementation.{}",
                                preserved_msg
                            ),
                            is_error: false,
                            images: vec![],
                            plan_decision: PlanDecision::Approve,
                        }
                    }
                    PlanDecision::ApproveAndClearContext => {
                        let plan_file_path = self.plan_state.get_plan_file_path();
                        self.plan_state.exit();
                        let preserved_msg = plan_file_path
                            .as_deref()
                            .map(|p| format!("\nPlan file preserved at: {}", p))
                            .unwrap_or_default();
                        ToolResult {
                            output: format!(
                                "Plan approved with context clear! Exited plan mode.{}",
                                preserved_msg
                            ),
                            is_error: false,
                            images: vec![],
                            plan_decision: PlanDecision::ApproveAndClearContext,
                        }
                    }
                    PlanDecision::Reject | PlanDecision::None => {
                        ToolResult {
                            output: "Plan was not approved. Still in plan mode. Please revise your plan and try ExitPlanMode again.".to_string(),
                            is_error: false,
                            images: vec![],
                            plan_decision: PlanDecision::Reject,
                        }
                    }
                }
            } else {
                // 无队列（不应该出现），回退错误
                ToolResult {
                    output: "Plan approval not available in sub-agent mode (no queue). Add permission rules to avoid plan mode in teammates.".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
        } else {
            // 主 agent 模式：走原有的 ask_tx 通道
            self.execute_via_ask_tx(&plan_content)
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

impl ExitPlanModeTool {
    /// 主 agent 通过 ask_tx 通道审批
    fn execute_via_ask_tx(&self, plan_content: &str) -> ToolResult {
        // 通过 Ask 机制发送审批请求
        let (response_tx, response_rx) = mpsc::channel::<String>();

        let question_text = format!("请审阅以下实施计划，选择操作：\n\n{}", plan_content);

        // 从 plan 文件路径提取计划名（plan-xxx.md → xxx）
        let plan_file_path = self.plan_state.get_plan_file_path().unwrap_or_default();
        let plan_name = std::path::Path::new(&plan_file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_prefix("plan-"))
            .unwrap_or("Plan");

        let ask_request = AskRequest {
            questions: vec![AskQuestion {
                question: question_text,
                header: plan_name.to_string(),
                options: vec![
                    AskOption {
                        label: "批准计划".to_string(),
                        description: "批准此计划，保留当前上下文，开始实施".to_string(),
                    },
                    AskOption {
                        label: "批准并清空上下文".to_string(),
                        description: "批准计划并清空探索过程中的对话上下文，仅保留计划内容继续实施"
                            .to_string(),
                    },
                    AskOption {
                        label: "驳回计划".to_string(),
                        description: "拒绝此计划，留在 Plan Mode 中修改方案".to_string(),
                    },
                ],
                multi_select: false,
            }],
            response_tx,
        };

        if self.ask_tx.send(ask_request).is_err() {
            return ToolResult {
                output: "Failed to send approval request (main thread may have exited)".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        // 阻塞等待用户审批结果
        match response_rx.recv() {
            Ok(response) => {
                if response.contains("批准并清空上下文") {
                    let plan_file_path = self.plan_state.get_plan_file_path();
                    self.plan_state.exit();
                    let preserved_msg = plan_file_path
                        .as_deref()
                        .map(|p| format!("\nPlan file preserved at: {}", p))
                        .unwrap_or_default();
                    ToolResult {
                        output: format!(
                            "Plan approved with context clear! Exited plan mode.{}",
                            preserved_msg
                        ),
                        is_error: false,
                        images: vec![],
                        plan_decision: PlanDecision::ApproveAndClearContext,
                    }
                } else if response.contains("批准") {
                    let plan_file_path = self.plan_state.get_plan_file_path();
                    self.plan_state.exit();
                    let preserved_msg = plan_file_path
                        .as_deref()
                        .map(|p| format!("\nPlan file preserved at: {}", p))
                        .unwrap_or_default();
                    ToolResult {
                        output: format!(
                            "Plan approved! Exited plan mode. You can now proceed with implementation.{}",
                            preserved_msg
                        ),
                        is_error: false,
                        images: vec![],
                        plan_decision: PlanDecision::Approve,
                    }
                } else {
                    // 保持 plan mode，让 agent 修改 plan
                    ToolResult {
                        output: format!(
                            "Plan was not approved. Still in plan mode. User response: {}\nPlease revise your plan and try ExitPlanMode again.",
                            response
                        ),
                        is_error: false,
                        images: vec![],
                        plan_decision: PlanDecision::Reject,
                    }
                }
            }
            Err(_) => ToolResult {
                output: "Connection lost while waiting for approval".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }
}
