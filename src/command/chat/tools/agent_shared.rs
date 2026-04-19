use crate::command::chat::api::{build_request_with_tools, create_openai_client};
use crate::command::chat::app::AskRequest;
use crate::command::chat::compact::new_invoked_skills_map;
use crate::command::chat::error::ChatError;
use crate::command::chat::hook::HookManager;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::permission_queue::{PendingAgentPerm, PermissionQueue};
use crate::command::chat::storage::{ChatMessage, ModelProvider, ToolCallItem};
use crate::command::chat::teammate::current_agent_name;
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::background::BackgroundManager;
use crate::command::chat::tools::plan::PlanApprovalQueue;
use crate::command::chat::tools::task::TaskManager;
use crate::util::log::write_info_log;
use rand::Rng;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    mpsc,
};
use std::time::Instant;

// ========== SubAgentTracker ==========

/// 子 Agent 细粒度运行状态
#[derive(Clone, Debug, PartialEq)]
pub enum SubAgentStatus {
    /// 刚注册，尚未进入循环
    Initializing,
    /// 正在调用 LLM 或执行工具
    Working,
    /// 正常完成
    Completed,
    /// 用户取消或父 agent 取消
    Cancelled,
    /// 出错（LLM 失败、工具异常等）
    Error(String),
}

impl SubAgentStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Initializing => "◐",
            Self::Working => "●",
            Self::Completed => "✓",
            Self::Cancelled => "✗",
            Self::Error(_) => "✗",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Initializing => "初始化",
            Self::Working => "工作中",
            Self::Completed => "已完成",
            Self::Cancelled => "已取消",
            Self::Error(_) => "错误",
        }
    }

    #[allow(dead_code)]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Error(_))
    }
}

/// 一个正在运行（或刚结束）的子 Agent 的快照
pub struct SubAgentSnapshot {
    pub id: String,
    pub description: String,
    pub mode: &'static str, // "foreground" | "background"
    pub is_running: Arc<AtomicBool>,
    pub system_prompt: Arc<Mutex<String>>,
    pub messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// 细粒度状态
    pub status: Arc<Mutex<SubAgentStatus>>,
    /// 当前正在执行的工具名
    pub current_tool: Arc<Mutex<Option<String>>>,
    /// 累计工具调用次数
    pub tool_calls_count: Arc<AtomicUsize>,
    /// 当前轮次（1-based）
    pub current_round: Arc<AtomicUsize>,
    /// 启动时刻（用于计算运行时长）
    pub started_at: Instant,
}

/// 子 Agent UI 展示快照（克隆无锁，给 UI 渲染用）
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct SubAgentDisplay {
    pub id: String,
    pub description: String,
    pub mode: &'static str,
    pub status: SubAgentStatus,
    pub current_tool: Option<String>,
    pub tool_calls_count: usize,
    pub current_round: usize,
    pub elapsed_secs: u64,
}

/// 管理所有运行中的子 Agent 快照，供 /dump 读取
pub struct SubAgentTracker {
    agents: Mutex<Vec<SubAgentSnapshot>>,
    counter: AtomicU64,
}

