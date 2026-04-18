use crate::command::chat::permission::JcliConfig;
use crate::command::chat::storage::{ChatMessage, ModelProvider};
use crate::command::chat::teammate::{clear_thread_cwd, set_thread_cwd, thread_cwd};
use crate::command::chat::tools::agent_shared::{
    AgentToolShared, call_llm_non_stream, create_runtime_and_client, execute_tool_with_permission,
    extract_tool_items,
};
use crate::command::chat::tools::worktree::{create_agent_worktree, remove_agent_worktree};
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolRegistry, ToolResult, parse_tool_args, schema_to_tool_params,
};
use crate::util::log::write_info_log;
use crate::util::safe_lock;
use async_openai::types::chat::ChatCompletionTools;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

/// 子 Agent 运行时快照（系统提示 + 消息列表），供 /dump 读取。
type SubAgentSnapshotRefs = (Arc<Mutex<String>>, Arc<Mutex<Vec<ChatMessage>>>);

/// 无 UI 子代理循环的参数集合
struct HeadlessAgentParams {
    provider: ModelProvider,
    system_prompt: Option<String>,
    prompt: String,
    tools: Vec<ChatCompletionTools>,
    registry: Arc<ToolRegistry>,
    jcli_config: Arc<JcliConfig>,
    snapshot: Option<SubAgentSnapshotRefs>,
    description: String,
}

/// AgentTool 参数
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

// ========== AgentTool ==========

/// Agent 工具：启动子代理执行复杂多步任务
#[allow(dead_code)]
pub struct AgentTool {
    pub shared: AgentToolShared,
}

impl AgentTool {
    pub const NAME: &'static str = "Agent";
}

