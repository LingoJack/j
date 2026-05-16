use crate::constants::{HOOK_DEFAULT_LLM_TIMEOUT_SECS, HOOK_DEFAULT_TIMEOUT_SECS};
use crate::storage::ChatMessage;
use serde::{Deserialize, Serialize};
use std::env;

// ========== 常量 ==========

/// Hook 链总超时（秒）：整条链执行超过此时间后，中止未执行的 hook
pub(crate) const MAX_CHAIN_DURATION_SECS: u64 = 30;

// ========== HookEvent ==========

/// Hook 事件类型
///
/// 各事件的触发时机及可读/可写字段：
///
/// stop / skip 语义（统一规则）：
/// - `stop`：中止当前步骤及其所属子管线（不发送/不请求/不结束/不保存/中止 compact）
/// - `skip`：跳过当前步骤，同级步骤继续（仅 PreToolExecution：跳过该工具，其他工具继续）
///
/// | 事件                          | 触发时机           | 可读字段                              | 可写字段（HookResult 中返回即生效）        |
/// |-------------------------------|--------------------|-----------------------------------------|----------------------------------------------|
/// | `PreSendMessage`              | 用户消息入队前     | `user_input`, `messages`               | `user_input`（修改发送内容）, `action=stop`, `retry_feedback` |
/// | `PostSendMessage`             | 用户消息入队后     | `user_input`, `messages`               | 仅通知，返回值被忽略                         |
/// | `PreLlmRequest`               | LLM API 请求前     | `messages`, `system_prompt`, `model`   | `messages`, `system_prompt`, `inject_messages`, `additional_context`, `action=stop`, `retry_feedback` |
/// | `PostLlmResponse`             | LLM 回复完成后     | `assistant_output`, `messages`, `model` | `assistant_output`（修改最终回复）, `action=stop`, `retry_feedback`, `system_message` |
/// | `PreToolExecution`            | 工具执行前         | `tool_name`, `tool_arguments`          | `tool_arguments`（修改参数）, `action=skip`  |
/// | `PostToolExecution`           | 工具执行后         | `tool_name`, `tool_result`             | `tool_result`（修改结果）                    |
/// | `PostToolExecutionFailure`    | 工具执行失败后     | `tool_name`, `tool_error`              | `tool_error`（修改错误信息）, `additional_context` |
/// | `Stop`                        | LLM 即将结束回复   | `user_input`（回复文本）, `messages`, `system_prompt`, `model` | `retry_feedback`（带反馈重试）, `additional_context`, `action=stop` |
/// | `PreMicroCompact`             | micro_compact 前   | `messages`, `model`                   | `action=stop`                               |
/// | `PostMicroCompact`            | micro_compact 后   | `messages`                             | `messages`（修改压缩结果）                    |
/// | `PreAutoCompact`              | auto_compact 前    | `messages`, `system_prompt`, `model`   | `additional_context`, `action=stop`         |
/// | `PostAutoCompact`             | auto_compact 后    | `messages`                             | `messages`（修改压缩结果）                    |
/// | `SessionStart`                | 会话启动时         | `messages`                             | 仅通知，返回值被忽略                         |
/// | `SessionEnd`                  | 会话退出时         | `messages`                             | 仅通知，返回值被忽略                         |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreSendMessage,
    PostSendMessage,
    PreLlmRequest,
    PostLlmResponse,
    PreToolExecution,
    PostToolExecution,
    PostToolExecutionFailure,
    Stop,
    PreMicroCompact,
    PostMicroCompact,
    PreAutoCompact,
    PostAutoCompact,
    SessionStart,
    SessionEnd,
}

impl std::str::FromStr for HookEvent {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pre_send_message" => Ok(HookEvent::PreSendMessage),
            "post_send_message" => Ok(HookEvent::PostSendMessage),
            "pre_llm_request" => Ok(HookEvent::PreLlmRequest),
            "post_llm_response" => Ok(HookEvent::PostLlmResponse),
            "pre_tool_execution" => Ok(HookEvent::PreToolExecution),
            "post_tool_execution" => Ok(HookEvent::PostToolExecution),
            "post_tool_execution_failure" => Ok(HookEvent::PostToolExecutionFailure),
            "stop" => Ok(HookEvent::Stop),
            "pre_micro_compact" => Ok(HookEvent::PreMicroCompact),
            "post_micro_compact" => Ok(HookEvent::PostMicroCompact),
            "pre_auto_compact" => Ok(HookEvent::PreAutoCompact),
            "post_auto_compact" => Ok(HookEvent::PostAutoCompact),
            "session_start" => Ok(HookEvent::SessionStart),
            "session_end" => Ok(HookEvent::SessionEnd),
            _ => Err(()),
        }
    }
}

