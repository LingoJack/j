use crate::command::chat::command::CustomCommand;
use crate::command::chat::skill::Skill;
use crate::command::chat::storage::{AgentConfig, ChatMessage, ChatSession};
use std::sync::{Arc, Mutex};

// ========== 后端状态 ==========

/// Chat 后端数据状态：对话、配置、模型相关
pub struct ChatState {
    /// Agent 配置
    pub agent_config: AgentConfig,
    /// 当前对话会话
    pub session: ChatSession,
    /// 当前正在流式接收的 AI 回复内容（实时更新）
    pub streaming_content: Arc<Mutex<String>>,
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
}
