use crate::command::chat::storage::ToolCallItem;
use std::sync::mpsc;

// ========== 消息类型（跨线程通信）==========

/// 后台线程发送给 TUI 的消息类型
pub enum StreamMsg {
    /// 收到一个流式文本块
    Chunk,
    /// LLM 请求执行工具（附带完整工具调用列表）
    ToolCallRequest(Vec<ToolCallItem>),
    /// 流式响应完成
    Done,
    /// 发生错误
    Error(String),
    /// 用户主动取消
    Cancelled,
}

/// 工具执行状态
#[allow(dead_code)]
pub enum ToolExecStatus {
    /// 等待用户确认
    PendingConfirm,
    /// 执行中
    Executing,
    /// 完成（摘要）
    Done(String),
    /// 用户拒绝
    Rejected,
    /// 执行失败
    Failed(String),
}

/// 工具调用执行状态（运行时，不序列化）
pub struct ToolCallStatus {
    pub tool_call_id: String,
    pub tool_name: String,
    pub arguments: String,
    pub confirm_message: String,
    pub status: ToolExecStatus,
}

/// 主线程 → 后台线程的工具结果消息
pub struct ToolResultMsg {
    pub tool_call_id: String,
    pub result: String,
    #[allow(dead_code)]
    pub is_error: bool,
    /// 工具返回的图片数据（用于多模态模型）
    pub images: Vec<crate::command::chat::tools::ImageData>,
}

/// Worker 线程完成后写入共享状态，供 UI poll 更新显示
pub struct CompletedToolResult {
    pub tool_call_id: String,
    pub summary: String,
    pub is_error: bool,
}

/// ask 工具选项
#[derive(Clone)]
pub struct AskOption {
    pub label: String,
    pub description: String,
}

/// ask 工具单个问题
#[derive(Clone)]
pub struct AskQuestion {
    pub question: String,
    pub header: String,
    pub options: Vec<AskOption>,
    pub multi_select: bool,
}

/// ask 工具单题答案
#[derive(Clone)]
pub enum AskAnswer {
    /// 选中的选项索引（单选/多选）
    Selected(Vec<usize>),
    /// 自由输入文本
    FreeText(String),
}

/// ask 工具 → 主线程的请求消息
pub struct AskRequest {
    pub questions: Vec<AskQuestion>,
    pub response_tx: mpsc::Sender<String>,
}
