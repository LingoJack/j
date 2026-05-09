//! WebSocket 远程控制协议类型定义

use serde::{Deserialize, Serialize};

use crate::command::chat::storage::SessionMeta;

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
    /// 选择模型（按索引）
    #[serde(rename = "select_model")]
    SelectModel { index: usize },
    /// 选择主题（按索引）
    #[serde(rename = "select_theme")]
    SelectTheme { index: usize },
    /// 请求配置数据
    #[serde(rename = "request_config")]
    RequestConfig { tab: String },
    /// 配置编辑提交
    #[serde(rename = "config_edit_submit")]
    ConfigEditSubmit { value: String },
    /// 配置项切换（toggle enabled/disabled）
    #[serde(rename = "config_toggle")]
    ConfigToggle { index: usize },
    /// 开始归档确认
    #[serde(rename = "start_archive")]
    StartArchive,
    /// 使用默认名称归档
    #[serde(rename = "archive_with_default")]
    ArchiveWithDefault,
    /// 使用自定义名称归档
    #[serde(rename = "archive_with_custom")]
    ArchiveWithCustom { name: String },
    /// 不归档，直接清空会话
    #[serde(rename = "clear_session")]
    ClearSession,
    /// 请求归档列表
    #[serde(rename = "start_archive_list")]
    StartArchiveList,
    /// 恢复归档（按索引）
    #[serde(rename = "restore_archive")]
    RestoreArchive { index: usize },
    /// 删除归档（按索引）
    #[serde(rename = "delete_archive")]
    DeleteArchive { index: usize },
    /// 删除会话（按索引，在会话列表中）
    #[serde(rename = "delete_session")]
    DeleteSession { index: usize },
    /// Agent 权限确认
    #[serde(rename = "agent_perm_confirm")]
    AgentPermConfirm { approve: bool },
    /// Plan 审批
    #[serde(rename = "plan_approval")]
    PlanApproval {
        approve: bool,
        #[serde(default)]
        content: Option<String>,
    },
    /// 切换自动审批
    #[serde(rename = "toggle_auto_approve")]
    ToggleAutoApprove,
    // ── 文件操作 ──
    /// 列出目录内容
    #[serde(rename = "file_list")]
    FileList { path: String },
    /// 读取文件内容
    #[serde(rename = "file_read")]
    FileRead { path: String },
    /// 写入文件内容
    #[serde(rename = "file_write")]
    FileWrite { path: String, content: String },
    // ── 终端操作 ──
    /// 执行终端命令
    #[serde(rename = "terminal_exec")]
    TerminalExec { command: String },
    /// 终端中断 (Ctrl-C)
    #[serde(rename = "terminal_interrupt")]
    TerminalInterrupt,
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
    ToolCall {
        id: String,
        name: String,
        arguments: String,
    },
    /// 工具执行结果
    #[serde(rename = "tool_result")]
    ToolResult {
        id: String,
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
        #[serde(default)]
        context_tokens: usize,
        #[serde(default)]
        message_count: usize,
        #[serde(default)]
        auto_approve: bool,
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
    /// 模型列表
    #[serde(rename = "model_list")]
    ModelList {
        models: Vec<ModelInfo>,
        active_index: usize,
    },
    /// 主题列表
    #[serde(rename = "theme_list")]
    ThemeList {
        themes: Vec<ThemeInfo>,
        active_index: usize,
    },
    /// 配置数据
    #[serde(rename = "config_data")]
    ConfigData {
        tab: String,
        fields: Vec<ConfigField>,
    },
    /// 归档列表
    #[serde(rename = "archive_list")]
    ArchiveList { archives: Vec<ArchiveInfo> },
    /// Agent 权限确认请求
    #[serde(rename = "agent_perm_request")]
    AgentPermRequest {
        agent_name: String,
        tool_name: String,
        arguments: String,
    },
    /// Plan 审批请求
    #[serde(rename = "plan_approval_request")]
    PlanApprovalRequest {
        agent_name: String,
        plan_summary: String,
    },
    // ── 文件操作 ──
    /// 目录列表结果
    #[serde(rename = "file_list_result")]
    FileListResult {
        path: String,
        entries: Vec<FileEntry>,
    },
    /// 文件内容结果
    #[serde(rename = "file_read_result")]
    FileReadResult {
        path: String,
        content: String,
        #[serde(default)]
        error: Option<String>,
    },
    /// 文件写入结果
    #[serde(rename = "file_write_result")]
    FileWriteResult {
        path: String,
        success: bool,
        #[serde(default)]
        error: Option<String>,
    },
    // ── 终端操作 ──
    /// 终端命令输出
    #[serde(rename = "terminal_output")]
    TerminalOutput {
        output: String,
        exit_code: Option<i32>,
    },
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

/// 模型信息
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub name: String,
    pub model: String,
    pub provider: String,
    pub supports_vision: bool,
}

/// 主题信息
#[derive(Debug, Clone, Serialize)]
pub struct ThemeInfo {
    pub name: String,
    pub display_name: String,
}

/// 配置字段
#[derive(Debug, Clone, Serialize)]
pub struct ConfigField {
    pub key: String,
    pub label: String,
    pub value: String,
    /// "text" | "bool" | "select" | "action"
    pub field_type: String,
    pub editable: bool,
    /// select 类型的选项列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

/// 归档信息
#[derive(Debug, Clone, Serialize)]
pub struct ArchiveInfo {
    pub name: String,
    pub created_at: String,
    pub message_count: usize,
}

/// 文件条目
#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
}
