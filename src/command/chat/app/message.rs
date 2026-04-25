use super::chat_app::ChatApp;
use super::system_prompt::build_system_prompt_fn;
use crate::command::chat::agent::config::{AgentLoopConfig, AgentLoopSharedState};
use crate::command::chat::infra::command;
use crate::command::chat::infra::hook::{HookContext, HookEvent, HookManager};
use crate::command::chat::storage::{ChatMessage, MessageRole};
use crate::util::safe_lock;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

impl ChatApp {
    /// 发送消息（非阻塞，启动后台线程流式接收）
    pub fn send_message(&mut self) {
        let text = self.ui.input_text().trim().to_string();
        if text.is_empty() {
            return;
        }

        self.ui.at_popup_active = false;
        self.ui.file_popup_active = false;
        self.ui.skill_popup_active = false;
        self.ui.clear_input();

        self.send_message_internal(text);
    }

    /// 发送指定文本消息并启动 agent loop
    pub fn send_message_internal(&mut self, text: String) {
        // ★ PreSendMessage hook（同步，需要返回值来决定是否 abort / 修改 text）
        let hook_result = {
            let has_hooks = self
                .hook_manager
                .lock()
                .map(|m| m.has_hooks_for(HookEvent::PreSendMessage))
                .unwrap_or(false);
            if has_hooks {
                let ctx = HookContext {
                    event: HookEvent::PreSendMessage,
                    user_input: Some(text.clone()),
                    messages: Some(self.state.session.messages.clone()),
                    session_id: Some(self.session_id.clone()),
                    cwd: std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| ".".to_string()),
                    ..Default::default()
                };
                if let Ok(manager) = self.hook_manager.lock() {
                    manager.execute(
                        HookEvent::PreSendMessage,
                        ctx,
                        &self.state.agent_config.disabled_hooks,
                    )
                } else {
                    None
                }
            } else {
                None
            }
        };
        let text = if let Some(result) = hook_result {
            if result.is_stop() {
                self.show_toast("消息发送被 hook 拦截", true);
                return;
            }
            result.user_input.unwrap_or(text)
        } else {
            text
        };

        // 展开 @command:name 引用
        let text = command::expand_command_mentions(
            &text,
            &self.state.loaded_commands,
            &self.state.agent_config.disabled_commands,
        );

        // 添加用户消息
        self.state
            .session
            .messages
            .push(ChatMessage::text(MessageRole::User, &text));
        self.ui.auto_scroll = true;
        self.ui.scroll_offset = u16::MAX;

