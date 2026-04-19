use super::super::hook::HookManager;
use super::super::storage::{ChatMessage, ModelProvider};
use super::super::tools::background::BackgroundManager;
use super::super::tools::todo::TodoManager;
use super::compact::{CompactConfig, InvokedSkillsMap};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

/// Agent loop 的静态配置（不含每次请求独有的消息/通道）
#[derive(Debug)]
pub struct AgentLoopConfig {
    /// 模型提供商配置
    pub provider: ModelProvider,
    /// 最大工具调用轮次
    pub max_tool_rounds: usize,
    /// Context compact 配置
    pub compact_config: CompactConfig,
    /// Hook 管理器
    pub hook_manager: HookManager,
    /// 取消令牌
    pub cancel_token: CancellationToken,
}

/// Agent loop 的共享状态（Arc 引用，跨线程共享）
#[derive(Debug)]
pub struct AgentSharedState {
    /// 流式内容缓冲区（agent 写入，UI 读取）
    pub streaming_content: Arc<Mutex<String>>,
    /// 用户在 agent loop 期间追加的消息队列
    pub pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// 后台任务管理器（由内置 PreLlmRequest hook 通过 Arc 引用使用）
    #[allow(dead_code)]
    pub background_manager: Arc<BackgroundManager>,
    /// 待办管理器
    pub todo_manager: Arc<TodoManager>,
    /// Agent/Teammate → UI 显示通道（agent 写入，UI 读取）
    pub ui_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// Agent 实际使用的上下文 token 估算值（agent 每轮更新，UI 读取显示）
    pub context_tokens: Arc<Mutex<usize>>,
    /// 会话内已调用技能追踪（LoadSkill 执行时记录，auto_compact 后恢复）
    pub invoked_skills: InvokedSkillsMap,
    /// 当前会话 ID（用于 auto_compact 写入 session 级 .transcripts/）
    pub session_id: String,
}
