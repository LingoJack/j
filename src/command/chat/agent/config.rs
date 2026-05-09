use crate::command::chat::context::compact::{CompactConfig, InvokedSkillsMap};
use crate::command::chat::infra::hook::HookManager;
use crate::command::chat::storage::{ChatMessage, ModelProvider};
use crate::command::chat::tools::background::BackgroundManager;
use crate::command::chat::tools::definition::ToolRegistry;
use crate::command::chat::tools::derived_shared::SubAgentMetrics;
use crate::command::chat::tools::todo::TodoManager;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

/// Agent loop 的静态配置（不含每次请求独有的消息/通道）
#[derive(Debug)]
pub struct AgentLoopConfig {
    /// 模型提供商配置
    pub provider: ModelProvider,
    /// 最大 LLM 调用轮次
    pub max_llm_rounds: usize,
    /// Context compact 配置
    pub compact_config: CompactConfig,
    /// Hook 管理器
    pub hook_manager: HookManager,
    /// 被禁用的 hook 标识列表
    pub disabled_hooks: Vec<String>,
    /// 取消令牌
    pub cancel_token: CancellationToken,
}

/// Agent loop 的共享状态（Arc 引用，跨线程共享）
#[derive(Debug)]
pub struct AgentLoopSharedState {
    /// 流式内容缓冲区（agent 写入，UI 读取）
    pub streaming_content: Arc<Mutex<String>>,
    /// 流式思考内容缓冲区（reasoning_content，agent 写入，UI 读取）
    pub streaming_reasoning_content: Arc<Mutex<String>>,
    /// 用户在 agent loop 期间追加的消息队列
    pub pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// 后台任务管理器（由内置 PreLlmRequest hook 通过 Arc 引用使用）
    #[allow(dead_code)]
    pub background_manager: Arc<BackgroundManager>,
    /// 待办管理器
    pub todo_manager: Arc<TodoManager>,
    /// Agent/Teammate → UI 显示通道（仅用于 UI 渲染，不作为 LLM context 数据源）
    pub display_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// Agent/Teammate → LLM context 同步通道（persist_new_messages 直接从此持久化）
    pub context_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// Agent 实际使用的上下文 token 估算值（agent 每轮更新，UI 读取显示）
    pub estimated_context_tokens: Arc<Mutex<usize>>,
    /// 会话内已调用技能追踪（LoadSkill 执行时记录，auto_compact 后恢复）
    pub invoked_skills: InvokedSkillsMap,
    /// 当前会话 ID（用于 auto_compact 写入 session 级 .transcripts/）
    pub session_id: String,
    /// 子 Agent 共用 system_prompt（每轮构建后更新，供 AgentTool / TeammateTool 读取）
    pub derived_system_prompt: Arc<Mutex<Option<String>>>,
    /// 工具注册表（用于每轮动态获取可用工具）
    pub tool_registry: Arc<ToolRegistry>,
    /// 用户禁用的工具列表
    pub disabled_tools: Vec<String>,
    /// 延迟加载的工具列表（LoadTool 加载后才可用）
    pub deferred_tools: Arc<Mutex<Vec<String>>>,
    /// 工具是否启用
    pub tools_enabled: bool,
    /// 子 Agent metrics 累加器（SubAgent/Teammate 的 LLM/tool 统计）
    /// Main agent loop 结束时读取并合并到 `SessionMetrics`
    pub sub_agent_metrics: Arc<Mutex<SubAgentMetrics>>,
}
