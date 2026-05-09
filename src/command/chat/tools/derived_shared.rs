use crate::command::chat::agent::api::{build_request_with_tools, create_llm_client};
use crate::command::chat::agent::thread_identity::{current_agent_name, current_agent_type};
use crate::command::chat::app::AskRequest;
use crate::command::chat::context::compact::{CompactConfig, new_invoked_skills_map};
use crate::command::chat::error::ChatError;
use crate::command::chat::infra::hook::HookManager;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::permission::queue::{PendingAgentPerm, PermissionQueue};
use crate::command::chat::storage::{
    ChatMessage, DisplayHint, MessageRole, ModelProvider, ToolCallItem,
};
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::background::BackgroundManager;
use crate::command::chat::tools::plan::{PlanApprovalQueue, PlanModeState};
use crate::command::chat::tools::task::TaskManager;
use crate::llm::{ChatResponse, ToolCall, ToolDefinition};
use crate::util::log::write_info_log;
use rand::Rng;
use serde::{Deserialize, Serialize};
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
    /// 正在调用 LLM（等待模型回复）
    Thinking,
    /// 正在执行工具
    Working,
    /// LLM API 重试中（指数退避）
    Retrying {
        attempt: u32,
        max_attempts: u32,
        delay_ms: u64,
        error: String,
    },
    /// 正常完成
    Completed,
    /// 用户取消或父 agent 取消
    Cancelled,
    /// 出错（LLM 失败、工具异常等）
    Error(String),
}

impl SubAgentStatus {
    /// 返回当前状态对应的图标字符
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Initializing => "◐",
            Self::Thinking => "◐",
            Self::Working => "●",
            Self::Retrying { .. } => "↻",
            Self::Completed => "✓",
            Self::Cancelled => "✗",
            Self::Error(_) => "✗",
        }
    }

    /// 返回当前状态的中文标签
    pub fn label(&self) -> &'static str {
        match self {
            Self::Initializing => "初始化",
            Self::Thinking => "思考中",
            Self::Working => "执行中",
            Self::Retrying { .. } => "重试中",
            Self::Completed => "已完成",
            Self::Cancelled => "已取消",
            Self::Error(_) => "错误",
        }
    }

    /// 判断当前状态是否为终止状态（已完成、已取消或出错）
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

/// 管理所有运行中的子 Agent 快照，供 /dump 读取。
///
/// **仅追踪 `AgentTool` 创建的临时子代理（SubAgent），不含 Teammate
/// （Teammate 由 `TeammateManager` 独立管理）。**
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
    /// 创建新的子 Agent 追踪器实例
    pub fn new() -> Self {
        Self {
            agents: Mutex::new(Vec::new()),
            counter: AtomicU64::new(1),
        }
    }

    /// 分配一个新的 sub_id（不注册，仅递增计数器）。
    ///
    /// 用于在注册前提前确定 sub_id，以便构建子 agent 独立存储路径（transcript、todos）。
    pub fn allocate_id(&self) -> String {
        format!("sub_{:04}", self.counter.fetch_add(1, Ordering::Relaxed))
    }

    /// 使用已分配的 id 注册子 Agent；返回 Handle 集合。
    pub fn register_with_id(
        &self,
        id: String,
        description: &str,
        mode: &'static str,
    ) -> SubAgentHandle {
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
                        Ok(SubAgentStatus::Working)
                            | Ok(SubAgentStatus::Initializing)
                            | Ok(SubAgentStatus::Retrying { .. })
                    )
            });
        }
    }

    /// 清除所有子 Agent（session 切换/clear 时调用）。
    ///
    /// 将所有仍在运行的子 Agent 标记为非运行，然后清空追踪列表。
    pub fn clear_all(&self) {
        if let Ok(mut list) = self.agents.lock() {
            for s in list.iter() {
                s.is_running.store(false, Ordering::Relaxed);
            }
            list.clear();
        }
        self.counter.store(1, Ordering::Relaxed);
    }
}

impl Default for SubAgentTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ========== SubAgentMetrics ==========

