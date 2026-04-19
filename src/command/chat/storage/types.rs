use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub role: String, // "user" | "assistant" | "system" | "tool"
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
}

impl ChatMessage {
    /// 创建普通文本消息
    pub fn text(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            images: None,
        }
    }

    /// 创建带图片的 user 消息
    #[allow(dead_code)]
    pub fn with_images(
        role: impl Into<String>,
        content: impl Into<String>,
        images: Vec<ImageData>,
    ) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            images: if images.is_empty() {
                None
            } else {
                Some(images)
            },
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