/// 单次 snapshot 元素：(id, description, mode, system_prompt, messages)
pub type RunningSubAgentDump = (String, String, &'static str, String, Vec<ChatMessage>);

/// register 返回的 handle 集合，供 loop 写入状态
#[allow(dead_code)]
pub struct SubAgentHandle {
    pub id: String,
    pub is_running: Arc<AtomicBool>,
    pub system_prompt: Arc<Mutex<String>>,
    pub messages: Arc<Mutex<Vec<ChatMessage>>>,
    pub status: Arc<Mutex<SubAgentStatus>>,
    pub current_tool: Arc<Mutex<Option<String>>>,
    pub tool_calls_count: Arc<AtomicUsize>,
    pub current_round: Arc<AtomicUsize>,
}

impl SubAgentTracker {
    pub fn new() -> Self {
        Self {
            agents: Mutex::new(Vec::new()),
            counter: AtomicU64::new(1),
        }
    }

    /// 注册一个子 Agent；返回 Handle 集合，供 loop 写入状态
    pub fn register(&self, description: &str, mode: &'static str) -> SubAgentHandle {
        let id = format!("sub_{:04}", self.counter.fetch_add(1, Ordering::Relaxed));
        let is_running = Arc::new(AtomicBool::new(true));
        let system_prompt = Arc::new(Mutex::new(String::new()));
        let messages = Arc::new(Mutex::new(Vec::new()));
        let status = Arc::new(Mutex::new(SubAgentStatus::Initializing));
        let current_tool = Arc::new(Mutex::new(None));
        let tool_calls_count = Arc::new(AtomicUsize::new(0));
        let current_round = Arc::new(AtomicUsize::new(0));
        if let Ok(mut list) = self.agents.lock() {
            list.push(SubAgentSnapshot {
                id: id.clone(),
                description: description.to_string(),
                mode,
                is_running: Arc::clone(&is_running),
                system_prompt: Arc::clone(&system_prompt),
                messages: Arc::clone(&messages),
                status: Arc::clone(&status),
                current_tool: Arc::clone(&current_tool),
                tool_calls_count: Arc::clone(&tool_calls_count),
                current_round: Arc::clone(&current_round),
                started_at: Instant::now(),
            });
        }
        SubAgentHandle {
            id,
            is_running,
            system_prompt,
            messages,
            status,
            current_tool,
            tool_calls_count,
            current_round,
        }
    }

    /// 采集当前所有仍在运行的子 Agent 的完整快照（供 /dump 使用）
    pub fn snapshot_running(&self) -> Vec<RunningSubAgentDump> {
        let list = match self.agents.lock() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };
        list.iter()
            .filter(|s| s.is_running.load(Ordering::Relaxed))
            .map(|s| {
                let sp = s
                    .system_prompt
                    .lock()
                    .map(|x| x.clone())
                    .unwrap_or_default();
                let msgs = s.messages.lock().map(|x| x.clone()).unwrap_or_default();
                (s.id.clone(), s.description.clone(), s.mode, sp, msgs)
            })
            .collect()
    }

    /// 采集所有子 Agent（含刚完成的）的 UI 展示快照
    pub fn display_snapshots(&self) -> Vec<SubAgentDisplay> {
        let list = match self.agents.lock() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };
        list.iter()
            .map(|s| {
                let status = s
                    .status
                    .lock()
                    .map(|x| x.clone())
                    .unwrap_or(SubAgentStatus::Working);
                let current_tool = s.current_tool.lock().ok().and_then(|t| t.clone());
                SubAgentDisplay {
                    id: s.id.clone(),
                    description: s.description.clone(),
                    mode: s.mode,
                    status,
                    current_tool,
                    tool_calls_count: s.tool_calls_count.load(Ordering::Relaxed),
                    current_round: s.current_round.load(Ordering::Relaxed),
                    elapsed_secs: s.started_at.elapsed().as_secs(),
                }
            })
            .collect()
    }

    /// 清理已结束的子 Agent（可在 register 时调用，防止列表无限增长）
    ///
    /// 保留完成/错误状态超过 30 秒后清理，给 UI 显示终态的时间。
    pub fn gc_finished(&self) {
        if let Ok(mut list) = self.agents.lock() {
            list.retain(|s| {
                if s.is_running.load(Ordering::Relaxed) {
                    return true;
                }
                // 非运行中：保留 30 秒后清理
                s.started_at.elapsed().as_secs() < 30
                    || matches!(
                        s.status.lock().map(|x| x.clone()),
                        Ok(SubAgentStatus::Working) | Ok(SubAgentStatus::Initializing)
                    )
            });
        }
    }
}

impl Default for SubAgentTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ========== AgentToolShared ==========

