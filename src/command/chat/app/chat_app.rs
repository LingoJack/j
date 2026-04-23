use super::action::{Action, CursorDirection};
use super::agent_handle::MainAgentHandle;
use super::chat_state::ChatState;
use super::tool_executor::ToolExecutor;
use super::types::{AskAnswer, AskRequest, ToolExecStatus};
use super::ui_state::{ChatMode, ConfigTab, UIState};
use crate::command::chat::agent_md;
use crate::command::chat::constants::{
    FINE_SCROLL_LINES, INPUT_BUFFER_MAX_LEN, PAGE_SCROLL_LINES, TODO_NAG_INTERVAL_ROUNDS,
    TOOL_INTERACT_MAX_OPTIONS,
};
use crate::command::chat::infra::archive;
use crate::command::chat::infra::command;
use crate::command::chat::infra::hook::{HookContext, HookEvent, HookManager, HookResult};
use crate::command::chat::infra::sandbox::Sandbox;
use crate::command::chat::infra::skill;
use crate::command::chat::markdown::image_cache::ImageCache;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::permission::queue::PermissionQueue;
use crate::command::chat::remote::protocol::{ToolConfirmInfo, WsOutbound};
use crate::command::chat::storage::MessageRole;
use crate::command::chat::storage::{
    ChatMessage, ChatSession, ModelProvider, delete_session, generate_session_id, list_sessions,
    load_agent_config, load_session, memory_path, save_agent_config, save_memory, save_soul,
    save_system_prompt, session_file_path, soul_path, system_prompt_path,
};
use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::background::{BackgroundManager, build_running_summary};
use crate::command::chat::tools::derived_shared::{DerivedAgentShared, SubAgentTracker};
use crate::command::chat::tools::plan::PlanApprovalQueue;
use crate::command::chat::tools::task::{TaskManager, build_tasks_summary};
use crate::constants::{CONFIG_FIELDS, TOAST_DURATION_SECS};
use crate::theme::{Theme, ThemeName};
use crate::tui::editor_core::text_buffer::TextBuffer;
use crate::util::safe_lock;
use ratatui::widgets::ListState;
use std::sync::{Arc, Mutex, mpsc};

// ========== 主应用结构体 ==========

/// TUI 应用状态（组合结构）
pub struct ChatApp {
    /// 前端 UI 状态
    pub ui: UIState,
    /// 后端数据状态
    pub state: ChatState,
    /// 工具执行器
    pub tool_executor: ToolExecutor,
    /// 主 Agent 生命周期句柄（存在时表示有进行中的请求）
    pub main_agent: Option<MainAgentHandle>,
    /// 工具注册表
    pub tool_registry: Arc<ToolRegistry>,
    /// .jcli/ 权限配置
    pub jcli_config: Arc<JcliConfig>,
    /// 后台任务管理器
    pub background_manager: Arc<BackgroundManager>,
    /// Task 管理器（由内置 hook 和工具通过 Arc 引用使用）
    #[allow(dead_code)]
    pub task_manager: Arc<TaskManager>,
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
    /// 与 `DerivedAgentShared` 共享的 session id 槽；切换 session 时用 `switch_session_id` 同步更新。
    pub shared_session_id: Arc<Mutex<String>>,
    /// 已持久化到 JSONL 的消息数量（用于增量追加）
    pub persisted_message_count: usize,
    /// 远程控制 WebSocket 桥接器
    pub ws_bridge: Option<crate::command::chat::remote::bridge::WsBridge>,
    /// 远程客户端是否已连接
    pub remote_connected: bool,
    /// 子 Agent 共用 provider（每次发送请求前更新，AgentTool / CreateTeammateTool / AgentTeamTool 共用）
    pub derived_agent_provider: Arc<Mutex<ModelProvider>>,
    /// 子 Agent 共用 system_prompt（每次发送请求前更新，AgentTool / CreateTeammateTool / AgentTeamTool 共用）
    pub derived_agent_system_prompt: Arc<Mutex<Option<String>>>,
    /// Agent/Teammate → UI 的显示通道（agent 线程 push，UI 线程 poll len 变化）
    pub ui_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// UI 侧已读取到的位置（用于增量检测）
    pub ui_messages_read_offset: usize,
    /// Agent 实际使用的上下文 token 估算值（agent 每轮更新，UI 读取显示）
    pub context_tokens: Arc<Mutex<usize>>,
    /// Teammate 管理器（多 agent 协作）
    #[allow(dead_code)]
    pub teammate_manager: Arc<Mutex<TeammateManager>>,
    /// 子 Agent（AgentTool）运行快照追踪器（供 /dump 读取）
    pub sub_agent_tracker: Arc<SubAgentTracker>,
    /// 派生 Agent 权限请求队列（DerivedAgentShared 和 TUI 共享同一个 Arc）
    pub permission_queue: Arc<PermissionQueue>,
    /// Plan 审批请求队列（Teammate ExitPlanMode 和 TUI 共享同一个 Arc）
    pub plan_approval_queue: Arc<PlanApprovalQueue>,
    /// 会话内已调用技能追踪（LoadSkill 执行时记录，auto_compact 后恢复）
    pub invoked_skills: crate::command::chat::context::compact::InvokedSkillsMap,
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
        ConfigTab::Hooks => app
            .hook_manager
            .lock()
            .map(|m| m.list_hooks().len())
            .unwrap_or(0),
        ConfigTab::Session => app.ui.session_list.len(),
        ConfigTab::Teammates => app
            .teammate_manager
            .lock()
            .map(|m| m.teammates.len())
            .unwrap_or(0),
        ConfigTab::Archive => app.ui.archives.len(),
    }
}

