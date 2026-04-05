use super::action::{Action, CursorDirection};
use super::agent_handle::AgentHandle;
use super::chat_state::ChatState;
use super::tool_executor::ToolExecutor;
use super::types::{
    AskAnswer, AskRequest, StreamMsg, ToolCallStatus, ToolExecStatus, ToolResultMsg,
};
use super::ui_state::{ChatMode, ConfigTab, MsgLinesCache, UIState};
use crate::command::chat::agent_config::{AgentLoopConfig, AgentSharedState};
use crate::command::chat::command;
use crate::command::chat::constants::{INPUT_BUFFER_MAX_LEN, ROLE_ASSISTANT, ROLE_TOOL, ROLE_USER};
use crate::command::chat::hook::{HookContext, HookEvent, HookManager};
use crate::command::chat::markdown::image_cache::ImageCache;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::sandbox::Sandbox;
use crate::command::chat::skill::{self, skills_dir};
use crate::command::chat::storage::{
    ChatMessage, ChatSession, ModelProvider, SessionEvent, append_session_event, delete_session,
    generate_session_id, list_sessions, load_agent_config, load_session, memory_path,
    save_agent_config, save_memory, save_soul, save_system_prompt, soul_path, system_prompt_path,
};
use crate::command::chat::theme::Theme;
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::background::BackgroundManager;
use crate::constants::{CONFIG_FIELDS, TOAST_DURATION_SECS};
use crate::util::log::write_info_log;
use crate::util::safe_lock;
use ratatui::widgets::ListState;
use std::sync::{Arc, Mutex, mpsc};
use tokio_util::sync::CancellationToken;

// ========== 主应用结构体 ==========

/// TUI 应用状态（组合结构）
pub struct ChatApp {
    /// 前端 UI 状态
    pub ui: UIState,
    /// 后端数据状态
    pub state: ChatState,
    /// 工具执行器
    pub tool_executor: ToolExecutor,
    /// Agent 生命周期句柄（存在时表示有进行中的请求）
    pub agent: Option<AgentHandle>,
    /// 工具注册表
    pub tool_registry: Arc<ToolRegistry>,
    /// .jcli/ 权限配置
    pub jcli_config: Arc<JcliConfig>,
    /// 后台任务管理器
    pub background_manager: Arc<BackgroundManager>,
    /// Todo 管理器
    pub todo_manager: Arc<crate::command::chat::tools::todo::TodoManager>,
    /// ask 工具响应发送通道
    pub ask_response_tx: Option<mpsc::Sender<String>>,
    /// ask 工具请求接收通道
    pub ask_request_rx: Option<mpsc::Receiver<AskRequest>>,
    /// Hook 管理器
    pub hook_manager: Arc<Mutex<HookManager>>,
    /// 安全沙箱（限制工具操作路径范围）
    pub sandbox: Sandbox,
    /// 本次会话 ID（启动时生成，对应 sessions/{id}.jsonl）
    pub session_id: String,
    /// 已持久化到 JSONL 的消息数量（用于增量追加）
    pub last_persisted_len: usize,
    /// 远程控制 WebSocket 桥接器
    pub ws_bridge: Option<crate::command::chat::remote::bridge::WsBridge>,
    /// 远程客户端是否已连接
    pub remote_connected: bool,
    /// AgentTool 的 provider 共享引用（每次发送请求前更新）
    #[allow(dead_code)]
    pub agent_tool_provider: Arc<Mutex<ModelProvider>>,
    /// AgentTool 的 system_prompt 共享引用（每次发送请求前更新）
    #[allow(dead_code)]
    pub agent_tool_system_prompt: Arc<Mutex<Option<String>>>,
    /// Agent 与 UI 共享的消息列表（agent 线程 push，UI 线程 poll len 变化）
    pub shared_agent_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// UI 侧已读取到的位置（用于增量检测）
    pub shared_messages_read_cursor: usize,
}

/// 所有字段数 = provider 字段 + 全局字段
/// 根据当前 tab 计算字段总数
pub fn config_tab_field_count(app: &ChatApp) -> usize {
    use crate::constants::CONFIG_GLOBAL_FIELDS_TAB;
    match app.ui.config_tab {
        ConfigTab::Model => CONFIG_FIELDS.len(),
        ConfigTab::Global => CONFIG_GLOBAL_FIELDS_TAB.len(),
        ConfigTab::Tools => app.tool_registry.tool_names().len(),
        ConfigTab::Skills => app.state.loaded_skills.len(),
        ConfigTab::Commands => app.state.loaded_commands.len(),
        ConfigTab::Hooks => 0,
        ConfigTab::Session => app.ui.session_list.len(),
        ConfigTab::Archive => app.ui.archives.len(),
    }
}

impl ChatApp {
    pub fn new(session_id: String) -> Self {
        let agent_config = load_agent_config();
        // 首次运行：各数据文件不存在时写入默认内容
        if !system_prompt_path().exists() {
            let _ = save_system_prompt(&crate::assets::default_system_prompt());
        }
        if !memory_path().exists() {
            let _ = save_memory(&crate::assets::default_memory());
        }
        if !soul_path().exists() {
            let _ = save_soul(&crate::assets::default_soul());
        }
        // 安装预设 skills
        if let Err(e) = crate::assets::install_default_skills(&skill::skills_dir()) {
            crate::util::log::write_error_log(
                "[ChatApp::new]",
                &format!("安装预设 skills 失败: {}", e),
            );
        }

        // 每次启动创建全新会话（session_id 由调用方生成）
        let session = ChatSession::default();
        let mut model_list_state = ListState::default();
        if !agent_config.providers.is_empty() {
            model_list_state.select(Some(agent_config.active_index));
        }
        let theme = Theme::from_name(&agent_config.theme);
        let loaded_skills = skill::load_all_skills();
        let loaded_commands = command::load_all_commands();
        let (ask_req_tx, ask_req_rx) = mpsc::channel::<AskRequest>();
        let queued_tasks: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let background_manager = Arc::new(BackgroundManager::new());
        let task_manager = Arc::new(crate::command::chat::tools::task::TaskManager::new());
        let hook_manager = Arc::new(Mutex::new(HookManager::load()));
        let mut tool_registry = ToolRegistry::new(
            loaded_skills.clone(),
            ask_req_tx,
            Arc::clone(&background_manager),
            Arc::clone(&task_manager),
            Arc::clone(&hook_manager),
        );
        let todo_manager = Arc::clone(&tool_registry.todo_manager);

        // AgentTool 需要 provider 和 system_prompt 的共享引用（运行时动态获取）
        let default_provider = agent_config
            .providers
            .get(agent_config.active_index)
            .cloned()
            .unwrap_or_else(|| ModelProvider {
                name: String::new(),
                api_base: String::new(),
                api_key: String::new(),
                model: String::new(),
                supports_vision: false,
            });
        let agent_provider: Arc<Mutex<ModelProvider>> = Arc::new(Mutex::new(default_provider));
        let agent_system_prompt: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let disabled_tools_arc = Arc::new(agent_config.disabled_tools.clone());
        tool_registry.register(Box::new(crate::command::chat::tools::agent::AgentTool {
            background_manager: Arc::clone(&background_manager),
            provider: Arc::clone(&agent_provider),
            system_prompt: Arc::clone(&agent_system_prompt),
            jcli_config: Arc::new(JcliConfig::load()),
            compact_config: agent_config.compact.clone(),
            hook_manager: Arc::clone(&hook_manager),
            task_manager: Arc::clone(&task_manager),
            todo_manager: Arc::clone(&todo_manager),
            disabled_tools: Arc::clone(&disabled_tools_arc),
        }));
        let tool_registry = Arc::new(tool_registry);
        let jcli_config = Arc::new(JcliConfig::load());

        let new_app = Self {
            ui: UIState {
                input: String::new(),
                cursor_pos: 0,
                mode: ChatMode::Chat,
                scroll_offset: u16::MAX,
                auto_scroll: true,
                browse_msg_index: 0,
                browse_scroll_offset: 0,
                model_list_state,
                toast: None,
                msg_lines_cache: None,
                cached_mention_ranges: None,
                last_rendered_streaming_len: 0,
                last_stream_render_time: std::time::Instant::now(),
                config_provider_idx: 0,
                config_field_idx: 0,
                config_editing: false,
                config_edit_buf: String::new(),
                config_edit_cursor: 0,
                theme,
                archives: Vec::new(),
                archive_list_index: 0,
                archive_default_name: String::new(),
                archive_custom_name: String::new(),
                archive_editing_name: false,
                archive_edit_cursor: 0,
                restore_confirm_needed: false,
                at_popup_active: false,
                at_popup_filter: String::new(),
                at_popup_start_pos: 0,
                at_popup_selected: 0,
                file_popup_active: false,
                file_popup_start_pos: 0,
                file_popup_filter: String::new(),
                file_popup_selected: 0,
                skill_popup_active: false,
                skill_popup_start_pos: 0,
                skill_popup_filter: String::new(),
                skill_popup_selected: 0,
                command_popup_active: false,
                command_popup_start_pos: 0,
                command_popup_filter: String::new(),
                command_popup_selected: 0,
                tool_interact_selected: 0,
                tool_interact_typing: false,
                tool_interact_input: String::new(),
                tool_interact_cursor: 0,
                tool_ask_mode: false,
                tool_ask_questions: Vec::new(),
                tool_ask_current_idx: 0,
                tool_ask_answers: Vec::new(),
                tool_ask_selections: Vec::new(),
                tool_ask_cursor: 0,
                pending_system_prompt_edit: false,
                pending_style_edit: false,
                image_cache: Arc::new(Mutex::new(ImageCache::new())),
                expand_tools: false,
                plan_mode_active: false,
                config_scroll_offset: 0,
                config_tab: ConfigTab::Model,
                session_list: Vec::new(),
                session_list_index: 0,
                session_restore_confirm: false,
            },
            state: ChatState {
                agent_config,
                session,
                streaming_content: Arc::new(Mutex::new(String::new())),
                is_loading: false,
                loaded_skills,
                loaded_commands,
                queued_tasks,
                pending_user_messages: Arc::new(Mutex::new(Vec::new())),
            },
            tool_executor: ToolExecutor::new(),
            agent: None,
            tool_registry,
            jcli_config,
            background_manager,
            todo_manager,
            ask_response_tx: None,
            ask_request_rx: Some(ask_req_rx),
            hook_manager: Arc::clone(&hook_manager),
            sandbox: Sandbox::new(),
            session_id,
            last_persisted_len: 0,
            ws_bridge: None,
            remote_connected: false,
            agent_tool_provider: agent_provider,
            agent_tool_system_prompt: agent_system_prompt,
            shared_agent_messages: Arc::new(Mutex::new(Vec::new())),
            shared_messages_read_cursor: 0,
        };

        // 执行 SessionStart hook（fire-and-forget，不阻塞启动）
        {
            let should_fire = new_app
                .hook_manager
                .lock()
                .map(|m| m.has_hooks_for(HookEvent::SessionStart))
                .unwrap_or(false);
            if should_fire {
                let ctx = HookContext {
                    event: HookEvent::SessionStart,
                    messages: Some(new_app.state.session.messages.clone()),
                    cwd: std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| ".".to_string()),
                    ..Default::default()
                };
                HookManager::execute_fire_and_forget(
                    Arc::clone(&new_app.hook_manager),
                    HookEvent::SessionStart,
                    ctx,
                );
            }
        }

