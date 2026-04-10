use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::{Tool, ToolResult, parse_tool_args, schema_to_tool_params};
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
    /// Optional: short summary of the message (5-10 words)
    #[serde(default)]
    #[allow(dead_code)]
    summary: Option<String>,
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
        Send a message to teammates in the chatroom. Messages are visible to all agents.

        Usage:
        - message: The text content to send
        - to: Optional target agent name. The message is always broadcast to everyone,
              but @mentioning focuses the target agent's attention.
        - summary: Optional 5-10 word summary

        Messages appear in the chat as: <YourName> @Target message
        All agents receive the message, but the @mentioned agent knows it's addressed to them.

        Example:
        {"message": "API 端点已完成，请查看 routes/api.js", "to": "Frontend"}
        {"message": "所有前端页面已完成", "to": "Main"}
        {"message": "大家注意：数据库 schema 有更新"}
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
            };
        }

        let from = crate::command::chat::teammate::current_agent_name();
        let to = params.to.as_deref().map(|s| s.trim_start_matches('@'));

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
                }
            }
            Err(_) => ToolResult {
                output: "Failed to acquire teammate manager lock".to_string(),
                is_error: true,
                images: vec![],
            },
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
