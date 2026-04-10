use crate::command::chat::compact::CompactConfig;
use crate::command::chat::hook::HookManager;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::storage::ModelProvider;
use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::background::BackgroundManager;
use crate::command::chat::tools::task::TaskManager;
use crate::command::chat::tools::{
    Tool, ToolRegistry, ToolResult, parse_tool_args, schema_to_tool_params,
};
use crate::util::log::write_info_log;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

/// AgentTeam 参数：批量创建多个 teammate
#[derive(Deserialize, JsonSchema)]
struct AgentTeamParams {
    /// Array of teammate definitions to create
    members: Vec<AgentTeamMember>,
}

/// Team member definition
#[derive(Deserialize, JsonSchema)]
struct AgentTeamMember {
    /// Teammate name (e.g. "Frontend", "Backend")
    name: String,
    /// Role description (e.g. "React frontend developer")
    #[serde(default)]
    role: Option<String>,
    /// Initial task prompt for this teammate
    prompt: String,
}

// ========== AgentTeamTool ==========

/// Agent Team 工具：批量创建多个 teammate（CreateTeammate 的便捷封装）
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
    pub teammate_manager: Arc<Mutex<TeammateManager>>,
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
        Create multiple teammates at once for parallel collaboration.
        This is a convenience wrapper around CreateTeammate — it creates several teammates
        in one call, each with their own agent loop running independently.

        All teammates communicate via SendMessage tool (broadcast with @mentions).

        Usage:
        - members: Array of {name, role?, prompt} objects

        Example:
        ```json
        {
          "members": [
            {"name": "Frontend", "role": "React developer", "prompt": "Create a React Todo app..."},
            {"name": "Backend", "role": "Express developer", "prompt": "Create an Express API..."}
          ]
        }
        ```

        Best for:
        - Full-stack development (Frontend + Backend + DevOps)
        - Multi-domain research tasks
        - Any task that benefits from parallel work by specialized agents
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<AgentTeamParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: AgentTeamParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        if params.members.is_empty() {
            return ToolResult {
                output: "Team must have at least one member".to_string(),
                is_error: true,
                images: vec![],
            };
        }

        if params.members.len() > 10 {
            return ToolResult {
                output: "Team size limited to 10 members".to_string(),
                is_error: true,
                images: vec![],
            };
        }

        // 为每个成员调用 CreateTeammate 的逻辑
        let create_tool = crate::command::chat::tools::create_teammate::CreateTeammateTool {
            teammate_manager: Arc::clone(&self.teammate_manager),
            background_manager: Arc::clone(&self.background_manager),
            provider: Arc::clone(&self.provider),
            system_prompt: Arc::clone(&self.system_prompt),
            jcli_config: Arc::clone(&self.jcli_config),
            compact_config: self.compact_config.clone(),
            hook_manager: Arc::clone(&self.hook_manager),
            task_manager: Arc::clone(&self.task_manager),
            disabled_tools: Arc::clone(&self.disabled_tools),
        };

        let mut results = Vec::new();
        let cancelled = Arc::new(AtomicBool::new(false));

        for member in &params.members {
            let role = member.role.clone().unwrap_or_else(|| member.name.clone());
            let args = serde_json::json!({
                "name": member.name,
                "role": role,
                "prompt": member.prompt,
            })
            .to_string();

            let result = create_tool.execute(&args, &cancelled);
            results.push(format!("- {}: {}", member.name, result.output));
        }

        ToolResult {
            output: format!("## Team Created\n\n{}", results.join("\n")),
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
#[allow(dead_code, clippy::too_many_arguments)]
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
