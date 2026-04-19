use crate::command::chat::agent::thread_identity::{
    set_current_agent_name, set_current_agent_type, set_thread_cwd,
};
use crate::command::chat::permission_queue::AgentType;
use crate::command::chat::teammate::{TeammateHandle, TeammateManager, TeammateStatus};
use crate::command::chat::teammate_loop::{TeammateLoopConfig, run_teammate_loop};
use crate::command::chat::tools::derived_shared::DerivedAgentShared;
use crate::command::chat::tools::send_message::SendMessageTool;
use crate::command::chat::tools::work_done::WorkDoneTool;
use crate::command::chat::tools::worktree::{create_agent_worktree, remove_agent_worktree};
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params,
};
use crate::util::log::write_info_log;
use crate::util::safe_lock;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio_util::sync::CancellationToken;

/// CreateTeammate 参数
#[derive(Deserialize, JsonSchema)]
struct CreateTeammateParams {
    /// Teammate name (e.g. "Frontend", "Backend", "DevOps")
    name: String,
    /// Role description (e.g. "React frontend developer")
    role: String,
    /// Initial task prompt for this teammate
    prompt: String,
    /// If true, create an isolated git worktree for this teammate.
    /// Strongly recommended when multiple teammates may edit overlapping files.
    /// Each teammate gets its own branch (worktree-agent-{name}) under .jcli/worktrees/.
    /// The worktree is automatically cleaned up when the teammate finishes.
    #[serde(default)]
    worktree: bool,
    /// If true, the teammate inherits all tool permissions (allow_all=true).
    /// Use this when you trust the teammate to run tools without confirmation prompts.
    #[serde(default)]
    inherit_permissions: bool,
}

/// CreateTeammate 工具：创建一个新的 teammate agent
#[allow(dead_code)]
pub struct CreateTeammateTool {
    pub shared: DerivedAgentShared,
    pub teammate_manager: Arc<Mutex<TeammateManager>>,
}

impl CreateTeammateTool {
    pub const NAME: &'static str = "CreateTeammate";
}