        // ★ PostSendMessage hook（fire-and-forget，不阻塞主线程）
        {
            let has_hooks = self
                .hook_manager
                .lock()
                .map(|m| m.has_hooks_for(HookEvent::PostSendMessage))
                .unwrap_or(false);
            if has_hooks {
                let ctx = HookContext {
                    event: HookEvent::PostSendMessage,
                    user_input: Some(text.clone()),
                    messages: Some(self.state.session.messages.clone()),
                    session_id: Some(self.session_id.clone()),
                    cwd: std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| ".".to_string()),
                    ..Default::default()
                };
                HookManager::execute_fire_and_forget(
                    Arc::clone(&self.hook_manager),
                    HookEvent::PostSendMessage,
                    ctx,
                    self.state.agent_config.disabled_hooks.clone(),
                );
            }
        }

        let provider = match self.active_provider() {
            Some(p) => p.clone(),
            None => {
                self.show_toast("未配置模型提供方，请先编辑配置文件", true);
                return;
            }
        };

        // 同步更新子 Agent 的 provider
        {
            let mut p = safe_lock(&self.derived_agent_provider, "send_message::agent_provider");
            *p = provider.clone();
        }

        // 同步更新子 Agent 的上下文配置快照（供 SubAgent/Teammate 复用 select_messages + micro_compact）
        {
            let mut cfg = safe_lock(
                &self.derived_agent_context_config,
                "send_message::agent_context_config",
            );
            cfg.max_history_messages = self.state.agent_config.max_history_messages;
            cfg.max_context_tokens = self.state.agent_config.max_context_tokens;
            cfg.compact = self.state.agent_config.compact.clone();
        }
        // 同步更新子 Agent 使用的 disabled_hooks 快照（Teammate 走 hook 链时用）
        {
            let mut dh = safe_lock(
                &self.derived_agent_disabled_hooks,
                "send_message::agent_disabled_hooks",
            );
            *dh = self.state.agent_config.disabled_hooks.clone();
        }

        self.state.is_loading = true;
        self.ui.last_rendered_streaming_len = 0;
        self.ui.last_stream_render_time = std::time::Instant::now();
        self.ui.msg_lines_cache = None;
        self.tool_executor.reset();

        let api_messages = self.build_api_messages();

        // 清空待处理用户消息队列
        {
            let mut pending = safe_lock(
                &self.state.pending_user_messages,
                "send_message::pending_user_messages",
            );
            pending.clear();
        }

        // 清空流式内容缓冲
        {
            let mut sc = safe_lock(
                &self.state.streaming_content,
                "send_message::streaming_content",
            );
            sc.clear();
        }

        self.spawn_agent_loop(provider, api_messages);
    }

    /// 此方法清空 inbox 并启动新的 agent loop，让 main agent 响应 teammate 的消息。
    /// 不走 send_message_internal，因为消息已在 session 中，无需重复 push user message。
    pub fn wake_from_teammate_inbox(&mut self) {
        {
            let mut pending =
                safe_lock(&self.state.pending_user_messages, "wake_from_inbox::clear");
            if pending.is_empty() {
                return;
            }
            pending.clear();
        }

        let provider = match self.active_provider() {
            Some(p) => p.clone(),
            None => {
                self.show_toast("未配置模型提供方，请先编辑配置文件", true);
                return;
            }
        };

        {
            let mut p = safe_lock(
                &self.derived_agent_provider,
                "wake_from_inbox::agent_provider",
            );
            *p = provider.clone();
        }

        self.state.is_loading = true;
        self.ui.last_rendered_streaming_len = 0;
        self.ui.last_stream_render_time = std::time::Instant::now();
        self.ui.msg_lines_cache = None;
        self.tool_executor.reset();

        let api_messages = self.build_api_messages();

        {
            let mut sc = safe_lock(
                &self.state.streaming_content,
                "wake_from_inbox::streaming_content",
            );
            sc.clear();
        }

        self.spawn_agent_loop(provider, api_messages);
    }

    /// 公共：构建 system_prompt_fn 并启动 agent loop（消除 send_message_internal 和
    /// wake_from_teammate_inbox 的重复代码）
    fn spawn_agent_loop(
        &mut self,
        provider: crate::command::chat::storage::ModelProvider,
        api_messages: Vec<ChatMessage>,
    ) {
        use super::agent_handle::MainAgentHandle;

        let streaming_content = Arc::clone(&self.state.streaming_content);
        let streaming_reasoning_content = Arc::clone(&self.state.streaming_reasoning_content);
        let tools_enabled = self.state.agent_config.tools_enabled;
        let max_llm_rounds = self.state.agent_config.max_tool_rounds;
        let tools = if tools_enabled {
            self.tool_registry
                .to_llm_tools_filtered(&self.state.agent_config.disabled_tools)
        } else {
            vec![]
        };

        let pending_user_messages = Arc::clone(&self.state.pending_user_messages);
        let background_manager = Arc::clone(&self.background_manager);
        let compact_config = self.state.agent_config.compact.clone();

        let loaded_skills = self.state.loaded_skills.clone();
        let disabled_skills = self.state.agent_config.disabled_skills.clone();
        let disabled_tools = self.state.agent_config.disabled_tools.clone();
        let tool_registry_arc = Arc::clone(&self.tool_registry);
        let system_prompt_fn = build_system_prompt_fn(
            loaded_skills,
            disabled_skills,
            disabled_tools,
            tool_registry_arc,
        );

        let hook_manager_clone = match self.hook_manager.lock() {
            Ok(manager) => manager.clone(),
            Err(_) => HookManager::default(),
        };

        let todo_manager = Arc::clone(&self.todo_manager);

        // 重置共享消息状态
        {
            let mut shared = safe_lock(&self.display_messages, "spawn_agent::clear_display");
            shared.clear();
            let mut shared = safe_lock(&self.context_messages, "spawn_agent::clear_context");
            shared.clear();
        }
        self.display_read_offset = 0;
        self.context_read_offset = 0;

        let agent_config = AgentLoopConfig {
            provider,
            max_llm_rounds,
            compact_config,
            hook_manager: hook_manager_clone,
            disabled_hooks: self.state.agent_config.disabled_hooks.clone(),
            cancel_token: CancellationToken::new(),
        };
        let agent_shared = AgentLoopSharedState {
            streaming_content,
            streaming_reasoning_content,
            pending_user_messages,
            background_manager,
            todo_manager,
            display_messages: Arc::clone(&self.display_messages),
            context_messages: Arc::clone(&self.context_messages),
            estimated_context_tokens: Arc::clone(&self.context_tokens),
            invoked_skills: Arc::clone(&self.invoked_skills),
            session_id: self.session_id.clone(),
            derived_system_prompt: Arc::clone(&self.derived_agent_system_prompt),
        };
        let (handle, tool_result_tx) = MainAgentHandle::spawn(
            agent_config,
            agent_shared,
            api_messages,
            tools,
            system_prompt_fn,
        );

        self.main_agent = Some(handle);
        self.tool_executor.tool_result_tx = Some(tool_result_tx);
    }
}
