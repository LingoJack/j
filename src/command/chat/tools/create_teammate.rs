use crate::command::chat::compact::CompactConfig;
use crate::command::chat::hook::HookManager;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::storage::ModelProvider;
use crate::command::chat::teammate::{TeammateHandle, TeammateManager, set_current_agent_name};
use crate::command::chat::tools::background::BackgroundManager;
use crate::command::chat::tools::task::TaskManager;
use crate::command::chat::tools::{
    Tool, ToolRegistry, ToolResult, parse_tool_args, schema_to_tool_params,
};
use crate::util::log::write_info_log;
use crate::util::safe_lock;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
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
}

/// CreateTeammate 工具：创建一个新的 teammate agent
#[allow(dead_code)]
pub struct CreateTeammateTool {
    pub teammate_manager: Arc<Mutex<TeammateManager>>,
    pub background_manager: Arc<BackgroundManager>,
    pub provider: Arc<Mutex<ModelProvider>>,
    pub system_prompt: Arc<Mutex<Option<String>>>,
    pub jcli_config: Arc<JcliConfig>,
    pub compact_config: CompactConfig,
    pub hook_manager: Arc<Mutex<HookManager>>,
    pub task_manager: Arc<TaskManager>,
    pub disabled_tools: Arc<Vec<String>>,
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
                    };
                }
            };
            if manager.teammates.contains_key(&params.name) {
                return ToolResult {
                    output: format!("Teammate '{}' already exists", params.name),
                    is_error: true,
                    images: vec![],
                };
            }
        }

        // 准备 teammate 的资源
        let pending_user_messages = Arc::new(Mutex::new(Vec::new()));
        let streaming_content = Arc::new(Mutex::new(String::new()));
        let cancel_token = CancellationToken::new();
        let is_running = Arc::new(AtomicBool::new(true));

        // 获取 provider 快照
        let provider = safe_lock(&self.provider, "CreateTeammate::provider").clone();
        let system_prompt = safe_lock(&self.system_prompt, "CreateTeammate::system_prompt").clone();

        // 构建子工具注册表（排除 CreateTeammate 和 AgentTeam 防递归）
        let (ask_tx, _ask_rx) = mpsc::channel::<crate::command::chat::app::AskRequest>();
        let sub_registry = ToolRegistry::new(
            vec![],
            ask_tx,
            Arc::clone(&self.background_manager),
            Arc::clone(&self.task_manager),
            Arc::clone(&self.hook_manager),
        );

        // 注册 SendMessage 工具到子注册表
        let mut sub_registry = sub_registry;
        sub_registry.register(Box::new(
            crate::command::chat::tools::send_message::SendMessageTool {
                teammate_manager: Arc::clone(&self.teammate_manager),
            },
        ));
        let sub_registry = Arc::new(sub_registry);

        let mut disabled = self.disabled_tools.as_ref().clone();
        disabled.push("CreateTeammate".to_string());
        disabled.push("AgentTeam".to_string());
        disabled.push("Agent".to_string());
        let tools = sub_registry.to_openai_tools_filtered(&disabled);

        let jcli_config = Arc::clone(&self.jcli_config);
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

            is_running_clone.store(false, Ordering::Relaxed);

            write_info_log(
                "CreateTeammate",
                &format!(
                    "Teammate '{}' agent loop ended: {}",
                    teammate_name,
                    &result[..result.len().min(200)]
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
                };
            }
        }

        ToolResult {
            output: format!(
                "Teammate '{}' ({}) created and started working on: {}",
                params.name,
                params.role,
                &params.prompt[..params.prompt.len().min(100)]
            ),
            is_error: false,
            images: vec![],
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
