use crate::command::chat::storage::ChatMessage;
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tokio_util::sync::CancellationToken;

/// Default timeout in seconds (matches MAX_CONSECUTIVE_IDLE_POLLS)
const DEFAULT_TIMEOUT_SECS: u64 = 120;
/// Poll interval in milliseconds
const POLL_INTERVAL_MS: u64 = 100;

#[derive(Deserialize, JsonSchema)]
struct WaitForMessageParams {
    /// Maximum time to wait in seconds (default: 120). Returns error on timeout.
    #[serde(default = "default_timeout")]
    timeout: u64,
    /// Optional: only return messages from this agent (e.g. "Backend", "Main").
    /// Messages from other agents are preserved for the next round.
    #[serde(default)]
    from: Option<String>,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

/// WaitForMessage 工具：teammate 阻塞等待其他 agent 的消息
///
/// 调用后阻塞当前线程，直到收到匹配的广播消息或超时/取消。
/// 消息从 `pending_user_messages` 中 drain，不会与 teammate_loop 的 drain 冲突。
pub struct WaitForMessageTool {
    /// 该 teammate 的 pending_user_messages（与 TeammateHandle 共享）
    pub pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// 取消令牌（与 TeammateHandle 共享）
    pub cancel_token: CancellationToken,
}

impl WaitForMessageTool {
    pub const NAME: &'static str = "WaitForMessage";
}

impl Tool for WaitForMessageTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Block and wait for a message from another agent in the chatroom.

        Use this when you need input from a teammate before proceeding.
        The tool blocks until a matching message arrives or the timeout expires.

        Usage:
        - timeout: Max wait time in seconds (default 120). Returns error on timeout.
        - from: Optional sender filter. Only messages from this agent will be returned.
                Other agents' messages are preserved for your next round.

        Examples:
        {}                                    // Wait for any message
        {"from": "Backend"}                   // Wait for a message from Backend
        {"from": "Main", "timeout": 60}       // Wait up to 60s for Main's message

        IMPORTANT:
        - While waiting, you cannot use other tools or respond to messages.
        - If you need to do work while waiting, DO NOT call this tool -- stay idle instead.
        - After receiving a message, use SendMessage to reply if needed.
        - On timeout, consider whether to retry, call WorkDone, or take other action.
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<WaitForMessageParams>()
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: WaitForMessageParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(params.timeout);
        let poll_interval = std::time::Duration::from_millis(POLL_INTERVAL_MS);

        loop {
            // 取消检查
            if cancelled.load(Ordering::Relaxed) || self.cancel_token.is_cancelled() {
                return ToolResult {
                    output: "WaitForMessage cancelled".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }

            // 超时检查
            if start.elapsed() >= timeout {
                return ToolResult {
                    output: format!(
                        "WaitForMessage timed out after {}s (no message arrived)",
                        params.timeout
                    ),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }

            // Drain 所有 pending 消息
            let drained: Vec<ChatMessage> = match self.pending_user_messages.lock() {
                Ok(mut pending) => std::mem::take(&mut *pending),
                Err(_) => {
                    std::thread::sleep(poll_interval);
                    continue;
                }
            };

            if !drained.is_empty() {
                if let Some(ref from) = params.from {
                    // 按 from 过滤：matching 返回，non-matching 放回
                    let (matching, non_matching): (Vec<_>, Vec<_>) = drained
                        .into_iter()
                        .partition(|m| is_from_sender(&m.content, from));

                    // 放回不匹配的消息
                    if !non_matching.is_empty()
                        && let Ok(mut pending) = self.pending_user_messages.lock()
                    {
                        let mut combined = non_matching;
                        combined.append(&mut *pending);
                        *pending = combined;
                    }

                    if !matching.is_empty() {
                        return ToolResult {
                            output: format_messages(&matching),
                            is_error: false,
                            images: vec![],
                            plan_decision: PlanDecision::None,
                        };
                    }
                    // 没有匹配的消息，继续等待
                } else {
                    // 无过滤 — 返回所有消息
                    return ToolResult {
                        output: format_messages(&drained),
                        is_error: false,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
            }

            // 无消息，休眠后继续轮询
            std::thread::sleep(poll_interval);
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

/// 检查消息是否来自指定发送者
///
/// 消息格式：`<Main> text` 或 `<Teammate@Backend> text`
fn is_from_sender(content: &str, from: &str) -> bool {
    if from == "Main" {
        content.starts_with("<Main>")
    } else {
        content.starts_with(&format!("<Teammate@{}>", from))
    }
}

/// 将多条消息格式化为换行分隔的文本
fn format_messages(messages: &[ChatMessage]) -> String {
    messages
        .iter()
        .map(|m| m.content.clone())
        .collect::<Vec<_>>()
        .join("\n")
}
