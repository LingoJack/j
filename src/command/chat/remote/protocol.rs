//! WebSocket 远程控制协议类型定义

use serde::{Deserialize, Serialize};

use super::super::storage::SessionMeta;

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
    /// Ask 工具回答
    #[serde(rename = "ask_response")]
    AskResponse {
        /// 问题文本 → 用户答案（选项 label 或自由文本）
        answers: std::collections::HashMap<String, String>,
    },
    /// 取消当前流式请求
    #[serde(rename = "cancel")]
    Cancel,
    /// 请求全量状态同步
    #[serde(rename = "sync")]
    Sync,
    /// ECDH 密钥交换（客户端发送公钥）
    #[serde(rename = "key_exchange")]
    KeyExchange { client_pk: String },
    /// 心跳 ping
    #[serde(rename = "ping")]
    Ping,
    /// 请求会话列表
    #[serde(rename = "list_sessions")]
    ListSessions,
    /// 切换到指定会话
    #[serde(rename = "switch_session")]
    SwitchSession { session_id: String },
    /// 新建会话
    #[serde(rename = "new_session")]
    NewSession,
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
    /// Ask 工具提问请求
    #[serde(rename = "ask_request")]
    AskRequest { questions: Vec<AskQuestionInfo> },
    /// 工具开始执行
    #[serde(rename = "tool_call")]
    ToolCall { name: String, arguments: String },
    /// 工具执行结果
    #[serde(rename = "tool_result")]
    ToolResult {
        name: String,
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
    /// ECDH 密钥协商：服务端发送公钥
    #[serde(rename = "server_hello")]
    ServerHello { server_pk: String },
    /// ECDH 密钥协商成功确认
    #[serde(rename = "key_exchange_ok")]
    KeyExchangeOk,
    /// 错误消息
    #[serde(rename = "error")]
    Error { message: String },
    /// 会话列表
    #[serde(rename = "session_list")]
    SessionList { sessions: Vec<SessionMeta> },
    /// 会话已切换
    #[serde(rename = "session_switched")]
    SessionSwitched { session_id: String },
}

/// 工具确认信息
#[derive(Debug, Clone, Serialize)]
pub struct ToolConfirmInfo {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub confirm_message: String,
}

/// Ask 工具问题信息
#[derive(Debug, Clone, Serialize)]
pub struct AskQuestionInfo {
    pub question: String,
    pub header: String,
    pub options: Vec<AskOptionInfo>,
    pub multi_select: bool,
}

/// Ask 工具选项信息
#[derive(Debug, Clone, Serialize)]
pub struct AskOptionInfo {
    pub label: String,
    pub description: String,
}

/// 同步工具调用信息
#[derive(Debug, Clone, Serialize)]
pub struct SyncToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// 同步消息（完整版 ChatMessage）
#[derive(Debug, Clone, Serialize)]
pub struct SyncMessage {
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<SyncToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}
