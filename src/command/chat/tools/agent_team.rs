use crate::command::chat::compact::CompactConfig;
use crate::command::chat::hook::HookManager;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::storage::ModelProvider;
use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::background::BackgroundManager;
use crate::command::chat::tools::task::TaskManager;
use crate::command::chat::tools::{Tool, ToolResult, parse_tool_args, schema_to_tool_params};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex, atomic::AtomicBool};

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
