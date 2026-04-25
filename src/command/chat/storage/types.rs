use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// 消息角色（API 层 + 存储层共用）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
    System,
}

impl MessageRole {
    /// 返回对应的字符串表示（用于日志、外部协议等）
    pub const fn as_str(self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
            MessageRole::System => "system",
        }
    }
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 显示类型（渲染层专用，面向 UI 语义细分）
///
/// 将 `role` + `tool_calls` 组合映射为精确的渲染语义，
/// 渲染层只需 `match msg.display_type()` 即可，无需二次判断。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayType {
    /// 用户消息（右对齐气泡）
    User,
    /// AI 文本回复（左对齐气泡 + Markdown）
    AssistantText,
    /// 工具调用请求（折叠/展开参数）
    ToolCallRequest,
    /// 工具执行结果（带状态图标 + 摘要）
    ToolResult,
    /// 系统消息（灰色缩进）
    System,
}

/// 单次工具调用请求（序列化到历史记录）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallItem {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// 图片数据（用于多模态消息，序列化时跳过以节省存储空间）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    /// base64 编码的图片数据
    pub base64: String,
    /// MIME 类型（如 "image/png", "image/jpeg"）
    pub media_type: String,
}

/// 对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    /// 消息内容（tool_call 类消息可为空）
    #[serde(default)]
    pub content: String,
    /// LLM 发起的工具调用列表（仅 assistant 角色且有 tool_calls 时非 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallItem>>,
    /// 工具执行结果对应的 tool_call_id（仅 tool 角色时非 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// 图片数据（用于多模态 user message，不持久化到 session 文件）
    #[serde(skip)]
    pub images: Option<Vec<ImageData>>,
    /// LLM 思考内容（thinking mode 返回的 reasoning_content，需传回下一轮请求）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// 消息发送者名称（如 `Teammate@Frontend`、`SubAgent@search`）。
    /// 仅由 teammate/subagent 消息设置；主 agent 消息为 None。
    /// 不持久化到 session 文件，仅用于运行时 UI 渲染。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender_name: Option<String>,
}

impl ChatMessage {
    /// 创建普通文本消息
    pub fn text(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            images: None,
            reasoning_content: None,
            sender_name: None,
        }
    }

    /// 设置消息发送者名称（Builder 模式）。
    ///
    /// 用于 teammate/subagent 消息标记发送者，UI 渲染层据此显示气泡标签。
    pub fn with_sender(mut self, name: impl Into<String>) -> Self {
        self.sender_name = Some(name.into());
        self
    }

    /// 推断显示类型（渲染层入口）
    ///
    /// 将 `role` + `tool_calls` 组合映射为精确的 `DisplayType`，
    /// 渲染层无需再做 `role == "assistant" && tool_calls.is_some()` 的判断。
    pub fn display_type(&self) -> DisplayType {
        match self.role {
            MessageRole::User => DisplayType::User,
            MessageRole::System => DisplayType::System,
            MessageRole::Assistant => {
                if self.tool_calls.is_some() {
                    DisplayType::ToolCallRequest
                } else {
                    DisplayType::AssistantText
                }
            }
            MessageRole::Tool => DisplayType::ToolResult,
        }
    }
}

/// 对话会话
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatSession {
    pub messages: Vec<ChatMessage>,
}

pub(super) fn is_zero_u64(v: &u64) -> bool {
    *v == 0
}

/// 当前时刻（epoch milliseconds）
pub fn current_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Session JSONL 事件类型（每行一个事件，append-only）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEvent {
    /// 新增一条消息
    Msg {
        #[serde(flatten)]
        message: ChatMessage,
        /// 消息产生时刻（epoch milliseconds）；老数据反序列化为 0。
        #[serde(default, skip_serializing_if = "is_zero_u64")]
        timestamp_ms: u64,
    },
    /// 对话清空
    Clear,
    /// 归档还原（messages 为还原后的完整消息列表）
    Restore { messages: Vec<ChatMessage> },
}

/// Session 操作审计记录，追加到 sessions/<id>/ops.jsonl
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionOp {
    /// 操作类型
    pub op: SessionOpKind,
    /// 时间戳（epoch ms）
    pub timestamp_ms: u64,
    /// 是否执行失败
    pub is_error: bool,
}

/// 操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionOpKind {
    /// 文件编辑（Edit 工具）
    Edit {
        /// 被编辑的文件路径
        path: String,
    },
    /// 文件写入（Write 工具）
    Write {
        /// 被写入的文件路径
        path: String,
    },
    /// Shell 命令执行（Bash 工具）
    Bash {
        /// 执行的命令
        command: String,
    },
}

impl SessionEvent {
    /// 构造一条带当前时间戳的 Msg 事件
    pub fn msg(message: ChatMessage) -> Self {
        Self::Msg {
            message,
            timestamp_ms: current_millis(),
        }
    }
}
