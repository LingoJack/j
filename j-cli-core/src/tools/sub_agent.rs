use crate::agent::thread_identity::{
    clear_thread_cwd, current_agent_name, set_current_agent_name, set_current_agent_type,
    set_thread_cwd, thread_cwd,
};
use crate::permission::JcliConfig;
use crate::permission::queue::AgentType;
use crate::storage::{
    ChatMessage, DisplayHint, MessageRole, ModelProvider, SessionEvent, SessionPaths, ToolCallItem,
    append_event_to_path,
};
use crate::tools::derived_shared::{
    DerivedAgentShared, LlmNonStreamRequest, SubAgentHandle, SubAgentStatus, ToolExecContext,
    call_llm_non_stream, create_runtime_and_client, execute_tool_with_permission,
    extract_tool_items,
};
use crate::tools::worktree::{create_agent_worktree, remove_agent_worktree};
use crate::tools::{
    PlanDecision, Tool, ToolRegistry, ToolResult, parse_tool_args, schema_to_tool_params,
};
use crate::llm::ToolDefinition;
use crate::util::log::write_info_log;
use crate::util::safe_lock;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::borrow::Cow;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

/// 子 Agent 运行时状态引用集合，供 loop 同步状态/系统提示/消息列表。
struct SubAgentLoopStateRefs {
    system_prompt: Arc<Mutex<String>>,
    messages: Arc<Mutex<Vec<ChatMessage>>>,
    status: Arc<Mutex<SubAgentStatus>>,
    current_tool: Arc<Mutex<Option<String>>>,
    tool_calls_count: Arc<AtomicUsize>,
    current_round: Arc<AtomicUsize>,
}

impl SubAgentLoopStateRefs {
    fn from_handle(handle: &SubAgentHandle) -> Self {
        Self {
            system_prompt: Arc::clone(&handle.system_prompt),
            messages: Arc::clone(&handle.messages),
            status: Arc::clone(&handle.status),
            current_tool: Arc::clone(&handle.current_tool),
            tool_calls_count: Arc::clone(&handle.tool_calls_count),
            current_round: Arc::clone(&handle.current_round),
        }
    }

    fn set_status(&self, status: SubAgentStatus) {
        if let Ok(mut s) = self.status.lock() {
            *s = status;
        }
    }

    fn set_current_tool(&self, name: Option<String>) {
        if let Ok(mut t) = self.current_tool.lock() {
            *t = name;
        }
    }
}

/// 无 UI 子代理循环的参数集合
struct SubAgentLoopParams {
    provider: ModelProvider,
    system_prompt: Option<String>,
    prompt: String,
    tools: Vec<ToolDefinition>,
    registry: Arc<ToolRegistry>,
    jcli_config: Arc<JcliConfig>,
    snapshot: Option<SubAgentLoopStateRefs>,
    description: String,
    /// 独立 transcript JSONL 路径：每轮消息 append 到此（崩溃安全）。
    transcript_path: Option<std::path::PathBuf>,
    /// 父 agent 的上下文配置快照（供 select_messages + micro_compact 复用）
    context_config: crate::tools::derived_shared::AgentContextConfig,
    /// 子 Agent metrics 累加器（与 DerivedAgentShared.sub_agent_metrics 共享）
    sub_agent_metrics: Arc<Mutex<crate::tools::derived_shared::SubAgentMetrics>>,
}

/// 将任意描述转为适合作为 <前缀> 显示的名字（去空白，限长度）
fn sanitize_agent_name(description: &str) -> String {
    let cleaned: String = description
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect();
    // 控制显示长度，避免前缀挤占正文
    if cleaned.chars().count() <= 24 {
        cleaned
    } else {
        let truncated: String = cleaned.chars().take(24).collect();
        format!("{}…", truncated)
    }
}

/// 构建 SubAgent 专用的 system prompt
///
/// 从嵌入模板加载，将 `{{.base_prompt}}` 替换为父 agent 的 system prompt，
/// 使 SubAgent 继承基础能力同时拥有独立的身份和限制说明。
fn build_sub_agent_system_prompt(base_prompt: Option<&str>) -> String {
    let template = crate::template::sub_agent_system_prompt_template();
    let base = base_prompt.unwrap_or("You are a helpful assistant.");
    template.replace("{{.base_prompt}}", base)
}