/// 子 Agent（SubAgent / Teammate）的 metrics 累加器
///
/// 每个子 Agent loop 内部累加，Main agent loop 结束时读取并合并到 `SessionMetrics`。
/// 使用 `Arc<Mutex<SubAgentMetrics>>` 实现多 Agent 并发安全写入。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubAgentMetrics {
    /// LLM API 调用次数
    pub total_llm_calls: u32,
    /// 工具调用次数
    pub total_tool_calls: u32,
    /// 输入 token 总数
    pub total_input_tokens: u64,
    /// 输出 token 总数
    pub total_output_tokens: u64,
    /// LLM 调用总耗时（毫秒）
    pub total_llm_elapsed_ms: u64,
    /// 工具执行总耗时（毫秒）
    pub total_tool_elapsed_ms: u64,
    /// 每次非流式 LLM 调用的耗时（等同于整次调用耗时，非流式无 TTFT 概念）
    pub llm_elapsed_ms_per_call: Vec<u64>,
}

// ========== DerivedAgentShared ==========

// NOTE: Cannot derive Debug - contains PermissionQueue, PlanApprovalQueue, SubAgentTracker
//       which do not implement Debug, and multiple Arc<Mutex<Option<T>>> fields
/// 派生 Agent（SubAgent / Teammate）共享字段
///
/// 所有字段均为 Arc 引用，Clone 开销极小。
/// 消除了三个 Tool struct 之间逐字段 hand-copy Arc 的重复代码。
#[derive(Clone)]
pub struct DerivedAgentShared {
    pub background_manager: Arc<BackgroundManager>,
    pub provider: Arc<Mutex<ModelProvider>>,
    pub system_prompt: Arc<Mutex<Option<String>>>,
    pub jcli_config: Arc<JcliConfig>,
    pub hook_manager: Arc<Mutex<HookManager>>,
    pub task_manager: Arc<TaskManager>,
    pub disabled_tools: Arc<Vec<String>>,
    /// 延迟加载的工具列表（子 agent 需继承父 agent 的 defer 设置）
    pub deferred_tools: Arc<Mutex<Vec<String>>>,
    /// 本会话 LoadTool 已加载的 deferred 工具（子 agent 需继承以保持一致）
    pub session_loaded_deferred: Arc<Mutex<Vec<String>>>,
    /// 子 agent 权限请求队列（与主 TUI 共享同一个实例）
    pub permission_queue: Arc<PermissionQueue>,
    /// Plan 审批请求队列（与主 TUI 共享同一个实例，teammate ExitPlanMode 走此队列）
    pub plan_approval_queue: Arc<PlanApprovalQueue>,
    /// 子 agent 运行时快照追踪器（供 /dump 读取）
    pub sub_agent_tracker: Arc<SubAgentTracker>,
    /// Agent/Teammate → UI 显示通道（子 agent 的 UI 状态行推送到这里，仅 UI 渲染用）
    pub display_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// Agent/Teammate → LLM context 同步通道（显式注入 Main Agent context）
    pub context_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// 当前 session id（session 切换时由 chat_app 更新，teammate/subagent 用来定位自己的 transcript 路径）
    pub session_id: Arc<Mutex<String>>,
    /// 父 agent 的 plan mode 状态（子 agent 据此决定是否进入只读模式）
    pub plan_mode_state: Arc<PlanModeState>,
    /// 父 agent 的上下文配置快照（子 agent 据此复用 select_messages + micro_compact）。
    /// chat_app 在 send_message 时刷新，确保子 agent 拿到最新配置。
    pub agent_context_config: Arc<Mutex<AgentContextConfig>>,
    /// 父 agent 禁用的 hook 列表（Teammate 走 PreLlmRequest hook 链时需要）。
    pub disabled_hooks: Arc<Mutex<Vec<String>>>,
    /// 子 Agent metrics 累加器（SubAgent/Teammate 的 LLM/tool 统计）
    /// Main agent loop 结束时读取并合并到 `SessionMetrics`
    pub sub_agent_metrics: Arc<Mutex<SubAgentMetrics>>,
}

