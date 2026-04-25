use crate::command::chat::storage::ChatMessage;
use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

/// Default timeout in seconds (matches MAX_CONSECUTIVE_IDLE_POLLS)
const DEFAULT_TIMEOUT_SECS: u64 = 120;
/// Poll interval in milliseconds
const POLL_INTERVAL_MS: u64 = 100;

#[derive(Deserialize, JsonSchema)]
struct WaitForMessageParams {
    /// Maximum time to wait in seconds (default: 120). Returns error on timeout.
    #[serde(default = "default_timeout")]
    timeout: u64,
    /// Optional: only return messages from this agent (e.g. "Backend", "Frontend").
    /// Messages from other agents are skipped (but not removed).
    #[serde(default)]
    from: Option<String>,
    /// Optional: only return messages containing this keyword/substring.
    /// Messages not containing the keyword are skipped (but not removed).
    #[serde(default)]
    keyword: Option<String>,
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_SECS
}

/// Main Agent 专用 WaitForMessage 工具（peek 模式）
///
/// 与 teammate 版本不同，Main Agent 的 `context_messages` 被 UI 的
/// `poll_stream_actions` peek（用 `context_read_offset`），所以这里也用 peek：
/// 只读取增量（`last_seen_len` 之后的消息），不移除。
///
/// 两边互不干扰：UI 的 `context_read_offset` 和工具的 `last_seen_len` 独立。
pub struct MainWaitForMessageTool {
    /// Main Agent 的 context_messages（与 TeammateManager 共享）
    pub context_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// Teammate 管理器（用于 is_available 检查）
    pub teammate_manager: Arc<Mutex<TeammateManager>>,
    /// 已读取位置（peek 模式：只读不取）
    pub last_seen_len: AtomicUsize,
}

impl MainWaitForMessageTool {
    pub const NAME: &'static str = "WaitForMessage";
}

impl Tool for MainWaitForMessageTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Block and wait for a message from a teammate in the chatroom.

        Use this when you need input from a teammate before proceeding.
        The tool blocks until a matching message arrives or the timeout expires.

        Usage:
        - timeout: Max wait time in seconds (default 120). Returns error on timeout.
        - from: Optional sender filter. Only messages from this teammate will be returned.
                Other teammates' messages are skipped (but preserved for UI display).
        - keyword: Optional content filter. Only messages containing this keyword will be returned.
                   Non-matching messages are skipped (but preserved for UI display).

        Examples:
        {}                                               // Wait for any teammate message
        {"from": "Backend"}                              // Wait for a message from Backend
        {"keyword": "deploy"}                            // Wait for any message containing "deploy"
        {"from": "Backend", "keyword": "approved"}      // Wait for Backend to say "approved"
        {"from": "Backend", "timeout": 60}              // Wait up to 60s for Backend's message

        IMPORTANT:
        - While waiting, you cannot use other tools or respond to messages.
        - If you need to do work while waiting, DO NOT call this tool -- stay idle instead.
        - After receiving a message, use SendMessage to reply if needed.
        - On timeout, consider whether to retry or take other action.
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
            if cancelled.load(Ordering::Relaxed) {
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

            // Peek 模式：从 context_messages 读取增量（last_seen_len 之后）
            let (current_len, new_messages) = match self.context_messages.lock() {
                Ok(messages) => {
                    let current_len = messages.len();
                    // auto_compact 可能已清空并重建 messages，此时归零 last_seen_len
                    let last_seen = self.last_seen_len.load(Ordering::Relaxed);
                    let effective_start = if current_len < last_seen {
                        0
                    } else {
                        last_seen
                    };

                    // 读取增量切片
                    let new_msgs: Vec<ChatMessage> =
                        messages[effective_start..current_len].to_vec();
                    (current_len, new_msgs)
                }
                Err(_) => {
                    std::thread::sleep(poll_interval);
                    continue;
                }
            };

            // 更新 last_seen_len（即使没找到匹配的，也要更新以便下次只看新增量）
            // 注意：匹配过滤的消息才返回，不匹配的消息跳过但更新 last_seen_len
            if !new_messages.is_empty() {
                // 过滤出匹配的消息
                let matching: Vec<ChatMessage> = new_messages
                    .iter()
                    .filter(|m| {
                        message_matches(
                            &m.content,
                            params.from.as_deref(),
                            params.keyword.as_deref(),
                        )
                    })
                    .cloned()
                    .collect();

                // 更新 last_seen_len 到当前已读位置
                self.last_seen_len.store(current_len, Ordering::Relaxed);

                if !matching.is_empty() {
                    return ToolResult {
                        output: format_messages(&matching),
                        is_error: false,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
                // 没有匹配的消息，继续等待（但 last_seen_len 已更新，下次只看新消息）
            }

            // 无消息或无匹配，休眠后继续轮询
            std::thread::sleep(poll_interval);
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

/// 检查消息是否匹配过滤条件（from + keyword 都满足才匹配）
///
/// 消息格式：`<Teammate@Backend> text`（来自 teammate）
fn message_matches(content: &str, from: Option<&str>, keyword: Option<&str>) -> bool {
    if let Some(from) = from {
        // Main Agent 接收的是 teammate 发的消息，格式为 <Teammate@Name>
        if !content.starts_with(&format!("<Teammate@{}>", from)) {
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
