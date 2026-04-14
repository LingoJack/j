use crate::command::chat::teammate::{TeammateHandle, TeammateManager, set_current_agent_name};
use crate::command::chat::tools::agent_shared::AgentToolShared;
use crate::command::chat::tools::send_message::SendMessageTool;
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
    atomic::{AtomicBool, Ordering},
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
    pub shared: AgentToolShared,
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
            match crate::command::chat::tools::worktree::create_agent_worktree(&params.name) {
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

        // 获取 provider 快照
        let provider = safe_lock(&self.shared.provider, "CreateTeammate::provider").clone();
        let system_prompt =
            safe_lock(&self.shared.system_prompt, "CreateTeammate::system_prompt").clone();

        // 构建子工具注册表
        let (mut sub_registry, _) = self.shared.build_sub_registry();

        // 注册 SendMessage 工具到子注册表
        sub_registry.register(Box::new(SendMessageTool {
            teammate_manager: Arc::clone(&self.teammate_manager),
        }));
        let sub_registry = Arc::new(sub_registry);

        let mut disabled = self.shared.disabled_tools.as_ref().clone();
        disabled.push("CreateTeammate".to_string());
        disabled.push("AgentTeam".to_string());
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
        let teammate_manager = Arc::clone(&self.teammate_manager);

        // 构建 teammate 专用 system prompt
        let teammate_name = params.name.clone();
        let teammate_role = params.role.clone();
        let initial_prompt = params.prompt.clone();

        // Clone 用于线程
        let pending_clone = Arc::clone(&pending_user_messages);
        let is_running_clone = Arc::clone(&is_running);
        let cancel_token_clone = cancel_token.clone();

        let thread_handle = std::thread::spawn(move || {
            // 设置线程的 agent 身份
            set_current_agent_name(&teammate_name);

            // 设置 worktree CWD（若有）
            if let Some((ref wt_path, _)) = worktree_info {
                crate::command::chat::teammate::set_thread_cwd(wt_path);
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

            let result = crate::command::chat::teammate_loop::run_teammate_loop(
                crate::command::chat::teammate_loop::TeammateLoopConfig {
                    name: teammate_name.clone(),
                    role: teammate_role,
                    initial_prompt,
                    provider,
                    base_system_prompt: system_prompt,
                    tools,
                    registry: sub_registry,
                    jcli_config,
                    teammate_manager,
                    pending_user_messages: pending_clone,
                    cancel_token: cancel_token_clone,
                },
            );

            // 清理 worktree
            if let Some((ref wt_path, ref branch)) = worktree_info {
                write_info_log(
                    "CreateTeammate",
                    &format!("Teammate '{}' cleaning up worktree", teammate_name),
                );
                crate::command::chat::tools::worktree::remove_agent_worktree(wt_path, branch);
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
