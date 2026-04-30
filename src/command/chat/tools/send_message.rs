use crate::command::chat::agent::thread_identity::current_agent_name;
use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex, atomic::AtomicBool};

/// SendMessage 参数
#[derive(Deserialize, JsonSchema)]
struct SendMessageParams {
    /// Message content to send
    message: String,
    /// Optional: target agent name (e.g. "@Backend"). If omitted, message is broadcast to all.
    #[serde(default)]
    to: Option<String>,
}

/// SendMessage 工具：在聊天室中发送消息给其他 agent
pub struct SendMessageTool {
    pub teammate_manager: Arc<Mutex<TeammateManager>>,
}

impl SendMessageTool {
    pub const NAME: &'static str = "SendMessage";
}

impl Tool for SendMessageTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Send a message to teammates. This is the ONLY way to make your words visible to other agents.

        Your plain text output (prose without tool calls) is private thinking — only you and the human user can see it.
        To communicate with teammates, you MUST use this tool.

        Usage:
        - message: The text content to send
        - to: Optional target agent name. The message is broadcast to everyone,
              but @mentioning wakes the target agent (interrupts their idle state).

        Messages appear in the chat as: <YourName> @Target message
        All agents receive the message, but the @mentioned agent knows it's addressed to them.

        Note: If new messages arrive while you're thinking, your SendMessage may be held for
        re-evaluation. You'll receive a system_reminder and can choose to resend or revise.

        Example:
        {"message": "API endpoints are done, check routes/api.js", "to": "Frontend"}
        {"message": "All frontend pages are complete", "to": "Main"}
        {"message": "Heads up: database schema has been updated"}
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<SendMessageParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: SendMessageParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        if params.message.trim().is_empty() {
            return ToolResult {
                output: "Message cannot be empty".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        let from = current_agent_name();
        let to = params.to.as_deref().map(|s| {
            let s = s.trim_start_matches('@');
            // 兼容全名格式：Teammate@Frontend → Frontend, SubAgent@search → search
            s.strip_prefix("Teammate@")
                .or_else(|| s.strip_prefix("SubAgent@"))
                .unwrap_or(s)
        });

        match self.teammate_manager.lock() {
            Ok(manager) => {
                manager.broadcast(&from, &params.message, to);
                let target_desc = to
                    .map(|t| format!("@{}", t))
                    .unwrap_or_else(|| "all teammates".to_string());
                ToolResult {
                    output: format!("Message sent to {}", target_desc),
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
            Err(_) => ToolResult {
                output: "Failed to acquire teammate manager lock".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }

    fn is_available(&self) -> bool {
        self.teammate_manager
            .lock()
            .map(|m| m.has_active_teammates())
            .unwrap_or(false)
    }
}
