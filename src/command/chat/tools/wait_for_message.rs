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

/// Default timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 30;
/// Poll interval in milliseconds
const POLL_INTERVAL_MS: u64 = 100;

#[derive(Deserialize, JsonSchema)]
struct WaitForMessageParams {
    /// Maximum time to wait in seconds (default: 30).
    /// Returns whatever messages are available on timeout (even if none match from/keyword).
    #[serde(default = "default_timeout")]
    timeout: u64,
    /// Optional: wait until a message from this agent arrives (e.g. "Backend", "Main").
    /// When a matching message arrives, ALL accumulated messages are returned (not just matching ones).
    /// On timeout, ALL accumulated messages are returned regardless of filter.
    #[serde(default)]
    from: Option<String>,
    /// Optional: wait until a message containing this keyword arrives.
    /// When a matching message arrives, ALL accumulated messages are returned.
    /// On timeout, ALL accumulated messages are returned regardless of filter.
    #[serde(default)]
    keyword: Option<String>,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

/// WaitForMessage 工具：teammate 阻塞等待其他 agent 的消息
///
/// 调用后阻塞当前线程，直到收到匹配的广播消息或超时/取消。
/// **关键语义**：from/keyword 仅控制「何时停止等待」，返回时始终给出 ALL 积累的消息。
/// 这避免了过滤导致的消息堆积和互等死锁。
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
        Wait for messages from other agents in the chatroom.

        Blocks until a matching message arrives or timeout. On return, delivers ALL
        accumulated messages (not just filtered ones) so you never miss context.

        Filter semantics:
        - from/keyword control WHEN to stop waiting, not WHAT you see.
        - When a matching message arrives → return immediately with ALL messages.
        - On timeout → return ALL accumulated messages (even if none match).

        Usage:
        - timeout: Max wait in seconds (default 30).
        - from: Wait until a message from this agent arrives.
        - keyword: Wait until a message containing this keyword arrives.

        Examples:
        {}                                               // Wait for any message, return all
        {"from": "Backend"}                              // Wait until Backend sends, return all
        {"keyword": "deploy"}                            // Wait until "deploy" mentioned, return all
        {"from": "Main", "keyword": "approved"}          // Wait for Main to say "approved"
        {"from": "Main", "timeout": 60}                  // Wait up to 60s

        IMPORTANT:
        - While waiting, you cannot use other tools or respond to messages.
        - If you need to do work while waiting, DO NOT call this tool -- stay idle instead.
        - Two teammates should NEVER both wait for each other simultaneously — that causes a deadlock.
        - After receiving a message, use SendMessage to reply if needed.
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
        let mut accumulated: Vec<ChatMessage> = Vec::new();

        loop {
            // 取消检查
            if cancelled.load(Ordering::Relaxed) || self.cancel_token.is_cancelled() {
                return ToolResult {
                    output: if accumulated.is_empty() {
                        "WaitForMessage cancelled".to_string()
                    } else {
                        format!(
                            "WaitForMessage cancelled, but {} message(s) received:\n{}",
                            accumulated.len(),
                            format_messages(&accumulated)
                        )
                    },
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
                accumulated.extend(drained);

                // 检查是否有匹配的消息（控制停止等待的条件）
                let has_match = accumulated.iter().any(|m| {
                    message_matches(
                        &m.content,
                        params.from.as_deref(),
                        params.keyword.as_deref(),
                    )
                });

                if has_match {
                    // 匹配消息到达 → 返回所有积累的消息
                    return ToolResult {
                        output: format_messages(&accumulated),
                        is_error: false,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
                // 无匹配但已有消息 → 继续等待匹配消息（但有消息在手，互等时可被打破）
            }

            // 超时检查
            if start.elapsed() >= timeout {
                if !accumulated.is_empty() {
                    // 超时但有消息 → 返回所有积累的消息
                    return ToolResult {
                        output: format!(
                            "WaitForMessage timed out after {}s. No message matched filter, but {} message(s) received:\n{}",
                            params.timeout,
                            accumulated.len(),
                            format_messages(&accumulated)
                        ),
                        is_error: false,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
                // 超时且无消息
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

            // 无消息 / 无匹配 → 休眠后继续轮询
            std::thread::sleep(poll_interval);
        }
    }

    fn requires_confirmation(&self) -> bool {
        false
    }
}

/// 检查消息是否匹配过滤条件（from + keyword 都满足才匹配）
///
/// 消息格式：`<Main> text` 或 `<Teammate@Backend> text`
fn message_matches(content: &str, from: Option<&str>, keyword: Option<&str>) -> bool {
    if let Some(from) = from {
        if from == "Main" {
            if !content.starts_with("<Main>") {
                return false;
            }
        } else if !content.starts_with(&format!("<Teammate@{}>", from)) {
            return false;
        }
    }
    if let Some(keyword) = keyword
        && !content.contains(keyword)
    {
        return false;
    }
    true
}

/// 将多条消息格式化为换行分隔的文本
fn format_messages(messages: &[ChatMessage]) -> String {
    messages
        .iter()
        .map(|m| m.content.clone())
        .collect::<Vec<_>>()
        .join("\n")
}
