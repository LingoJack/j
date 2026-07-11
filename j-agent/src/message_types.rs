use crate::chat_error::ChatError;
use crate::storage::ToolCallItem;
use crate::tools::ImageData;
use std::sync::mpsc;

// ========== Plan 审批决策类型 ==========

/// Plan 审批决策结果
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PlanDecision {
    /// 无 Plan 决策（普通工具结果）
    #[default]
    None,
    /// 批准计划，保留当前上下文
    Approve,
    /// 批准计划并清空探索上下文
    ApproveAndClearContext,
    /// 驳回计划，留在 Plan Mode 修改
    Reject,
}

// ========== 消息类型（跨线程通信）==========

/// 工具执行状态（用于 ToolResult 消息）
#[derive(Debug, Clone, PartialEq)]
pub enum ToolResultStatus {
    /// 正常执行完成
    Executed,
    /// 执行失败
    Failed,
    /// 用户拒绝
    Rejected,
    /// 自动批准执行
    AutoApproved,
}

/// 工具执行结果消息（用于 UI 展示真实工具输出）
#[derive(Debug, Clone)]
pub struct ToolResultStreamMsg {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: String,
    pub is_error: bool,
    pub images: Vec<ImageData>,
    pub status: ToolResultStatus,
}

/// 后台线程发送给 TUI 的消息类型
pub enum StreamMsg {
    /// 收到一个流式文本块
    Chunk,
    /// LLM 请求执行工具（附带完整工具调用列表）
    ToolCallRequest(Vec<ToolCallItem>),
    /// 工具执行结果（UI 可展示真实输出）
    ToolResult(ToolResultStreamMsg),
    /// 流式响应完成
    Done,
    /// 发生错误
    Error(ChatError),
    /// 用户主动取消
    Cancelled,
    /// 正在重试（网络波动等可恢复错误）
    Retrying {
        /// 当前第几次重试（从 1 开始）
        attempt: u32,
        /// 最大重试次数
        max_attempts: u32,
        /// 本次等待毫秒数
        delay_ms: u64,
        /// 原始错误简要描述
        error: String,
    },
    /// 上下文正在压缩（auto_compact 执行中）
    Compacting,
    /// 上下文压缩完成（携带压缩前的消息数量，供 UI 展示）
    Compacted {
        /// 压缩前的消息数量
        messages_before: usize,
    },
}

/// 工具执行状态
#[derive(Debug)]
pub enum ToolExecStatus {
    /// 等待用户确认
    PendingConfirm,
    /// 执行中
    Executing,
    /// 完成（摘要）
    #[allow(dead_code)]
    Done(String),
    /// 用户拒绝
    Rejected,
    /// 执行失败
    Failed(String),
}

/// 工具调用执行状态（运行时，不序列化）
#[derive(Debug)]
pub struct ToolCallStatus {
    pub tool_call_id: String,
    pub tool_name: String,
    pub arguments: String,
    pub confirm_message: String,
    pub status: ToolExecStatus,
    /// 工具调用的简短描述（仅 Bash 工具从 arguments.description 提取）
    pub tool_description: Option<String>,
}

/// 主线程 → 后台线程的工具结果消息
#[derive(Debug)]
pub struct ToolResultMsg {
    pub tool_call_id: String,
    pub result: String,
    #[allow(dead_code)]
    pub is_error: bool,
    /// 工具返回的图片数据（用于多模态模型）
    pub images: Vec<ImageData>,
    /// Plan 审批决策（仅 ExitPlanMode 工具会设置非 None 值）
    pub plan_decision: PlanDecision,
}

/// Worker 线程完成后写入共享状态，供 UI poll 更新显示
#[derive(Debug)]
pub struct CompletedToolResult {
    pub tool_call_id: String,
    pub summary: String,
    pub is_error: bool,
}

/// ask 工具选项
#[derive(Debug, Clone)]
pub struct AskOption {
    pub label: String,
    pub description: String,
}

/// ask 工具单个问题
#[derive(Debug, Clone)]
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
#[derive(Debug)]
pub struct AskRequest {
    pub questions: Vec<AskQuestion>,
    pub response_tx: mpsc::Sender<String>,
}