impl Tool for CreateTeammateTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Create a new teammate agent that runs independently in the chatroom.
        Each teammate has its own LLM connection and conversation context.
        Teammates communicate via SendMessage tool (broadcast with @mentions).

        Usage:
        - name: Short name for the teammate (e.g. "Frontend", "Backend")
        - role: Role description (shown in team summary)
        - prompt: Initial task/instructions for this teammate

        The teammate starts working immediately on the given prompt.
        It can use all tools except CreateTeammate (no recursive spawning).
        Teammates are session-scoped and cleaned up when the session ends.

        Example:
        {
          "name": "Frontend",
          "role": "React TypeScript developer",
          "prompt": "Create a React Todo app with components in src/components/..."
        }
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<CreateTeammateParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: CreateTeammateParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        if params.name.trim().is_empty() {
            return ToolResult {
                output: "Teammate name cannot be empty".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        // 检查是否已存在同名 teammate
        {
            let manager = match self.teammate_manager.lock() {
                Ok(m) => m,
                Err(_) => {
                    return ToolResult {
                        output: "Failed to acquire teammate manager lock".to_string(),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
            };
            if manager.teammates.contains_key(&params.name) {
                return ToolResult {
                    output: format!("Teammate '{}' already exists", params.name),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        }

        // 若请求 worktree 隔离，提前创建（在主线程中；失败则提前退出）
        let worktree_info: Option<(std::path::PathBuf, String)> = if params.worktree {
            match create_agent_worktree(&params.name) {
                Ok(info) => {
                    write_info_log(
                        "CreateTeammate",
                        &format!(
                            "Teammate '{}' worktree created: {}",
                            params.name,
                            info.0.display()
                        ),
                    );
                    Some(info)
                }
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

        // 准备 teammate 的资源
        let pending_user_messages = Arc::new(Mutex::new(Vec::new()));
        let streaming_content = Arc::new(Mutex::new(String::new()));
        let cancel_token = CancellationToken::new();
        let is_running = Arc::new(AtomicBool::new(true));
        let system_prompt_snapshot = Arc::new(Mutex::new(String::new()));
        let messages_snapshot = Arc::new(Mutex::new(Vec::new()));
        let status = Arc::new(Mutex::new(TeammateStatus::Initializing));
        let tool_calls_count = Arc::new(AtomicUsize::new(0));
        let current_tool: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let wake_flag = Arc::new(AtomicBool::new(false));
        let work_done = Arc::new(AtomicBool::new(false));

        // 获取 provider 快照
        let provider = safe_lock(&self.shared.provider, "CreateTeammate::provider").clone();
        let system_prompt =
            safe_lock(&self.shared.system_prompt, "CreateTeammate::system_prompt").clone();

        // 构建子工具注册表（teammate 独立 todos：sessions/<sid>/teammates/<name>/todos.json）
        let teammate_todos_path = {
            let sid = safe_lock(&self.shared.session_id, "CreateTeammate::session_id").clone();
            let sanitized = crate::command::chat::storage::sanitize_filename(&params.name);
            crate::command::chat::storage::SessionPaths::new(&sid).teammate_todos_file(&sanitized)
        };
        let (mut child_registry, _) = self.shared.build_child_registry(teammate_todos_path);

        // 注册 SendMessage 工具到子注册表
        child_registry.register(Box::new(SendMessageTool {
            teammate_manager: Arc::clone(&self.teammate_manager),
        }));
        // 注册 WorkDone 工具（与本 teammate 的 work_done 标志绑定）
        child_registry.register(Box::new(WorkDoneTool {
            work_done: Arc::clone(&work_done),
            teammate_manager: Arc::clone(&self.teammate_manager),
        }));
        let child_registry = Arc::new(child_registry);

        let mut disabled = self.shared.disabled_tools.as_ref().clone();
        disabled.push("CreateTeammate".to_string());
        disabled.push("AgentTeam".to_string());
        disabled.push("Agent".to_string());
        let tools = child_registry.to_openai_tools_filtered(&disabled);

        // inherit_permissions：复制 JcliConfig 并启用 allow_all
        let jcli_config = if params.inherit_permissions {
            let mut cfg = self.shared.jcli_config.as_ref().clone();
            cfg.permissions.allow_all = true;
            Arc::new(cfg)
        } else {
            Arc::clone(&self.shared.jcli_config)
        };
        let teammate_manager = Arc::clone(&self.teammate_manager);

        // 构建 teammate 专用 system prompt
        let teammate_name = params.name.clone();
        let teammate_role = params.role.clone();
        let initial_prompt = params.prompt.clone();

        // Clone 用于线程
        let pending_clone = Arc::clone(&pending_user_messages);
        let is_running_clone = Arc::clone(&is_running);
        let cancel_token_clone = cancel_token.clone();
        let sp_snapshot_clone = Arc::clone(&system_prompt_snapshot);
        let msgs_snapshot_clone = Arc::clone(&messages_snapshot);
        let status_clone = Arc::clone(&status);
        let tool_calls_count_clone = Arc::clone(&tool_calls_count);
        let current_tool_clone = Arc::clone(&current_tool);
        let wake_flag_clone = Arc::clone(&wake_flag);
        let work_done_clone = Arc::clone(&work_done);
        let session_id_clone = Arc::clone(&self.shared.session_id);

        let thread_handle = std::thread::spawn(move || {
            // 设置线程的 agent 身份
            set_current_agent_name(&teammate_name);
            set_current_agent_type(AgentType::Teammate);

            // 设置 worktree CWD（若有）
            if let Some((ref wt_path, _)) = worktree_info {
                set_thread_cwd(wt_path);
                write_info_log(
                    "CreateTeammate",
                    &format!(
                        "Teammate '{}' working in worktree: {}",
                        teammate_name,
                        wt_path.display()
                    ),
                );
            }

            write_info_log(
                "CreateTeammate",
                &format!("Teammate '{}' agent loop starting", teammate_name),
            );

            let result = run_teammate_loop(TeammateLoopConfig {
                name: teammate_name.clone(),
                role: teammate_role,
                initial_prompt,
                provider,
                base_system_prompt: system_prompt,
                session_id: session_id_clone,
                tools,
                registry: child_registry,
                jcli_config,
                teammate_manager,
                pending_user_messages: pending_clone,
                cancel_token: cancel_token_clone,
                system_prompt_snapshot: sp_snapshot_clone,
                messages_snapshot: msgs_snapshot_clone,
                status: status_clone,
                tool_calls_count: tool_calls_count_clone,
                current_tool: current_tool_clone,
                wake_flag: wake_flag_clone,
                work_done: work_done_clone,
            });

            // 清理 worktree
            if let Some((ref wt_path, ref branch)) = worktree_info {
                write_info_log(
                    "CreateTeammate",
                    &format!("Teammate '{}' cleaning up worktree", teammate_name),
                );
                remove_agent_worktree(wt_path, branch);
            }

            is_running_clone.store(false, Ordering::Relaxed);

            write_info_log(
                "CreateTeammate",
                &format!(
                    "Teammate '{}' agent loop ended: {}",
                    teammate_name,
                    &result[..{
                        let mut b = result.len().min(200);
                        while b > 0 && !result.is_char_boundary(b) {
                            b -= 1;
                        }
                        b
                    }]
                ),
            );
        });

        // 注册 teammate
        let handle = TeammateHandle {
            name: params.name.clone(),
            role: params.role.clone(),
            pending_user_messages,
            streaming_content,
            cancel_token,
            is_running,
            thread_handle: Some(thread_handle),
            system_prompt_snapshot,
            messages_snapshot,
            status,
            tool_calls_count,
            current_tool,
            wake_flag,
            work_done,
        };

        match self.teammate_manager.lock() {
            Ok(mut manager) => manager.register_teammate(handle),
            Err(_) => {
                return ToolResult {
                    output: "Failed to register teammate".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        }

        let worktree_note = if params.worktree {
            " [worktree 隔离]"
        } else {
            ""
        };
        ToolResult {
            output: format!(
                "Teammate '{}' ({}){} created and started working on: {}",
                params.name,
                params.role,
                worktree_note,
                &params.prompt[..{
                    let mut b = params.prompt.len().min(100);
                    while b > 0 && !params.prompt.is_char_boundary(b) {
                        b -= 1;
                    }
                    b
                }]
            ),
            is_error: false,
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