        new_app
    }

    // ========== 中央 update() reducer ==========

    /// Redux-like reducer：集中处理所有 Action，分发到具体方法
    ///
    /// 该方法是 unidirectional data flow 的核心：
    /// 1. 接收 Action（用户输入或系统事件）
    /// 2. 根据 Action 类型和当前状态执行相应操作
    /// 3. 修改 self.state、self.ui、self.tool_executor 等
    /// 4. 不再直接在 handler 中修改状态
    ///
    /// 初始阶段：委托到现有的具体方法，维持兼容性
    /// 后续阶段：逐步将逻辑内联到 update() 中以优化
    pub fn update(&mut self, action: Action) {
        match action {
            // ========== Chat 输入和文本编辑 ==========
            Action::SendMessage => self.send_message(),
            Action::InsertChar(ch) => {
                if self.ui.input.len() < INPUT_BUFFER_MAX_LEN {
                    self.ui.input.insert(self.ui.cursor_pos, ch);
                    self.ui.cursor_pos += ch.len_utf8();
                }
            }
            Action::DeleteChar => {
                if self.ui.cursor_pos > 0 {
                    let ch_len = self.ui.input[..self.ui.cursor_pos]
                        .chars()
                        .last()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    self.ui.cursor_pos = self.ui.cursor_pos.saturating_sub(ch_len);
                    self.ui.input.remove(self.ui.cursor_pos);
                }
            }
            Action::DeleteForward => {
                if self.ui.cursor_pos < self.ui.input.len() {
                    self.ui.input.remove(self.ui.cursor_pos);
                }
            }
            Action::MoveCursor(dir) => match dir {
                CursorDirection::Up => {
                    self.ui.cursor_pos = 0;
                }
                CursorDirection::Down => {
                    self.ui.cursor_pos = self.ui.input.len();
                }
            },
            Action::ClearInput => {
                self.ui.input.clear();
                self.ui.cursor_pos = 0;
            }

            // ========== 弹窗交互 ==========
            Action::AtPopupActivate => {
                self.ui.at_popup_active = true;
                self.ui.at_popup_filter.clear();
                self.ui.at_popup_selected = 0;
            }
            Action::AtPopupClose => {
                self.ui.at_popup_active = false;
            }
            Action::AtPopupFilter(text) => {
                self.ui.at_popup_filter = text;
                self.ui.at_popup_selected = 0;
            }
            Action::AtPopupNavigate(dir) => {
                // Will delegate to helper (Step 5 refactor)
                match dir {
                    CursorDirection::Up => {
                        if self.ui.at_popup_selected > 0 {
                            self.ui.at_popup_selected -= 1;
                        }
                    }
                    CursorDirection::Down => {
                        self.ui.at_popup_selected += 1;
                    }
                }
            }
            Action::AtPopupConfirm => {
                // Will delegate to helper (Step 5 refactor)
            }

            Action::FilePopupActivate => {
                self.ui.file_popup_active = true;
                self.ui.file_popup_filter.clear();
                self.ui.file_popup_selected = 0;
            }
            Action::FilePopupClose => {
                self.ui.file_popup_active = false;
            }
            Action::FilePopupFilter(text) => {
                self.ui.file_popup_filter = text;
                self.ui.file_popup_selected = 0;
            }
            Action::FilePopupNavigate(_dir) => {
                // Will delegate to helper (Step 5 refactor)
            }
            Action::FilePopupConfirm => {
                // Will delegate to helper (Step 5 refactor)
            }

            Action::SkillPopupActivate => {
                self.ui.skill_popup_active = true;
                self.ui.skill_popup_filter.clear();
                self.ui.skill_popup_selected = 0;
            }
            Action::SkillPopupClose => {
                self.ui.skill_popup_active = false;
            }
            Action::SkillPopupFilter(text) => {
                self.ui.skill_popup_filter = text;
                self.ui.skill_popup_selected = 0;
            }
            Action::SkillPopupNavigate(dir) => match dir {
                CursorDirection::Up => {
                    if self.ui.skill_popup_selected > 0 {
                        self.ui.skill_popup_selected -= 1;
                    }
                }
                CursorDirection::Down => {
                    self.ui.skill_popup_selected += 1;
                }
            },
            Action::SkillPopupConfirm => {}

            // ========== 流式生命周期 ==========
            Action::StreamChunk => {
                if self.ui.auto_scroll {
                    self.ui.scroll_offset = u16::MAX;
                }
                // 广播流式 chunk 到远程客户端
                if self.ws_bridge.is_some() {
                    let content =
                        safe_lock(&self.state.streaming_content, "ws_stream_chunk").clone();
                    // 只发最新增量（简单实现：发整段，客户端会替换）
                    self.broadcast_ws(
                        crate::command::chat::remote::protocol::WsOutbound::StreamChunk { content },
                    );
                }
            }
            Action::ToolCallRequest(_tool_calls) => {
                // Will delegate to helper (existing poll_stream logic)
            }
            Action::StreamDone => {
                // 广播完整消息和状态到远程
                if self.ws_bridge.is_some() {
                    if let Some(last_msg) = self.state.session.messages.last()
                        && last_msg.role == "assistant"
                    {
                        self.broadcast_ws(
                            crate::command::chat::remote::protocol::WsOutbound::Message {
                                role: "assistant".to_string(),
                                content: last_msg.content.clone(),
                            },
                        );
                    }
                    self.broadcast_ws(crate::command::chat::remote::protocol::WsOutbound::Status {
                        state: "idle".to_string(),
                    });
                }
                self.finish_loading(false, false);
            }
            Action::StreamError(ref e) => {
                self.broadcast_ws(crate::command::chat::remote::protocol::WsOutbound::Error {
                    message: format!("请求失败: {}", e),
                });
                self.broadcast_ws(crate::command::chat::remote::protocol::WsOutbound::Status {
                    state: "idle".to_string(),
                });
                self.show_toast(format!("请求失败: {}", e), true);
                self.finish_loading(true, false);
            }
            Action::StreamCancelled => {
                self.broadcast_ws(crate::command::chat::remote::protocol::WsOutbound::Status {
                    state: "idle".to_string(),
                });
                self.finish_loading(false, true);
            }

            // ========== 工具执行 ==========
            Action::ExecutePendingTool => {
                self.execute_pending_tool();
            }
            Action::RejectPendingTool => {
                self.reject_pending_tool("");
            }
            Action::RejectPendingToolWithReason(ref reason) => {
                self.reject_pending_tool(reason);
            }
            Action::AllowAndExecutePendingTool => {
                self.allow_and_execute_pending_tool();
            }

            // ========== Ask 工具交互 ==========
            Action::AskNavigate(dir) => {
                let total = self.ui.tool_ask_questions.len();
                match dir {
                    CursorDirection::Up => {
                        // Go back to previous question
                        if self.ui.tool_ask_current_idx > 0 {
                            self.ui.tool_ask_current_idx -= 1;
                            if self.ui.tool_ask_answers.len() > self.ui.tool_ask_current_idx {
                                self.ui
                                    .tool_ask_answers
                                    .truncate(self.ui.tool_ask_current_idx);
                            }
                            self.init_ask_question_state();
                        }
                    }
                    CursorDirection::Down => {
                        // Go forward (only if already answered)
                        if self.ui.tool_ask_current_idx < total - 1
                            && self.ui.tool_ask_current_idx < self.ui.tool_ask_answers.len()
                        {
                            self.ui.tool_ask_current_idx += 1;
                            self.init_ask_question_state();
                        }
                    }
                }
            }
            Action::AskOptionNavigate(dir) => {
                if let Some(q) = self.ui.tool_ask_questions.get(self.ui.tool_ask_current_idx) {
                    let option_count = q.options.len() + 1; // +1 for free input
                    match dir {
                        CursorDirection::Up => {
                            if self.ui.tool_ask_cursor > 0 {
                                self.ui.tool_ask_cursor -= 1;
                            }
                        }
                        CursorDirection::Down => {
                            if self.ui.tool_ask_cursor < option_count - 1 {
                                self.ui.tool_ask_cursor += 1;
                            }
                        }
                    }
                }
            }
            Action::AskSingleSelect => {
                if let Some(q) = self
                    .ui
                    .tool_ask_questions
                    .get(self.ui.tool_ask_current_idx)
                    .cloned()
                {
                    let cursor = self.ui.tool_ask_cursor;
                    if cursor == q.options.len() {
                        // "自由输入"选项：进入输入模式
                        self.ui.tool_interact_typing = true;
                        self.ui.tool_interact_input.clear();
                        self.ui.tool_interact_cursor = 0;
                    } else {
                        self.ask_submit_answer(AskAnswer::Selected(vec![cursor]));
                    }
                }
            }
            Action::AskToggleMultiSelect => {
                if let Some(q) = self.ui.tool_ask_questions.get(self.ui.tool_ask_current_idx)
                    && self.ui.tool_ask_cursor < q.options.len()
                {
                    let idx = self.ui.tool_ask_cursor;
                    if idx < self.ui.tool_ask_selections.len() {
                        self.ui.tool_ask_selections[idx] = !self.ui.tool_ask_selections[idx];
                    }
                }
            }
            Action::AskInputChar(c) => {
                let byte_idx = self
                    .ui
                    .tool_interact_input
                    .char_indices()
                    .nth(self.ui.tool_interact_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(self.ui.tool_interact_input.len());
                self.ui.tool_interact_input.insert(byte_idx, c);
                self.ui.tool_interact_cursor += 1;
            }
            Action::AskDeleteChar => {
                if self.ui.tool_interact_cursor > 0 {
                    let start = self
                        .ui
                        .tool_interact_input
                        .char_indices()
                        .nth(self.ui.tool_interact_cursor - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end = self
                        .ui
                        .tool_interact_input
                        .char_indices()
                        .nth(self.ui.tool_interact_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(self.ui.tool_interact_input.len());
                    self.ui.tool_interact_input.drain(start..end);
                    self.ui.tool_interact_cursor -= 1;
                }
            }
            Action::AskSubmitAnswer => {
                let input_text = self.ui.tool_interact_input.trim().to_string();
                let answer = if input_text.is_empty() {
                    AskAnswer::FreeText("（空）".to_string())
                } else {
                    AskAnswer::FreeText(input_text)
                };
                self.ask_submit_answer(answer);
                self.ui.tool_interact_input.clear();
                self.ui.tool_interact_cursor = 0;
                self.ui.tool_interact_typing = false;
            }
            Action::AskCancel => {
                // 取消整个问答
                if let Some(tx) = self.ask_response_tx.take() {
                    let _ = tx.send("用户取消了问答".to_string());
                }
                self.ui.tool_ask_mode = false;
                self.ui.tool_ask_questions.clear();
                self.ui.tool_ask_current_idx = 0;
                self.ui.tool_ask_answers.clear();
                self.ui.tool_ask_selections.clear();
                self.ui.tool_ask_cursor = 0;
                // 如果还有待确认的工具，保持 ToolConfirm 模式
                if !self.tool_executor.has_pending_confirm() {
                    self.ui.mode = ChatMode::Chat;
                }
            }

            // ========== 工具交互区 ==========
            Action::ToolInteractNavigate(dir) => match dir {
                CursorDirection::Up => {
                    if self.ui.tool_interact_selected > 0 {
                        self.ui.tool_interact_selected -= 1;
                    }
                }
                CursorDirection::Down => {
                    if self.ui.tool_interact_selected < 3 {
                        self.ui.tool_interact_selected += 1;
                    }
                }
            },
            Action::ToolInteractInputChar(c) => {
                let byte_idx = self
                    .ui
                    .tool_interact_input
                    .char_indices()
                    .nth(self.ui.tool_interact_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(self.ui.tool_interact_input.len());
                self.ui.tool_interact_input.insert(byte_idx, c);
                self.ui.tool_interact_cursor += 1;
            }
            Action::ToolInteractDeleteChar => {
                if self.ui.tool_interact_cursor > 0 {
                    let start = self
                        .ui
                        .tool_interact_input
                        .char_indices()
                        .nth(self.ui.tool_interact_cursor - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end = self
                        .ui
                        .tool_interact_input
                        .char_indices()
                        .nth(self.ui.tool_interact_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(self.ui.tool_interact_input.len());
                    self.ui.tool_interact_input.drain(start..end);
                    self.ui.tool_interact_cursor -= 1;
                }
            }
            Action::ToolInteractConfirm => match self.ui.tool_interact_selected {
                0 => self.execute_pending_tool(),
                1 => self.allow_and_execute_pending_tool(),
                2 => self.reject_pending_tool(""),
                3 => {
                    self.ui.tool_interact_typing = true;
                    self.ui.tool_interact_input.clear();
                    self.ui.tool_interact_cursor = 0;
                }
                _ => {}
            },

            // ========== 模式切换和导航 ==========
            Action::EnterMode(mode) => {
                // 广播工具确认请求到远程
                if mode == ChatMode::ToolConfirm && self.ws_bridge.is_some() {
                    let tools: Vec<crate::command::chat::remote::protocol::ToolConfirmInfo> = self
                        .tool_executor
                        .active_tool_calls
                        .iter()
                        .filter(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))
                        .map(
                            |tc| crate::command::chat::remote::protocol::ToolConfirmInfo {
                                id: tc.tool_call_id.clone(),
                                name: tc.tool_name.clone(),
                                arguments: tc.arguments.clone(),
                                confirm_message: tc.confirm_message.clone(),
                            },
                        )
                        .collect();
                    if !tools.is_empty() {
                        self.broadcast_ws(
                            crate::command::chat::remote::protocol::WsOutbound::ToolConfirmRequest { tools },
                        );
                    }
                    self.broadcast_ws(crate::command::chat::remote::protocol::WsOutbound::Status {
                        state: "tool_confirm".to_string(),
                    });
                }
                self.ui.mode = mode;
            }
            Action::ExitToChat => {
                self.ui.mode = ChatMode::Chat;
            }
            Action::Scroll(dir) => match dir {
                CursorDirection::Up => self.scroll_up(),
                CursorDirection::Down => self.scroll_down(),
            },
            Action::PageScroll(dir) => match dir {
                CursorDirection::Up => {
                    for _ in 0..10 {
                        self.scroll_up();
                    }
                }
                CursorDirection::Down => {
                    for _ in 0..10 {
                        self.scroll_down();
                    }
                }
            },
            Action::BrowseNavigate(dir) => {
                let msg_count = self.state.session.messages.len();
                if msg_count == 0 {
                    self.ui.mode = ChatMode::Chat;
                    self.ui.msg_lines_cache = None;
                    return;
                }
                match dir {
                    CursorDirection::Up => {
                        if self.ui.browse_msg_index > 0 {
                            self.ui.browse_msg_index -= 1;
                            self.ui.browse_scroll_offset = 0;
                            self.ui.msg_lines_cache = None;
                        }
                    }
                    CursorDirection::Down => {
                        if self.ui.browse_msg_index < msg_count - 1 {
                            self.ui.browse_msg_index += 1;
                            self.ui.browse_scroll_offset = 0;
                            self.ui.msg_lines_cache = None;
                        }
                    }
                }
            }
            Action::BrowseFineScroll(dir) => match dir {
                CursorDirection::Up => {
                    self.ui.browse_scroll_offset = self.ui.browse_scroll_offset.saturating_sub(3);
                }
                CursorDirection::Down => {
                    self.ui.browse_scroll_offset = self.ui.browse_scroll_offset.saturating_add(3);
                }
            },
            Action::BrowseCopyMessage => {
                use crate::command::chat::render_cache::copy_to_clipboard;
                if let Some(msg) = self.state.session.messages.get(self.ui.browse_msg_index) {
                    let content = msg.content.clone();
                    let role_label = if msg.role == ROLE_ASSISTANT {
                        "Sprite"
                    } else if msg.role == ROLE_USER {
                        "用户"
                    } else {
                        "系统"
                    };
                    if copy_to_clipboard(&content) {
                        self.show_toast(
                            format!(
                                "已复制第 {} 条{}消息",
                                self.ui.browse_msg_index + 1,
                                role_label
                            ),
                            false,
                        );
                    } else {
                        self.show_toast("复制到剪切板失败", true);
                    }
                }
            }

            // ========== 配置编辑 ==========
            Action::ConfigNavigate(dir) => {
                let total_fields = config_tab_field_count(self);
                if total_fields == 0 {
                    return;
                }
                match dir {
                    CursorDirection::Up => {
                        if self.ui.config_field_idx > 0 {
                            self.ui.config_field_idx -= 1;
                        }
                    }
                    CursorDirection::Down => {
                        if self.ui.config_field_idx < total_fields - 1 {
                            self.ui.config_field_idx += 1;
                        }
                    }
                }
            }
            Action::ConfigSwitchProvider(dir) => {
                if self.ui.config_tab != ConfigTab::Model {
                    return;
                }
                let count = self.state.agent_config.providers.len();
                if count > 1 {
                    match dir {
                        CursorDirection::Down => {
                            self.ui.config_provider_idx = (self.ui.config_provider_idx + 1) % count;
                        }
                        CursorDirection::Up => {
                            if self.ui.config_provider_idx == 0 {
                                self.ui.config_provider_idx = count - 1;
                            } else {
                                self.ui.config_provider_idx -= 1;
                            }
                        }
                    }
                }
            }
            Action::ConfigEnter => {
                use crate::command::chat::ui_helpers::{
                    config_field_raw_value_global, config_field_raw_value_model,
                };
                use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS_TAB};
                match self.ui.config_tab {
                    ConfigTab::Model => {
                        if self.state.agent_config.providers.is_empty() {
                            self.show_toast("还没有 Provider，按 a 新增", true);
                            return;
                        }
                        // supports_vision 是布尔开关，直接 toggle
                        if self.ui.config_field_idx < CONFIG_FIELDS.len()
                            && CONFIG_FIELDS[self.ui.config_field_idx] == "supports_vision"
                            && let Some(p) = self
                                .state
                                .agent_config
                                .providers
                                .get_mut(self.ui.config_provider_idx)
                        {
                            p.supports_vision = !p.supports_vision;
                            let status = if p.supports_vision {
                                "开启"
                            } else {
                                "关闭"
                            };
                            self.show_toast(format!("当前 Provider 支持视觉已{}", status), false);
                            return;
                        }
                        self.ui.config_edit_buf =
                            config_field_raw_value_model(self, self.ui.config_field_idx);
                        self.ui.config_edit_cursor = self.ui.config_edit_buf.chars().count();
                        self.ui.config_editing = true;
                    }
                    ConfigTab::Global => {
                        let idx = self.ui.config_field_idx;
                        if idx < CONFIG_GLOBAL_FIELDS_TAB.len() {
                            let field = CONFIG_GLOBAL_FIELDS_TAB[idx];
                            if field == "auto_restore_session" {
                                self.state.agent_config.auto_restore_session =
                                    !self.state.agent_config.auto_restore_session;
                                let status = if self.state.agent_config.auto_restore_session {
                                    "开启"
                                } else {
                                    "关闭"
                                };
                                self.show_toast(format!("自动恢复会话已{}", status), false);
                                return;
                            }
                            if field == "theme" {
                                self.switch_theme();
                                return;
                            }
                            if field == "system_prompt" {
                                self.ui.pending_system_prompt_edit = true;
                                return;
                            }
                            if field == "style" {
                                self.ui.pending_style_edit = true;
                                return;
                            }
                            self.ui.config_edit_buf = config_field_raw_value_global(self, idx);
                            self.ui.config_edit_cursor = self.ui.config_edit_buf.chars().count();
                            self.ui.config_editing = true;
                        }
                    }
                    ConfigTab::Tools => {
                        // Toggle individual tool
                        self.update(Action::ToggleMenuToggle);
                    }
                    ConfigTab::Skills => {
                        // Toggle individual skill
                        self.update(Action::ToggleMenuToggle);
                    }
                    ConfigTab::Commands => {
                        // Toggle individual command
                        self.update(Action::ToggleMenuToggle);
                    }
                    _ => {}
                }
            }
            Action::ConfigEditChar(c) => {
                let byte_idx = self
                    .ui
                    .config_edit_buf
                    .char_indices()
                    .nth(self.ui.config_edit_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(self.ui.config_edit_buf.len());
                self.ui.config_edit_buf.insert(byte_idx, c);
                self.ui.config_edit_cursor += 1;
            }
            Action::ConfigEditDelete => {
                if self.ui.config_edit_cursor > 0 {
                    let idx = self
                        .ui
                        .config_edit_buf
                        .char_indices()
                        .nth(self.ui.config_edit_cursor - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end_idx = self
                        .ui
                        .config_edit_buf
                        .char_indices()
                        .nth(self.ui.config_edit_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(self.ui.config_edit_buf.len());
                    self.ui.config_edit_buf = format!(
                        "{}{}",
                        &self.ui.config_edit_buf[..idx],
                        &self.ui.config_edit_buf[end_idx..]
                    );
                    self.ui.config_edit_cursor -= 1;
                }
            }
            Action::ConfigEditMoveCursor(dir) => match dir {
                CursorDirection::Up => {
                    self.ui.config_edit_cursor = self.ui.config_edit_cursor.saturating_sub(1);
                }
                CursorDirection::Down => {
                    let char_count = self.ui.config_edit_buf.chars().count();
                    if self.ui.config_edit_cursor < char_count {
                        self.ui.config_edit_cursor += 1;
                    }
                }
            },
            Action::ConfigEditSubmit => {
                use crate::command::chat::ui_helpers::{
                    config_field_set_global, config_field_set_model,
                };
                let val = self.ui.config_edit_buf.clone();
                match self.ui.config_tab {
                    ConfigTab::Model => {
                        config_field_set_model(self, self.ui.config_field_idx, &val);
                    }
                    ConfigTab::Global => {
                        config_field_set_global(self, self.ui.config_field_idx, &val);
                    }
                    _ => {}
                }
                self.ui.config_editing = false;
            }
            Action::ConfigAddProvider => {
                let new_provider = ModelProvider {
                    name: format!("Provider-{}", self.state.agent_config.providers.len() + 1),
                    api_base: "https://api.openai.com/v1".to_string(),
                    api_key: String::new(),
                    model: String::new(),
                    supports_vision: false,
                };
                self.state.agent_config.providers.push(new_provider);
                self.ui.config_provider_idx = self.state.agent_config.providers.len() - 1;
                self.ui.config_field_idx = 0;
                self.show_toast("已新增 Provider，请填写配置", false);
            }
            Action::ConfigDeleteProvider => {
                let count = self.state.agent_config.providers.len();
                if count == 0 {
                    self.show_toast("没有可删除的 Provider", true);
                } else {
                    let removed_name = self.state.agent_config.providers
                        [self.ui.config_provider_idx]
                        .name
                        .clone();
                    self.state
                        .agent_config
                        .providers
                        .remove(self.ui.config_provider_idx);
                    if self.ui.config_provider_idx >= self.state.agent_config.providers.len()
                        && self.ui.config_provider_idx > 0
                    {
                        self.ui.config_provider_idx -= 1;
                    }
                    if self.state.agent_config.active_index
                        >= self.state.agent_config.providers.len()
                        && self.state.agent_config.active_index > 0
                    {
                        self.state.agent_config.active_index -= 1;
                    }
                    self.show_toast(format!("已删除 Provider: {}", removed_name), false);
                }
            }
            Action::ConfigSetActiveProvider => {
                if !self.state.agent_config.providers.is_empty() {
                    self.state.agent_config.active_index = self.ui.config_provider_idx;
                    let name = self.state.agent_config.providers[self.ui.config_provider_idx]
                        .name
                        .clone();
                    self.show_toast(format!("已设为活跃模型: {}", name), false);
                }
            }
            Action::ConfigSwitchTab(dir) => {
                self.ui.config_tab = match dir {
                    CursorDirection::Down => self.ui.config_tab.next(),
                    CursorDirection::Up => self.ui.config_tab.prev(),
                };
                self.ui.config_field_idx = 0;
                self.ui.config_scroll_offset = 0;
                self.ui.config_editing = false;
                // 切换到 Session tab 时自动加载列表
                if self.ui.config_tab == ConfigTab::Session {
                    self.update(Action::LoadSessionList);
                }
                // 切换到 Archive tab 时自动加载归档列表
                if self.ui.config_tab == ConfigTab::Archive {
                    use crate::command::chat::archive::list_archives;
                    self.ui.archives = list_archives();
                    self.ui.archive_list_index = 0;
                    self.ui.restore_confirm_needed = false;
                }
            }
            Action::ToggleMenuNavigate(dir) => {
                // Used by Tools and Skills tabs via config_field_idx
                let total = config_tab_field_count(self);
                if total == 0 {
                    return;
                }
                match dir {
                    CursorDirection::Up => {
                        if self.ui.config_field_idx == 0 {
                            self.ui.config_field_idx = total - 1;
                        } else {
                            self.ui.config_field_idx -= 1;
                        }
                    }
                    CursorDirection::Down => {
                        self.ui.config_field_idx = (self.ui.config_field_idx + 1) % total;
                    }
                }
            }
            Action::ToggleMenuToggle => {
                if self.ui.config_tab == ConfigTab::Tools {
                    let tool_names = self.tool_registry.tool_names();
                    if let Some(name) = tool_names.get(self.ui.config_field_idx) {
                        let name = name.to_string();
                        if let Some(pos) = self
                            .state
                            .agent_config
                            .disabled_tools
                            .iter()
                            .position(|d| d == &name)
                        {
                            self.state.agent_config.disabled_tools.remove(pos);
                        } else {
                            self.state.agent_config.disabled_tools.push(name);
                        }
                    }
                } else if self.ui.config_tab == ConfigTab::Skills
                    && let Some(skill) = self.state.loaded_skills.get(self.ui.config_field_idx)
                {
                    let name = skill.frontmatter.name.clone();
                    if let Some(pos) = self
                        .state
                        .agent_config
                        .disabled_skills
                        .iter()
                        .position(|d| d == &name)
                    {
                        self.state.agent_config.disabled_skills.remove(pos);
                    } else {
                        self.state.agent_config.disabled_skills.push(name);
                    }
                } else if self.ui.config_tab == ConfigTab::Commands
                    && let Some(cmd) = self.state.loaded_commands.get(self.ui.config_field_idx)
                {
                    let name = cmd.frontmatter.name.clone();
                    if let Some(pos) = self
                        .state
                        .agent_config
                        .disabled_commands
                        .iter()
                        .position(|d| d == &name)
                    {
                        self.state.agent_config.disabled_commands.remove(pos);
                    } else {
                        self.state.agent_config.disabled_commands.push(name);
                    }
                }
            }
            Action::ToggleMenuEnableAll => {
                if self.ui.config_tab == ConfigTab::Tools {
                    self.state.agent_config.disabled_tools.clear();
                    self.show_toast("已启用全部工具", false);
                } else if self.ui.config_tab == ConfigTab::Skills {
                    self.state.agent_config.disabled_skills.clear();
                    self.show_toast("已启用全部 Skills", false);
                } else if self.ui.config_tab == ConfigTab::Commands {
                    self.state.agent_config.disabled_commands.clear();
                    self.show_toast("已启用全部命令", false);
                }
            }
            Action::ToggleMenuDisableAll => {
                if self.ui.config_tab == ConfigTab::Tools {
                    self.state.agent_config.disabled_tools = self
                        .tool_registry
                        .tool_names()
                        .iter()
                        .map(|n| n.to_string())
                        .collect();
                    self.show_toast("已禁用全部工具", false);
                } else if self.ui.config_tab == ConfigTab::Skills {
                    self.state.agent_config.disabled_skills = self
                        .state
                        .loaded_skills
                        .iter()
                        .map(|s| s.frontmatter.name.clone())
                        .collect();
                    self.show_toast("已禁用全部 Skills", false);
                } else if self.ui.config_tab == ConfigTab::Commands {
                    self.state.agent_config.disabled_commands = self
                        .state
                        .loaded_commands
                        .iter()
                        .map(|c| c.frontmatter.name.clone())
                        .collect();
                    self.show_toast("已禁用全部命令", false);
                }
            }

            // ========== 模型选择 ==========
            Action::ModelSelectNavigate(dir) => {
                let count = self.state.agent_config.providers.len();
                if count > 0 {
                    match dir {
                        CursorDirection::Up => {
                            let i = self
                                .ui
                                .model_list_state
                                .selected()
                                .map(|i| if i == 0 { count - 1 } else { i - 1 })
                                .unwrap_or(0);
                            self.ui.model_list_state.select(Some(i));
                        }
                        CursorDirection::Down => {
                            let i = self
                                .ui
                                .model_list_state
                                .selected()
                                .map(|i| if i >= count - 1 { 0 } else { i + 1 })
                                .unwrap_or(0);
                            self.ui.model_list_state.select(Some(i));
                        }
                    }
                }
            }
            Action::ModelSelectConfirm => {
                self.switch_model();
            }

            // ========== 归档管理 ==========
            Action::StartArchiveConfirm => {
                self.start_archive_confirm();
            }
            Action::ArchiveConfirmEditName => {
                self.ui.archive_editing_name = true;
            }
            Action::ArchiveConfirmMoveCursor(dir) => match dir {
                CursorDirection::Up => {
                    self.ui.archive_edit_cursor = self.ui.archive_edit_cursor.saturating_sub(1);
                }
                CursorDirection::Down => {
                    let char_count = self.ui.archive_custom_name.chars().count();
                    if self.ui.archive_edit_cursor < char_count {
                        self.ui.archive_edit_cursor += 1;
                    }
                }
            },
            Action::ArchiveConfirmInputChar(c) => {
                let chars: Vec<char> = self.ui.archive_custom_name.chars().collect();
                self.ui.archive_custom_name = chars[..self.ui.archive_edit_cursor]
                    .iter()
                    .chain(std::iter::once(&c))
                    .chain(chars[self.ui.archive_edit_cursor..].iter())
                    .collect();
                self.ui.archive_edit_cursor += 1;
            }
            Action::ArchiveConfirmDeleteChar => {
                if self.ui.archive_edit_cursor > 0 {
                    let chars: Vec<char> = self.ui.archive_custom_name.chars().collect();
                    self.ui.archive_custom_name = chars[..self.ui.archive_edit_cursor - 1]
                        .iter()
                        .chain(chars[self.ui.archive_edit_cursor..].iter())
                        .collect();
                    self.ui.archive_edit_cursor -= 1;
                }
            }
            Action::ArchiveWithDefault => {
                self.do_archive(&self.ui.archive_default_name.clone());
            }
            Action::ArchiveWithCustom => {
                self.do_archive(&self.ui.archive_custom_name.clone());
            }
            Action::ClearSession => {
                self.clear_session();
            }

            Action::ListSessions => {
                let sessions = list_sessions();
                self.broadcast_ws(
                    crate::command::chat::remote::protocol::WsOutbound::SessionList { sessions },
                );
            }
            Action::SwitchSession { session_id } => {
                if self.state.is_loading {
                    self.broadcast_ws(crate::command::chat::remote::protocol::WsOutbound::Error {
                        message: "AI 正在回复中，无法切换会话".to_string(),
                    });
                } else if self.ui.mode == ChatMode::ToolConfirm {
                    self.broadcast_ws(crate::command::chat::remote::protocol::WsOutbound::Error {
                        message: "等待工具确认中，无法切换会话".to_string(),
                    });
                } else {
                    // 检查目标文件是否存在
                    let target_path = crate::command::chat::storage::session_file_path(&session_id);
                    if !target_path.exists() {
                        self.broadcast_ws(
                            crate::command::chat::remote::protocol::WsOutbound::Error {
                                message: "会话不存在".to_string(),
                            },
                        );
                    } else {
                        // 保存当前会话
                        self.persist_new_messages();
                        // 加载目标会话
                        let session = load_session(&session_id);
                        self.session_id = session_id.clone();
                        self.last_persisted_len = session.messages.len();
                        self.state.session = session;
                        self.ui.scroll_offset = 0;
                        self.ui.msg_lines_cache = None;
                        // 广播同步 + 切换通知
                        let sync = self.build_sync_outbound();
                        self.broadcast_ws(sync);
                        self.broadcast_ws(
                            crate::command::chat::remote::protocol::WsOutbound::SessionSwitched {
                                session_id,
                            },
                        );
                    }
                }
            }
            Action::NewSession => {
                if self.state.is_loading {
                    self.broadcast_ws(crate::command::chat::remote::protocol::WsOutbound::Error {
                        message: "AI 正在回复中，无法新建会话".to_string(),
                    });
                } else if self.ui.mode == ChatMode::ToolConfirm {
                    self.broadcast_ws(crate::command::chat::remote::protocol::WsOutbound::Error {
                        message: "等待工具确认中，无法新建会话".to_string(),
                    });
                } else {
                    // 保存当前会话
                    self.persist_new_messages();
                    // 生成新会话
                    let new_id = generate_session_id();
                    self.session_id = new_id.clone();
                    self.state.session.messages.clear();
                    self.last_persisted_len = 0;
                    self.ui.scroll_offset = 0;
                    self.ui.msg_lines_cache = None;
                    // 广播同步 + 切换通知
                    let sync = self.build_sync_outbound();
                    self.broadcast_ws(sync);
                    self.broadcast_ws(
                        crate::command::chat::remote::protocol::WsOutbound::SessionSwitched {
                            session_id: new_id,
                        },
                    );
                }
            }

            Action::LoadSessionList => {
                let mut sessions = list_sessions();
                // 过滤掉当前 session
                sessions.retain(|s| s.id != self.session_id);
                self.ui.session_list = sessions;
                self.ui.session_list_index = 0;
                self.ui.session_restore_confirm = false;
            }
            Action::SessionListNavigate(dir) => {
                let count = self.ui.session_list.len();
                if count > 0 {
                    match dir {
                        CursorDirection::Up => {
                            self.ui.session_list_index = if self.ui.session_list_index == 0 {
                                count - 1
                            } else {
                                self.ui.session_list_index - 1
                            };
                        }
                        CursorDirection::Down => {
                            self.ui.session_list_index = (self.ui.session_list_index + 1) % count;
                        }
                    }
                }
            }
            Action::RestoreSession => {
                if self.ui.session_list.is_empty() {
                    return;
                }
                let idx = self.ui.session_list_index;
                if let Some(meta) = self.ui.session_list.get(idx) {
                    let target_id = meta.id.clone();
                    // 保存当前会话
                    self.persist_new_messages();
                    // 加载目标会话
                    let session = load_session(&target_id);
                    self.last_persisted_len = session.messages.len();
                    self.session_id = target_id;
                    self.state.session = session;
                    self.ui.scroll_offset = u16::MAX;
                    self.ui.msg_lines_cache = None;
                    self.ui.session_restore_confirm = false;
                    self.ui.mode = ChatMode::Chat;
                    self.show_toast("会话已恢复".to_string(), false);
                }
            }
            Action::DeleteSession => {
                if self.ui.session_list.is_empty() {
                    return;
                }
                let idx = self.ui.session_list_index;
                if let Some(meta) = self.ui.session_list.get(idx) {
                    let id = meta.id.clone();
                    if delete_session(&id) {
                        self.ui.session_list.remove(idx);
                        if self.ui.session_list_index >= self.ui.session_list.len()
                            && self.ui.session_list_index > 0
                        {
                            self.ui.session_list_index -= 1;
                        }
                        self.show_toast("会话已删除".to_string(), false);
                    } else {
                        self.show_toast("删除失败".to_string(), true);
                    }
                }
            }
            Action::NewSessionFromList => {
                // 保存当前会话
                self.persist_new_messages();
                // 生成新会话
                let new_id = generate_session_id();
                self.session_id = new_id;
                self.state.session.messages.clear();
                self.last_persisted_len = 0;
                self.ui.scroll_offset = 0;
                self.ui.msg_lines_cache = None;
                self.ui.mode = ChatMode::Chat;
                self.show_toast("已新建会话".to_string(), false);
            }

            Action::StartArchiveList => {
                self.start_archive_list();
            }
            Action::ArchiveListNavigate(dir) => {
                let count = self.ui.archives.len();
                if count > 0 {
                    match dir {
                        CursorDirection::Up => {
                            self.ui.archive_list_index = if self.ui.archive_list_index == 0 {
                                count - 1
                            } else {
                                self.ui.archive_list_index - 1
                            };
                        }
                        CursorDirection::Down => {
                            self.ui.archive_list_index = if self.ui.archive_list_index >= count - 1
                            {
                                0
                            } else {
                                self.ui.archive_list_index + 1
                            };
                        }
                    }
                }
            }
            Action::RestoreArchive => {
                self.do_restore();
            }
            Action::DeleteArchive => {
                self.do_delete_archive();
            }

            // ========== 模型和主题 ==========
            Action::SwitchModel => {
                self.ui.mode = ChatMode::SelectModel;
            }
            Action::SwitchTheme => {
                self.switch_theme();
            }
            // ========== 流式控制 ==========
            Action::CancelStream => {
                self.cancel_stream();
            }
            Action::CancelToolsOnly => {
                self.cancel_tools_only();
            }

            // ========== UI 管理 ==========
            Action::ShowToast(msg, is_error) => {
                self.show_toast(msg, is_error);
            }
            Action::TickToast => {
                self.tick_toast();
            }
            Action::SaveConfig => {
                let _ = save_agent_config(&self.state.agent_config);
                self.ui.mode = ChatMode::Chat;
            }

            // ========== 快速操作 ==========
            Action::CopyLastAiReply => {
                use crate::command::chat::render_cache::copy_to_clipboard;
                if let Some(last_ai) = self
                    .state
                    .session
                    .messages
                    .iter()
                    .rev()
                    .find(|m| m.role == ROLE_ASSISTANT)
                {
                    if copy_to_clipboard(&last_ai.content) {
                        self.show_toast("已复制最后一条 AI 回复", false);
                    } else {
                        self.show_toast("复制到剪切板失败", true);
                    }
                } else {
                    self.show_toast("暂无 AI 回复可复制", true);
                }
            }
            Action::ShowHelp => {
                self.ui.mode = ChatMode::Help;
            }
            Action::OpenLogWindows => {
                use crate::constants::{AGENT_DIR, AGENT_LOG_DIR, AGENT_LOG_ERROR, AGENT_LOG_INFO};
                let log_dir = crate::config::YamlConfig::data_dir()
                    .join(AGENT_DIR)
                    .join(AGENT_LOG_DIR);
                let info_log = log_dir.join(AGENT_LOG_INFO);
                let error_log = log_dir.join(AGENT_LOG_ERROR);
                let info_cmd = format!("tail -f '{}'; exit", info_log.to_string_lossy())
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"");
                let error_cmd = format!("tail -f '{}'; exit", error_log.to_string_lossy())
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"");
                let apple_script = format!(
                    "tell application \"Terminal\"\n\
                        do script \"{}\"\n\
                        do script \"{}\"\n\
                        activate\n\
                    end tell",
                    info_cmd, error_cmd
                );
                let _ = std::process::Command::new("osascript")
                    .arg("-e")
                    .arg(&apple_script)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
            }

            // ========== 应用控制 ==========
            Action::Quit => {
                // Will be handled by event loop
            }
            Action::ToggleExpandTools => {
                self.ui.expand_tools = !self.ui.expand_tools;
                self.ui.msg_lines_cache = None;
                self.show_toast(
                    if self.ui.expand_tools {
                        "展开工具详情"
                    } else {
                        "折叠工具详情"
                    },
                    false,
                );
            }
        }
    }

    /// 切换到下一个主题
    pub fn switch_theme(&mut self) {
        self.state.agent_config.theme = self.state.agent_config.theme.next();
        self.ui.theme = Theme::from_name(&self.state.agent_config.theme);
        self.ui.msg_lines_cache = None;
    }

    /// 显示一条 toast 通知
    pub fn show_toast(&mut self, msg: impl Into<String>, is_error: bool) {
        self.ui.toast = Some((msg.into(), is_error, std::time::Instant::now()));
    }

    /// 广播 WebSocket 消息给远程客户端
    pub fn broadcast_ws(&self, msg: crate::command::chat::remote::protocol::WsOutbound) {
        if let Some(ref ws) = self.ws_bridge {
            ws.broadcast(msg);
        }
    }

    /// 构建全量同步消息（复用于 Sync / SwitchSession / NewSession）
    pub fn build_sync_outbound(&self) -> crate::command::chat::remote::protocol::WsOutbound {
        use crate::command::chat::remote::protocol::{SyncMessage, SyncToolCall, WsOutbound};
        let messages: Vec<SyncMessage> = self
            .state
            .session
            .messages
            .iter()
            .map(|m| SyncMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                tool_calls: m.tool_calls.as_ref().map(|tc| {
                    tc.iter()
                        .map(|t| SyncToolCall {
                            id: t.id.clone(),
                            name: t.name.clone(),
                            arguments: t.arguments.clone(),
                        })
                        .collect()
                }),
                tool_call_id: m.tool_call_id.clone(),
            })
            .collect();
        let status = if self.state.is_loading {
            "loading"
        } else if self.ui.mode == ChatMode::ToolConfirm {
            "tool_confirm"
        } else {
            "idle"
        };
        let model = self.active_model_name().to_string();
        WsOutbound::SessionSync {
            messages,
            status: status.to_string(),
            model,
        }
    }

    /// 从远程客户端注入一条消息（模拟用户输入并发送）
    /// 注意：不广播 user message 回去，发送方 Web 端已经本地显示了
    ///
    /// 如果当前正在 loading（agent loop 运行中），消息追加到待处理队列，
    /// 与 TUI 本地模式下 Enter 的行为一致。
    pub fn inject_remote_message(&mut self, content: String) {
        let text = content.trim().to_string();
        if text.is_empty() {
            return;
        }
        if self.state.is_loading {
            // agent loop 运行中：追加到 pending 队列，下一轮 loop 会处理
            use crate::command::chat::storage::ChatMessage;
            self.state
                .session
                .messages
                .push(ChatMessage::text("user", &text));
            {
                let mut pending = crate::util::safe_lock(
                    &self.state.pending_user_messages,
                    "inject_remote_message::pending",
                );
                pending.push(ChatMessage::text("user", &text));
            }
            self.ui.msg_lines_cache = None;
            self.ui.auto_scroll = true;
            self.ui.scroll_offset = u16::MAX;
        } else {
            self.send_message_internal(text);
        }
    }

    /// 清理过期的 toast
    pub fn tick_toast(&mut self) {
        if let Some((_, _, created)) = &self.ui.toast
            && created.elapsed().as_secs() >= TOAST_DURATION_SECS
        {
            self.ui.toast = None;
        }
    }

    /// 获取当前活跃的 provider
    pub fn active_provider(&self) -> Option<&ModelProvider> {
        if self.state.agent_config.providers.is_empty() {
            return None;
        }
        let idx = self
            .state
            .agent_config
            .active_index
            .min(self.state.agent_config.providers.len() - 1);
        Some(&self.state.agent_config.providers[idx])
    }

    /// 获取当前模型名称
    pub fn active_model_name(&self) -> String {
        self.active_provider()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "未配置".to_string())
    }

    /// 构建发送给 API 的消息列表（安全裁剪：不从 tool pair 中间截断）
    pub fn build_api_messages(&self) -> Vec<ChatMessage> {
        let max_history = self.state.agent_config.max_history_messages;
        let msgs = &self.state.session.messages;
        if msgs.len() <= max_history {
            return msgs.clone();
        }
        let mut start = msgs.len() - max_history;
        // 向前退到安全位置：不从 tool pair 中间截断
        while start > 0
            && (msgs[start].role == ROLE_TOOL
                || (msgs[start].role == ROLE_ASSISTANT && msgs[start].tool_calls.is_some()))
        {
            start -= 1;
        }
        msgs[start..].to_vec()
    }

    /// 发送消息（非阻塞，启动后台线程流式接收）
    pub fn send_message(&mut self) {
        let text = self.ui.input.trim().to_string();
        if text.is_empty() {
            return;
        }

        // 关闭弹窗
        self.ui.at_popup_active = false;
        self.ui.file_popup_active = false;
        self.ui.skill_popup_active = false;
        self.ui.input.clear();
        self.ui.cursor_pos = 0;

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
                    cwd: std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| ".".to_string()),
                    ..Default::default()
                };
                if let Ok(manager) = self.hook_manager.lock() {
                    manager.execute(HookEvent::PreSendMessage, ctx)
                } else {
                    None
                }
            } else {
                None
            }
        };
        let text = if let Some(result) = hook_result {
            if result.abort {
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
            .push(ChatMessage::text("user", &text));
        // 发送新消息时恢复自动滚动并滚到底部
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
                    cwd: std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| ".".to_string()),
                    ..Default::default()
                };
                HookManager::execute_fire_and_forget(
                    Arc::clone(&self.hook_manager),
                    HookEvent::PostSendMessage,
                    ctx,
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

        // 同步更新 AgentTool 的 provider（子代理使用最新的 provider）
        {
            let mut p = safe_lock(&self.agent_tool_provider, "send_message::agent_provider");
            *p = provider.clone();
        }

        self.state.is_loading = true;
        // 重置流式节流状态和缓存
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

        let streaming_content = Arc::clone(&self.state.streaming_content);
        let tools_enabled = self.state.agent_config.tools_enabled;
        let max_tool_rounds = self.state.agent_config.max_tool_rounds;
        let tools = if tools_enabled {
            self.tool_registry
                .to_openai_tools_filtered(&self.state.agent_config.disabled_tools)
        } else {
            vec![]
        };

        let pending_user_messages = Arc::clone(&self.state.pending_user_messages);
        let background_manager = Arc::clone(&self.background_manager);
        let compact_config = self.state.agent_config.compact.clone();

        // 把 resolve_system_prompt 所需数据 clone 出来，在后台线程里执行文件 IO，避免阻塞主线程
        let loaded_skills = self.state.loaded_skills.clone();
        let loaded_commands = self.state.loaded_commands.clone();
        let disabled_skills = self.state.agent_config.disabled_skills.clone();
        let disabled_commands = self.state.agent_config.disabled_commands.clone();
        let disabled_tools = self.state.agent_config.disabled_tools.clone();
        let tool_registry = Arc::clone(&self.tool_registry);
        let system_prompt_fn: Box<dyn FnOnce() -> Option<String> + Send> = Box::new(move || {
            use crate::command::chat::storage::{
                load_memory, load_soul, load_style, load_system_prompt,
            };
            let template = load_system_prompt()?;
            let skills_summary = skill::build_skills_summary(&loaded_skills, &disabled_skills);
            let commands_summary =
                command::build_commands_summary(&loaded_commands, &disabled_commands);
            let tools_summary = tool_registry.build_tools_summary(&disabled_tools);
            let style_text = load_style().unwrap_or_else(|| "（未设置）".to_string());
            let memory_text = load_memory().unwrap_or_default();
            let soul_text = load_soul().unwrap_or_default();
            let current_dir = std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string());
            let skill_dir = skills_dir().to_string_lossy().to_string();
            let project_skill_dir = skill::project_skills_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let resolved = template
                .replace("{{.current_dir}}", &current_dir)
                .replace("{{.skills}}", &skills_summary)
                .replace("{{.skill_dir}}", &skill_dir)
                .replace("{{.project_skill_dir}}", &project_skill_dir)
                .replace("{{.commands}}", &commands_summary)
                .replace("{{.tools}}", &tools_summary)
                .replace("{{.style}}", &style_text)
                .replace("{{.memory}}", &memory_text)
                .replace("{{.soul}}", &soul_text);
            Some(resolved)
        });

        // Clone hook_manager for agent thread
        let hook_manager_clone = match self.hook_manager.lock() {
            Ok(manager) => manager.clone(),
            Err(_) => HookManager::default(),
        };

        let todo_manager = Arc::clone(&self.todo_manager);

        // 重置共享消息状态
        {
            let mut shared = safe_lock(&self.shared_agent_messages, "start_agent::clear_shared");
            shared.clear();
        }
        self.shared_messages_read_cursor = 0;

        // 启动 agent handle
        let agent_config = AgentLoopConfig {
            provider,
            max_tool_rounds,
            compact_config,
            hook_manager: hook_manager_clone,
            cancel_token: CancellationToken::new(),
        };
        let agent_shared = AgentSharedState {
            streaming_content,
            pending_user_messages,
            background_manager,
            todo_manager,
            shared_messages: Arc::clone(&self.shared_agent_messages),
        };
        let (handle, tool_result_tx) = AgentHandle::spawn(
            agent_config,
            agent_shared,
            api_messages,
            tools,
            system_prompt_fn,
        );

        self.agent = Some(handle);
        self.tool_executor.tool_result_tx = Some(tool_result_tx);
    }

    /// 处理后台流式消息（在主循环中每帧调用）
    /// 轮询后台流式消息并收集 Actions（Step 6: collect + dispatch 分离）
    ///
    /// 该方法完成以下职责：
    /// 1. 从共享消息列表中增量检测新消息，追加到 session.messages
    /// 2. 轮询工具执行结果（ToolExecutor 内部状态更新）
    /// 3. 轮询 Ask 工具请求（初始化 ask mode）
    /// 4. 处理延迟工具执行（pending_tool_execution）
    /// 5. 轮询 Agent StreamMsg 并映射为 Action
    ///
    /// 返回需要通过 update() 分发的 Actions 列表
    pub fn poll_stream_actions(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();

        if self.agent.is_none() {
            return actions;
        }

        // ★ 从共享消息列表中检测新消息，增量追加到 session.messages
        //   无条件执行，不受模式限制，确保任何模式下消息都能实时显示
        {
            let shared = safe_lock(&self.shared_agent_messages, "poll::shared_msgs");
            let new_count = shared.len();
            if new_count > self.shared_messages_read_cursor {
                for msg in &shared[self.shared_messages_read_cursor..] {
                    self.state.session.messages.push(msg.clone());
                }
                self.shared_messages_read_cursor = new_count;
                self.ui.msg_lines_cache = None;
            }
        }

        // 如果在 ToolConfirm 模式，仍然需要轮询工具执行结果（但暂停流式消息轮询）
        if self.ui.mode == ChatMode::ToolConfirm {
            let completed = self.tool_executor.poll_results();
            for (name, output, is_error) in completed {
                self.broadcast_ws(
                    crate::command::chat::remote::protocol::WsOutbound::ToolResult {
                        name,
                        output,
                        is_error,
                    },
                );
            }
            // 轮询 ask 请求
            if let Some(ref rx) = self.ask_request_rx
                && let Ok(ask_req) = rx.try_recv()
            {
                self.init_ask_mode(ask_req);
                self.ui.msg_lines_cache = None;
            }
            return actions;
        }

        // 如果上一帧设置了 pending_tool_execution，本帧才真正执行
        if self.tool_executor.pending_tool_execution {
            self.tool_executor.pending_tool_execution = false;

            // 广播工具开始执行到远程客户端
            if self.ws_bridge.is_some() {
                for tc in &self.tool_executor.active_tool_calls {
                    self.broadcast_ws(
                        crate::command::chat::remote::protocol::WsOutbound::ToolCall {
                            name: tc.tool_name.clone(),
                            arguments: tc.arguments.clone(),
                        },
                    );
                }
            }

            // 处理被 .jcli/ deny 拒绝的工具
            for tc in &self.tool_executor.active_tool_calls {
                if let ToolExecStatus::Failed(ref msg) = tc.status
                    && let Some(ref tx) = self.tool_executor.tool_result_tx
                {
                    let _ = tx.send(ToolResultMsg {
                        tool_call_id: tc.tool_call_id.clone(),
                        result: msg.clone(),
                        is_error: true,
                        images: vec![],
                    });
                }
            }

            // 找第一个需要确认的工具
            let first_confirm_idx = self
                .tool_executor
                .active_tool_calls
                .iter()
                .position(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm));

            if let Some(idx) = first_confirm_idx {
                self.tool_executor.pending_tool_idx = idx;
                self.tool_executor.tool_confirm_entered_at = std::time::Instant::now();
                self.tool_executor.execute_batch(&self.tool_registry);
                // 重置交互区状态
                self.ui.tool_interact_selected = 0;
                self.ui.tool_interact_typing = false;
                self.ui.tool_interact_input.clear();
                self.ui.tool_interact_cursor = 0;
                self.ui.tool_ask_mode = false;
                self.ui.tool_ask_questions.clear();
                self.ui.tool_ask_current_idx = 0;
                self.ui.tool_ask_answers.clear();
                self.ui.tool_ask_selections.clear();
                self.ui.tool_ask_cursor = 0;
                actions.push(Action::EnterMode(ChatMode::ToolConfirm));
                write_info_log(
                    "poll_stream",
                    &format!(
                        "进入 ToolConfirm 模式, pending_tool_idx={}, active_tool_calls={}, tools_executing_count={}",
                        self.tool_executor.pending_tool_idx,
                        self.tool_executor.active_tool_calls.len(),
                        self.tool_executor.tools_executing_count,
                    ),
                );
            } else {
                write_info_log(
                    "poll_stream",
                    &format!(
                        "无需确认的工具, 直接执行, active_tool_calls={}",
                        self.tool_executor.active_tool_calls.len(),
                    ),
                );
                self.tool_executor.execute_batch(&self.tool_registry);
            }
            return actions;
        }

        // 轮询后台工具执行结果
        let completed = self.tool_executor.poll_results();
        for (name, output, is_error) in completed {
            self.broadcast_ws(
                crate::command::chat::remote::protocol::WsOutbound::ToolResult {
                    name,
                    output,
                    is_error,
                },
            );
        }

        // 轮询 ask 工具请求
        if let Some(ref rx) = self.ask_request_rx
            && let Ok(ask_req) = rx.try_recv()
        {
            self.init_ask_mode(ask_req);
            actions.push(Action::EnterMode(ChatMode::ToolConfirm));
            self.ui.msg_lines_cache = None;
            return actions;
        }

        // 直接轮询 agent channel 中的流式消息
        if let Some(ref agent) = self.agent {
            let msgs = agent.poll();
            for msg in msgs {
                match msg {
                    StreamMsg::Chunk => {
                        actions.push(Action::StreamChunk);
                    }
                    StreamMsg::ToolCallRequest(tool_calls) => {
                        // 初始化工具调用状态（需要访问 jcli_config 和 tool_registry）
                        self.tool_executor.active_tool_calls.clear();
                        self.tool_executor.pending_tool_idx = 0;

                        for mut tc in tool_calls {
                            // ★ PreToolExecution hook（同步，需要返回值）
                            {
                                let has_hooks = self
                                    .hook_manager
                                    .lock()
                                    .map(|m| m.has_hooks_for(HookEvent::PreToolExecution))
                                    .unwrap_or(false);
                                if has_hooks {
                                    let ctx = HookContext {
                                        event: HookEvent::PreToolExecution,
                                        tool_name: Some(tc.name.clone()),
                                        tool_arguments: Some(tc.arguments.clone()),
                                        cwd: std::env::current_dir()
                                            .map(|p| p.display().to_string())
                                            .unwrap_or_else(|_| ".".to_string()),
                                        ..Default::default()
                                    };
                                    if let Ok(manager) = self.hook_manager.lock()
                                        && let Some(result) =
                                            manager.execute(HookEvent::PreToolExecution, ctx)
                                    {
                                        if result.abort {
                                            self.tool_executor.active_tool_calls.push(
                                                ToolCallStatus {
                                                    tool_call_id: tc.id.clone(),
                                                    tool_name: tc.name.clone(),
                                                    arguments: tc.arguments.clone(),
                                                    confirm_message: format!(
                                                        "🚫 {} 被 hook 拦截",
                                                        tc.name
                                                    ),
                                                    status: ToolExecStatus::Failed(
                                                        "该工具调用被 hook 拦截".to_string(),
                                                    ),
                                                },
                                            );
                                            continue;
                                        }
                                        if let Some(new_args) = result.tool_arguments {
                                            tc.arguments = new_args;
                                        }
                                    }
                                }
                            }

                            if self.jcli_config.is_denied(&tc.name, &tc.arguments) {
                                self.tool_executor.active_tool_calls.push(ToolCallStatus {
                                    tool_call_id: tc.id.clone(),
                                    tool_name: tc.name.clone(),
                                    arguments: tc.arguments.clone(),
                                    confirm_message: format!(
                                        "🚫 {} 被 .jcli/ 权限配置拒绝",
                                        tc.name
                                    ),
                                    status: ToolExecStatus::Failed(
                                        "该命令被 .jcli/ 权限配置拒绝".to_string(),
                                    ),
                                });
                                continue;
                            }

                            let sandbox_outside = self.sandbox.is_outside(&tc.name, &tc.arguments);
                            let confirm_msg = if sandbox_outside {
                                self.sandbox.outside_message(&tc.name, &tc.arguments)
                            } else if let Some(tool) = self.tool_registry.get(&tc.name) {
                                tool.confirmation_message(&tc.arguments)
                            } else {
                                format!("调用工具 {} 参数: {}", tc.name, tc.arguments)
                            };
                            let tool_needs_confirm = self
                                .tool_registry
                                .get(&tc.name)
                                .map(|t| t.requires_confirmation())
                                .unwrap_or(false);
                            let needs_confirm = (tool_needs_confirm || sandbox_outside)
                                && !self.jcli_config.is_allowed(&tc.name, &tc.arguments);
                            self.tool_executor.active_tool_calls.push(ToolCallStatus {
                                tool_call_id: tc.id.clone(),
                                tool_name: tc.name.clone(),
                                arguments: tc.arguments.clone(),
                                confirm_message: confirm_msg,
                                status: if needs_confirm {
                                    ToolExecStatus::PendingConfirm
                                } else {
                                    ToolExecStatus::Executing
                                },
                            });
                        }

                        // 延迟一帧再执行
                        self.tool_executor.pending_tool_execution = true;
                        break;
                    }
                    StreamMsg::Done => {
                        actions.push(Action::StreamDone);
                        break;
                    }
                    StreamMsg::Error(e) => {
                        actions.push(Action::StreamError(e));
                        break;
                    }
                    StreamMsg::Cancelled => {
                        actions.push(Action::StreamCancelled);
                        break;
                    }
                }
            }
        }

        actions
    }

    /// 初始化 ask 模式状态
    fn init_ask_mode(&mut self, ask_req: AskRequest) {
        // 广播 Ask 请求到远程客户端
        if self.ws_bridge.is_some() {
            let questions: Vec<crate::command::chat::remote::protocol::AskQuestionInfo> = ask_req
                .questions
                .iter()
                .map(
                    |q| crate::command::chat::remote::protocol::AskQuestionInfo {
                        question: q.question.clone(),
                        header: q.header.clone(),
                        options: q
                            .options
                            .iter()
                            .map(|o| crate::command::chat::remote::protocol::AskOptionInfo {
                                label: o.label.clone(),
                                description: o.description.clone(),
                            })
                            .collect(),
                        multi_select: q.multi_select,
                    },
                )
                .collect();
            self.broadcast_ws(
                crate::command::chat::remote::protocol::WsOutbound::AskRequest { questions },
            );
            self.broadcast_ws(crate::command::chat::remote::protocol::WsOutbound::Status {
                state: "ask".to_string(),
            });
        }

        self.ui.tool_ask_mode = true;
        self.ui.tool_ask_questions = ask_req.questions;
        self.ui.tool_ask_current_idx = 0;
        self.ui.tool_ask_answers = Vec::new();
        self.ask_response_tx = Some(ask_req.response_tx);
        // 初始化当前问题的选中状态
        self.init_ask_question_state();
        self.ui.tool_interact_selected = 0;
        self.ui.tool_interact_typing = false;
        self.ui.tool_interact_input.clear();
        self.ui.tool_interact_cursor = 0;
    }

    /// 初始化当前 ask 问题的选项状态
    pub fn init_ask_question_state(&mut self) {
        if let Some(q) = self.ui.tool_ask_questions.get(self.ui.tool_ask_current_idx) {
            self.ui.tool_ask_selections = vec![false; q.options.len() + 1];
            self.ui.tool_ask_cursor = 0;
        }
    }

    /// 提交当前问题的答案，前进到下一题或完成全部
    pub fn ask_submit_answer(&mut self, answer: AskAnswer) {
        let total = self.ui.tool_ask_questions.len();

        // 存储答案
        if self.ui.tool_ask_current_idx < self.ui.tool_ask_answers.len() {
            self.ui.tool_ask_answers[self.ui.tool_ask_current_idx] = answer;
        } else {
            self.ui.tool_ask_answers.push(answer);
        }

        if self.ui.tool_ask_current_idx + 1 < total {
            // 下一题
            self.ui.tool_ask_current_idx += 1;
            self.init_ask_question_state();
        } else {
            // 全部完成，构建 JSON 响应
            let mut answers_map = serde_json::Map::new();
            for (i, q) in self.ui.tool_ask_questions.iter().enumerate() {
                if let Some(ans) = self.ui.tool_ask_answers.get(i) {
                    let val = match ans {
                        AskAnswer::Selected(indices) => {
                            let labels: Vec<&str> = indices
                                .iter()
                                .filter_map(|&idx| q.options.get(idx).map(|o| o.label.as_str()))
                                .collect();
                            labels.join(", ")
                        }
                        AskAnswer::FreeText(text) => text.clone(),
                    };
                    answers_map.insert(q.question.clone(), serde_json::Value::String(val));
                }
            }

            let response = serde_json::json!({ "answers": answers_map }).to_string();
            if let Some(tx) = self.ask_response_tx.take() {
                let _ = tx.send(response);
            }

            // 清理状态
            self.ui.tool_ask_mode = false;
            self.ui.tool_ask_questions.clear();
            self.ui.tool_ask_current_idx = 0;
            self.ui.tool_ask_answers.clear();
            self.ui.tool_ask_selections.clear();
            self.ui.tool_ask_cursor = 0;
            // 如果还有待确认的工具，保持 ToolConfirm 模式
            if !self.tool_executor.has_pending_confirm() {
                self.ui.mode = ChatMode::Chat;
            }
        }
    }

    /// 结束加载状态（流式完成或错误）
    fn finish_loading(&mut self, had_error: bool, was_cancelled: bool) {
        self.agent = None;
        self.tool_executor.tool_result_tx = None;
        self.tool_executor.tools_executing_count = 0;
        self.state.is_loading = false;
        self.ui.last_rendered_streaming_len = 0;
        self.ui.msg_lines_cache = None;
        self.tool_executor.active_tool_calls.clear();

        if was_cancelled {
            let content = {
                let sc = safe_lock(
                    &self.state.streaming_content,
                    "finish_loading::streaming_content",
                );
                sc.clone()
            };
            if !content.is_empty() {
                let cancelled_content = format!("{}\n\n*[已取消]*", content);
                self.state
                    .session
                    .messages
                    .push(ChatMessage::text(ROLE_ASSISTANT, cancelled_content));
            }
            safe_lock(
                &self.state.streaming_content,
                "finish_loading::streaming_content_clear",
            )
            .clear();
            if self.ui.auto_scroll {
                self.ui.scroll_offset = u16::MAX;
            }
            self.show_toast("已取消", false);
        } else if !had_error {
            let mut content = {
                let sc = safe_lock(
                    &self.state.streaming_content,
                    "finish_loading::streaming_content_done",
                );
                sc.clone()
            };
            if !content.is_empty() {
                // ★ PostLlmResponse hook（同步，需要返回值来修改 content）
                {
                    let has_hooks = self
                        .hook_manager
                        .lock()
                        .map(|m| m.has_hooks_for(HookEvent::PostLlmResponse))
                        .unwrap_or(false);
                    if has_hooks {
                        let ctx = HookContext {
                            event: HookEvent::PostLlmResponse,
                            assistant_output: Some(content.clone()),
                            messages: Some(self.state.session.messages.clone()),
                            cwd: std::env::current_dir()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|_| ".".to_string()),
                            ..Default::default()
                        };
                        if let Ok(manager) = self.hook_manager.lock()
                            && let Some(result) = manager.execute(HookEvent::PostLlmResponse, ctx)
                            && let Some(new_msg) = result.assistant_output
                        {
                            content = new_msg;
                        }
                    }
                }

                self.state
                    .session
                    .messages
                    .push(ChatMessage::text(ROLE_ASSISTANT, content));
                safe_lock(
                    &self.state.streaming_content,
                    "finish_loading::streaming_content_done_clear",
                )
                .clear();
                self.show_toast("回复完成 ✓", false);
            }
            if self.ui.auto_scroll {
                self.ui.scroll_offset = u16::MAX;
            }
        } else {
            safe_lock(
                &self.state.streaming_content,
                "finish_loading::streaming_content_error",
            )
            .clear();
        }

        self.persist_new_messages();

        // 检查排队的任务
        let next_task = {
            let mut tasks = safe_lock(&self.state.queued_tasks, "finish_loading::queued_tasks");
            if !tasks.is_empty() {
                Some(tasks.remove(0))
            } else {
                None
            }
        };
        if let Some(task_text) = next_task {
            self.send_message_internal(task_text);
        }
    }

    /// 只取消工具执行，不终止 agent loop
    pub fn cancel_tools_only(&mut self) {
        self.tool_executor.cancel();
        self.show_toast("工具已取消", false);
    }

    /// 取消当前流式请求
    pub fn cancel_stream(&mut self) {
        if let Some(ref agent) = self.agent {
            agent.cancel();
        }
        // drop tool_result_tx
        self.tool_executor.tool_result_tx = None;
        // 通知 ShellTool kill 正在执行的子进程
        self.tool_executor
            .tool_cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// 将 session.messages 中尚未持久化的新消息追加到 JSONL
    fn persist_new_messages(&mut self) {
        let start = self.last_persisted_len;
        let msgs: Vec<_> = self.state.session.messages[start..].to_vec();
        for msg in msgs {
            append_session_event(&self.session_id, &SessionEvent::Msg(msg));
        }
        self.last_persisted_len = self.state.session.messages.len();
    }

    /// 清空对话
    pub fn clear_session(&mut self) {
        self.state.session.messages.clear();
        self.ui.scroll_offset = 0;
        self.ui.msg_lines_cache = None;
        append_session_event(&self.session_id, &SessionEvent::Clear);
        self.last_persisted_len = 0;
        self.show_toast("对话已清空", false);
    }

    /// 切换模型
    pub fn switch_model(&mut self) {
        if let Some(sel) = self.ui.model_list_state.selected() {
            self.state.agent_config.active_index = sel;
            let _ = save_agent_config(&self.state.agent_config);
            let name = self.active_model_name();
            self.show_toast(format!("已切换到: {}", name), false);
        }
        self.ui.mode = ChatMode::Chat;
    }

    /// 向上滚动消息
    pub fn scroll_up(&mut self) {
        self.ui.scroll_offset = self.ui.scroll_offset.saturating_sub(3);
        self.ui.auto_scroll = false;
    }

    /// 向下滚动消息
    pub fn scroll_down(&mut self) {
        self.ui.scroll_offset = self.ui.scroll_offset.saturating_add(3);
    }

    /// 预渲染阶段：更新消息行缓存（Step 8）
    /// 该方法在 render() 之前调用，确保所有 UI 状态已准备好
    #[allow(dead_code)]
    pub fn prepare_for_render(&mut self) {
        // 消息行缓存检查和更新
        let inner_width = 100_usize; // 占位值，实际在 render 时计算
        let bubble_max_width = (inner_width * 75 / 100).max(20);

        let msg_count = self.state.session.messages.len();
        let last_msg_len = self
            .state
            .session
            .messages
            .last()
            .map(|m| m.content.len())
            .unwrap_or(0);
        let streaming_len = safe_lock(
            &self.state.streaming_content,
            "prepare_for_render::streaming_content",
        )
        .len();
        let current_browse_index = if self.ui.mode == ChatMode::Browse {
            Some(self.ui.browse_msg_index)
        } else {
            None
        };
        let current_tool_confirm_idx = if self.ui.mode == ChatMode::ToolConfirm {
            Some(self.tool_executor.pending_tool_idx)
        } else {
            None
        };

        let cache_hit = if let Some(ref cache) = self.ui.msg_lines_cache {
            cache.msg_count == msg_count
                && cache.last_msg_len == last_msg_len
                && cache.streaming_len == streaming_len
                && cache.is_loading == self.state.is_loading
                && cache.browse_index == current_browse_index
                && cache.tool_confirm_idx == current_tool_confirm_idx
        } else {
            false
        };

        if !cache_hit {
            let old_cache = self.ui.msg_lines_cache.take();
            let (
                new_msg_start_lines,
                new_per_msg,
                new_streaming_lines,
                new_stable_lines,
                new_stable_offset,
            ) = crate::command::chat::render_cache::build_message_lines_incremental(
                self,
                inner_width,
                bubble_max_width,
                old_cache.as_ref(),
            );
            let total_line_count: usize = new_per_msg.iter().map(|p| p.lines.len()).sum::<usize>()
                + new_streaming_lines.len();
            let history_line_count: usize = new_per_msg.iter().map(|p| p.lines.len()).sum();
            self.ui.msg_lines_cache = Some(MsgLinesCache {
                msg_count,
                last_msg_len,
                streaming_len,
                is_loading: self.state.is_loading,
                bubble_max_width,
                browse_index: current_browse_index,
                tool_confirm_idx: current_tool_confirm_idx,
                total_line_count,
                history_line_count,
                msg_start_lines: new_msg_start_lines,
                per_msg_lines: new_per_msg,
                streaming_lines: new_streaming_lines,
                streaming_stable_lines: new_stable_lines,
                streaming_stable_offset: new_stable_offset,
                expand_tools: self.ui.expand_tools,
            });
        }
    }

    /// 预渲染阶段：管理滚动状态（Step 8）
    /// 根据模式和窗口大小，调整滚动偏移以确保内容可见
    #[allow(dead_code)]
    pub fn prepare_scroll_state(&mut self, visible_height: u16, max_scroll: u16) {
        if self.ui.mode != ChatMode::Browse {
            if self.ui.mode == ChatMode::ToolConfirm {
                if self.ui.auto_scroll || self.ui.scroll_offset == u16::MAX {
                    self.ui.scroll_offset = max_scroll;
                    self.ui.auto_scroll = true;
                } else if self.ui.scroll_offset > max_scroll {
                    self.ui.scroll_offset = max_scroll;
                }
            } else if self.ui.scroll_offset == u16::MAX || self.ui.scroll_offset > max_scroll {
                self.ui.scroll_offset = max_scroll;
                self.ui.auto_scroll = true;
            }
        } else if let Some(cache) = self.ui.msg_lines_cache.as_ref()
            && let Some(msg_start) = cache
                .msg_start_lines
                .iter()
                .find(|(idx, _)| *idx == self.ui.browse_msg_index)
                .map(|(_, line)| *line as u16)
        {
            let msg_line_count = cache
                .per_msg_lines
                .get(self.ui.browse_msg_index)
                .map(|c| c.lines.len())
                .unwrap_or(1) as u16;
            let msg_max_scroll = msg_line_count.saturating_sub(visible_height);
            if self.ui.browse_scroll_offset > msg_max_scroll {
                self.ui.browse_scroll_offset = msg_max_scroll;
            }
            self.ui.scroll_offset = (msg_start + self.ui.browse_scroll_offset).min(max_scroll);
        }
    }

    // ========== 归档相关方法 ==========

    /// 开始归档确认流程
    pub fn start_archive_confirm(&mut self) {
        use crate::command::chat::archive::generate_default_archive_name;
        self.ui.archive_default_name = generate_default_archive_name();
        self.ui.archive_custom_name = String::new();
        self.ui.archive_editing_name = false;
        self.ui.archive_edit_cursor = 0;
        self.ui.mode = ChatMode::ArchiveConfirm;
    }

    /// 开始还原流程（加载归档列表）
    pub fn start_archive_list(&mut self) {
        use crate::command::chat::archive::list_archives;
        self.ui.archives = list_archives();
        self.ui.archive_list_index = 0;
        self.ui.restore_confirm_needed = false;
        self.ui.mode = ChatMode::ArchiveList;
    }

    /// 执行归档
    pub fn do_archive(&mut self, name: &str) {
        use crate::command::chat::archive::create_archive;

        match create_archive(name, self.state.session.messages.clone()) {
            Ok(_) => {
                self.clear_session();
                self.show_toast(format!("对话已归档: {}", name), false);
            }
            Err(e) => {
                self.show_toast(e, true);
            }
        }
        self.ui.mode = ChatMode::Chat;
    }

    /// 执行还原归档
    pub fn do_restore(&mut self) {
        use crate::command::chat::archive::restore_archive;

        if let Some(archive) = self.ui.archives.get(self.ui.archive_list_index) {
            match restore_archive(&archive.name) {
                Ok(messages) => {
                    self.state.session.messages = messages.clone();
                    self.ui.scroll_offset = u16::MAX;
                    self.ui.msg_lines_cache = None;
                    self.ui.input.clear();
                    self.ui.cursor_pos = 0;
                    append_session_event(&self.session_id, &SessionEvent::Restore { messages });
                    self.last_persisted_len = self.state.session.messages.len();
                    self.show_toast(format!("已还原归档: {}", archive.name), false);
                }
                Err(e) => {
                    self.show_toast(e, true);
                }
            }
        }
        self.ui.mode = ChatMode::Chat;
    }

    /// 删除选中的归档
    pub fn do_delete_archive(&mut self) {
        use crate::command::chat::archive::delete_archive;

        if let Some(archive) = self.ui.archives.get(self.ui.archive_list_index) {
            match delete_archive(&archive.name) {
                Ok(_) => {
                    self.show_toast(format!("归档已删除: {}", archive.name), false);
                    self.ui.archives = crate::command::chat::archive::list_archives();
                    if self.ui.archive_list_index >= self.ui.archives.len()
                        && self.ui.archive_list_index > 0
                    {
                        self.ui.archive_list_index -= 1;
                    }
                }
                Err(e) => {
                    self.show_toast(e, true);
                }
            }
        }
    }

    // ========== 兼容方法（保持现有 handler 可编译，后续 Step 5 逐步替换为 Action）==========

    /// 执行当前待处理工具（兼容旧接口）
    pub fn execute_pending_tool(&mut self) {
        if let Some(new_mode) = self.tool_executor.execute_current(&self.tool_registry) {
            self.ui.mode = new_mode;
        }
    }

    /// 拒绝当前待处理工具（兼容旧接口）
    pub fn reject_pending_tool(&mut self, reason: &str) {
        if let Some(new_mode) = self.tool_executor.reject_current(reason) {
            self.ui.mode = new_mode;
        }
    }

    /// 允许并执行当前待处理工具（兼容旧接口）
    pub fn allow_and_execute_pending_tool(&mut self) {
        if let Some(new_mode) = self
            .tool_executor
            .allow_and_execute(&self.tool_registry, &mut self.jcli_config)
        {
            self.ui.mode = new_mode;
        }
    }
}
