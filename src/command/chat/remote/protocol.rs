//! WebSocket 远程控制协议类型定义

use serde::{Deserialize, Serialize};

/// 客户端 → 服务端 消息
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum WsInbound {
    /// 发送聊天消息
    #[serde(rename = "send_message")]
    SendMessage { content: String },
    /// 工具确认（allow / allow_always / reject / reject_with_reason）
    #[serde(rename = "tool_confirm")]
    ToolConfirm {
        action: String,
        #[serde(default)]
        reason: Option<String>,
    },
    /// 取消当前流式请求
    #[serde(rename = "cancel")]
    Cancel,
    /// 请求全量状态同步
    #[serde(rename = "sync")]
    Sync,
    /// 心跳 ping
    #[serde(rename = "ping")]
    Ping,
}

/// 服务端 → 客户端 消息
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum WsOutbound {
    /// 流式文本块
    #[serde(rename = "stream_chunk")]
    StreamChunk { content: String },
    /// 完整消息（流结束后或用户消息）
    #[serde(rename = "message")]
    Message { role: String, content: String },
    /// 工具确认请求
    #[serde(rename = "tool_confirm_request")]
    ToolConfirmRequest { tools: Vec<ToolConfirmInfo> },
    /// 工具执行结果
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_call_id: String,
        output: String,
        is_error: bool,
    },
    /// 状态变化
    #[serde(rename = "status")]
    Status { state: String },
    /// 全量状态同步
    #[serde(rename = "session_sync")]
    SessionSync {
        messages: Vec<SyncMessage>,
        status: String,
        model: String,
    },
    /// 心跳 pong
    #[serde(rename = "pong")]
    Pong,
    /// 错误消息
    #[serde(rename = "error")]
    Error { message: String },
}

/// 工具确认信息
#[derive(Debug, Clone, Serialize)]
pub struct ToolConfirmInfo {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub confirm_message: String,
}

/// 同步消息（简化版 ChatMessage）
#[derive(Debug, Clone, Serialize)]
pub struct SyncMessage {
    pub role: String,
    pub content: String,
}