impl ChatApp {
    /// 创建新的 ChatApp 实例，初始化所有子系统和通道
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
        if !agent_md::agent_md_path().exists() {
            let _ = std::fs::write(
                agent_md::agent_md_path(),
                crate::assets::default_agent_md().as_ref(),
            );
        }
        // 安装预设 skills
        if let Err(e) = crate::assets::install_default_skills(&skill::skills_dir()) {
            crate::util::log::write_error_log(
                "[ChatApp::new]",
                &format!("安装预设 skills 失败: {}", e),
            );
        }
        // 安装预设 commands
        if let Err(e) = crate::assets::install_default_commands(&command::commands_dir()) {
            crate::util::log::write_error_log(
                "[ChatApp::new]",
                &format!("安装预设 commands 失败: {}", e),
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
        let pending_user_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(Vec::new()));
        let ui_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(Vec::new()));
        let teammate_manager: Arc<Mutex<TeammateManager>> = Arc::new(Mutex::new(
            TeammateManager::new(Arc::clone(&pending_user_messages), Arc::clone(&ui_messages)),
        ));
        let background_manager = Arc::new(BackgroundManager::new());
        let task_manager = Arc::new(TaskManager::new_with_session(&session_id));
        let hook_manager = Arc::new(Mutex::new(HookManager::load()));
        let invoked_skills = crate::command::chat::context::compact::new_invoked_skills_map();
        let mut tool_registry = ToolRegistry::new(
            loaded_skills.clone(),
            ask_req_tx,
            Arc::clone(&background_manager),
            Arc::clone(&task_manager),
            Arc::clone(&hook_manager),
            Arc::clone(&invoked_skills),
            crate::command::chat::storage::SessionPaths::new(&session_id).todos_file(),
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

        // 注入 LLM provider 到 HookManager（LLM hook 执行时使用）
        if let Ok(mut mgr) = hook_manager.lock() {
            mgr.set_provider(Arc::clone(&agent_provider));
        }

        let disabled_tools_arc = Arc::new(agent_config.disabled_tools.clone());

        // 子 agent 权限请求队列（TUI 和所有 agent 共享同一个 Arc）
        let permission_queue = Arc::new(PermissionQueue::new());
        // Plan 审批请求队列（TUI 和所有 teammate 共享同一个 Arc）
        let plan_approval_queue = Arc::new(PlanApprovalQueue::new());
        // 子 Agent 快照追踪器（/dump 从中读取正在运行的子 Agent）
        let sub_agent_tracker = Arc::new(SubAgentTracker::new());

        // 共享的 session id 槽：session 切换时 chat_app 会同步更新，teammate/subagent 据此定位 transcript
        let shared_session_id = Arc::new(Mutex::new(session_id.clone()));

        // 构建 DerivedAgentShared（SubAgentTool / AgentTeamTool / CreateTeammateTool 共用）
        let derived_agent_shared = DerivedAgentShared {
            background_manager: Arc::clone(&background_manager),
            provider: Arc::clone(&agent_provider),
            system_prompt: Arc::clone(&agent_system_prompt),
            jcli_config: Arc::new(JcliConfig::load()),
            hook_manager: Arc::clone(&hook_manager),
            task_manager: Arc::clone(&task_manager),
            disabled_tools: Arc::clone(&disabled_tools_arc),
            permission_queue: Arc::clone(&permission_queue),
            plan_approval_queue: Arc::clone(&plan_approval_queue),
            sub_agent_tracker: Arc::clone(&sub_agent_tracker),
            ui_messages: Arc::clone(&ui_messages),
            session_id: Arc::clone(&shared_session_id),
            plan_mode_state: Arc::clone(&tool_registry.plan_mode_state),
        };
        tool_registry.register(Box::new(
            crate::command::chat::tools::sub_agent::SubAgentTool {
                shared: derived_agent_shared.clone(),
            },
        ));
        tool_registry.register(Box::new(
            crate::command::chat::tools::agent_team::AgentTeamTool {
                shared: derived_agent_shared,
                teammate_manager: Arc::clone(&teammate_manager),
            },
        ));
        let tool_registry = Arc::new(tool_registry);
        let jcli_config = Arc::new(JcliConfig::load());

        // ── 注册内置 hook ──
        // 将状态占位符替换和事件驱动提醒从硬编码逻辑迁移到 hook 系统，
        // 统一通过 PreLlmRequest hook 链执行（内置→用户→项目→session）
        if let Ok(mut manager) = hook_manager.lock() {
            // 内置 hook 1: tasks_status — 替换 system_prompt 中的 {{.tasks}} 占位符
            let tasks_tm = Arc::clone(&task_manager);
            manager.register_builtin(HookEvent::PreLlmRequest, "tasks_status", move |ctx| {
                let summary = build_tasks_summary(&tasks_tm);
                if let Some(ref prompt) = ctx.system_prompt
                    && prompt.contains("{{.tasks}}")
                {
                    return Some(HookResult {
                        system_prompt: Some(prompt.replace("{{.tasks}}", &summary)),
                        ..Default::default()
                    });
                }
                None
            });

            // 内置 hook 2: background_status — 替换 {{.background_tasks}} 占位符 + 注入完成通知
            let bg_mgr = Arc::clone(&background_manager);
            manager.register_builtin(
                HookEvent::PreLlmRequest,
                "background_status",
                move |ctx| {
                    // ★ 先清理已死进程，确保状态准确
                    bg_mgr.cleanup_dead_tasks();

                    let running_summary =
                        build_running_summary(&bg_mgr);
                    let notifications = bg_mgr.drain_notifications();

                    let mut result = HookResult::default();

                    // 替换运行中任务占位符
                    if let Some(ref prompt) = ctx.system_prompt
                        && prompt.contains("{{.background_tasks}}")
                    {
                        result.system_prompt =
                            Some(prompt.replace("{{.background_tasks}}", &running_summary));
                    }

                    // 注入完成通知为 inject_messages
                    if !notifications.is_empty() {
                        let mut inject = Vec::new();
                        for notif in notifications {
                            let body = format!(
                                "<background_task_completed>\n<task_id>{}</task_id>\n<command>{}</command>\n<status>{}</status>\n<result>\n{}\n</result>\n</background_task_completed>",
                                notif.task_id, notif.command, notif.status, notif.result
                            );
                            inject.push(ChatMessage {
                                role: MessageRole::User,
                                content: format!("<system-reminder>\n{}\n</system-reminder>", body),
                                tool_calls: None,
                                tool_call_id: None,
                                images: None,
                            });
                        }
                        result.inject_messages = Some(inject);
                    }

                    if result.system_prompt.is_some() || result.inject_messages.is_some() {
                        Some(result)
                    } else {
                        None
                    }
                },
            );

            // 内置 hook 3: session_state — 替换 {{.session_state}} 占位符
            let session_tr = Arc::clone(&tool_registry);
            manager.register_builtin(HookEvent::PreLlmRequest, "session_state", move |ctx| {
                let summary = session_tr.build_session_state_summary();
                if let Some(ref prompt) = ctx.system_prompt
                    && prompt.contains("{{.session_state}}")
                {
                    return Some(HookResult {
                        system_prompt: Some(prompt.replace("{{.session_state}}", &summary)),
                        ..Default::default()
                    });
                }
                None
            });

            // 内置 hook 4: teammates_status — 替换 {{.teammates}} 占位符
            let tm_mgr = Arc::clone(&teammate_manager);
            manager.register_builtin(HookEvent::PreLlmRequest, "teammates_status", move |ctx| {
                let summary = tm_mgr.lock().map(|m| m.team_summary()).unwrap_or_default();
                if let Some(ref prompt) = ctx.system_prompt
                    && prompt.contains("{{.teammates}}")
                {
                    return Some(HookResult {
                        system_prompt: Some(prompt.replace("{{.teammates}}", &summary)),
                        ..Default::default()
                    });
                }
                None
            });

            // 内置 hook 5: todo_nag — 当 todo 列表活跃但长时间未更新时注入提醒
            let todo_mgr = Arc::clone(&todo_manager);
            manager.register_builtin(
                HookEvent::PreLlmRequest,
                "todo_nag",
                move |_ctx| {
                    if !todo_mgr.has_todos() {
                        return None;
                    }
                    let turns = todo_mgr.turns_since_last_call();
                    if turns < TODO_NAG_INTERVAL_ROUNDS {
                        return None;
                    }
                    let todos_summary = todo_mgr.format_todos_summary();
                    let body = format!(
                        "<todo_reminder>\nYou have an active todo list but haven't updated it in 15+ rounds. Update it if progress has been made, or ignore this reminder if you are currently working on an item.\n<todos>\n{}\n</todos>\n</todo_reminder>",
                        todos_summary
                    );
                    let inject = vec![ChatMessage {
                        role: MessageRole::User,
                        content: format!("<system-reminder>\n{}\n</system-reminder>", body),
                        tool_calls: None,
                        tool_call_id: None,
                        images: None,
                    }];
                    Some(HookResult {
                        inject_messages: Some(inject),
                        ..Default::default()
                    })
                },
            );
        }

        let new_app = Self {
            ui: UIState {
                input_buffer: TextBuffer::new(),
                mode: ChatMode::Chat,
                scroll_offset: u16::MAX,
                auto_scroll: true,
                browse_msg_index: 0,
                browse_scroll_offset: 0,
                browse_filter: String::new(),
                browse_role_filter: None,
                model_list_state,
                theme_list_state: ListState::default(),
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
                slash_popup_active: false,
                slash_popup_filter: String::new(),
                slash_popup_selected: 0,
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
                pending_agent_md_edit: false,
                pending_style_edit: false,
                image_cache: Arc::new(Mutex::new(ImageCache::new())),
                expand_tools: false,
                config_scroll_offset: 0,
                config_provider_scroll_offset: 0,
                config_tab: ConfigTab::Model,
                session_list: Vec::new(),
                session_list_index: 0,
                session_restore_confirm: false,
                teammate_list_index: 0,
                quote_idx: {
                    let ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as usize;
                    ms % crate::command::chat::ui::quotes::quotes_count()
                },
                input_wrap_width: 0,
                pending_agent_perm: None,
                pending_plan_approval: None,
                compact_exempt_sublist: false,
                compact_exempt_idx: 0,
                auto_approve: false,
            },
            state: ChatState {
                agent_config,
                session,
                streaming_content: Arc::new(Mutex::new(String::new())),
                is_loading: false,
                loaded_skills,
                loaded_commands,
                queued_tasks,
                pending_user_messages: Arc::clone(&pending_user_messages),
                retry_hint: None,
            },
            tool_executor: ToolExecutor::new(),
            main_agent: None,
            tool_registry,
            jcli_config,
            background_manager,
            task_manager,
            todo_manager,
            ask_response_tx: None,
            ask_request_rx: Some(ask_req_rx),
            hook_manager: Arc::clone(&hook_manager),
            sandbox: Sandbox::new(),
            session_id,
            shared_session_id,
            persisted_message_count: 0,
            ws_bridge: None,
            remote_connected: false,
            derived_agent_provider: agent_provider,
            derived_agent_system_prompt: agent_system_prompt,
            ui_messages,
            ui_messages_read_offset: 0,
            context_tokens: Arc::new(Mutex::new(0)),
            teammate_manager,
            sub_agent_tracker,
            permission_queue,
            plan_approval_queue,
            invoked_skills,
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
                    session_id: Some(new_app.session_id.clone()),
                    cwd: std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| ".".to_string()),
                    ..Default::default()
                };
                HookManager::execute_fire_and_forget(
                    Arc::clone(&new_app.hook_manager),
                    HookEvent::SessionStart,
                    ctx,
                    new_app.state.agent_config.disabled_hooks.clone(),
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
                if self.ui.input_text().len() < INPUT_BUFFER_MAX_LEN {
                    self.ui.input_buffer.insert_char(ch);
                }
            }
            Action::DeleteChar => {
                self.ui.input_buffer.backspace();
            }
            Action::DeleteForward => {
                self.ui.input_buffer.delete_char();
            }
            Action::MoveCursor(dir) => match dir {
                CursorDirection::Up => {
                    self.ui.input_buffer.move_cursor_up();
                }
                CursorDirection::Down => {
                    self.ui.input_buffer.move_cursor_down();
                }
            },
            Action::ClearInput => {
                self.ui.clear_input();
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
                self.state.retry_hint = None;
                if self.ui.auto_scroll {
                    self.ui.scroll_offset = u16::MAX;
                }
                // 广播流式 chunk 到远程客户端
                if self.ws_bridge.is_some() {
                    let content =
                        safe_lock(&self.state.streaming_content, "ws_stream_chunk").clone();
                    // 只发最新增量（简单实现：发整段，客户端会替换）
                    self.broadcast_ws(WsOutbound::StreamChunk { content });
                }
            }
            Action::ToolCallRequest(_tool_calls) => {
                // Will delegate to helper (existing poll_stream logic)
            }
            Action::StreamDone => {
                self.state.retry_hint = None;
                // 广播完整消息和状态到远程
                if self.ws_bridge.is_some() {
                    if let Some(last_msg) = self.state.session.messages.last()
                        && last_msg.role == MessageRole::Assistant
                    {
                        self.broadcast_ws(WsOutbound::Message {
                            role: MessageRole::Assistant.to_string(),
                            content: last_msg.content.clone(),
                        });
                    }
                    self.broadcast_ws(WsOutbound::Status {
                        state: "idle".to_string(),
                    });
                }
                self.finish_loading(false, false);
            }
            Action::StreamError(ref e) => {
                self.state.retry_hint = None;
                let msg = e.display_message();
                self.broadcast_ws(WsOutbound::Error {
                    message: format!("请求失败: {}", msg),
                });
                self.broadcast_ws(WsOutbound::Status {
                    state: "idle".to_string(),
                });
                self.show_toast(format!("请求失败: {}", msg), true);
                self.finish_loading(true, false);
            }
            Action::StreamCancelled => {
                self.state.retry_hint = None;
                self.broadcast_ws(WsOutbound::Status {
                    state: "idle".to_string(),
                });
                self.finish_loading(false, true);
            }
            Action::StreamRetrying {
                attempt,
                max_attempts,
                delay_ms,
                error,
            } => {
                let delay_s = delay_ms.div_ceil(1000);
                self.state.retry_hint = Some(format!(
                    "⟳ 重试 {}/{} · {}s · {}",
                    attempt, max_attempts, delay_s, error
                ));
            }
            Action::StreamCompacting => {
                self.state.retry_hint = Some("📦 压缩上下文中...".to_string());
            }
            Action::StreamCompacted { messages_before: _ } => {
                self.state.retry_hint = None;
                // 从 ui_messages 同步压缩后的消息列表
                // ui_messages 已被 clear+re-push，需要从头替换
                {
                    let shared = crate::util::safe_lock(&self.ui_messages, "StreamCompacted::sync");
                    self.state.session.messages = shared.clone();
                    self.ui_messages_read_offset = shared.len();
                }
                self.ui.msg_lines_cache = None;
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
                    let free_input_idx = q.options.len();

                    let new_cursor = match dir {
                        CursorDirection::Up => {
                            if self.ui.tool_ask_cursor > 0 {
                                self.ui.tool_ask_cursor - 1
                            } else {
                                self.ui.tool_ask_cursor
                            }
                        }
                        CursorDirection::Down => {
                            if self.ui.tool_ask_cursor < option_count - 1 {
                                self.ui.tool_ask_cursor + 1
                            } else {
                                self.ui.tool_ask_cursor
                            }
                        }
                    };

                    // 如果光标位置发生变化
                    if new_cursor != self.ui.tool_ask_cursor {
                        self.ui.tool_ask_cursor = new_cursor;

                        // 移动到自由输入选项时，自动进入输入模式
                        if new_cursor == free_input_idx {
                            self.ui.tool_interact_typing = true;
                            self.ui.tool_interact_input.clear();
                            self.ui.tool_interact_cursor = 0;
                        } else {
                            // 移动到其他选项时，退出输入模式
                            self.ui.tool_interact_typing = false;
                            self.ui.tool_interact_input.clear();
                            self.ui.tool_interact_cursor = 0;
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
                    if self.ui.tool_interact_selected < TOOL_INTERACT_MAX_OPTIONS {
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
                    let tools: Vec<ToolConfirmInfo> = self
                        .tool_executor
                        .active_tool_calls
                        .iter()
                        .filter(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))
                        .map(|tc| ToolConfirmInfo {
                            id: tc.tool_call_id.clone(),
                            name: tc.tool_name.clone(),
                            arguments: tc.arguments.clone(),
                            confirm_message: tc.confirm_message.clone(),
                        })
                        .collect();
                    if !tools.is_empty() {
                        self.broadcast_ws(WsOutbound::ToolConfirmRequest { tools });
                    }
                    self.broadcast_ws(WsOutbound::Status {
                        state: "tool_confirm".to_string(),
                    });
                }
                // 进入浏览模式时清除过滤状态
                if mode == ChatMode::Browse {
                    self.ui.browse_filter.clear();
                    self.ui.browse_role_filter = None;
                }
                self.ui.mode = mode;
            }
            Action::ExitToChat => {
                self.ui.mode = ChatMode::Chat;
                self.ui.browse_filter.clear();
                self.ui.browse_role_filter = None;
            }
            Action::Scroll(dir) => match dir {
                CursorDirection::Up => self.scroll_up(),
                CursorDirection::Down => self.scroll_down(),
            },
            Action::PageScroll(dir) => match dir {
                CursorDirection::Up => {
                    for _ in 0..PAGE_SCROLL_LINES {
                        self.scroll_up();
                    }
                }
                CursorDirection::Down => {
                    for _ in 0..PAGE_SCROLL_LINES {
                        self.scroll_down();
                    }
                }
            },
            Action::BrowseNavigate(dir) => {
                let filtered = self.browse_filtered_indices();
                if filtered.is_empty() {
                    self.ui.mode = ChatMode::Chat;
                    self.ui.msg_lines_cache = None;
                    return;
                }
                let current_in_filtered =
                    filtered.iter().position(|&i| i == self.ui.browse_msg_index);
                match dir {
                    CursorDirection::Up => {
                        let new_idx = match current_in_filtered {
                            Some(pos) if pos > 0 => filtered[pos - 1],
                            Some(_) => filtered[filtered.len() - 1],
                            None => *filtered.last().expect("browse mode 下 filtered 不应为空"),
                        };
                        self.ui.browse_msg_index = new_idx;
                        self.ui.browse_scroll_offset = 0;
                        self.ui.msg_lines_cache = None;
                    }
                    CursorDirection::Down => {
                        let new_idx = match current_in_filtered {
                            Some(pos) if pos < filtered.len() - 1 => filtered[pos + 1],
                            Some(_) => filtered[0],
                            None => filtered[0],
                        };
                        self.ui.browse_msg_index = new_idx;
                        self.ui.browse_scroll_offset = 0;
                        self.ui.msg_lines_cache = None;
                    }
                }
            }
            Action::BrowseFineScroll(dir) => match dir {
                CursorDirection::Up => {
                    self.ui.browse_scroll_offset = self
                        .ui
                        .browse_scroll_offset
                        .saturating_sub(FINE_SCROLL_LINES);
                }
                CursorDirection::Down => {
                    self.ui.browse_scroll_offset = self
                        .ui
                        .browse_scroll_offset
                        .saturating_add(FINE_SCROLL_LINES);
                }
            },
            Action::BrowseCopyMessage => {
                use crate::command::chat::render::cache::copy_to_clipboard;
                if let Some(msg) = self.state.session.messages.get(self.ui.browse_msg_index) {
                    let content = msg.content.clone();
                    let filtered = self.browse_filtered_indices();
                    let pos_in_filtered =
                        filtered.iter().position(|&i| i == self.ui.browse_msg_index);
                    let role_label = if msg.role == MessageRole::Assistant {
                        "AI"
                    } else if msg.role == MessageRole::User {
                        "用户"
                    } else {
                        "系统"
                    };
                    let extra = if let Some(pos) = pos_in_filtered {
                        format!(" ({}/{})", pos + 1, filtered.len())
                    } else {
                        String::new()
                    };
                    if copy_to_clipboard(&content) {
                        self.show_toast(format!("已复制{}消息{}", role_label, extra), false);
                    } else {
                        self.show_toast("复制到剪切板失败", true);
                    }
                }
            }
            Action::BrowseInputChar(c) => {
                self.ui.browse_filter.push(c);
                self.browse_jump_to_first_match();
                self.ui.msg_lines_cache = None;
            }
            Action::BrowseDeleteChar => {
                self.ui.browse_filter.pop();
                self.browse_jump_to_first_match();
                self.ui.msg_lines_cache = None;
            }
            Action::BrowseClearFilter => {
                self.ui.browse_filter.clear();
                self.ui.browse_role_filter = None;
                self.ui.msg_lines_cache = None;
            }
            Action::BrowseToggleRole => {
                self.ui.browse_role_filter = match &self.ui.browse_role_filter {
                    None => Some("ai".to_string()),
                    Some(r) if r == "ai" => Some("user".to_string()),
                    _ => None,
                };
                self.browse_jump_to_first_match();
                self.ui.msg_lines_cache = None;
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
                use crate::command::chat::render::helpers::{
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
                            if field == "compact_enabled" {
                                self.state.agent_config.compact.enabled =
                                    !self.state.agent_config.compact.enabled;
                                let status = if self.state.agent_config.compact.enabled {
                                    "开启"
                                } else {
                                    "关闭"
                                };
                                self.show_toast(format!("上下文压缩已{}", status), false);
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
                            if field == "agent_md" {
                                self.ui.pending_agent_md_edit = true;
                                return;
                            }
                            if field == "compact_exempt_tools" {
                                self.ui.compact_exempt_sublist = true;
                                self.ui.compact_exempt_idx = 0;
                                self.ui.config_scroll_offset = 0;
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
            Action::ConfigEditDeleteForward => {
                let char_count = self.ui.config_edit_buf.chars().count();
                if self.ui.config_edit_cursor < char_count {
                    let idx = self
                        .ui
                        .config_edit_buf
                        .char_indices()
                        .nth(self.ui.config_edit_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(self.ui.config_edit_buf.len());
                    let end_idx = self
                        .ui
                        .config_edit_buf
                        .char_indices()
                        .nth(self.ui.config_edit_cursor + 1)
                        .map(|(i, _)| i)
                        .unwrap_or(self.ui.config_edit_buf.len());
                    self.ui.config_edit_buf = format!(
                        "{}{}",
                        &self.ui.config_edit_buf[..idx],
                        &self.ui.config_edit_buf[end_idx..]
                    );
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
            Action::ConfigEditMoveHome => {
                self.ui.config_edit_cursor = 0;
            }
            Action::ConfigEditMoveEnd => {
                self.ui.config_edit_cursor = self.ui.config_edit_buf.chars().count();
            }
            Action::ConfigEditClearLine => {
                self.ui.config_edit_buf.clear();
                self.ui.config_edit_cursor = 0;
            }
            Action::ConfigEditSubmit => {
                use crate::command::chat::render::helpers::{
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
                    use archive::list_archives;
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
                } else if self.ui.config_tab == ConfigTab::Hooks {
                    // Hooks 切换处理
                    if let Ok(manager) = self.hook_manager.lock() {
                        let hooks = manager.list_hooks();
                        if let Some(entry) = hooks.get(self.ui.config_field_idx) {
                            let uid = entry.unique_id.clone();
                            if let Some(pos) = self
                                .state
                                .agent_config
                                .disabled_hooks
                                .iter()
                                .position(|d| d == &uid)
                            {
                                self.state.agent_config.disabled_hooks.remove(pos);
                            } else {
                                self.state.agent_config.disabled_hooks.push(uid);
                            }
                        }
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
                } else if self.ui.config_tab == ConfigTab::Hooks {
                    self.state.agent_config.disabled_hooks.clear();
                    self.show_toast("已启用全部 Hooks", false);
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
                } else if self.ui.config_tab == ConfigTab::Hooks {
                    if let Ok(manager) = self.hook_manager.lock() {
                        self.state.agent_config.disabled_hooks = manager
                            .list_hooks()
                            .iter()
                            .map(|h| h.unique_id.clone())
                            .collect();
                    }
                    self.show_toast("已禁用全部 Hooks", false);
                }
            }
            Action::CompactExemptToggle => {
                let tool_names = self.tool_registry.tool_names();
                if let Some(name) = tool_names.get(self.ui.compact_exempt_idx) {
                    let name_str = name.to_string();
                    let exempt = &mut self.state.agent_config.compact.micro_compact_exempt_tools;
                    if let Some(pos) = exempt.iter().position(|t| t == &name_str) {
                        exempt.remove(pos);
                    } else {
                        exempt.push(name_str);
                    }
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

            // ========== 主题选择 ==========
            Action::ThemeSelectNavigate(dir) => {
                let count = ThemeName::all().len();
                if count > 0 {
                    match dir {
                        CursorDirection::Up => {
                            let i = self
                                .ui
                                .theme_list_state
                                .selected()
                                .map(|i| if i == 0 { count - 1 } else { i - 1 })
                                .unwrap_or(0);
                            self.ui.theme_list_state.select(Some(i));
                        }
                        CursorDirection::Down => {
                            let i = self
                                .ui
                                .theme_list_state
                                .selected()
                                .map(|i| if i >= count - 1 { 0 } else { i + 1 })
                                .unwrap_or(0);
                            self.ui.theme_list_state.select(Some(i));
                        }
                    }
                }
            }
            Action::ThemeSelectConfirm => {
                if let Some(sel) = self.ui.theme_list_state.selected() {
                    let all = ThemeName::all();
                    if sel < all.len() {
                        self.state.agent_config.theme = all[sel].clone();
                        self.ui.theme = Theme::from_name(&all[sel]);
                        self.ui.msg_lines_cache = None;
                        let _ = save_agent_config(&self.state.agent_config);
                        let name = all[sel].display_name();
                        self.show_toast(format!("已切换主题: {}", name), false);
                    }
                }
                self.ui.mode = ChatMode::Chat;
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
                self.broadcast_ws(WsOutbound::SessionList { sessions });
            }
            Action::SwitchSession { session_id } => {
                if self.state.is_loading {
                    self.broadcast_ws(WsOutbound::Error {
                        message: "AI 正在回复中，无法切换会话".to_string(),
                    });
                } else if self.ui.mode == ChatMode::ToolConfirm {
                    self.broadcast_ws(WsOutbound::Error {
                        message: "等待工具确认中，无法切换会话".to_string(),
                    });
                } else {
                    // 检查目标文件是否存在
                    let target_path = session_file_path(&session_id);
                    if !target_path.exists() {
                        self.broadcast_ws(WsOutbound::Error {
                            message: "会话不存在".to_string(),
                        });
                    } else {
                        // 保存当前会话（消息 + 状态）
                        self.persist_new_messages();
                        self.save_session_state();
                        // 清除运行时状态
                        self.clear_runtime_state();
                        // 加载目标会话
                        let session = load_session(&session_id);
                        self.session_id = session_id.clone();
                        if let Ok(mut s) = self.shared_session_id.lock() {
                            *s = session_id.clone();
                        }
                        self.persisted_message_count = session.messages.len();
                        self.state.session = session;
                        // 恢复目标会话的状态
                        self.restore_session_state();
                        self.ui.scroll_offset = 0;
                        self.ui.msg_lines_cache = None;
                        if let Ok(mut ct) = self.context_tokens.lock() {
                            *ct = 0;
                        }
                        // 广播同步 + 切换通知
                        let sync = self.build_sync_outbound();
                        self.broadcast_ws(sync);
                        self.broadcast_ws(WsOutbound::SessionSwitched { session_id });
                    }
                }
            }
            Action::NewSession => {
                if self.state.is_loading {
                    self.broadcast_ws(WsOutbound::Error {
                        message: "AI 正在回复中，无法新建会话".to_string(),
                    });
                } else if self.ui.mode == ChatMode::ToolConfirm {
                    self.broadcast_ws(WsOutbound::Error {
                        message: "等待工具确认中，无法新建会话".to_string(),
                    });
                } else {
                    // 保存当前会话（消息 + 状态）
                    self.persist_new_messages();
                    self.save_session_state();
                    // 清除运行时状态
                    self.clear_runtime_state();
                    // 生成新会话
                    let new_id = generate_session_id();
                    self.session_id = new_id.clone();
                    if let Ok(mut s) = self.shared_session_id.lock() {
                        *s = new_id.clone();
                    }
                    self.state.session.messages.clear();
                    self.persisted_message_count = 0;
                    self.ui.scroll_offset = 0;
                    self.ui.msg_lines_cache = None;
                    if let Ok(mut ct) = self.context_tokens.lock() {
                        *ct = 0;
                    }
                    // 广播同步 + 切换通知
                    let sync = self.build_sync_outbound();
                    self.broadcast_ws(sync);
                    self.broadcast_ws(WsOutbound::SessionSwitched { session_id: new_id });
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
                    // 保存当前会话（消息 + 状态）
                    self.persist_new_messages();
                    self.save_session_state();
                    // 清除运行时状态
                    self.clear_runtime_state();
                    // 加载目标会话
                    let session = load_session(&target_id);
                    self.persisted_message_count = session.messages.len();
                    self.session_id = target_id.clone();
                    if let Ok(mut s) = self.shared_session_id.lock() {
                        *s = target_id;
                    }
                    self.state.session = session;
                    // 恢复目标会话的状态
                    self.restore_session_state();
                    self.ui.scroll_offset = u16::MAX;
                    self.ui.msg_lines_cache = None;
                    self.ui.session_restore_confirm = false;
                    if let Ok(mut ct) = self.context_tokens.lock() {
                        *ct = 0;
                    }
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
                // 保存当前会话（消息 + 状态）
                self.persist_new_messages();
                self.save_session_state();
                self.clear_runtime_state();
                // 生成新会话
                let new_id = generate_session_id();
                self.session_id = new_id.clone();
                if let Ok(mut s) = self.shared_session_id.lock() {
                    *s = new_id;
                }
                self.state.session.messages.clear();
                self.persisted_message_count = 0;
                self.ui.scroll_offset = 0;
                self.ui.msg_lines_cache = None;
                if let Ok(mut ct) = self.context_tokens.lock() {
                    *ct = 0;
                }
                self.ui.mode = ChatMode::Chat;
                self.show_toast("已新建会话".to_string(), false);
            }

            Action::RespawnTeammate { name } => {
                // 从 recovered_teammates 取出保存的 prompt/role，重新创建 teammate
                let snapshot = if let Ok(mgr) = self.teammate_manager.lock() {
                    mgr.get_recovered_teammate(&name)
                } else {
                    None
                };
                if let Some(_snapshot) = snapshot {
                    // 使用 CreateTeammate 工具的参数格式创建 teammate
                    // 这里简单实现：将 prompt 信息注入 pending_user_messages
                    if let Ok(mut mgr) = self.teammate_manager.lock() {
                        mgr.remove_recovered_teammate(&name);
                    }
                    // TODO: 完整的 RespawnTeammate 需要调用 CreateTeammateTool 的逻辑
                    // 当前简化实现：提示用户手动重新创建
                    self.show_toast(
                        format!("Teammate '{}' 的上下文已恢复，请通过 AI 重新创建", name),
                        false,
                    );
                } else {
                    self.show_toast(format!("未找到 teammate '{}'", name), true);
                }
            }

            Action::SessionStateRestored => {
                self.show_toast("会话状态已恢复".to_string(), false);
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
                // 取消所有挂起的子 agent 权限请求，防止线程永久阻塞
                self.permission_queue.deny_all();
                // 取消所有挂起的 Plan 审批请求
                self.plan_approval_queue.deny_all();
                if let Some(req) = self.ui.pending_agent_perm.take() {
                    req.resolve(false);
                }
                if let Some(req) = self.ui.pending_plan_approval.take() {
                    req.resolve(crate::command::chat::app::types::PlanDecision::Reject);
                }
                if matches!(
                    self.ui.mode,
                    ChatMode::AgentPermConfirm | ChatMode::PlanApprovalConfirm
                ) {
                    self.ui.mode = ChatMode::Chat;
                }
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
                use crate::command::chat::render::cache::copy_to_clipboard;
                if let Some(last_ai) = self
                    .state
                    .session
                    .messages
                    .iter()
                    .rev()
                    .find(|m| m.role == MessageRole::Assistant)
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
                // 展开工具后自动滚动到底部，确保用户能立即看到展开的内容
                self.ui.auto_scroll = true;
                self.ui.scroll_offset = u16::MAX;
                self.show_toast(
                    if self.ui.expand_tools {
                        "展开工具详情"
                    } else {
                        "折叠工具详情"
                    },
                    false,
                );
            }
            Action::ToggleAutoApprove => {
                self.ui.auto_approve = !self.ui.auto_approve;
                // 持久化到 session.json
                let sid = self.session_id.clone();
                if let Some(mut meta) = crate::command::chat::storage::load_session_meta_file(&sid)
                {
                    meta.auto_approve = self.ui.auto_approve;
                    crate::command::chat::storage::save_session_meta_file(&meta);
                }
                self.show_toast(
                    if self.ui.auto_approve {
                        "Bypass 模式已开启"
                    } else {
                        "Bypass 模式已关闭"
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

    pub fn show_toast(&mut self, msg: impl Into<String>, is_error: bool) {
        self.ui.toast = Some((msg.into(), is_error, std::time::Instant::now()));
    }

    /// 广播 WebSocket 消息给远程客户端
    pub fn broadcast_ws(&self, msg: WsOutbound) {
        if let Some(ref ws) = self.ws_bridge {
            ws.broadcast(msg);
        }
    }

    /// 构建全量同步消息（复用于 Sync / SwitchSession / NewSession）
    pub fn build_sync_outbound(&self) -> WsOutbound {
        use crate::command::chat::remote::protocol::{SyncMessage, SyncToolCall};
        let messages: Vec<SyncMessage> = self
            .state
            .session
            .messages
            .iter()
            .map(|m| SyncMessage {
                role: m.role.to_string(),
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
    pub fn inject_remote_message(&mut self, content: &str) {
        use crate::command::chat::infra::command;
        use crate::command::chat::storage::{ChatMessage, MessageRole};

        let text = content.trim().to_string();
        if text.is_empty() {
            return;
        }

        // 展开 @command:name 引用
        let text = command::expand_command_mentions(
            &text,
            &self.state.loaded_commands,
            &self.state.agent_config.disabled_commands,
        );

        if self.state.is_loading {
            // agent loop 运行中：追加到 pending 队列，下一轮 loop 会处理
            self.state
                .session
                .messages
                .push(ChatMessage::text(MessageRole::User, &text));
            {
                let mut pending = crate::util::safe_lock(
                    &self.state.pending_user_messages,
                    "inject_remote_message::pending",
                );
                pending.push(ChatMessage::text(MessageRole::User, &text));
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
    /// 构造当前真实传给 AI 的 system prompt。
    /// 与 send_message 的 system_prompt_fn + 内置 PreLlmRequest hook 合起来的效果一致：
    /// 同时替换静态占位符（.tools/.skills/.memory/…）和状态占位符
    /// (.tasks/.background_tasks/.session_state/.teammates)。
    /// 仅取消工具执行，不取消整个流式请求
    pub fn cancel_tools_only(&mut self) {
        self.tool_executor.cancel();
        self.tool_executor.tools_executing_count = 0;
        self.tool_executor.active_tool_calls.clear();
        self.tool_executor.pending_tool_execution = false;
        self.show_toast("工具已取消", false);
    }

    /// 取消当前流式请求
    ///
    /// 立即执行 finish_loading() 清除加载状态，不等 agent 线程响应取消信号。
    /// 这确保 Esc 按键后 UI 瞬间恢复可交互状态。
    pub fn cancel_stream(&mut self) {
        self.finish_loading(false, true);
    }

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