// NOTE: Cannot derive Debug - contains PermissionQueue, PlanApprovalQueue, SubAgentTracker
//       which do not implement Debug, and multiple Arc<Mutex<Option<T>>> fields
/// Agent 工具共享字段（AgentTool / AgentTeamTool / CreateTeammateTool 共用）
///
/// 所有字段均为 Arc 引用，Clone 开销极小。
/// 消除了三个 Tool struct 之间逐字段 hand-copy Arc 的重复代码。
#[derive(Clone)]
pub struct AgentToolShared {
    pub background_manager: Arc<BackgroundManager>,
    pub provider: Arc<Mutex<ModelProvider>>,
    pub system_prompt: Arc<Mutex<Option<String>>>,
    pub jcli_config: Arc<JcliConfig>,
    pub hook_manager: Arc<Mutex<HookManager>>,
    pub task_manager: Arc<TaskManager>,
    pub disabled_tools: Arc<Vec<String>>,
    /// 子 agent 权限请求队列（与主 TUI 共享同一个实例）
    pub permission_queue: Arc<PermissionQueue>,
    /// Plan 审批请求队列（与主 TUI 共享同一个实例，teammate ExitPlanMode 走此队列）
    pub plan_approval_queue: Arc<PlanApprovalQueue>,
    /// 子 agent 运行时快照追踪器（供 /dump 读取）
    pub sub_agent_tracker: Arc<SubAgentTracker>,
    /// 主 TUI 的 shared_agent_messages（子 agent 的 UI 状态行推送到这里）
    pub shared_messages: Arc<Mutex<Vec<ChatMessage>>>,
}

impl AgentToolShared {
    /// 构建子工具注册表（不含 skills，标准 ask channel）
    ///
    /// 返回未 Arc 包装的 ToolRegistry，调用者可在包装前注册额外工具（如 SendMessage）。
    /// 子注册表自动继承父 shared 的 permission_queue，使子 agent 权限请求能路由到主 TUI。
    pub fn build_sub_registry(&self) -> (ToolRegistry, mpsc::Receiver<AskRequest>) {
        let (ask_tx, ask_rx) = mpsc::channel::<AskRequest>();
        let mut registry = ToolRegistry::new(
            vec![], // 不传 skills
            ask_tx,
            Arc::clone(&self.background_manager),
            Arc::clone(&self.task_manager),
            Arc::clone(&self.hook_manager),
            new_invoked_skills_map(),
            "_sub_agent_", // 子 agent 不需要独立 session 存储
        );
        // 将权限队列传入子注册表，使子 agent 的阻塞式确认请求能到达主 TUI
        registry.permission_queue = Some(Arc::clone(&self.permission_queue));
        // 将 Plan 审批队列传入子注册表，使 teammate 的 ExitPlanMode 能路由到主 TUI
        registry.plan_approval_queue = Some(Arc::clone(&self.plan_approval_queue));
        (registry, ask_rx)
    }
}

// ========== Headless Loop 共享 Helper ==========

/// 创建 tokio runtime 和 OpenAI client
///
/// 供 run_headless_agent_loop 和 run_teammate_loop 共用。
pub fn create_runtime_and_client(
    provider: &ModelProvider,
) -> Result<
    (
        tokio::runtime::Runtime,
        async_openai::Client<async_openai::config::OpenAIConfig>,
    ),
    String,
> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create async runtime: {}", e))?;
    let client = create_openai_client(provider);
    Ok((rt, client))
}

