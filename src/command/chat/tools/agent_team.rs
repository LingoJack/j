use crate::command::chat::compact::CompactConfig;
use crate::command::chat::hook::HookManager;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::storage::ModelProvider;
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
use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread;

/// Agent team 参数
#[derive(Deserialize, JsonSchema)]
struct AgentTeamParams {
    /// Array of prompts for each team member (each agent runs concurrently)
    prompts: Vec<AgentTeamMember>,
    /// Optional team coordinator prompt to aggregate results
    #[serde(default)]
    coordinator_prompt: Option<String>,
    /// Optional timeout in seconds for the entire team (default: 300)
    #[serde(default)]
    timeout_secs: Option<u64>,
}

/// Team member definition
#[derive(Deserialize, JsonSchema)]
struct AgentTeamMember {
    /// Short name/role for this team member
    name: String,
    /// Task prompt for this team member
    prompt: String,
}

// ========== AgentTeamState ==========

/// Shared state for team coordination
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TeamMemberResult {
    pub name: String,
    pub status: String, // "running", "completed", "failed", "timeout"
    pub output: String,
}

pub struct AgentTeamState {
    members: Mutex<BTreeMap<String, TeamMemberResult>>,
}

impl AgentTeamState {
    pub fn new() -> Self {
        Self {
            members: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn set_result(&self, name: String, status: String, output: String) {
        if let Ok(mut map) = self.members.lock() {
            map.insert(
                name.clone(),
                TeamMemberResult {
                    name,
                    status,
                    output,
                },
            );
        }
    }

    pub fn get_all_results(&self) -> BTreeMap<String, TeamMemberResult> {
        self.members
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}

impl Default for AgentTeamState {
    fn default() -> Self {
        Self::new()
    }
}

// ========== AgentTeamTool ==========

/// Agent Team 工具：协调多个子代理并行执行任务
#[allow(dead_code)]
pub struct AgentTeamTool {
    pub background_manager: Arc<BackgroundManager>,
    pub provider: Arc<Mutex<ModelProvider>>,
    pub system_prompt: Arc<Mutex<Option<String>>>,
    pub jcli_config: Arc<JcliConfig>,
    pub compact_config: CompactConfig,
    pub hook_manager: Arc<Mutex<HookManager>>,
    pub task_manager: Arc<TaskManager>,
    pub todo_manager: Arc<crate::command::chat::tools::todo::TodoManager>,
    pub disabled_tools: Arc<Vec<String>>,
}

impl AgentTeamTool {
    pub const NAME: &'static str = "AgentTeam";
}

impl Tool for AgentTeamTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Coordinate a team of sub-agents to work on multiple tasks in parallel.
        Each team member is a sub-agent with its own prompt and runs concurrently.
        Optionally, a coordinator agent can aggregate and synthesize the results.

        Usage:
        - prompts: Array of {name, prompt} objects. Each agent runs independently.
        - coordinator_prompt: Optional prompt for a coordinator agent to review all results
        - timeout_secs: Maximum time for the entire team (default 300s)

        Example structure:
        ```json
        {
          "prompts": [
            {"name": "Backend Researcher", "prompt": "Research..."},
            {"name": "Frontend Researcher", "prompt": "Research..."}
          ],
          "coordinator_prompt": "Synthesize the research findings...",
          "timeout_secs": 300
        }
        ```

        Best for:
        - Multi-domain research tasks
        - Parallel code analysis from different angles
        - Distributed investigation across many files
        - Testing different implementation approaches simultaneously

        NOT ideal for:
        - Tightly dependent tasks (use Agent instead)
        - Single complex task (use Agent)
        - Tasks needing frequent back-and-forth
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<AgentTeamParams>()
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: AgentTeamParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        if params.prompts.is_empty() {
            return ToolResult {
                output: "Team must have at least one member (empty prompts array)".to_string(),
                is_error: true,
                images: vec![],
            };
        }

        if params.prompts.len() > 10 {
            return ToolResult {
                output: "Team size limited to 10 members (for safety)".to_string(),
                is_error: true,
                images: vec![],
            };
        }

        let timeout = std::time::Duration::from_secs(params.timeout_secs.unwrap_or(300));
        let state = Arc::new(AgentTeamState::new());
        let mut handles = vec![];

        // 获取共享配置快照
        let provider = safe_lock(&self.provider, "AgentTeamTool::provider").clone();
        let system_prompt = safe_lock(&self.system_prompt, "AgentTeamTool::system_prompt").clone();

        // 获取 sub_registry（排除 Agent 和 AgentTeam 防止递归）
        let (ask_tx, _ask_rx) = mpsc::channel::<crate::command::chat::app::AskRequest>();
        let sub_registry = ToolRegistry::new(
            vec![], // 不传 skills
            ask_tx,
            Arc::clone(&self.background_manager),
            Arc::clone(&self.task_manager),
            Arc::clone(&self.hook_manager),
        );
        let sub_registry = Arc::new(sub_registry);

        let mut disabled = self.disabled_tools.as_ref().clone();
        disabled.push("Agent".to_string());
        disabled.push("AgentTeam".to_string());
        let tools = sub_registry.to_openai_tools_filtered(&disabled);

        let jcli_config = Arc::clone(&self.jcli_config);

        // 为每个团队成员 spawn 一个线程
        for member in params.prompts {
            let state_clone = Arc::clone(&state);
            let provider_clone = provider.clone();
            let system_prompt_clone = system_prompt.clone();
            let registry_clone = Arc::clone(&sub_registry);
            let tools_clone = tools.clone();
            let jcli_config_clone = Arc::clone(&jcli_config);
            let cancelled_clone = Arc::clone(cancelled);
            let member_name = member.name.clone();
            let member_prompt = member.prompt;

            let handle = thread::spawn(move || {
                if cancelled_clone.load(Ordering::Relaxed) {
                    state_clone.set_result(
                        member_name,
                        "cancelled".to_string(),
                        "[Team cancelled]".to_string(),
                    );
                    return;
                }

                write_info_log("AgentTeam", &format!("Starting member: {}", member_name));

                let result = run_team_member_agent(
                    provider_clone,
                    system_prompt_clone,
                    member_prompt,
                    tools_clone,
                    registry_clone,
                    jcli_config_clone,
                    &cancelled_clone,
                    &member_name,
                );

                state_clone.set_result(member_name, "completed".to_string(), result);
            });

            handles.push(handle);
        }

        // 等待所有成员完成或超时
        let start = std::time::Instant::now();
        for handle in handles {
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.as_secs() > 0 {
                let _ = handle.join();
            }
        }

        // 收集所有成员的结果
        let member_results = state.get_all_results();
        let mut team_output = String::new();

        team_output.push_str("## Team Results\n\n");
        for (name, result) in member_results.iter() {
            team_output.push_str(&format!("### {}\n", name));
            team_output.push_str(&format!("**Status:** {}\n", result.status));
            team_output.push_str(&format!("**Output:**\n```\n{}\n```\n\n", result.output));
        }

        // 如果指定了 coordinator 提示，运行协调代理
        if let Some(coord_prompt) = params.coordinator_prompt {
            write_info_log("AgentTeam", "Running coordinator agent");

            let coordinator_input = format!(
                "{}\n\n## Team Member Results:\n{}",
                coord_prompt, team_output
            );

            let coord_result = run_team_member_agent(
                provider,
                system_prompt,
                coordinator_input,
                tools,
                sub_registry,
                jcli_config,
                cancelled,
                "Coordinator",
            );

            team_output.push_str("## Coordinator Analysis\n\n");
            team_output.push_str(&coord_result);
        }

        ToolResult {
            output: team_output,
            is_error: false,
            images: vec![],
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

// ========== Team Member Agent Loop ==========

/// 团队成员代理循环：与主代理循环类似，但针对团队优化
fn run_team_member_agent(
    provider: ModelProvider,
    system_prompt: Option<String>,
    prompt: String,
    tools: Vec<async_openai::types::chat::ChatCompletionTools>,
    registry: Arc<ToolRegistry>,
    jcli_config: Arc<JcliConfig>,
    cancelled: &Arc<AtomicBool>,
    member_name: &str,
) -> String {
    let max_rounds = 30;
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            return format!("Failed to create runtime: {}", e);
        }
    };

    let client = crate::command::chat::api::create_openai_client(&provider);

    let mut messages: Vec<crate::command::chat::storage::ChatMessage> =
        vec![crate::command::chat::storage::ChatMessage {
            role: "user".to_string(),
            content: prompt,
            tool_calls: None,
            tool_call_id: None,
            images: None,
        }];

    let mut final_text = String::new();

    for round in 0..max_rounds {
        if cancelled.load(Ordering::Relaxed) {
            return format!("{}\n[Cancelled]", final_text);
        }

        write_info_log(
            "AgentTeam",
            &format!("{}: Round {}/{}", member_name, round + 1, max_rounds),
        );

        let request = match crate::command::chat::api::build_request_with_tools(
            &provider,
            &messages,
            tools.clone(),
            system_prompt.as_deref(),
        ) {
            Ok(req) => req,
            Err(e) => {
                return format!("Failed to build request: {}", e);
            }
        };

        let response = rt.block_on(async {
            let chat_client = client.chat();
            chat_client.create(request).await
        });

        let response = match response {
            Ok(r) => r,
            Err(e) => {
                return format!("API error: {}", e);
            }
        };

        let choice = match response.choices.first() {
            Some(c) => c,
            None => {
                return format!("{}\n[No response from API]", final_text);
            }
        };

        let assistant_text = choice.message.content.clone().unwrap_or_default();
        if !assistant_text.is_empty() {
            final_text = assistant_text.clone();
        }

        // 检查是否有工具调用
        let is_tool_calls = matches!(
            choice.finish_reason,
            Some(async_openai::types::chat::FinishReason::ToolCalls)
        );

        if !is_tool_calls || choice.message.tool_calls.is_none() {
            break;
        }

        let tool_calls = choice.message.tool_calls.as_ref().unwrap();
        let tool_items: Vec<crate::command::chat::storage::ToolCallItem> = tool_calls
            .iter()
            .filter_map(|tc| {
                if let async_openai::types::chat::ChatCompletionMessageToolCalls::Function(f) = tc {
                    Some(crate::command::chat::storage::ToolCallItem {
                        id: f.id.clone(),
                        name: f.function.name.clone(),
                        arguments: f.function.arguments.clone(),
                    })
                } else {
                    None
                }
            })
            .collect();

        if tool_items.is_empty() {
            break;
        }

        messages.push(crate::command::chat::storage::ChatMessage {
            role: "assistant".to_string(),
            content: assistant_text,
            tool_calls: Some(tool_items.clone()),
            tool_call_id: None,
            images: None,
        });

        // 执行工具
        for item in &tool_items {
            if cancelled.load(Ordering::Relaxed) {
                messages.push(crate::command::chat::storage::ChatMessage {
                    role: "tool".to_string(),
                    content: "[Cancelled]".to_string(),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                });
                continue;
            }

            // 权限检查
            if jcli_config.is_denied(&item.name, &item.arguments) {
                messages.push(crate::command::chat::storage::ChatMessage {
                    role: "tool".to_string(),
                    content: format!("Tool '{}' denied by permission rules.", item.name),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                });
                continue;
            }

            let tool_ref = registry.get(&item.name);
            let requires_confirm = tool_ref.map(|t| t.requires_confirmation()).unwrap_or(false);

            if requires_confirm && !jcli_config.is_allowed(&item.name, &item.arguments) {
                messages.push(crate::command::chat::storage::ChatMessage {
                    role: "tool".to_string(),
                    content: format!(
                        "Tool '{}' requires confirmation. Add a permission rule to allow it.",
                        item.name
                    ),
                    tool_calls: None,
                    tool_call_id: Some(item.id.clone()),
                    images: None,
                });
                continue;
            }

            let result = registry.execute(&item.name, &item.arguments, cancelled);

            messages.push(crate::command::chat::storage::ChatMessage {
                role: "tool".to_string(),
                content: result.output,
                tool_calls: None,
                tool_call_id: Some(item.id.clone()),
                images: None,
            });
        }
    }

    if final_text.is_empty() {
        "[No output]".to_string()
    } else {
        final_text
    }
}