impl HookEvent {
    /// 返回 Hook 事件的字符串标识（如 "pre_send_message"）
    pub fn as_str(&self) -> &'static str {
        match self {
            HookEvent::PreSendMessage => "pre_send_message",
            HookEvent::PostSendMessage => "post_send_message",
            HookEvent::PreLlmRequest => "pre_llm_request",
            HookEvent::PostLlmResponse => "post_llm_response",
            HookEvent::PreToolExecution => "pre_tool_execution",
            HookEvent::PostToolExecution => "post_tool_execution",
            HookEvent::PostToolExecutionFailure => "post_tool_execution_failure",
            HookEvent::Stop => "stop",
            HookEvent::PreMicroCompact => "pre_micro_compact",
            HookEvent::PostMicroCompact => "post_micro_compact",
            HookEvent::PreAutoCompact => "pre_auto_compact",
            HookEvent::PostAutoCompact => "post_auto_compact",
            HookEvent::SessionStart => "session_start",
            HookEvent::SessionEnd => "session_end",
        }
    }

    /// 返回所有 HookEvent 枚举值的静态切片，用于遍历/校验
    pub fn all() -> &'static [HookEvent] {
        &[
            HookEvent::PreSendMessage,
            HookEvent::PostSendMessage,
            HookEvent::PreLlmRequest,
            HookEvent::PostLlmResponse,
            HookEvent::PreToolExecution,
            HookEvent::PostToolExecution,
            HookEvent::PostToolExecutionFailure,
            HookEvent::Stop,
            HookEvent::PreMicroCompact,
            HookEvent::PostMicroCompact,
            HookEvent::PreAutoCompact,
            HookEvent::PostAutoCompact,
            HookEvent::SessionStart,
            HookEvent::SessionEnd,
        ]
    }

    /// 从字符串解析，不匹配时返回 None
    pub fn parse(s: &str) -> Option<HookEvent> {
        s.parse().ok()
    }
}

// ========== OnError ==========

/// Shell hook 失败时的处理策略
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnError {
    /// 记录错误日志后继续执行后续 hook（默认）
    #[default]
    Skip,
    /// 中止整条 hook 链
    Stop,
}

// ========== HookFilter ==========

/// Hook 条件过滤：仅当条件匹配时才执行该 hook
///
/// 所有字段为可选，未设置的字段不参与过滤（即视为匹配）。
/// 多个字段同时设置时取 AND 关系。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookFilter {
    /// 工具名过滤（精确匹配，仅对工具相关事件生效）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    /// 工具名模式匹配（管道分隔，如 "Write|Edit|Bash"，仅对工具相关事件生效）
    /// 优先级低于 tool_name：当 tool_name 设置时忽略此字段
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_matcher: Option<String>,
    /// 模型名前缀过滤（如 "gpt-4" 匹配 "gpt-4o"、"gpt-4-turbo"）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_prefix: Option<String>,
}

impl HookFilter {
    /// 是否为空过滤器（无任何条件，始终匹配）
    pub fn is_empty(&self) -> bool {
        self.tool_name.is_none() && self.tool_matcher.is_none() && self.model_prefix.is_none()
    }

    /// 根据 HookContext 判断是否匹配
    pub fn matches(&self, context: &HookContext) -> bool {
        // 精确匹配 tool_name（优先级最高）
        if let Some(ref expected_tool) = self.tool_name {
            match &context.tool_name {
                Some(actual) if actual == expected_tool => {}
                Some(_) => return false,
                None => return false,
            }
        } else if let Some(ref pattern) = self.tool_matcher {
            // 管道分隔模式匹配（如 "Write|Edit|Bash"）
            let actual = match &context.tool_name {
                Some(a) => a,
                None => return false,
            };
            let matched = pattern.split('|').any(|p| p.trim() == actual);
            if !matched {
                return false;
            }
        }
        if let Some(ref prefix) = self.model_prefix {
            match &context.model {
                Some(actual) if actual.starts_with(prefix.as_str()) => {}
                Some(_) => return false,
                None => return false,
            }
        }
        true
    }
}

// ========== HookType ==========

/// Hook 类型（YAML `type` 字段）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    /// Shell 命令 hook（默认，通过 `sh -c` 子进程执行）
    #[default]
    Bash,
    /// LLM hook（通过 prompt 模板调用 LLM，返回 HookResult JSON）
    Llm,
}

impl std::fmt::Display for HookType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookType::Bash => write!(f, "bash"),
            HookType::Llm => write!(f, "llm"),
        }
    }
}

// ========== HookContext ==========

/// Hook 执行上下文（通过 stdin JSON 传给脚本）
///
/// 各字段按事件类型有选择性地填充，未填充的字段序列化时会被跳过（`skip_serializing_if`）。
/// 脚本可通过 stdin 读取此 JSON 来获取当前事件的上下文信息。
#[derive(Debug, Serialize)]
pub struct HookContext {
    /// 当前触发的事件类型
    pub event: HookEvent,

    /// 当前对话的完整消息列表
    /// - 可读事件：PreSendMessage, PostSendMessage, PreLlmRequest, PostLlmResponse, SessionStart, SessionEnd
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<ChatMessage>>,

    /// 当前系统提示词
    /// - 可读事件：PreLlmRequest
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// 当前使用的模型名称
    /// - 可读事件：PreLlmRequest
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// 本轮用户输入的消息文本
    /// - 可读事件：PreSendMessage（发送前，可通过 HookResult 修改）、PostSendMessage（发送后，只读）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_input: Option<String>,

