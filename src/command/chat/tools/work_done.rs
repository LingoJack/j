use crate::command::chat::agent::thread_identity::current_agent_name;
use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::borrow::Cow;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Deserialize, JsonSchema)]
struct WorkDoneParams {
    /// Short summary of what you accomplished (one or two sentences)
    #[serde(default)]
    summary: Option<String>,
}

/// WorkDone 工具：teammate 明确声明自己完成工作并进入终态
///
/// 调用后：
/// - set work_done=true，teammate_loop 下次检查时 break 主循环
/// - 自动广播一条 `<self> [work completed] summary` 给团队（帮 @Main 感知）
pub struct WorkDoneTool {
    /// 该 teammate 的 work_done 标志（与 TeammateHandle 共享）
    pub work_done: Arc<AtomicBool>,
    pub teammate_manager: Arc<Mutex<TeammateManager>>,
}

impl WorkDoneTool {
    pub const NAME: &'static str = "WorkDone";
}

impl Tool for WorkDoneTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> Cow<'_, str> {
        r#"
        Declare that you have finished your assigned work and exit the chat loop.

        Call this when your role's task is complete and you don't need to do anything else.
        Once called, you will stop responding to general messages — the team will see you as finished.

        IMPORTANT:
        - Before calling WorkDone, send a SendMessage to @Main summarizing your results.
        - If another agent @mentions you after WorkDone, you will be re-activated and can continue working.
        - If another agent might still need you, DO NOT call WorkDone — just stay idle and wait.
        - Do not call WorkDone just because the conversation is quiet; only call when your role's task is objectively done.

        Usage:
        {"summary": "Frontend pages complete, all components in src/components/"}
        "#.into()
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<WorkDoneParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: WorkDoneParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let from = current_agent_name();

        // 通过 broadcast 通知所有 agent（包括其他 teammate 和 Main）
        // broadcast 会处理 display/context 双通道 + main_agent_inbox 唤醒信号
        let broadcast_text = match params.summary.as_deref() {
            Some(s) if !s.trim().is_empty() => {
                format!("[work completed] {}", s.trim())
            }
            _ => "[work completed]".to_string(),
        };
        if let Ok(manager) = self.teammate_manager.lock() {
            manager.broadcast(&from, &broadcast_text, None);
        }

        self.work_done.store(true, Ordering::Relaxed);

        ToolResult {
            output: format!("Teammate '{}' marked as done; exiting chat loop.", from),
            is_error: false,
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}
