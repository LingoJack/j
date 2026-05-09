use crate::infra::command::CustomCommand;
use crate::infra::skill::Skill;
use crate::storage::{AgentConfig, ChatMessage};
use std::sync::{Arc, Mutex};

// ========== 后端状态 ==========

/// Chat 后端数据状态：对话、配置、模型相关
#[derive(Debug)]
pub struct ChatState {
    /// Agent 配置
    pub agent_config: AgentConfig,
    /// 当前正在流式接收的 AI 回复内容（实时更新）
    pub streaming_content: Arc<Mutex<String>>,
    /// 当前正在流式接收的 AI 思考内容（reasoning_content，实时更新）
    pub streaming_reasoning_content: Arc<Mutex<String>>,
    /// 是否正在等待 AI 回复
    pub is_loading: bool,
    /// 已加载的 skills（用于补全和高亮）
    pub loaded_skills: Vec<Skill>,
    /// 已加载的自定义命令
    pub loaded_commands: Vec<CustomCommand>,
    /// 排队的任务列表（processing期间产生，当前任务完成后自动执行）
    pub queued_tasks: Arc<Mutex<Vec<String>>>,
    /// 用户在 agent loop 期间发送的待处理消息队列
    pub pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// 重试提示（展示在状态栏，重试成功或结束后清空）
    pub retry_hint: Option<String>,
}