/// 非流式调用 LLM（含指数退避重试）
///
/// 返回第一个 choice 的 message；出错时返回 Err(error_text)。
/// 对瞬时错误（网络超时、5xx、429）自动重试，策略比主 agent 更保守：
/// - 最多 2 次重试（主 agent 最多 5 次）
/// - 退避上限 15s（主 agent 30s）
/// - 仍失败则直接返回错误文本
pub fn call_llm_non_stream(
    rt: &tokio::runtime::Runtime,
    client: &async_openai::Client<async_openai::config::OpenAIConfig>,
    provider: &ModelProvider,
    messages: &[ChatMessage],
    tools: &[async_openai::types::chat::ChatCompletionTools],
    system_prompt: Option<&str>,
) -> Result<async_openai::types::chat::ChatChoice, String> {
    let request = build_request_with_tools(provider, messages, tools.to_vec(), system_prompt)
        .map_err(|e| format!("Failed to build request: {}", e))?;

    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        match rt.block_on(async { client.chat().create(request.clone()).await }) {
            Ok(response) => {
                return response
                    .choices
                    .into_iter()
                    .next()
                    .ok_or_else(|| "[No response from API]".to_string());
            }
            Err(e) => {
                let chat_err = ChatError::from(e);
                if let Some(policy) = headless_retry_policy(&chat_err)
                    && attempt <= policy.max_attempts
                {
                    let delay_ms = backoff_delay_ms(attempt, policy.base_ms, policy.cap_ms);
                    write_info_log(
                        "SubAgentLLM",
                        &format!(
                            "API 请求失败，{}ms 后重试 ({}/{})",
                            delay_ms, attempt, policy.max_attempts
                        ),
                    );
                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                    continue;
                }
                // 不可重试或已耗尽重试次数
                return Err(chat_err.display_message());
            }
        }
    }
}

// ========== Headless 重试策略 ==========

/// 子 agent / teammate 的重试策略（比主 agent 更保守）
struct HeadlessRetryPolicy {
    /// 最大重试次数（不含首次请求）
    max_attempts: u32,
    /// 首次退避基础延迟（毫秒）
    base_ms: u64,
    /// 延迟上限（毫秒）
    cap_ms: u64,
}

/// 根据错误类型确定重试策略
///
/// 策略设计原则（比主 agent 弱一档）：
/// - 网络瞬断（超时/断连）：基础 2s，最多 2 次
/// - 5xx 服务端过载（503/504/529）：基础 3s，最多 2 次
/// - 5xx 服务端错误（500/502）：基础 3s，最多 1 次
/// - 429：基础 5s，最多 2 次
/// - 消息中含过载关键词：基础 3s，最多 2 次
fn headless_retry_policy(error: &ChatError) -> Option<HeadlessRetryPolicy> {
    match error {
        ChatError::NetworkTimeout(_) | ChatError::NetworkError(_) => Some(HeadlessRetryPolicy {
            max_attempts: 2,
            base_ms: 2_000,
            cap_ms: 15_000,
        }),
        ChatError::ApiServerError { status, .. } => match status {
            503 | 504 | 529 => Some(HeadlessRetryPolicy {
                max_attempts: 2,
                base_ms: 3_000,
                cap_ms: 15_000,
            }),
            500 | 502 => Some(HeadlessRetryPolicy {
                max_attempts: 1,
                base_ms: 3_000,
                cap_ms: 15_000,
            }),
            _ => None,
        },
        ChatError::ApiRateLimit { .. } => Some(HeadlessRetryPolicy {
            max_attempts: 2,
            base_ms: 5_000,
            cap_ms: 30_000,
        }),
        ChatError::AbnormalFinish(reason)
            if matches!(reason.as_str(), "network_error" | "timeout" | "overloaded") =>
        {
            Some(HeadlessRetryPolicy {
                max_attempts: 2,
                base_ms: 2_000,
                cap_ms: 15_000,
            })
        }
        ChatError::Other(msg)
            if msg.contains("访问量过大")
                || msg.contains("过载")
                || msg.contains("overloaded")
                || msg.contains("too busy")
                || msg.contains("1305") =>
        {
            Some(HeadlessRetryPolicy {
                max_attempts: 2,
                base_ms: 3_000,
                cap_ms: 15_000,
            })
        }
        _ => None,
    }
}

/// 计算第 `attempt`（从 1 开始）次重试的退避延迟（毫秒）
///
/// 公式：`clamp(base * 2^(attempt-1), 0, cap) + jitter(0..20%)`
fn backoff_delay_ms(attempt: u32, base_ms: u64, cap_ms: u64) -> u64 {
    let shift = (attempt - 1).min(10) as u64;
    let exp = base_ms.saturating_mul(1u64 << shift).min(cap_ms);
    let jitter = rand::thread_rng().gen_range(0..=(exp / 5));
    exp + jitter
}

