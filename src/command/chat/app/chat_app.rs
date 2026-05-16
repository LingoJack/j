mod update;
mod update_config;
mod update_misc;
mod update_session;
mod update_tool_interact;

use super::agent_handle::MainAgentHandle;
use super::chat_state::ChatState;
use super::tool_executor::ToolExecutor;
use super::types::AskRequest;
use super::ui_state::{ChatMode, CommandsMode, ConfigTab, UIState};
use crate::command::chat::agent_md;
use crate::command::chat::constants::TODO_NAG_INTERVAL_ROUNDS;
use crate::command::chat::context::message_compress::{
    DEFAULT_OTHER_AGENT_TOOLCALL_THRESHOLD, compress_other_agent_toolcalls,
};
use crate::command::chat::infra::command::{self, CommandSource};
use crate::command::chat::infra::hook::{HookContext, HookEvent, HookManager, HookResult};
use crate::command::chat::infra::sandbox::Sandbox;
use crate::command::chat::infra::skill;
use crate::command::chat::input::file_index::FileIndex;
use crate::command::chat::permission::JcliConfig;
use crate::command::chat::permission::queue::PermissionQueue;
use crate::command::chat::remote::protocol::WsOutbound;
use crate::command::chat::storage::MessageRole;
use crate::command::chat::storage::{
    ChatMessage, DisplayHint, ModelProvider, load_agent_config, memory_path, save_agent_config,
    save_memory, save_soul, save_system_prompt, soul_path, system_prompt_path,
};
use crate::command::chat::teammate::TeammateManager;
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::background::{BackgroundManager, build_running_summary};
use crate::command::chat::tools::derived_shared::{
    AgentContextConfig, DerivedAgentShared, SubAgentTracker,
};
use crate::command::chat::tools::plan::PlanApprovalQueue;
use crate::command::chat::tools::task::{TaskManager, build_tasks_summary};
use crate::command::chat::tools::todo::TodoManager;
use crate::constants::{CONFIG_FIELDS, TOAST_DURATION_SECS};
use crate::markdown::image_cache::ImageCache;
use crate::theme::Theme;
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
    pub todo_manager: Arc<TodoManager>,
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
    /// 已持久化到 display.jsonl 的消息数量（用于增量追加）
    pub persisted_display_count: usize,
    /// 远程控制 WebSocket 桥接器
    pub ws_bridge: Option<crate::command::chat::remote::bridge::WsBridge>,
    /// 远程客户端是否已连接
    pub remote_connected: bool,
    /// 子 Agent 共用 provider（每次发送请求前更新，AgentTool / TeammateTool 共用）
    pub derived_agent_provider: Arc<Mutex<ModelProvider>>,
    /// 子 Agent 共用 system_prompt（每次发送请求前更新，AgentTool / TeammateTool 共用）
    pub derived_agent_system_prompt: Arc<Mutex<Option<String>>>,
    /// 子 Agent 共用上下文配置快照（每次发送请求前刷新）
    pub derived_agent_context_config: Arc<Mutex<AgentContextConfig>>,
    /// 子 Agent 使用的 disabled_hooks 快照（每次发送请求前刷新）
    pub derived_agent_disabled_hooks: Arc<Mutex<Vec<String>>>,
    /// Agent/Teammate → UI 的显示通道（agent 线程 push，UI 线程 poll len 变化）
    /// 仅用于 UI 渲染，不作为 LLM context 数据源。
    pub display_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// UI 侧已读取到的位置（用于增量检测）
    pub display_read_offset: usize,
    /// Agent/Teammate → LLM context 同步通道
    /// `persist_new_messages` 直接从此通道读取并持久化到 transcript.jsonl。
    /// 只有需要进入 Main Agent LLM context 的消息才写入此通道。
    pub context_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// context 侧已读取到的位置（用于增量检测）
    pub context_read_offset: usize,
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
    /// 子 Agent metrics 累加器（SubAgent/Teammate 的 LLM/tool 统计，传递给 AgentLoopSharedState）
    pub sub_agent_metrics: Arc<Mutex<crate::command::chat::tools::derived_shared::SubAgentMetrics>>,
    /// 延迟加载的工具列表（与 DerivedAgentShared 共享，UI 可修改）
    pub deferred_tools: Arc<Mutex<Vec<String>>>,
    /// 本会话通过 LoadTool 加载的 deferred 工具（会话级，不写入用户配置）
    pub session_loaded_deferred: Arc<Mutex<Vec<String>>>,
    /// 会话内已调用技能追踪（LoadSkill 执行时记录，auto_compact 后恢复）
    pub invoked_skills: crate::command::chat::context::compact::InvokedSkillsMap,
    /// 项目文件索引（后台维护，弹窗使用）
    pub file_index: FileIndex,
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
        let display_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(Vec::new()));
        let context_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(Vec::new()));
        let teammate_manager: Arc<Mutex<TeammateManager>> =
            Arc::new(Mutex::new(TeammateManager::new(
                Arc::clone(&pending_user_messages),
                Arc::clone(&display_messages),
                Arc::clone(&context_messages),
            )));
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
        let deferred_tools_arc: Arc<Mutex<Vec<String>>> =
            Arc::new(Mutex::new(agent_config.deferred_tools.clone()));
        let session_loaded_deferred_arc: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        // 子 agent 权限请求队列（TUI 和所有 agent 共享同一个 Arc）
        let permission_queue = Arc::new(PermissionQueue::new());
        // Plan 审批请求队列（TUI 和所有 teammate 共享同一个 Arc）
        let plan_approval_queue = Arc::new(PlanApprovalQueue::new());
        // 子 Agent 快照追踪器（/dump 从中读取正在运行的子 Agent）
        let sub_agent_tracker = Arc::new(SubAgentTracker::new());

        // 共享的 session id 槽：session 切换时 chat_app 会同步更新，teammate/subagent 据此定位 transcript
        let shared_session_id = Arc::new(Mutex::new(session_id.clone()));

        // 子 agent 上下文配置快照（send_message 时刷新）
        let agent_context_config = Arc::new(Mutex::new(
            crate::command::chat::tools::derived_shared::AgentContextConfig {
                max_history_messages: agent_config.max_history_messages,
                max_context_tokens: agent_config.max_context_tokens,
                compact: agent_config.compact.clone(),
            },
        ));
        // 子 agent 使用的 disabled_hooks 快照（Teammate 走 hook 链时用）
        let shared_disabled_hooks = Arc::new(Mutex::new(agent_config.disabled_hooks.clone()));

        // 构建 DerivedAgentShared（SubAgentTool / TeammateTool 共用）
        let shared_sub_agent_metrics = Arc::new(Mutex::new(
            crate::command::chat::tools::derived_shared::SubAgentMetrics::default(),
        ));
        let derived_agent_shared = DerivedAgentShared {
            background_manager: Arc::clone(&background_manager),
            provider: Arc::clone(&agent_provider),
            system_prompt: Arc::clone(&agent_system_prompt),
            jcli_config: Arc::new(JcliConfig::load()),
            hook_manager: Arc::clone(&hook_manager),
            task_manager: Arc::clone(&task_manager),
            disabled_tools: Arc::clone(&disabled_tools_arc),
            deferred_tools: Arc::clone(&deferred_tools_arc),
            session_loaded_deferred: Arc::clone(&session_loaded_deferred_arc),
            permission_queue: Arc::clone(&permission_queue),
            plan_approval_queue: Arc::clone(&plan_approval_queue),
            sub_agent_tracker: Arc::clone(&sub_agent_tracker),
            display_messages: Arc::clone(&display_messages),
            context_messages: Arc::clone(&context_messages),
            session_id: Arc::clone(&shared_session_id),
            plan_mode_state: Arc::clone(&tool_registry.plan_mode_state),
            agent_context_config: Arc::clone(&agent_context_config),
            disabled_hooks: Arc::clone(&shared_disabled_hooks),
            sub_agent_metrics: Arc::clone(&shared_sub_agent_metrics),
        };
        tool_registry.register(Box::new(
            crate::command::chat::tools::sub_agent::SubAgentTool {
                shared: derived_agent_shared.clone(),
            },
        ));
        tool_registry.register(Box::new(
            crate::command::chat::tools::teammate_tool::TeammateTool {
                shared: derived_agent_shared,
                teammate_manager: Arc::clone(&teammate_manager),
            },
        ));
        tool_registry.register(Box::new(
            crate::command::chat::tools::send_message::SendMessageTool {
                teammate_manager: Arc::clone(&teammate_manager),
            },
        ));
        tool_registry.register(Box::new(
            crate::command::chat::tools::ignore_message::IgnoreMessageTool {
                teammate_manager: Some(Arc::clone(&teammate_manager)),
            },
        ));
        // 注册 LoadTool，它持有 deferred_tools + session_loaded_deferred 的共享引用
        tool_registry.register(Box::new(
            crate::command::chat::tools::load_tool::LoadTool::new(
                Arc::clone(&deferred_tools_arc),
                Arc::clone(&session_loaded_deferred_arc),
            ),
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
                                reasoning_content: None,
                                sender_name: None,
                                recipient_name: None,
                                display_hint: DisplayHint::Normal,
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
                        reasoning_content: None,
                        sender_name: None,
                        recipient_name: None,
                        display_hint: DisplayHint::Normal,
                    }];
                    Some(HookResult {
                        inject_messages: Some(inject),
                        ..Default::default()
                    })
                },
            );

            // 内置 hook 6: broadcast_compress — 折叠来自其他 agent 的 tool call 广播
            //
            // 注册在末位，确保它在所有其他 hook（含 inject_messages）之后执行，
            // 这样即便有 hook 追加了 <Name> [调用工具 X] 格式的消息也能被折叠。
            // self_agent_name 取自线程本地身份：Main 线程返回 "Main"，teammate 线程返回
            // 其 teammate 名，SubAgent 线程返回 sub_id（SubAgent 的 messages 里几乎不会有
            // 广播，折叠无副作用）。
            manager.register_builtin(HookEvent::PreLlmRequest, "broadcast_compress", |ctx| {
                let messages = ctx.messages.as_ref()?;
                let self_name = crate::command::chat::agent::thread_identity::current_agent_name();
                let compressed = compress_other_agent_toolcalls(
                    messages,
                    &self_name,
                    DEFAULT_OTHER_AGENT_TOOLCALL_THRESHOLD,
                );
                if compressed.len() == messages.len() {
                    return None;
                }
                Some(HookResult {
                    messages: Some(compressed),
                    ..Default::default()
                })
            });
        }

        let new_app = Self {
            ui: UIState {
                input_buffer: TextBuffer::new(),
                mode: ChatMode::Chat,
                scroll_offset: usize::MAX,
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
                model_in_fields: false,
                model_field_idx: 0,
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
                tool_ask_drafts: Vec::new(),
                pending_system_prompt_edit: false,
                pending_agent_md_edit: false,
                pending_style_edit: false,
                image_cache: Arc::new(Mutex::new(ImageCache::new())),
                expand_tools: false,
                config_scroll_offset: 0,
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
                tools_in_options: false,
                tools_option_idx: 0,
                auto_approve: false,
                commands_mode: CommandsMode::Normal,
                commands_source_idx: 0,
                pending_command_create: false,
                command_create_source: CommandSource::User,
                mouse_selection: None,
                msg_area_inner: None,
                context_menu: None,
                help_lines_cache: None,
                help_area_inner: None,
                help_scroll_offset: 0,
                config_list_area: None,
                config_tab_bar_y: None,
                config_field_lines: Vec::new(),
                config_tab_hitboxes: Vec::new(),
            },
            state: ChatState {
                agent_config,
                streaming_content: Arc::new(Mutex::new(String::new())),
                streaming_reasoning_content: Arc::new(Mutex::new(String::new())),
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
            persisted_display_count: 0,
            ws_bridge: None,
            remote_connected: false,
            derived_agent_provider: agent_provider,
            derived_agent_system_prompt: agent_system_prompt,
            derived_agent_context_config: agent_context_config,
            derived_agent_disabled_hooks: shared_disabled_hooks,
            display_messages,
            display_read_offset: 0,
            context_messages,
            context_read_offset: 0,
            context_tokens: Arc::new(Mutex::new(0)),
            teammate_manager,
            sub_agent_tracker,
            permission_queue,
            plan_approval_queue,
            sub_agent_metrics: shared_sub_agent_metrics,
            deferred_tools: deferred_tools_arc,
            session_loaded_deferred: session_loaded_deferred_arc,
            invoked_skills,
            file_index: FileIndex::new(),
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
                    messages: Some(
                        safe_lock(&new_app.context_messages, "SessionStart::ctx_msgs").clone(),
                    ),
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
        let messages: Vec<SyncMessage> = safe_lock(&self.context_messages, "build_sync_outbound")
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
        let context_tokens = *safe_lock(&self.context_tokens, "build_sync_outbound::ctx_tokens");
        let message_count =
            safe_lock(&self.context_messages, "build_sync_outbound::msg_count").len();
        WsOutbound::SessionSync {
            messages,
            status: status.to_string(),
            model,
            context_tokens,
            message_count,
            auto_approve: self.ui.auto_approve,
        }
    }

    /// 广播配置数据到远程客户端
    pub fn broadcast_config_state(&mut self) {
        use crate::command::chat::remote::protocol::{ConfigField, ModelInfo, ThemeInfo};

        let tab = match self.ui.config_tab {
            ConfigTab::Model => "model",
            ConfigTab::Session => "session",
            ConfigTab::Global => "global",
            ConfigTab::Tools => "tools",
            ConfigTab::Skills => "skills",
            ConfigTab::Hooks => "hooks",
            ConfigTab::Commands => "commands",
            ConfigTab::Teammates => "teammates",
            ConfigTab::Archive => "archive",
        };

        let fields = match self.ui.config_tab {
            ConfigTab::Model => {
                let mut fields = Vec::new();
                for (i, p) in self.state.agent_config.providers.iter().enumerate() {
                    let is_active = i == self.state.agent_config.active_index;
                    fields.push(ConfigField {
                        key: format!("provider_{}", i),
                        label: p.name.clone(),
                        value: format!("{} @ {}", p.model, p.api_base),
                        field_type: "select".to_string(),
                        editable: false,
                        options: None,
                    });
                    if is_active {
                        fields.push(ConfigField {
                            key: "active_provider".into(),
                            label: "当前模型".into(),
                            value: p.name.clone(),
                            field_type: "text".into(),
                            editable: false,
                            options: None,
                        });
                    }
                }
                // 也发送模型列表供快速切换
                let models: Vec<ModelInfo> = self
                    .state
                    .agent_config
                    .providers
                    .iter()
                    .map(|p| ModelInfo {
                        name: p.name.clone(),
                        model: p.model.clone(),
                        provider: p.api_base.clone(),
                        supports_vision: p.supports_vision,
                    })
                    .collect();
                self.broadcast_ws(WsOutbound::ModelList {
                    models,
                    active_index: self.state.agent_config.active_index,
                });
                // 同时发送主题列表供远程快速切换
                {
                    use crate::theme::ThemeName;
                    let all_themes = ThemeName::all();
                    let themes: Vec<ThemeInfo> = all_themes
                        .iter()
                        .map(|t| ThemeInfo {
                            name: t.to_str().to_string(),
                            display_name: t.display_name().to_string(),
                        })
                        .collect();
                    let active_idx = all_themes
                        .iter()
                        .position(|n| *n == self.state.agent_config.theme)
                        .unwrap_or(0);
                    self.broadcast_ws(WsOutbound::ThemeList {
                        themes,
                        active_index: active_idx,
                    });
                }
                fields
            }
            ConfigTab::Global => {
                let cfg = &self.state.agent_config;
                vec![
                    ConfigField {
                        key: "max_history_messages".into(),
                        label: "最大历史消息数".into(),
                        value: cfg.max_history_messages.to_string(),
                        field_type: "text".into(),
                        editable: true,
                        options: None,
                    },
                    ConfigField {
                        key: "max_context_tokens".into(),
                        label: "最大上下文 Token".into(),
                        value: cfg.max_context_tokens.to_string(),
                        field_type: "text".into(),
                        editable: true,
                        options: None,
                    },
                    ConfigField {
                        key: "max_tool_rounds".into(),
                        label: "最大工具轮数".into(),
                        value: cfg.max_tool_rounds.to_string(),
                        field_type: "text".into(),
                        editable: true,
                        options: None,
                    },
                    ConfigField {
                        key: "tools_enabled".into(),
                        label: "启用工具".into(),
                        value: cfg.tools_enabled.to_string(),
                        field_type: "bool".into(),
                        editable: true,
                        options: None,
                    },
                    ConfigField {
                        key: "tool_confirm_timeout".into(),
                        label: "工具确认超时(秒)".into(),
                        value: cfg.tool_confirm_timeout.to_string(),
                        field_type: "text".into(),
                        editable: true,
                        options: None,
                    },
                ]
            }
            ConfigTab::Tools => {
                let all_tools: Vec<String> = self
                    .tool_registry
                    .tool_names()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();
                let disabled = &self.state.agent_config.disabled_tools;
                all_tools
                    .into_iter()
                    .map(|name| ConfigField {
                        key: name.clone(),
                        label: name.clone(),
                        value: (!disabled.contains(&name)).to_string(),
                        field_type: "bool".into(),
                        editable: true,
                        options: None,
                    })
                    .collect()
            }
            ConfigTab::Skills => {
                let skills = &self.state.loaded_skills;
                let disabled = &self.state.agent_config.disabled_skills;
                skills
                    .iter()
                    .map(|s| {
                        let name = s.frontmatter.name.clone();
                        ConfigField {
                            key: name.clone(),
                            label: name.clone(),
                            value: (!disabled.contains(&name)).to_string(),
                            field_type: "bool".into(),
                            editable: true,
                            options: None,
                        }
                    })
                    .collect()
            }
            ConfigTab::Session
            | ConfigTab::Archive
            | ConfigTab::Hooks
            | ConfigTab::Commands
            | ConfigTab::Teammates => vec![],
        };

        self.broadcast_ws(WsOutbound::ConfigData {
            tab: tab.to_string(),
            fields,
        });
    }

    /// 广播归档确认状态到远程客户端
    pub fn broadcast_archive_confirm_state(&self) {
        // ArchiveConfirm 状态已通过 session_sync 的 status 字段表达
        // 这里额外广播默认归档名
        self.broadcast_ws(WsOutbound::Status {
            state: "archive_confirm".to_string(),
        });
    }

    /// 广播归档列表到远程客户端
    pub fn broadcast_archive_list_state(&self) {
        use crate::command::chat::remote::protocol::ArchiveInfo;
        let archives: Vec<ArchiveInfo> = self
            .ui
            .archives
            .iter()
            .map(|a| ArchiveInfo {
                name: a.name.clone(),
                created_at: a.created_at.clone(),
                message_count: a.messages.len(),
            })
            .collect();
        self.broadcast_ws(WsOutbound::ArchiveList { archives });
    }

    /// 广播会话列表状态到远程客户端
    pub fn broadcast_session_list_state(&self) {
        let sessions = crate::command::chat::storage::list_sessions();
        self.broadcast_ws(WsOutbound::SessionList { sessions });
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
            // agent loop 运行中：追加到 pending 队列 + 双通道，下一轮 loop 会处理
            let user_msg = ChatMessage::text(MessageRole::User, &text);
            self.push_both_channels(user_msg);
            {
                let mut pending = crate::util::safe_lock(
                    &self.state.pending_user_messages,
                    "inject_remote_message::pending",
                );
                pending.push(ChatMessage::text(MessageRole::User, &text));
            }
            self.ui.msg_lines_cache = None;
            self.ui.auto_scroll = true;
            self.ui.scroll_offset = usize::MAX;
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
    /// 同时停止所有 teammates，确保 Esc 按键后 UI 瞬间恢复可交互状态。
    pub fn cancel_stream(&mut self) {
        // 停止所有 teammates
        if let Ok(mut mgr) = self.teammate_manager.lock() {
            mgr.stop_all();
        }
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
        } else {
            self.reset_tool_confirm_interact_state();
        }
    }

    /// 拒绝当前待处理工具（兼容旧接口）
    pub fn reject_pending_tool(&mut self, reason: &str) {
        if let Some(new_mode) = self.tool_executor.reject_current(reason) {
            self.ui.mode = new_mode;
        } else {
            self.reset_tool_confirm_interact_state();
        }
    }

    /// 允许并执行当前待处理工具（兼容旧接口）
    pub fn allow_and_execute_pending_tool(&mut self) {
        if let Some(new_mode) = self
            .tool_executor
            .allow_and_execute(&self.tool_registry, &mut self.jcli_config)
        {
            self.ui.mode = new_mode;
        } else {
            self.reset_tool_confirm_interact_state();
        }
    }

    fn reset_tool_confirm_interact_state(&mut self) {
        self.ui.tool_interact_selected = 0;
        self.ui.tool_interact_typing = false;
        self.ui.tool_interact_input.clear();
        self.ui.tool_interact_cursor = 0;
    }

    // ── 远程文件/终端操作（静态方法） ──

    pub fn handle_file_list(path: &str) -> Vec<crate::command::chat::remote::protocol::FileEntry> {
        let dir = if path.is_empty() { "." } else { path };
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(dir) {
            let mut dirs: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
            dirs.sort_by(|a, b| {
                let a_dir = a.file_type().map(|t| !t.is_dir()).unwrap_or(true);
                let b_dir = b.file_type().map(|t| !t.is_dir()).unwrap_or(true);
                b_dir
                    .cmp(&a_dir)
                    .then_with(|| a.file_name().cmp(&b.file_name()))
            });
            for entry in dirs {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let modified = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs().to_string())
                    .unwrap_or_default();
                entries.push(crate::command::chat::remote::protocol::FileEntry {
                    name,
                    is_dir,
                    size,
                    modified,
                });
            }
        }
        entries
    }

    pub fn handle_file_read(path: &str) -> (String, Option<String>) {
        match std::fs::read_to_string(path) {
            Ok(content) => (content, None),
            Err(e) => (String::new(), Some(e.to_string())),
        }
    }

    pub fn handle_file_write(path: &str, content: &str) -> (bool, Option<String>) {
        match std::fs::write(path, content) {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        }
    }

    pub fn handle_terminal_exec(command: &str) -> (String, Option<i32>) {
        use std::process::Command;
        let output = Command::new("sh").arg("-c").arg(command).output();
        match output {
            Ok(out) => {
                let mut result = String::new();
                if !out.stdout.is_empty() {
                    result.push_str(&String::from_utf8_lossy(&out.stdout));
                }
                if !out.stderr.is_empty() {
                    if !result.is_empty() {
                        result.push('\n');
                    }
                    result.push_str(&String::from_utf8_lossy(&out.stderr));
                }
                let exit_code = out.status.code();
                (result, exit_code)
            }
            Err(e) => (e.to_string(), None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ChatApp;
    use crate::command::chat::app::types::{ToolCallStatus, ToolExecStatus};
    use crate::command::chat::app::ui_state::ChatMode;

    #[test]
    fn reject_with_reason_resets_tool_confirm_ui_for_next_pending_tool() {
        let mut app = ChatApp::new("test-reject-next-tool".to_string());
        app.ui.mode = ChatMode::ToolConfirm;
        app.ui.tool_interact_selected = 3;
        app.ui.tool_interact_typing = true;
        app.ui.tool_interact_input = "不要执行".to_string();
        app.ui.tool_interact_cursor = app.ui.tool_interact_input.chars().count();
        app.tool_executor.active_tool_calls = vec![
            ToolCallStatus {
                tool_call_id: "tool-1".to_string(),
                tool_name: "Write".to_string(),
                arguments: "{}".to_string(),
                confirm_message: "first".to_string(),
                status: ToolExecStatus::PendingConfirm,
                tool_description: None,
            },
            ToolCallStatus {
                tool_call_id: "tool-2".to_string(),
                tool_name: "Edit".to_string(),
                arguments: "{}".to_string(),
                confirm_message: "second".to_string(),
                status: ToolExecStatus::PendingConfirm,
                tool_description: None,
            },
        ];

        app.reject_pending_tool("不要执行");

        assert!(matches!(app.ui.mode, ChatMode::ToolConfirm));
        assert_eq!(app.tool_executor.pending_tool_idx, 1);
        assert_eq!(app.ui.tool_interact_selected, 0);
        assert!(!app.ui.tool_interact_typing);
        assert!(app.ui.tool_interact_input.is_empty());
        assert_eq!(app.ui.tool_interact_cursor, 0);
    }

    #[test]
    fn reject_last_pending_tool_exits_tool_confirm_mode() {
        let mut app = ChatApp::new("test-reject-last-tool".to_string());
        app.ui.mode = ChatMode::ToolConfirm;
        app.ui.tool_interact_selected = 3;
        app.ui.tool_interact_typing = true;
        app.ui.tool_interact_input = "不要执行".to_string();
        app.ui.tool_interact_cursor = app.ui.tool_interact_input.chars().count();
        app.tool_executor.active_tool_calls = vec![ToolCallStatus {
            tool_call_id: "tool-1".to_string(),
            tool_name: "Write".to_string(),
            arguments: "{}".to_string(),
            confirm_message: "only".to_string(),
            status: ToolExecStatus::PendingConfirm,
            tool_description: None,
        }];

        app.reject_pending_tool("不要执行");

        assert!(matches!(app.ui.mode, ChatMode::Chat));
    }
}
