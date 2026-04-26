use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex, atomic::AtomicBool};

#[derive(Deserialize, JsonSchema)]
struct IgnoreMessageParams {
    /// Optional: brief one-line note for your own log; not broadcast to anyone.
    #[serde(default)]
    #[allow(dead_code)]
    reason: Option<String>,
}

/// IgnoreMessage 工具：显式声明「我看到了广播但选择不响应」
///
/// 用于避免被无关 broadcast 唤醒后被迫凑话回应（thrash）。
/// 调用本工具时，本轮的 assistant 文本不会被广播到聊天室，
/// 因此不会触发其他 agent 的 wake_flag，loop 自然回到 idle。
///
/// teammate 与 Main 共用此工具：
/// - teammate 通过 `teammate_manager=None` 注册，始终可用
/// - Main 通过 `teammate_manager=Some(_)` 注册，仅在有活跃 teammate 时可用
pub struct IgnoreMessageTool {
    pub teammate_manager: Option<Arc<Mutex<TeammateManager>>>,
}

impl IgnoreMessageTool {
    pub const NAME: &'static str = "IgnoreMessage";
}

impl Tool for IgnoreMessageTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Acknowledge incoming broadcasts without producing any reply.

        Call this when you were notified of broadcasts or @mentions but, after
        reading them, decided no response is needed. This keeps you idle without
        sending a noisy "ok"/"got it" message that would wake other agents.

        Usage:
        - reason: Optional one-line note for your own log (NOT broadcast).

        Examples:
        {}
        {"reason": "B's progress update doesn't require my input"}
        {"reason": "Noted schema change, does not affect my current task"}

        IMPORTANT:
        - Calling IgnoreMessage suppresses the prose of THIS turn — do NOT also
          write any reply text; just call the tool.
        - If a real response is needed, use SendMessage instead.
        - You will remain idle until @mentioned again or until the chat is closed.
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<IgnoreMessageParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        // Validate args (reason is optional and unused at runtime; we just ensure parse OK)
        let _params: IgnoreMessageParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        ToolResult {
            output: "Messages acknowledged; no reply sent. Returning to idle.".to_string(),
            is_error: false,
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }

    fn is_available(&self) -> bool {
        match &self.teammate_manager {
            Some(mgr) => mgr
                .lock()
                .map(|m| m.has_active_teammates())
                .unwrap_or(false),
            None => true,
        }
    }
}