/// 从 LLM response 的 tool_calls 中提取 ToolCallItem 列表
pub fn extract_tool_items(
    tool_calls: &[async_openai::types::chat::ChatCompletionMessageToolCalls],
) -> Vec<ToolCallItem> {
    tool_calls
        .iter()
        .filter_map(|tc| {
            if let async_openai::types::chat::ChatCompletionMessageToolCalls::Function(f) = tc {
                Some(ToolCallItem {
                    id: f.id.clone(),
                    name: f.function.name.clone(),
                    arguments: f.function.arguments.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

/// 执行单个工具调用（含权限检查）
///
/// 返回 tool role 的 ChatMessage。
/// - 被拒绝/需要确认时返回拒绝消息
/// - 正常执行时返回工具结果
/// - 被取消时返回 [Cancelled]
pub fn execute_tool_with_permission(
    item: &ToolCallItem,
    registry: &Arc<ToolRegistry>,
    jcli_config: &Arc<JcliConfig>,
    cancelled: &Arc<AtomicBool>,
    log_tag: &str,
    verbose: bool,
) -> ChatMessage {
    if cancelled.load(Ordering::Relaxed) {
        return ChatMessage {
            role: "tool".to_string(),
            content: "[Cancelled]".to_string(),
            tool_calls: None,
            tool_call_id: Some(item.id.clone()),
            images: None,
        };
    }

    // deny 检查
    if jcli_config.is_denied(&item.name, &item.arguments) {
        if verbose {
            write_info_log(log_tag, &format!("Tool denied by deny rule: {}", item.name));
        }
        return ChatMessage {
            role: "tool".to_string(),
            content: format!("Tool '{}' was denied by permission rules.", item.name),
            tool_calls: None,
            tool_call_id: Some(item.id.clone()),
            images: None,
        };
    }

    // 确认检查
    let tool_ref = registry.get(&item.name);
    let requires_confirm = tool_ref.map(|t| t.requires_confirmation()).unwrap_or(false);

    if requires_confirm && !jcli_config.is_allowed(&item.name, &item.arguments) {
        // 尝试通过权限队列请求用户实时确认
        if let Some(queue) = registry.permission_queue.as_ref() {
            let agent_name = current_agent_name();
            let confirm_msg = tool_ref
                .map(|t| t.confirmation_message(&item.arguments))
                .unwrap_or_else(|| format!("调用工具 {}", item.name));
            let req = PendingAgentPerm::new(
                agent_name,
                item.name.clone(),
                item.arguments.clone(),
                confirm_msg,
            );
            write_info_log(
                log_tag,
                &format!(
                    "Tool '{}' queued for user permission (60s timeout)",
                    item.name
                ),
            );
            let approved = queue.request_blocking(req);
            if !approved {
                write_info_log(log_tag, &format!("Tool '{}' denied by user", item.name));
                return ChatMessage {
                    role: "tool".to_string(),
                    content: format!("Tool '{}' was denied by the user.", item.name),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                };
            }
            // 用户批准 → 继续往下执行
        } else {
            if verbose {
                write_info_log(
                    log_tag,
                    &format!(
                        "Tool '{}' requires confirmation but not auto-allowed, denying",
                        item.name
                    ),
                );
            }
            return ChatMessage {
                role: "tool".to_string(),
                content: format!(
                    "Tool '{}' requires user confirmation which is not available in sub-agent mode. \
                     Add a permission rule to allow this tool automatically.",
                    item.name
                ),
                tool_calls: None,
                tool_call_id: Some(item.id.clone()),
                images: None,
            };
        }
    }

    if verbose {
        write_info_log(
            log_tag,
            &format!("Executing tool: {} args: {}", item.name, item.arguments),
        );
    }

    let result = registry.execute(&item.name, &item.arguments, cancelled);

    if verbose {
        write_info_log(
            log_tag,
            &format!(
                "Tool result: {} is_error={} len={}",
                item.name,
                result.is_error,
                result.output.len()
            ),
        );
    }

    ChatMessage {
        role: "tool".to_string(),
        content: result.output,
        tool_calls: None,
        tool_call_id: Some(item.id.clone()),
        images: None,
    }
}