/// SubAgentTool 参数
#[derive(Deserialize, JsonSchema)]
struct AgentParams {
    /// The task for the sub-agent to perform
    prompt: String,
    /// A short (3-5 word) description of the task
    #[serde(default)]
    description: Option<String>,
    /// Set to true to run in background. Returns task_id immediately.
    #[serde(default)]
    run_in_background: bool,
    /// If true, create an isolated git worktree for this sub-agent.
    /// Recommended when running multiple parallel agents that may edit overlapping files.
    /// The worktree is automatically cleaned up when the agent finishes.
    #[serde(default)]
    worktree: bool,
    /// If true, the sub-agent inherits all tool permissions (allow_all=true).
    /// Use this when you trust the agent to run tools without confirmation prompts.
    #[serde(default)]
    inherit_permissions: bool,
}

// ========== SubAgentTool ==========

/// SubAgent 工具：启动子代理执行复杂多步任务
#[allow(dead_code)]
pub struct SubAgentTool {
    pub shared: DerivedAgentShared,
}

impl SubAgentTool {
    pub const NAME: &'static str = "Agent";
}

impl Tool for SubAgentTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> Cow<'_, str> {
        r#"
        Launch a sub-agent to handle complex, multi-step tasks autonomously.
        The sub-agent runs with a fresh context (system prompt + your prompt as user message).
        It can use all tools except Agent (to prevent recursion).

        When NOT to use the Agent tool:
        - If you want to read a specific file path, use Read or Glob instead
        - If you are searching for a specific class/function definition, use Grep or Glob instead
        - If you are searching code within a specific file or 2-3 files, use Read instead

        Usage notes:
        - Always include a short description (3-5 words) summarizing what the agent will do
        - The result returned by the agent is not visible to the user. To show the user the result, send a text message with a concise summary
        - Use foreground (default) when you need the agent's results before proceeding
        - Use background when you have genuinely independent work to do in parallel
        - Clearly tell the agent whether you expect it to write code or just do research (search, file reads, web fetches, etc.)
        - Provide clear, detailed prompts so the agent can work autonomously — explain what you're trying to accomplish, what you've already learned, and give enough context for the agent to make judgment calls
        "#.into()
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<AgentParams>()
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: AgentParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let prompt = params.prompt;
        let description = params
            .description
            .unwrap_or_else(|| "sub-agent task".to_string());
        let run_in_background = params.run_in_background;
        let use_worktree = params.worktree;

        // 获取 provider 和构建 SubAgent 独立 system prompt
        let provider = safe_lock(&self.shared.provider, "SubAgentTool::provider").clone();
        let base_prompt =
            safe_lock(&self.shared.system_prompt, "SubAgentTool::system_prompt").clone();
        let system_prompt = build_sub_agent_system_prompt(base_prompt.as_deref());

        // worktree 隔离：提前创建（在调用线程中；失败则提前退出，避免浪费 sub_id）
        let worktree_info: Option<(std::path::PathBuf, String)> = if use_worktree {
            match create_agent_worktree(&description) {
                Ok(info) => Some(info),
                Err(e) => {
                    return ToolResult {
                        output: format!("创建 worktree 失败: {}", e),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
            }
        } else {
            None
        };

        // 提前分配 sub_id，用于构造独立 todos/transcript 路径
        let sub_id = self.shared.sub_agent_tracker.allocate_id();
        let session_id_snapshot =
            safe_lock(&self.shared.session_id, "SubAgentTool::session_id").clone();
        let session_paths = SessionPaths::new(&session_id_snapshot);
        let subagent_todos_path = session_paths.subagent_todos_file(&sub_id);
        let subagent_transcript_path = session_paths.subagent_transcript(&sub_id);

        // 构建子 registry（排除 "Agent" 工具防递归，独立 todos 文件）
        let (child_registry, _) = self.shared.build_child_registry(subagent_todos_path);
        let child_registry = Arc::new(child_registry);

        let mut disabled = self.shared.disabled_tools.as_ref().clone();
        disabled.push(Self::NAME.to_string());
        // 子 agent 继承父 agent 的 deferred 快照作为初始工具过滤，
        // 但不支持动态 LoadTool（子 agent 是一次性执行，每轮不重建工具列表）
        let deferred = match self.shared.deferred_tools.lock() {
            Ok(guard) => guard,
            Err(e) => e.into_inner(),
        }
        .clone();
        let tools = child_registry.to_llm_tools_non_deferred(&disabled, &deferred);

        // inherit_permissions：复制 JcliConfig 并启用 allow_all
        let jcli_config = if params.inherit_permissions {
            let mut cfg = self.shared.jcli_config.as_ref().clone();
            cfg.permissions.allow_all = true;
            Arc::new(cfg)
        } else {
            Arc::clone(&self.shared.jcli_config)
        };

        // 复用父 agent 的上下文配置快照
        let context_config = safe_lock(
            &self.shared.agent_context_config,
            "SubAgentTool::context_config",
        )
        .clone();

        if run_in_background {
            // 后台模式：先注册到 tracker 获得 handle，再 spawn_command
            self.shared.sub_agent_tracker.gc_finished();
            let handle = self.shared.sub_agent_tracker.register_with_id(
                sub_id.clone(),
                &description,
                "background",
            );

            // 注册到 background_manager，传入线程存活标记
            let (task_id, output_buffer) = self.shared.background_manager.spawn_command(
                &format!("Agent: {}", description),
                None,
                0,
                Some(Arc::clone(&handle.is_running)), // 线程存活标记
            );

            let snap_running = Arc::clone(&handle.is_running);
            let snapshot_refs = SubAgentLoopStateRefs::from_handle(&handle);

            let bg_manager = Arc::clone(&self.shared.background_manager);
            let task_id_clone = task_id.clone();
            let cancelled_clone = Arc::clone(cancelled);

            let description_clone = description.clone();
            let display_clone = Arc::clone(&self.shared.display_messages);
            let context_clone = Arc::clone(&self.shared.context_messages);
            let transcript_path = subagent_transcript_path.clone();
            let _sub_id_for_thread = sub_id.clone();
            let context_config_clone = context_config.clone();
            let agent_identity = format!("SubAgent@{}", sanitize_agent_name(&description));
            let sub_agent_metrics_clone = Arc::clone(&self.shared.sub_agent_metrics);
            std::thread::spawn(move || {
                // 设置线程的 agent 身份（含类型前缀，与广播 <SubAgent@Name> 格式一致）
                set_current_agent_name(&agent_identity);
                set_current_agent_type(AgentType::SubAgent);

                // 设置 worktree CWD
                if let Some((ref wt_path, _)) = worktree_info {
                    set_thread_cwd(wt_path);
                }

                let result = run_sub_agent_loop(
                    SubAgentLoopParams {
                        provider,
                        system_prompt: Some(system_prompt),
                        prompt,
                        tools,
                        registry: child_registry,
                        jcli_config,
                        snapshot: Some(snapshot_refs),
                        description: description_clone.clone(),
                        transcript_path: Some(transcript_path),
                        context_config: context_config_clone,
                        sub_agent_metrics: sub_agent_metrics_clone,
                    },
                    &cancelled_clone,
                    &display_clone,
                    &context_clone,
                );

                snap_running.store(false, Ordering::Relaxed);

                // 清理 worktree
                if let Some((ref wt_path, ref branch)) = worktree_info {
                    remove_agent_worktree(wt_path, branch);
                }

                // 写入输出缓冲区
                {
                    let mut buf = safe_lock(&output_buffer, "SubAgentTool::bg_output");
                    buf.push_str(&result);
                }

                bg_manager.complete_task(&task_id_clone, "completed", result);
            });

            ToolResult {
                output: json!({
                    "task_id": task_id,
                    "sub_id": sub_id,
                    "description": description,
                    "status": "running in background"
                })
                .to_string(),
                is_error: false,
                images: vec![],
                plan_decision: PlanDecision::None,
            }
        } else {
            // 前台模式：阻塞执行
            // 保存旧身份 + CWD，执行完后恢复（前台 agent 在调用线程中运行）
            let old_agent_name = current_agent_name();
            let old_cwd = thread_cwd();
            let agent_identity = format!("SubAgent@{}", sanitize_agent_name(&description));
            set_current_agent_name(&agent_identity);
            set_current_agent_type(AgentType::SubAgent);
            if let Some((ref wt_path, _)) = worktree_info {
                set_thread_cwd(wt_path);
            }

            // 注册到子 Agent tracker 供 /dump + UI dashboard 读取
            self.shared.sub_agent_tracker.gc_finished();
            let handle = self.shared.sub_agent_tracker.register_with_id(
                sub_id.clone(),
                &description,
                "foreground",
            );
            let snap_running = Arc::clone(&handle.is_running);
            let snapshot_refs = SubAgentLoopStateRefs::from_handle(&handle);

            let cancelled_clone = Arc::clone(cancelled);
            let result = run_sub_agent_loop(
                SubAgentLoopParams {
                    provider,
                    system_prompt: Some(system_prompt),
                    prompt,
                    tools,
                    registry: child_registry,
                    jcli_config,
                    snapshot: Some(snapshot_refs),
                    description,
                    transcript_path: Some(subagent_transcript_path),
                    context_config,
                    sub_agent_metrics: Arc::clone(&self.shared.sub_agent_metrics),
                },
                &cancelled_clone,
                &self.shared.display_messages,
                &self.shared.context_messages,
            );

            snap_running.store(false, Ordering::Relaxed);

            // 清理 worktree 并恢复身份 + CWD
            if let Some((ref wt_path, ref branch)) = worktree_info {
                remove_agent_worktree(wt_path, branch);
            }
            set_current_agent_name(&old_agent_name);
            match old_cwd {
                Some(p) => set_thread_cwd(&p),
                None => clear_thread_cwd(),
            }

            ToolResult {
                output: result,
                is_error: false,
                images: vec![],
                plan_decision: PlanDecision::None,
            }
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ========== SubAgent Loop ==========

/// 无 UI 的子代理循环：执行工具调用直到完成或达到限制
///
/// - 不发送 StreamMsg（无 UI 交互）
/// - 需要确认的工具通过 permission 检查：允许则执行，否则返回 "Tool denied"
/// - 返回最终的 assistant 文本
fn run_sub_agent_loop(
    params: SubAgentLoopParams,
    cancelled: &Arc<AtomicBool>,
    display_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    context_messages: &Arc<Mutex<Vec<ChatMessage>>>,
) -> String {
    let agent_name = sanitize_agent_name(&params.description);
    let sender_label = format!("SubAgent@{}", agent_name);
    // SubAgent 的中间消息通过双通道推送：
    // - display_messages：UI 显示（TUI 渲染）— 纯文本/结构体 + sender_name 字段
    // - context_messages：显式注入 Main Agent LLM context — XML 包裹
    //
    // Main Agent 能看到子代理的中间文本回复和工具调用名，以便感知工作进度。
    // 文本消息双通道分异（display 干净文本，context XML 包裹）
    let push_display_and_context =
        |display_content: String, context_content: String, sender: &str| {
            if let Ok(mut display) = display_messages.lock() {
                display.push(
                    ChatMessage::text(MessageRole::Assistant, &display_content).with_sender(sender),
                );
            }
            if let Ok(mut context) = context_messages.lock() {
                context.push(
                    ChatMessage::text(MessageRole::Assistant, &context_content).with_sender(sender),
                );
            }
        };
    // 工具调用推入 display only（结构体格式，渲染为工具卡片）
    let push_tool_call_to_display = |item: &ToolCallItem, sender: &str| {
        if let Ok(mut display) = display_messages.lock() {
            display.push(ChatMessage {
                role: MessageRole::Assistant,
                content: String::new(),
                tool_calls: Some(vec![item.clone()]),
                tool_call_id: None,
                images: None,
                reasoning_content: None,
                sender_name: Some(sender.to_string()),
                recipient_name: None,
                display_hint: DisplayHint::Normal,
            });
        }
    };
    // 工具结果推入 display only
    let push_tool_result_to_display =
        |result_content: String, tool_call_id: String, sender: &str| {
            if let Ok(mut display) = display_messages.lock() {
                display.push(ChatMessage {
                    role: MessageRole::Tool,
                    content: result_content,
                    tool_calls: None,
                    tool_call_id: Some(tool_call_id),
                    images: None,
                    reasoning_content: None,
                    sender_name: Some(sender.to_string()),
                    recipient_name: None,
                    display_hint: DisplayHint::Normal,
                });
            }
        };
    let max_rounds = 30; // 子代理最大轮数

    // 进入 Thinking 状态（即将调用 LLM）
    if let Some(ref refs) = params.snapshot {
        refs.set_status(SubAgentStatus::Thinking);
    }

    let (rt, client) = match create_runtime_and_client(&params.provider) {
        Ok(pair) => pair,
        Err(e) => {
            if let Some(ref refs) = params.snapshot {
                refs.set_status(SubAgentStatus::Error(e.clone()));
            }
            return e;
        }
    };

    // 写入 system prompt 快照（供 /dump 读取）
    if let Some(ref refs) = params.snapshot
        && let Ok(mut sp) = refs.system_prompt.lock()
    {
        *sp = params.system_prompt.clone().unwrap_or_default();
    }

    let mut messages: Vec<ChatMessage> = vec![ChatMessage {
        role: MessageRole::User,
        content: params.prompt,
        tool_calls: None,
        tool_call_id: None,
        images: None,
        reasoning_content: None,
        sender_name: None,
        recipient_name: None,
        display_hint: DisplayHint::Normal,
    }];

    let sync_messages = |msgs: &Vec<ChatMessage>| {
        if let Some(ref refs) = params.snapshot
            && let Ok(mut snap) = refs.messages.lock()
        {
            *snap = msgs.clone();
        }
    };

    // 独立 transcript append：每条新消息 append 一行 SessionEvent::Msg 到 jsonl 文件
    let transcript_path = params.transcript_path.clone();
    let append_to_transcript = |msgs: &[ChatMessage]| {
        if let Some(ref path) = transcript_path {
            for m in msgs {
                let _ = append_event_to_path(path, &SessionEvent::msg(m.clone()));
            }
        }
    };

    sync_messages(&messages);
    append_to_transcript(&messages);

    let mut final_text = String::new();

    for round in 0..max_rounds {
        if cancelled.load(Ordering::Relaxed) {
            if let Some(ref refs) = params.snapshot {
                refs.set_status(SubAgentStatus::Cancelled);
                refs.set_current_tool(None);
            }
            return format!("{}\n[Sub-agent cancelled]", final_text);
        }

        if let Some(ref refs) = params.snapshot {
            refs.current_round.store(round + 1, Ordering::Relaxed);
        }

        write_info_log("SubAgent", &format!("Round {}/{}", round + 1, max_rounds));

        // 进入 Thinking 状态（即将调用 LLM，等待模型回复）
        if let Some(ref refs) = params.snapshot {
            refs.set_status(SubAgentStatus::Thinking);
        }

        // 构建重试回调：更新 SubAgent 状态为 Retrying
        let status_for_retry = params.snapshot.as_ref().map(|r| Arc::clone(&r.status));
        let retry_callback = move |attempt: u32, max_attempts: u32, delay_ms: u64, error: &str| {
            if let Some(ref status_arc) = status_for_retry
                && let Ok(mut s) = status_arc.lock()
            {
                *s = SubAgentStatus::Retrying {
                    attempt,
                    max_attempts,
                    delay_ms,
                    error: error.to_string(),
                };
            }
        };

        // 上下文裁剪：复用父 agent 的 select_messages + micro_compact
        // SubAgent 是隔离 loop，messages 里没有其他 agent 的广播，无需 compress_other_agent_toolcalls。
        let mut api_messages = crate::context::window::select_messages(
            &messages,
            params.context_config.max_history_messages,
            params.context_config.max_context_tokens,
            params.context_config.compact.keep_recent,
            &params.context_config.compact.micro_compact_exempt_tools,
        );
        if params.context_config.compact.enabled {
            crate::context::compact::micro_compact(
                &mut api_messages,
                params.context_config.compact.keep_recent,
                &params.context_config.compact.micro_compact_exempt_tools,
            );
        }

        let response = match call_llm_non_stream(&LlmNonStreamRequest {
            rt: &rt,
            client: &client,
            provider: &params.provider,
            messages: &api_messages,
            tools: &params.tools,
            system_prompt: params.system_prompt.as_deref(),
            on_retry: Some(&retry_callback),
        }) {
            Ok(r) => {
                // LLM 调用成功，保持 Thinking（等待工具执行时切换为 Working）
                r
            }
            Err(e) => {
                if let Some(ref refs) = params.snapshot {
                    refs.set_status(SubAgentStatus::Error(e.clone()));
                    refs.set_current_tool(None);
                }
                return format!("{}\n{}", final_text, e);
            }
        };

        // 提取第一个 choice（非流式调用保证有且仅有一个）
        let choice = response
            .choices
            .into_iter()
            .next()
            .expect("call_llm_non_stream validates non-empty choices");

        // 累加 LLM metrics（usage 可能为空，某些 provider 不返回）
        if let Some(usage) = response.usage
            && let Ok(mut m) = params.sub_agent_metrics.lock()
        {
            m.total_llm_calls += 1;
            m.total_input_tokens += usage.prompt_tokens;
            m.total_output_tokens += usage.completion_tokens;
        }

        let assistant_text = choice.message.content.clone().unwrap_or_default();
        let reasoning_content = choice.message.reasoning_content.clone();
        if !assistant_text.is_empty() {
            write_info_log("SubAgent", &format!("Reply: {}", &assistant_text));
            // UI 状态行：显示 sub-agent 的文字回复
            // ★ 此消息通过双通道推送（display + context），会同步到 Main Agent 的 LLM 上下文（有意为之的设计）。
            // display: 纯文本 + sender_name | context: XML 包裹
            push_display_and_context(
                assistant_text.clone(),
                format!("<{}>{}</{}>", sender_label, &assistant_text, sender_label),
                &sender_label,
            );
        }

        // 检查是否有工具调用
        let is_tool_calls = choice.finish_reason.as_deref() == Some("tool_calls");

        if !is_tool_calls || choice.message.tool_calls.is_none() {
            // 纯文本回复结束：用当前轮次的文本作为最终返回（而非之前的中间文本）
            final_text = assistant_text.clone();
            if !assistant_text.is_empty() {
                let final_msg = ChatMessage::text(MessageRole::Assistant, assistant_text.clone());
                messages.push(final_msg);
                if let Some(last) = messages.last() {
                    append_to_transcript(std::slice::from_ref(last));
                }
                sync_messages(&messages);
            }
            break;
        }

        // 上面已检查 tool_calls.is_none() 会 break，此处用 let else 确保安全
        let Some(tool_calls) = choice.message.tool_calls.as_ref() else {
            break;
        };
        let tool_items = extract_tool_items(tool_calls);
        if tool_items.is_empty() {
            break;
        }

        // UI 状态行：显示 sub-agent 的工具调用名（不含参数/结果）
        // ★ context 通道：XML 包裹文本（Main Agent LLM context）
        // ★ display 通道：tool_calls 结构体（渲染为工具卡片）
        for item in &tool_items {
            // context：文本格式（XML 包裹）
            if let Ok(mut context) = context_messages.lock() {
                context.push(
                    ChatMessage::text(
                        MessageRole::Assistant,
                        format!(
                            "<{}>[调用工具 {}]</{}>",
                            sender_label, item.name, sender_label
                        ),
                    )
                    .with_sender(&sender_label),
                );
            }
            // display：结构体格式
            push_tool_call_to_display(item, &sender_label);
        }

        // 将 assistant 消息（含 tool_calls）加入历史
        let assistant_msg = ChatMessage {
            role: MessageRole::Assistant,
            content: assistant_text,
            tool_calls: Some(tool_items.clone()),
            tool_call_id: None,
            images: None,
            reasoning_content,
            sender_name: None,
            recipient_name: None,
            display_hint: DisplayHint::Normal,
        };
        messages.push(assistant_msg);
        if let Some(last) = messages.last() {
            append_to_transcript(std::slice::from_ref(last));
        }

        // 逐个执行工具
        for item in &tool_items {
            if let Some(ref refs) = params.snapshot {
                refs.set_current_tool(Some(item.name.clone()));
                refs.set_status(SubAgentStatus::Working);
                refs.tool_calls_count.fetch_add(1, Ordering::Relaxed);
            }
            let result_msg = execute_tool_with_permission(
                item,
                &ToolExecContext {
                    registry: &params.registry,
                    jcli_config: &params.jcli_config,
                    cancelled,
                    log_tag: "SubAgent",
                    verbose: true,
                },
            );

            // 累加工具调用 metrics
            if let Ok(mut m) = params.sub_agent_metrics.lock() {
                m.total_tool_calls += 1;
            }

            // 工具结果推入 display only（完整内容，渲染为工具结果卡片）
            push_tool_result_to_display(
                result_msg.content.clone(),
                result_msg.tool_call_id.clone().unwrap_or_default(),
                &sender_label,
            );
            messages.push(result_msg);
            if let Some(last) = messages.last() {
                append_to_transcript(std::slice::from_ref(last));
            }
        }
        if let Some(ref refs) = params.snapshot {
            refs.set_current_tool(None);
            // 工具全部执行完毕，切回 Thinking（下一轮 LLM 调用前）
            refs.set_status(SubAgentStatus::Thinking);
        }

        // 本轮工具结果写入后同步快照
        sync_messages(&messages);
    }

    // UI 状态行：sub-agent 结束
    // ★ 此消息通过双通道推送（display + context），会同步到 Main Agent 的 LLM 上下文。
    push_display_and_context(
        "[已完成]".to_string(),
        format!("<{}>[已完成]</{}>", sender_label, sender_label),
        &sender_label,
    );

    if let Some(ref refs) = params.snapshot {
        refs.set_status(SubAgentStatus::Completed);
        refs.set_current_tool(None);
    }

    if final_text.is_empty() {
        "[Sub-agent completed with no text output]".to_string()
    } else {
        final_text
    }
}