/// 子 agent 调用 LLM 前用到的上下文配置（从父 AgentConfig 抽取）
#[derive(Clone, Debug)]
pub struct AgentContextConfig {
    pub max_history_messages: usize,
    pub max_context_tokens: usize,
    pub compact: CompactConfig,
}

impl DerivedAgentShared {
    /// 构建子工具注册表（不含 skills，标准 ask channel）
    ///
    /// 返回未 Arc 包装的 ToolRegistry，调用者可在包装前注册额外工具（如 SendMessage）。
    /// 子注册表自动继承父 shared 的 permission_queue，使子 agent 权限请求能路由到主 TUI。
    ///
    /// `todos_file_path`：该子 agent 独立的 todos.json 存储路径。
    /// - Teammate 传 `sessions/<sid>/teammates/<sanitized_name>/todos.json`
    /// - SubAgent 传 `sessions/<sid>/subagents/<sub_id>/todos.json`
    pub fn build_child_registry(
        &self,
        todos_file_path: std::path::PathBuf,
    ) -> (ToolRegistry, mpsc::Receiver<AskRequest>) {
        let (ask_tx, ask_rx) = mpsc::channel::<AskRequest>();

        let mut registry = ToolRegistry::new(
            vec![], // 不传 skills
            ask_tx,
            Arc::clone(&self.background_manager),
            Arc::clone(&self.task_manager),
            Arc::clone(&self.hook_manager),
            new_invoked_skills_map(),
            todos_file_path,
        );
        // 将权限队列传入子注册表，使子 agent 的阻塞式确认请求能到达主 TUI
        registry.permission_queue = Some(Arc::clone(&self.permission_queue));
        // 将 Plan 审批队列传入子注册表，使 teammate 的 ExitPlanMode 能路由到主 TUI
        registry.plan_approval_queue = Some(Arc::clone(&self.plan_approval_queue));
        // 共享主 agent 的 plan mode 状态，子 agent 据此继承只读限制
        registry.plan_mode_state = Arc::clone(&self.plan_mode_state);
        (registry, ask_rx)
    }
} // ========== Derived Agent Loop 共享 Helper ==========

/// 创建 tokio runtime 和 LlmClient
///
/// 供 run_sub_agent_loop 和 run_teammate_loop 共用。
pub fn create_runtime_and_client(
    provider: &ModelProvider,
) -> Result<(tokio::runtime::Runtime, crate::llm::LlmClient), String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create async runtime: {}", e))?;
    let client = create_llm_client(provider);
    Ok((rt, client))
}

/// call_llm_non_stream 的请求参数（封装 7 个独立参数为结构体）
pub struct LlmNonStreamRequest<'a> {
    pub rt: &'a tokio::runtime::Runtime,
    pub client: &'a crate::llm::LlmClient,
    pub provider: &'a ModelProvider,
    pub messages: &'a [ChatMessage],
    pub tools: &'a [ToolDefinition],
    pub system_prompt: Option<&'a str>,
    pub on_retry: Option<&'a RetryCallback>,
}

/// 非流式调用 LLM（含指数退避重试）
///
/// 返回第一个 choice 的 message；出错时返回 Err(error_text)。
/// LLM API 重试回调：参数为 (attempt, max_attempts, delay_ms, error_message)
pub type RetryCallback = dyn Fn(u32, u32, u64, &str);