impl Tool for AgentTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
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
        "#
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

        // 获取 provider 和 system prompt 的快照
        let provider = safe_lock(&self.shared.provider, "AgentTool::provider").clone();
        let system_prompt =
            safe_lock(&self.shared.system_prompt, "AgentTool::system_prompt").clone();

        // 构建子 registry（排除 "Agent" 工具防递归）
        let (sub_registry, _) = self.shared.build_sub_registry();
        let sub_registry = Arc::new(sub_registry);

        let mut disabled = self.shared.disabled_tools.as_ref().clone();
        disabled.push("Agent".to_string());
        let tools = sub_registry.to_openai_tools_filtered(&disabled);

        // inherit_permissions：复制 JcliConfig 并启用 allow_all
        let jcli_config = if params.inherit_permissions {
            let mut cfg = self.shared.jcli_config.as_ref().clone();
            cfg.permissions.allow_all = true;
            Arc::new(cfg)
        } else {
            Arc::clone(&self.shared.jcli_config)
        };

        // worktree 隔离：提前创建（在调用线程中；失败则提前退出）
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

        if run_in_background {
            // 后台模式：注册任务并 spawn 线程
            let (task_id, output_buffer) = self.shared.background_manager.spawn_command(
                &format!("Agent: {}", description),
                None,
                0,
            );

            // 注册到子 Agent tracker 供 /dump 读取
            self.shared.sub_agent_tracker.gc_finished();
            let (_snap_id, snap_running, snap_system_prompt, snap_messages) = self
                .shared
                .sub_agent_tracker
                .register(&description, "background");

            let bg_manager = Arc::clone(&self.shared.background_manager);
            let task_id_clone = task_id.clone();
            let cancelled_clone = Arc::clone(cancelled);

            let description_clone = description.clone();
            let shared_messages_clone = Arc::clone(&self.shared.shared_messages);
            std::thread::spawn(move || {
                // 设置 worktree CWD
                if let Some((ref wt_path, _)) = worktree_info {
                    set_thread_cwd(wt_path);
                }

                let result = run_headless_agent_loop(
                    HeadlessAgentParams {
                        provider,
                        system_prompt,
                        prompt,
                        tools,
                        registry: sub_registry,
                        jcli_config,
                        snapshot: Some((
                            Arc::clone(&snap_system_prompt),
                            Arc::clone(&snap_messages),
                        )),
                        description: description_clone.clone(),
                    },
                    &cancelled_clone,
                    &shared_messages_clone,
                );

                snap_running.store(false, Ordering::Relaxed);

                // 清理 worktree
                if let Some((ref wt_path, ref branch)) = worktree_info {
                    remove_agent_worktree(wt_path, branch);
                }

                // 写入输出缓冲区
                {
                    let mut buf = safe_lock(&output_buffer, "AgentTool::bg_output");
                    buf.push_str(&result);
                }

                bg_manager.complete_task(&task_id_clone, "completed", result);
            });

            ToolResult {
                output: json!({
                    "task_id": task_id,
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
            // 保存旧 CWD，执行完后恢复（前台 agent 在调用线程中运行）
            let old_cwd = thread_cwd();
            if let Some((ref wt_path, _)) = worktree_info {
                set_thread_cwd(wt_path);
            }

            // 注册到子 Agent tracker 供 /dump 读取
            self.shared.sub_agent_tracker.gc_finished();
            let (_snap_id, snap_running, snap_system_prompt, snap_messages) = self
                .shared
                .sub_agent_tracker
                .register(&description, "foreground");

            let cancelled_clone = Arc::clone(cancelled);
            let result = run_headless_agent_loop(
                HeadlessAgentParams {
                    provider,
                    system_prompt,
                    prompt,
                    tools,
                    registry: sub_registry,
                    jcli_config,
                    snapshot: Some((Arc::clone(&snap_system_prompt), Arc::clone(&snap_messages))),
                    description,
                },
                &cancelled_clone,
                &self.shared.shared_messages,
            );

            snap_running.store(false, Ordering::Relaxed);

            // 清理 worktree 并恢复 CWD
            if let Some((ref wt_path, ref branch)) = worktree_info {
                remove_agent_worktree(wt_path, branch);
            }
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

// ========== Headless Agent Loop ==========

/// 无 UI 的子代理循环：执行工具调用直到完成或达到限制
///
/// - 不发送 StreamMsg（无 UI 交互）
/// - 需要确认的工具通过 permission 检查：允许则执行，否则返回 "Tool denied"
/// - 返回最终的 assistant 文本
fn run_headless_agent_loop(
    params: HeadlessAgentParams,
    cancelled: &Arc<AtomicBool>,
    shared_messages: &Arc<Mutex<Vec<ChatMessage>>>,
) -> String {
    let agent_tag = format!("Agent: {}", params.description);
    let push_ui = |msg: ChatMessage| {
        if let Ok(mut shared) = shared_messages.lock() {
            shared.push(msg);
        }
    };
    let max_rounds = 30; // 子代理最大轮数

    let (rt, client) = match create_runtime_and_client(&params.provider) {
        Ok(pair) => pair,
        Err(e) => return e,
    };

    // 写入 system prompt 快照（供 /dump 读取）
    if let Some((ref sp_snap, _)) = params.snapshot
        && let Ok(mut sp) = sp_snap.lock()
    {
        *sp = params.system_prompt.clone().unwrap_or_default();
    }

    let mut messages: Vec<ChatMessage> = vec![ChatMessage {
        role: "user".to_string(),
        content: params.prompt,
        tool_calls: None,
        tool_call_id: None,
        images: None,
    }];

    let sync_messages = |msgs: &Vec<ChatMessage>| {
        if let Some((_, ref msgs_snap)) = params.snapshot
            && let Ok(mut snap) = msgs_snap.lock()
        {
            *snap = msgs.clone();
        }
    };
    sync_messages(&messages);

    let mut final_text = String::new();

    for round in 0..max_rounds {
        if cancelled.load(Ordering::Relaxed) {
            return format!("{}\n[Sub-agent cancelled]", final_text);
        }

        write_info_log("SubAgent", &format!("Round {}/{}", round + 1, max_rounds));

        let choice = match call_llm_non_stream(
            &rt,
            &client,
            &params.provider,
            &messages,
            &params.tools,
            params.system_prompt.as_deref(),
        ) {
            Ok(c) => c,
            Err(e) => return format!("{}\n{}", final_text, e),
        };

        let assistant_text = choice.message.content.clone().unwrap_or_default();
        if !assistant_text.is_empty() {
            final_text = assistant_text.clone();
            write_info_log("SubAgent", &format!("Reply: {}", &final_text));
            // UI 状态行：显示 sub-agent 的文字回复
            push_ui(ChatMessage::text(
                "assistant",
                format!("<{}> {}", agent_tag, &assistant_text),
            ));
        }

        // 检查是否有工具调用
        let is_tool_calls = matches!(
            choice.finish_reason,
            Some(async_openai::types::chat::FinishReason::ToolCalls)
        );

        if !is_tool_calls || choice.message.tool_calls.is_none() {
            break;
        }

        // 上面已检查 tool_calls.is_none() 会 break，此处逻辑必定为 Some
        let tool_calls = choice.message.tool_calls.as_ref().unwrap();
        let tool_items = extract_tool_items(tool_calls);
        if tool_items.is_empty() {
            break;
        }

        // UI 状态行：显示 sub-agent 的工具调用名（不含参数/结果）
        for item in &tool_items {
            push_ui(ChatMessage::text(
                "assistant",
                format!("<{}> [调用工具 {}]", agent_tag, item.name),
            ));
        }

        // 将 assistant 消息（含 tool_calls）加入历史
        messages.push(ChatMessage {
            role: "assistant".to_string(),
            content: assistant_text,
            tool_calls: Some(tool_items.clone()),
            tool_call_id: None,
            images: None,
        });

        // 逐个执行工具
        for item in &tool_items {
            let result_msg = execute_tool_with_permission(
                item,
                &params.registry,
                &params.jcli_config,
                cancelled,
                "SubAgent",
                true,
            );
            messages.push(result_msg);
        }

        // 本轮工具结果写入后同步快照
        sync_messages(&messages);
    }

    // UI 状态行：sub-agent 结束
    push_ui(ChatMessage::text(
        "assistant",
        format!("<{}> [已完成]", agent_tag),
    ));

    if final_text.is_empty() {
        "[Sub-agent completed with no text output]".to_string()
    } else {
        final_text
    }
}