    /// 本轮 AI 回复的完整文本
    /// - 可读事件：PostLlmResponse（可通过 HookResult 修改最终展示内容）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistant_output: Option<String>,

    /// 当前工具调用的工具名
    /// - 可读事件：PreToolExecution, PostToolExecution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,

    /// 当前工具调用的参数 JSON 字符串
    /// - 可读事件：PreToolExecution（可通过 HookResult 修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_arguments: Option<String>,

    /// 工具执行的结果内容
    /// - 可读事件：PostToolExecution（可通过 HookResult 修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<String>,

    /// 工具执行失败原因
    /// - 可读事件：PostToolExecutionFailure（可通过 HookResult 修改）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_error: Option<String>,

    /// 当前会话 ID
    /// - 可读事件：所有事件
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// 当前工作目录
    pub cwd: String,
}

impl Default for HookContext {
    fn default() -> Self {
        Self {
            event: HookEvent::SessionStart,
            messages: None,
            system_prompt: None,
            model: None,
            user_input: None,
            assistant_output: None,
            tool_name: None,
            tool_arguments: None,
            tool_result: None,
            tool_error: None,
            session_id: None,
            cwd: env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string()),
        }
    }
}

// ========== HookAction / HookResult / HookOutcome ==========

/// Hook 脚本返回结果中的控制流动作
///
/// - `action: "stop"` 中止当前步骤及其所属子管线
/// - `action: "skip"` 跳过当前步骤，同级步骤继续
/// - 旧字段 `abort=true` 等价于 `action="stop"`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookAction {
    /// 中止当前步骤及其所属子管线
    Stop,
    /// 跳过当前步骤，同级步骤继续
    Skip,
}

/// Hook 执行结果：允许替换消息列表、系统提示词、用户输入、工具参数等
#[derive(Debug, Deserialize, Default)]
pub struct HookResult {
    /// 替换消息列表（PreLlmRequest）
    #[serde(default)]
    pub messages: Option<Vec<ChatMessage>>,
    /// 替换系统提示词（PreLlmRequest）
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// 替换用户输入文本（PreSendMessage）
    #[serde(default)]
    pub user_input: Option<String>,
    /// 替换 AI 回复文本（PostLlmResponse）
    #[serde(default)]
    pub assistant_output: Option<String>,
    /// 替换工具调用参数（PreToolExecution）
    #[serde(default)]
    pub tool_arguments: Option<String>,
    /// 替换工具执行结果（PostToolExecution）
    #[serde(default)]
    pub tool_result: Option<String>,
    /// 替换工具执行失败原因（PostToolExecutionFailure）
    #[serde(default)]
    pub tool_error: Option<String>,
    /// 追加消息到消息列表末尾（PreLlmRequest）
    #[serde(default)]
    pub inject_messages: Option<Vec<ChatMessage>>,
    /// 审查反馈（Pre*/Stop/PostLlmResponse）：中止时附带反馈文本，触发 LLM 带反馈重试
    #[serde(default)]
    pub retry_feedback: Option<String>,
    /// 注入到模型上下文的额外信息（PreLlmRequest/Stop/PreAutoCompact）：纯文本追加到 system_prompt 末尾
    #[serde(default)]
    pub additional_context: Option<String>,
    /// 展示给用户的系统消息（所有事件：UI 上以 toast/提示形式显示）
    #[serde(default)]
    pub system_message: Option<String>,
    /// 控制流动作：`stop` = 中止当前步骤及其所属子管线，`skip` = 跳过当前步骤（同级继续）
    #[serde(default)]
    pub action: Option<HookAction>,
}

impl HookResult {
    /// 是否请求 stop（中止当前步骤及其所属子管线）
    pub fn is_stop(&self) -> bool {
        self.action == Some(HookAction::Stop)
    }

    /// 是否请求 skip（跳过当前步骤，同级继续）
    pub fn is_skip(&self) -> bool {
        self.action == Some(HookAction::Skip)
    }

    /// 是否请求 stop 或 skip（任何控制流中断）
    pub fn is_halt(&self) -> bool {
        self.is_stop() || self.is_skip()
    }
}

/// Hook 执行的三态结果
///
/// - `Success`：执行成功，可能包含修改
/// - `Retry`：执行失败但还有重试机会
/// - `Err`：执行失败（重试耗尽或不可重试）
#[derive(Debug)]
#[allow(dead_code, clippy::large_enum_variant)]
pub(crate) enum HookOutcome {
    Success(HookResult),
    Retry {
        error: String,
        #[allow(dead_code)]
        attempts_left: u32,
    },
    Err(String),
}

// ========== 辅助常量函数 ==========

pub(crate) fn default_timeout() -> u64 {
    HOOK_DEFAULT_TIMEOUT_SECS
}

pub(crate) fn default_llm_timeout() -> u64 {
    HOOK_DEFAULT_LLM_TIMEOUT_SECS
}