/// 对瞬时错误（网络超时、5xx、429）自动重试，策略针对多 Agent 并发优化：
/// - 最多 8 次重试（并发 Agent 更易触发 rate limit）
/// - 退避上限 30–60s（与主 agent 对齐）
/// - 仍失败则直接返回错误文本
///
/// 返回完整的 `ChatResponse`（包含 `choices` 和 `usage`），
/// 调用方需自行提取第一个 choice 并按需读取 usage。
pub fn call_llm_non_stream(req: &LlmNonStreamRequest) -> Result<ChatResponse, String> {
    let request = build_request_with_tools(
        req.provider,
        req.messages,
        req.tools.to_vec(),
        req.system_prompt,
    )
    .map_err(|e| format!("Failed to build request: {}", e))?;

    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        match req
            .rt
            .block_on(async { req.client.chat_completion(&request).await })
        {
            Ok(response) => {
                // 保留空 choice 校验，但返回完整 response（含 usage）
                if response.choices.is_empty() {
                    return Err("[No response from API]".to_string());
                }
                return Ok(response);
            }
            Err(e) => {
                let chat_err = ChatError::from(e);
                if let Some(policy) = derived_retry_policy(&chat_err)
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
                    if let Some(cb) = req.on_retry {
                        cb(
                            attempt,
                            policy.max_attempts,
                            delay_ms,
                            &chat_err.display_message(),
                        );
                    }
                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                    continue;
                }
                return Err(chat_err.display_message());
            }
        }
    }
}

// ========== Derived Agent 重试策略 ==========

/// 派生 Agent 的重试策略（比主 Agent 更保守）
struct DerivedRetryPolicy {
    /// 最大重试次数（不含首次请求）
    max_attempts: u32,
    /// 首次退避基础延迟（毫秒）
    base_ms: u64,
    /// 延迟上限（毫秒）
    cap_ms: u64,
}

