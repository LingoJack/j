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

/// 消息的上下文可见性范围
///
/// 用于控制消息在 UI 显示和各 Agent LLM context 之间的流转。
/// `poll_stream_actions` 根据 `context_scope` 决定是否将消息同步到 `session.messages`。
///
/// 注意：此类型不持久化到 session 文件，仅运行时使用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContextScope {
    /// 默认值：消息同时进入 UI 显示和 Main Agent 的 LLM context
    ///
    /// 用于 Main Agent 的正常对话消息（text reply、tool_call、tool result）。
    #[default]
    UiAndMainAgentContext,

    /// 仅 UI 显示，不进入任何 agent 的 LLM context
    ///
    /// 用于临时性的 UI 状态提示（如流式输出动画、进度条等）。
    Ui,

    /// 消息应进入 SubAgent 的 context
    ///
    /// 标记此消息来自 SubAgent，Main Agent context 也会包含（显式注入）。
    SubagentContext,

    /// 消息应进入 DerivedAgent 的 context
    DerivedAgentContext,

    /// 消息应进入 Teammate 的 context
    ///
    /// 标记此消息来自 Teammate，Main Agent context 也会包含（显式注入）。
    TeammateAgentContext,
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
    /// 上下文可见性范围（不持久化到 session 文件，运行时由推送通道决定）
    #[serde(skip)]
    pub context_scope: ContextScope,
}

impl ChatMessage {
    /// 创建普通文本消息（默认 context_scope 为 UiAndMainAgentContext）
    pub fn text(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            images: None,
            context_scope: ContextScope::default(),
        }
    }

    /// 创建带指定 context_scope 的文本消息
    pub fn text_with_scope(
        role: MessageRole,
        content: impl Into<String>,
        context_scope: ContextScope,
    ) -> Self {
        Self {
            role,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            images: None,
            context_scope,
        }
    }

    /// 创建带图片的 user 消息
    #[allow(dead_code)]
    pub fn with_images(
        role: MessageRole,
        content: impl Into<String>,
        images: Vec<ImageData>,
    ) -> Self {
        Self {
            role,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            images: if images.is_empty() {
                None
            } else {
                Some(images)
            },
            context_scope: ContextScope::default(),
        }
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

impl SessionEvent {
    /// 构造一条带当前时间戳的 Msg 事件
    pub fn msg(message: ChatMessage) -> Self {
        Self::Msg {
            message,
            timestamp_ms: current_millis(),
        }
    }
}