/// 派生 Agent 的重试策略（多 Agent 并发场景下更宽容）：
/// - 最多 8 次重试（并发 Agent 更易触发 rate limit，需更多重试机会）
/// - 退避上限 30–60s（与主 agent 对齐）
/// - 仍失败则直接返回错误文本
fn derived_retry_policy(error: &ChatError) -> Option<DerivedRetryPolicy> {
    match error {
        ChatError::NetworkTimeout(_) | ChatError::NetworkError(_) => Some(DerivedRetryPolicy {
            max_attempts: 8,
            base_ms: 2_000,
            cap_ms: 30_000,
        }),
        ChatError::ApiServerError { status, .. } => match status {
            503 | 504 | 529 => Some(DerivedRetryPolicy {
                max_attempts: 8,
                base_ms: 3_000,
                cap_ms: 30_000,
            }),
            500 | 502 => Some(DerivedRetryPolicy {
                max_attempts: 8,
                base_ms: 3_000,
                cap_ms: 30_000,
            }),
            _ => None,
        },
        ChatError::ApiRateLimit { .. } => Some(DerivedRetryPolicy {
            max_attempts: 8,
            base_ms: 5_000,
            cap_ms: 60_000,
        }),
        ChatError::AbnormalFinish(reason)
            if matches!(reason.as_str(), "network_error" | "timeout" | "overloaded") =>
        {
            Some(DerivedRetryPolicy {
                max_attempts: 8,
                base_ms: 2_000,
                cap_ms: 30_000,
            })
        }
        ChatError::Other(msg)
            if msg.contains("访问量过大")
                || msg.contains("过载")
                || msg.contains("overloaded")
                || msg.contains("too busy")
                || msg.contains("1305") =>
        {
            Some(DerivedRetryPolicy {
                max_attempts: 8,
                base_ms: 3_000,
                cap_ms: 30_000,
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
pub fn extract_tool_items(tool_calls: &[ToolCall]) -> Vec<ToolCallItem> {
    tool_calls
        .iter()
        .map(|tc| ToolCallItem {
            id: tc.id.clone(),
            name: tc.function.name.clone(),
            arguments: tc.function.arguments.clone(),
        })
        .collect()
}

/// execute_tool_with_permission 的上下文参数（封装 5 个独立参数为结构体）
pub struct ToolExecContext<'a> {
    pub registry: &'a Arc<ToolRegistry>,
    pub jcli_config: &'a Arc<JcliConfig>,
    pub cancelled: &'a Arc<AtomicBool>,
    pub log_tag: &'a str,
    pub verbose: bool,
}

/// 执行单个工具调用（含权限检查）
///
/// 返回 tool role 的 ChatMessage。
/// - 被拒绝/需要确认时返回拒绝消息
/// - 正常执行时返回工具结果
/// - 被取消时返回 [Cancelled]
pub fn execute_tool_with_permission(item: &ToolCallItem, ctx: &ToolExecContext) -> ChatMessage {
    if ctx.cancelled.load(Ordering::Relaxed) {
        return ChatMessage {
            role: MessageRole::Tool,
            content: "[Cancelled]".to_string(),
            tool_calls: None,
            tool_call_id: Some(item.id.clone()),
            images: None,
            reasoning_content: None,
            sender_name: None,
            recipient_name: None,
            display_hint: DisplayHint::Normal,
        };
    }

    // deny 检查
    if ctx.jcli_config.is_denied(&item.name, &item.arguments) {
        if ctx.verbose {
            write_info_log(
                ctx.log_tag,
                &format!("Tool denied by deny rule: {}", item.name),
            );
        }
        return ChatMessage {
            role: MessageRole::Tool,
            content: format!("Tool '{}' was denied by permission rules.", item.name),
            tool_calls: None,
            tool_call_id: Some(item.id.clone()),
            images: None,
            reasoning_content: None,
            sender_name: None,
            recipient_name: None,
            display_hint: DisplayHint::Normal,
        };
    }

    // 确认检查
    let tool_ref = ctx.registry.get(&item.name);
    let requires_confirm = tool_ref.map(|t| t.requires_confirmation()).unwrap_or(false);

    if requires_confirm && !ctx.jcli_config.is_allowed(&item.name, &item.arguments) {
        // 尝试通过权限队列请求用户实时确认
        if let Some(queue) = ctx.registry.permission_queue.as_ref() {
            let agent_type = current_agent_type();
            let agent_name = current_agent_name();
            let confirm_msg = tool_ref
                .map(|t| t.confirmation_message(&item.arguments))
                .unwrap_or_else(|| format!("调用工具 {}", item.name));
            let req = PendingAgentPerm::new(agent_type, agent_name, item.name.clone(), confirm_msg);
            write_info_log(
                ctx.log_tag,
                &format!(
                    "Tool '{}' queued for user permission (60s timeout)",
                    item.name
                ),
            );
            let approved = queue.request_blocking(req);
            if !approved {
                write_info_log(ctx.log_tag, &format!("Tool '{}' denied by user", item.name));
                return ChatMessage {
                    role: MessageRole::Tool,
                    content: format!("Tool '{}' was denied by the user.", item.name),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                    reasoning_content: None,
                    sender_name: None,
                    recipient_name: None,
                    display_hint: DisplayHint::Normal,
                };
            }
            // 用户批准 → 继续往下执行
        } else {
            if ctx.verbose {
                write_info_log(
                    ctx.log_tag,
                    &format!(
                        "Tool '{}' requires confirmation but not auto-allowed, denying",
                        item.name
                    ),
                );
            }
            return ChatMessage {
                role: MessageRole::Tool,
                content: format!(
                    "Tool '{}' requires user confirmation which is not available in sub-agent mode. \
                     Add a permission rule to allow this tool automatically.",
                    item.name
                ),
                tool_calls: None,
                tool_call_id: Some(item.id.clone()),
                images: None,
                reasoning_content: None,
                sender_name: None,
                recipient_name: None,
                display_hint: DisplayHint::Normal,
            };
        }
    }

    if ctx.verbose {
        write_info_log(
            ctx.log_tag,
            &format!("Executing tool: {} args: {}", item.name, item.arguments),
        );
    }

    let result = ctx
        .registry
        .execute(&item.name, &item.arguments, ctx.cancelled);

    if ctx.verbose {
        write_info_log(
            ctx.log_tag,
            &format!(
                "Tool result: {} is_error={} len={}",
                item.name,
                result.is_error,
                result.output.len()
            ),
        );
    }

    ChatMessage {
        role: MessageRole::Tool,
        content: result.output,
        tool_calls: None,
        tool_call_id: Some(item.id.clone()),
        images: None,
        reasoning_content: None,
        sender_name: None,
        recipient_name: None,
        display_hint: DisplayHint::Normal,
    }
}
