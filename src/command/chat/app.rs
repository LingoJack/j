use super::agent::run_agent_loop;
use super::markdown::image_cache::ImageCache;
use super::model::{
    AgentConfig, ChatMessage, ChatSession, ModelProvider, ToolCallItem, load_agent_config,
    load_chat_session, load_memory, load_soul, load_style, load_system_prompt, memory_path,
    save_agent_config, save_chat_session, save_memory, save_soul, save_system_prompt, soul_path,
    system_prompt_path,
};
use super::permission::JcliConfig;
use super::skill::{self, Skill};
use super::theme::Theme;
use super::tools::ToolRegistry;
use super::tools::background::BackgroundManager;
use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS, TOAST_DURATION_SECS};
use crate::util::log::write_info_log;
use ratatui::text::Line;
use ratatui::widgets::ListState;
use std::sync::{Arc, Mutex, mpsc};
use tokio_util::sync::CancellationToken;
// ========== TUI 界面 ==========

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
}

/// 工具后台线程 → 主线程的执行完成消息
pub struct ToolExecDoneMsg {
    pub tool_call_id: String,
    pub output: String,
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

/// TUI 应用状态
pub struct ChatApp {
    /// Agent 配置
    pub agent_config: AgentConfig,
    /// 当前对话会话
    pub session: ChatSession,
    /// 输入缓冲区
    pub input: String,
    /// 光标位置（字符索引）
    pub cursor_pos: usize,
    /// 当前模式
    pub mode: ChatMode,
    /// 消息列表滚动偏移
    pub scroll_offset: u16,
    /// 是否正在等待 AI 回复
    pub is_loading: bool,
    /// 模型选择列表状态
    pub model_list_state: ListState,
    /// Toast 通知消息 (内容, 是否错误, 创建时间)
    pub toast: Option<(String, bool, std::time::Instant)>,
    /// 用于接收后台流式回复的 channel
    pub stream_rx: Option<mpsc::Receiver<StreamMsg>>,
    /// 当前正在流式接收的 AI 回复内容（实时更新）
    pub streaming_content: Arc<Mutex<String>>,
    /// 消息渲染行缓存：(消息数, 最后一条消息内容hash, 气泡宽度) → 渲染好的行
    /// 避免每帧都重新解析 Markdown
    pub msg_lines_cache: Option<MsgLinesCache>,
    /// 消息浏览模式中选中的消息索引
    pub browse_msg_index: usize,
    /// 浏览模式下当前消息内部的滚动偏移（相对于消息起始行，A/D 控制）
    pub browse_scroll_offset: u16,
    /// 流式节流：上次实际渲染流式内容时的长度
    pub last_rendered_streaming_len: usize,
    /// 流式节流：上次实际渲染流式内容的时间
    pub last_stream_render_time: std::time::Instant,
    /// 配置界面：当前选中的 provider 索引
    pub config_provider_idx: usize,
    /// 配置界面：当前选中的字段索引
    pub config_field_idx: usize,
    /// 配置界面：是否正在编辑某个字段
    pub config_editing: bool,
    /// 配置界面：编辑缓冲区
    pub config_edit_buf: String,
    /// 配置界面：编辑光标位置
    pub config_edit_cursor: usize,
    /// 流式输出时是否自动滚动到底部（用户手动上滚后关闭，发送新消息或滚到底部时恢复）
    pub auto_scroll: bool,
    /// 当前主题
    pub theme: Theme,
    /// 归档列表（缓存）
    pub archives: Vec<super::archive::ChatArchive>,
    /// 归档列表选中索引
    pub archive_list_index: usize,
    /// 归档确认模式的默认名称
    pub archive_default_name: String,
    /// 归档确认模式的用户自定义名称
    pub archive_custom_name: String,
    /// 归档确认模式是否正在编辑名称
    pub archive_editing_name: bool,
    /// 归档确认模式的光标位置
    pub archive_edit_cursor: usize,
    /// 还原确认模式：是否需要确认当前会话有消息
    pub restore_confirm_needed: bool,
    /// 工具结果发送通道（主线程 → 后台线程）
    pub tool_result_tx: Option<mpsc::SyncSender<ToolResultMsg>>,
    /// 工具注册表
    pub tool_registry: Arc<ToolRegistry>,
    /// .jcli 权限配置
    pub jcli_config: Arc<JcliConfig>,
    /// 后台任务管理器
    pub background_manager: Arc<BackgroundManager>,
    /// 当前活跃的工具调用状态列表
    pub active_tool_calls: Vec<ToolCallStatus>,
    /// ToolConfirm 模式中当前待处理工具的索引
    pub pending_tool_idx: usize,
    /// 进入 ToolConfirm 模式的时间（用于超时自动执行）
    pub tool_confirm_entered_at: std::time::Instant,
    /// 配置界面：是否有待处理的 system_prompt 编辑（需弹出全屏编辑器）
    pub pending_system_prompt_edit: bool,
    /// 已加载的 skills（用于补全和高亮）
    pub loaded_skills: Vec<Skill>,
    /// @ 补全弹窗是否激活
    pub at_popup_active: bool,
    /// @ 之后的过滤文本
    pub at_popup_filter: String,
    /// @ 在 input 中的字符索引
    pub at_popup_start_pos: usize,
    /// 弹窗中选中项索引
    pub at_popup_selected: usize,
    /// 文件补全弹窗是否激活
    pub file_popup_active: bool,
    /// @file: 在 input 中的起始字符索引
    pub file_popup_start_pos: usize,
    /// @file: 之后的路径过滤文本
    pub file_popup_filter: String,
    /// 文件弹窗中选中项索引
    pub file_popup_selected: usize,
    /// 配置界面：是否有待处理的 style 编辑（需弹出全屏编辑器）
    pub pending_style_edit: bool,
    /// 流式请求取消令牌（存在时表示有进行中的请求）
    pub cancel_token: Option<CancellationToken>,
    /// 工具执行取消标志（Esc 时设为 true，让 ShellTool kill 子进程）
    pub tool_cancelled: Arc<std::sync::atomic::AtomicBool>,
    /// 是否有待执行的工具（已设为 Executing 状态但尚未实际调用，等待下一帧重绘后再执行）
    pub pending_tool_execution: bool,
    /// 工具后台线程 → 主线程的执行结果 channel（发送端，保持复用）
    pub tool_exec_tx: Option<mpsc::Sender<ToolExecDoneMsg>>,
    /// 工具后台线程 → 主线程的执行结果 channel
    pub tool_exec_rx: Option<mpsc::Receiver<ToolExecDoneMsg>>,
    /// 当前正在后台执行的工具数量（用于等待所有工具完成）
    pub tools_executing_count: usize,
    /// 统一交互区：当前选中项索引（0=continue, 1=refuse, 2=type）
    pub tool_interact_selected: usize,
    /// 统一交互区：是否处于输入模式
    pub tool_interact_typing: bool,
    /// 统一交互区：输入缓冲
    pub tool_interact_input: String,
    /// 统一交互区：输入光标位置
    pub tool_interact_cursor: usize,
    /// 是否为 ask 工具的交互模式（区别于普通工具确认）
    pub tool_ask_mode: bool,
    /// ask 工具的所有问题
    pub tool_ask_questions: Vec<AskQuestion>,
    /// ask 工具当前问题索引
    pub tool_ask_current_idx: usize,
    /// ask 工具每题答案
    pub tool_ask_answers: Vec<AskAnswer>,
    /// ask 工具当前问题各选项的选中状态（多选用）
    pub tool_ask_selections: Vec<bool>,
    /// ask 工具当前问题的选项游标位置
    pub tool_ask_cursor: usize,
    /// ask 工具响应发送通道（用于向 ask 工具发送用户响应）
    pub ask_response_tx: Option<mpsc::Sender<String>>,
    /// ask 工具请求接收通道（主线程接收 ask 请求）
    pub ask_request_rx: Option<mpsc::Receiver<AskRequest>>,
    /// 排队的任务列表（new_task 工具产生，当前任务完成后自动执行）
    pub queued_tasks: Arc<Mutex<Vec<String>>>,
    /// 用户在 agent loop 期间发送的待处理消息队列（单向：主线程 push → agent loop drain）
    pub pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// 图片缓存（渲染终端图片）
    pub image_cache: Arc<Mutex<ImageCache>>,
    /// 工具开关子菜单中选中的索引
    pub tool_toggle_index: usize,
    /// Skill 开关子菜单中选中的索引
    pub skill_toggle_index: usize,
}

/// 消息渲染行缓存
pub struct MsgLinesCache {
    /// 会话消息数量
    pub msg_count: usize,
    /// 最后一条消息的内容长度（用于检测流式更新）
    pub last_msg_len: usize,
    /// 流式内容长度
    pub streaming_len: usize,
    /// 是否正在加载
    pub is_loading: bool,
    /// 气泡最大宽度（窗口变化时需要重算）
    pub bubble_max_width: usize,
    /// 浏览模式选中索引（None 表示非浏览模式）
    pub browse_index: Option<usize>,
    /// 工具确认模式中待处理工具的索引（None 表示非确认模式）
    pub tool_confirm_idx: Option<usize>,
    /// 缓存的渲染行
    pub lines: Vec<Line<'static>>,
    /// 每条消息（按 msg_index）的起始行号（用于浏览模式自动滚动）
    pub msg_start_lines: Vec<(usize, usize)>, // (msg_index, start_line)
    /// 按消息粒度缓存：每条历史消息的渲染行（key: 消息索引）
    pub per_msg_lines: Vec<PerMsgCache>,
    /// 流式增量渲染缓存：已完成段落的渲染行
    pub streaming_stable_lines: Vec<Line<'static>>,
    /// 流式增量渲染缓存：已缓存到 streaming_content 的字节偏移
    pub streaming_stable_offset: usize,
}

/// 单条消息的渲染缓存
pub struct PerMsgCache {
    /// 消息内容长度（用于检测变化）
    pub content_len: usize,
    /// 渲染好的行
    pub lines: Vec<Line<'static>>,
    /// 对应的 msg_start_line（此消息在全局行列表中的起始行号，需在拼装时更新）
    pub msg_index: usize,
    /// 渲染时此消息是否被选中（用于浏览模式下检测选中状态变化）
    pub is_selected: bool,
}

#[derive(PartialEq)]
pub enum ChatMode {
    /// 正常对话模式（焦点在输入框）
    Chat,
    /// 模型选择模式
    SelectModel,
    /// 消息浏览模式（可选中消息并复制）
    Browse,
    /// 帮助
    Help,
    /// 配置编辑模式
    Config,
    /// 归档确认模式（确认归档名称）
    ArchiveConfirm,
    /// 归档列表模式（查看和还原归档）
    ArchiveList,
    /// 工具调用确认模式（选项式交互区域）
    ToolConfirm,
    /// 工具开关子菜单模式（逐个启用/禁用工具）
    ToolToggle,
    /// Skill 开关子菜单模式（逐个启用/禁用 skill）
    SkillToggle,
}

/// 所有字段数 = provider 字段 + 全局字段
pub fn config_total_fields() -> usize {
    CONFIG_FIELDS.len() + CONFIG_GLOBAL_FIELDS.len()
}

impl ChatApp {
    pub fn new() -> Self {
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
        // 安装预设 skills（assets/skills/ 目录下的所有项目，仅当 skill 目录不存在时才写入）
        crate::assets::install_default_skills(&skill::skills_dir());

        let session = load_chat_session();
        let mut model_list_state = ListState::default();
        if !agent_config.providers.is_empty() {
            model_list_state.select(Some(agent_config.active_index));
        }
        let theme = Theme::from_name(&agent_config.theme);
        let loaded_skills = skill::load_all_skills();
        let (ask_req_tx, ask_req_rx) = mpsc::channel::<AskRequest>();
        let queued_tasks: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let background_manager = Arc::new(BackgroundManager::new());
        let task_manager = Arc::new(super::tools::task::TaskManager::new());
        let tool_registry = Arc::new(ToolRegistry::new(
            loaded_skills.clone(),
            ask_req_tx,
            Arc::clone(&background_manager),
            Arc::clone(&task_manager),
        ));
        let jcli_config = Arc::new(JcliConfig::load());
        Self {
            agent_config,
            session,
            input: String::new(),
            cursor_pos: 0,
            mode: ChatMode::Chat,
            scroll_offset: u16::MAX, // 默认滚动到底部
            is_loading: false,
            model_list_state,
            toast: None,
            stream_rx: None,
            streaming_content: Arc::new(Mutex::new(String::new())),
            msg_lines_cache: None,
            browse_msg_index: 0,
            browse_scroll_offset: 0,
            last_rendered_streaming_len: 0,
            last_stream_render_time: std::time::Instant::now(),
            config_provider_idx: 0,
            config_field_idx: 0,
            config_editing: false,
            config_edit_buf: String::new(),
            config_edit_cursor: 0,
            auto_scroll: true,
            theme,
            archives: Vec::new(),
            archive_list_index: 0,
            archive_default_name: String::new(),
            archive_custom_name: String::new(),
            archive_editing_name: false,
            archive_edit_cursor: 0,
            restore_confirm_needed: false,
            tool_result_tx: None,
            tool_registry,
            jcli_config,
            background_manager,
            active_tool_calls: Vec::new(),
            pending_tool_idx: 0,
            tool_confirm_entered_at: std::time::Instant::now(),
            pending_system_prompt_edit: false,
            loaded_skills,
            at_popup_active: false,
            at_popup_filter: String::new(),
            at_popup_start_pos: 0,
            at_popup_selected: 0,
            file_popup_active: false,
            file_popup_start_pos: 0,
            file_popup_filter: String::new(),
            file_popup_selected: 0,
            pending_style_edit: false,
            cancel_token: None,
            tool_cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            pending_tool_execution: false,
            tool_exec_tx: None,
            tool_exec_rx: None,
            tools_executing_count: 0,
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
            ask_response_tx: None,
            ask_request_rx: Some(ask_req_rx),
            queued_tasks,
            pending_user_messages: Arc::new(Mutex::new(Vec::new())),
            image_cache: Arc::new(Mutex::new(ImageCache::new())),
            tool_toggle_index: 0,
            skill_toggle_index: 0,
        }
    }

    /// 解析系统提示词模板，替换 {{.skills}}、{{.tools}}、{{.style}}、{{.memory}}、{{.soul}} 占位符
    /// 每次调用时动态加载所有相关文件，确保内容最新
    pub fn resolve_system_prompt(&self) -> Option<String> {
        let template = load_system_prompt()?;
        let skills_summary =
            skill::build_skills_summary(&self.loaded_skills, &self.agent_config.disabled_skills);
        let tools_summary = self
            .tool_registry
            .build_tools_summary(&self.agent_config.disabled_tools);
        let style_text = load_style().unwrap_or_else(|| "（未设置）".to_string());
        let memory_text = load_memory().unwrap_or_default();
        let soul_text = load_soul().unwrap_or_default();

        let resolved = template
            .replace("{{.skills}}", &skills_summary)
            .replace("{{.tools}}", &tools_summary)
            .replace("{{.style}}", &style_text)
            .replace("{{.memory}}", &memory_text)
            .replace("{{.soul}}", &soul_text);
        Some(resolved)
    }

    /// 切换到下一个主题
    pub fn switch_theme(&mut self) {
        self.agent_config.theme = self.agent_config.theme.next();
        self.theme = Theme::from_name(&self.agent_config.theme);
        self.msg_lines_cache = None; // 清除缓存以触发重绘
    }

    /// 显示一条 toast 通知
    pub fn show_toast(&mut self, msg: impl Into<String>, is_error: bool) {
        self.toast = Some((msg.into(), is_error, std::time::Instant::now()));
    }

    /// 清理过期的 toast
    pub fn tick_toast(&mut self) {
        if let Some((_, _, created)) = &self.toast
            && created.elapsed().as_secs() >= TOAST_DURATION_SECS
        {
            self.toast = None;
        }
    }

    /// 获取当前活跃的 provider
    pub fn active_provider(&self) -> Option<&ModelProvider> {
        if self.agent_config.providers.is_empty() {
            return None;
        }
        let idx = self
            .agent_config
            .active_index
            .min(self.agent_config.providers.len() - 1);
        Some(&self.agent_config.providers[idx])
    }

    /// 获取当前模型名称
    pub fn active_model_name(&self) -> String {
        self.active_provider()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "未配置".to_string())
    }

    /// 构建发送给 API 的消息列表
    pub fn build_api_messages(&self) -> Vec<ChatMessage> {
        // 只取最近的 N 条历史消息，避免 token 消耗过大
        let max_history = self.agent_config.max_history_messages;
        if self.session.messages.len() > max_history {
            self.session.messages[self.session.messages.len() - max_history..].to_vec()
        } else {
            self.session.messages.clone()
        }
    }

    /// 发送消息（非阻塞，启动后台线程流式接收）
    pub fn send_message(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return;
        }

        // 关闭弹窗
        self.at_popup_active = false;
        self.file_popup_active = false;
        self.input.clear();
        self.cursor_pos = 0;

        self.send_message_internal(text);
    }

    /// 发送指定文本消息并启动 agent loop（供 send_message 和 queued_tasks 调用）
    pub fn send_message_internal(&mut self, text: String) {
        // 添加用户消息
        self.session.messages.push(ChatMessage::text("user", &text));
        // 发送新消息时恢复自动滚动并滚到底部
        self.auto_scroll = true;
        self.scroll_offset = u16::MAX;

        // 调用 API
        let provider = match self.active_provider() {
            Some(p) => p.clone(),
            None => {
                self.show_toast("未配置模型提供方，请先编辑配置文件", true);
                return;
            }
        };

        self.is_loading = true;
        // 重置流式节流状态和缓存
        self.last_rendered_streaming_len = 0;
        self.last_stream_render_time = std::time::Instant::now();
        self.msg_lines_cache = None;
        self.active_tool_calls.clear();
        self.pending_tool_idx = 0;
        self.tool_exec_tx = None;
        self.tool_exec_rx = None;
        self.tools_executing_count = 0;
        self.pending_tool_execution = false;
        // 重置工具取消标志
        self.tool_cancelled
            .store(false, std::sync::atomic::Ordering::Relaxed);

        let api_messages = self.build_api_messages();

        // 清空待处理用户消息队列
        {
            let mut pending = self.pending_user_messages.lock().unwrap();
            pending.clear();
        }

        // 清空流式内容缓冲
        {
            let mut sc = self.streaming_content.lock().unwrap();
            sc.clear();
        }

        // 创建双向 channel
        let (stream_tx, stream_rx) = mpsc::channel::<StreamMsg>();
        let (tool_result_tx, tool_result_rx) = mpsc::sync_channel::<ToolResultMsg>(16);
        self.stream_rx = Some(stream_rx);
        self.tool_result_tx = Some(tool_result_tx);

        let streaming_content = Arc::clone(&self.streaming_content);
        let use_stream = self.agent_config.stream_mode;
        let system_prompt = self.resolve_system_prompt();
        let tools_enabled = self.agent_config.tools_enabled;
        let max_tool_rounds = self.agent_config.max_tool_rounds;
        let tools = if tools_enabled {
            self.tool_registry
                .to_openai_tools_filtered(&self.agent_config.disabled_tools)
        } else {
            vec![]
        };

        // 创建取消令牌
        let cancel_token = CancellationToken::new();
        self.cancel_token = Some(cancel_token.clone());

        let pending_user_messages = Arc::clone(&self.pending_user_messages);
        let background_manager = Arc::clone(&self.background_manager);

        // 启动后台线程执行 Agent 循环
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = stream_tx.send(StreamMsg::Error(format!("创建异步运行时失败: {}", e)));
                    return;
                }
            };

            rt.block_on(run_agent_loop(
                provider,
                api_messages,
                tools,
                system_prompt,
                use_stream,
                streaming_content,
                stream_tx,
                tool_result_rx,
                max_tool_rounds,
                cancel_token,
                pending_user_messages,
                background_manager,
            ));
        });
    }

    /// 处理后台流式消息（在主循环中每帧调用）
    pub fn poll_stream(&mut self) {
        if self.stream_rx.is_none() {
            return;
        }

        // 如果在 ToolConfirm 模式，仍然需要轮询工具执行结果（但暂停流式消息轮询）
        if self.mode == ChatMode::ToolConfirm {
            self.poll_tool_exec_results();
            // 轮询 ask 请求
            if let Some(ref rx) = self.ask_request_rx
                && let Ok(ask_req) = rx.try_recv()
            {
                self.init_ask_mode(ask_req);
                self.msg_lines_cache = None;
            }
            return;
        }

        // 如果上一帧设置了 pending_tool_execution，本帧才真正执行（让上一帧渲染出 🔧 执行中 状态）
        if self.pending_tool_execution {
            self.pending_tool_execution = false;

            // 处理被 .jcli deny 拒绝的工具：直接发送错误结果给 agent 线程
            for tc in &self.active_tool_calls {
                if let ToolExecStatus::Failed(ref msg) = tc.status {
                    if let Some(ref tx) = self.tool_result_tx {
                        let _ = tx.send(ToolResultMsg {
                            tool_call_id: tc.tool_call_id.clone(),
                            result: msg.clone(),
                            is_error: true,
                        });
                    }
                }
            }

            // 找第一个需要确认的工具
            let first_confirm_idx = self
                .active_tool_calls
                .iter()
                .position(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm));

            if let Some(idx) = first_confirm_idx {
                self.pending_tool_idx = idx;
                self.tool_confirm_entered_at = std::time::Instant::now();
                self.execute_all_tools_no_confirm();
                // 重置交互区状态
                self.tool_interact_selected = 0;
                self.tool_interact_typing = false;
                self.tool_interact_input.clear();
                self.tool_interact_cursor = 0;
                self.tool_ask_mode = false;
                self.tool_ask_questions.clear();
                self.tool_ask_current_idx = 0;
                self.tool_ask_answers.clear();
                self.tool_ask_selections.clear();
                self.tool_ask_cursor = 0;
                self.mode = ChatMode::ToolConfirm;
                write_info_log(
                    "poll_stream",
                    &format!(
                        "进入 ToolConfirm 模式, pending_tool_idx={}, active_tool_calls={}, tools_executing_count={}",
                        self.pending_tool_idx,
                        self.active_tool_calls.len(),
                        self.tools_executing_count,
                    ),
                );
            } else {
                write_info_log(
                    "poll_stream",
                    &format!(
                        "无需确认的工具, 直接执行, active_tool_calls={}",
                        self.active_tool_calls.len(),
                    ),
                );
                self.execute_all_tools_no_confirm();
            }
            return;
        }

        // 轮询后台工具执行结果
        self.poll_tool_exec_results();

        // 轮询 ask 工具请求
        if let Some(ref rx) = self.ask_request_rx
            && let Ok(ask_req) = rx.try_recv()
        {
            self.init_ask_mode(ask_req);
            self.mode = ChatMode::ToolConfirm;
            self.msg_lines_cache = None;
            return;
        }

        let mut finished = false;
        let mut had_error = false;
        let mut was_cancelled = false;

        // 非阻塞地取出所有可用的消息
        if let Some(ref rx) = self.stream_rx {
            loop {
                match rx.try_recv() {
                    Ok(StreamMsg::Chunk) => {
                        if self.auto_scroll {
                            self.scroll_offset = u16::MAX;
                        }
                    }
                    Ok(StreamMsg::ToolCallRequest(tool_calls)) => {
                        // 初始化工具调用状态
                        self.active_tool_calls.clear();
                        self.pending_tool_idx = 0;

                        for tc in tool_calls {
                            // 检查 .jcli deny 列表：被拒绝的工具直接标记为 Failed
                            if self.jcli_config.is_denied(&tc.name, &tc.arguments) {
                                self.active_tool_calls.push(ToolCallStatus {
                                    tool_call_id: tc.id.clone(),
                                    tool_name: tc.name.clone(),
                                    arguments: tc.arguments.clone(),
                                    confirm_message: format!(
                                        "🚫 {} 被 .jcli 权限配置拒绝",
                                        tc.name
                                    ),
                                    status: ToolExecStatus::Failed(
                                        "该命令被 .jcli 权限配置拒绝".to_string(),
                                    ),
                                });
                                continue;
                            }

                            let confirm_msg = if let Some(tool) = self.tool_registry.get(&tc.name) {
                                tool.confirmation_message(&tc.arguments)
                            } else {
                                format!("调用工具 {} 参数: {}", tc.name, tc.arguments)
                            };
                            let needs_confirm = self
                                .tool_registry
                                .get(&tc.name)
                                .map(|t| t.requires_confirmation())
                                .unwrap_or(false)
                                && !self.jcli_config.is_allowed(&tc.name, &tc.arguments);
                            self.active_tool_calls.push(ToolCallStatus {
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

                        // 延迟一帧再执行：让本帧先渲染出 🔧 执行中 的状态
                        self.pending_tool_execution = true;
                        break;
                    }
                    Ok(StreamMsg::Done) => {
                        finished = true;
                        break;
                    }
                    Ok(StreamMsg::Error(e)) => {
                        self.show_toast(format!("请求失败: {}", e), true);
                        had_error = true;
                        finished = true;
                        break;
                    }
                    Ok(StreamMsg::Cancelled) => {
                        was_cancelled = true;
                        finished = true;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        finished = true;
                        break;
                    }
                }
            }
        }

        if finished {
            self.finish_loading(had_error, was_cancelled);
        }
    }

    /// 初始化 ask 模式状态
    fn init_ask_mode(&mut self, ask_req: AskRequest) {
        self.tool_ask_mode = true;
        self.tool_ask_questions = ask_req.questions;
        self.tool_ask_current_idx = 0;
        self.tool_ask_answers = Vec::new();
        self.ask_response_tx = Some(ask_req.response_tx);
        // 初始化当前问题的选中状态
        self.init_ask_question_state();
        self.tool_interact_selected = 0;
        self.tool_interact_typing = false;
        self.tool_interact_input.clear();
        self.tool_interact_cursor = 0;
    }

    /// 初始化当前 ask 问题的选项状态
    pub fn init_ask_question_state(&mut self) {
        if let Some(q) = self.tool_ask_questions.get(self.tool_ask_current_idx) {
            // options + 1 for "自由输入" option
            self.tool_ask_selections = vec![false; q.options.len() + 1];
            self.tool_ask_cursor = 0;
        }
    }

    /// 轮询后台工具执行结果，更新状态并转发给 agent loop
    fn poll_tool_exec_results(&mut self) {
        let mut exec_done_msgs: Vec<ToolExecDoneMsg> = Vec::new();
        if self.tool_exec_rx.is_some() {
            loop {
                let msg = self.tool_exec_rx.as_ref().unwrap().try_recv();
                match msg {
                    Ok(done) => {
                        exec_done_msgs.push(done);
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        write_info_log("poll_tool_exec_results", "tool_exec_rx disconnected");
                        self.tool_exec_tx = None;
                        self.tool_exec_rx = None;
                        break;
                    }
                }
            }
        }
        if !exec_done_msgs.is_empty() {
            write_info_log(
                "poll_tool_exec_results",
                &format!(
                    "收到 {} 个工具结果, tools_executing_count={}, tool_result_tx={}",
                    exec_done_msgs.len(),
                    self.tools_executing_count,
                    self.tool_result_tx.is_some(),
                ),
            );
        }
        for done in exec_done_msgs {
            let summary = if done.output.len() > 60 {
                let mut end = 60;
                while !done.output.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &done.output[..end])
            } else {
                done.output.clone()
            };
            if let Some(tc) = self
                .active_tool_calls
                .iter_mut()
                .find(|tc| tc.tool_call_id == done.tool_call_id)
            {
                tc.status = if done.is_error {
                    ToolExecStatus::Failed(summary)
                } else {
                    ToolExecStatus::Done(summary)
                };
            }
            // 转发结果给后台 agent 线程
            if let Some(ref tx) = self.tool_result_tx {
                let _ = tx.send(ToolResultMsg {
                    tool_call_id: done.tool_call_id,
                    result: done.output,
                    is_error: done.is_error,
                });
            }
            self.tools_executing_count = self.tools_executing_count.saturating_sub(1);
            if self.tools_executing_count == 0 {
                self.tool_exec_tx = None;
                self.tool_exec_rx = None;
                // 本批工具全部完成，重置取消标志，避免影响后续轮次
                self.tool_cancelled
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    /// 把所有 Executing 状态的工具放到后台线程执行，结果发到 tool_exec_rx
    fn execute_all_tools_no_confirm(&mut self) {
        // 收集需要执行的工具信息
        let tasks: Vec<(String, String, String)> = self
            .active_tool_calls
            .iter()
            .filter(|tc| matches!(tc.status, ToolExecStatus::Executing))
            .map(|tc| {
                (
                    tc.tool_call_id.clone(),
                    tc.tool_name.clone(),
                    tc.arguments.clone(),
                )
            })
            .collect();

        if tasks.is_empty() {
            return;
        }

        // 新一批工具开始前，清除上一批的取消标志
        self.tool_cancelled
            .store(false, std::sync::atomic::Ordering::Relaxed);

        self.tools_executing_count += tasks.len();

        // 复用已有 channel，或创建新的
        let exec_tx = if let Some(ref tx) = self.tool_exec_tx {
            tx.clone()
        } else {
            let (tx, rx) = mpsc::channel::<ToolExecDoneMsg>();
            self.tool_exec_tx = Some(tx.clone());
            self.tool_exec_rx = Some(rx);
            tx
        };

        // 为每个工具 spawn 一个线程（Arc<ToolRegistry> 可跨线程共享）
        for (tool_call_id, tool_name, arguments) in tasks {
            let tx = exec_tx.clone();
            let registry = Arc::clone(&self.tool_registry);
            let cancelled = Arc::clone(&self.tool_cancelled);
            std::thread::spawn(move || {
                let result = registry.execute(&tool_name, &arguments, &cancelled);
                let _ = tx.send(ToolExecDoneMsg {
                    tool_call_id,
                    output: result.output,
                    is_error: result.is_error,
                });
            });
        }
    }

    /// 用户选择 "允许并记住"：生成 allow 规则写入 .jcli，然后执行工具
    pub fn allow_and_execute_pending_tool(&mut self) {
        let idx = self.pending_tool_idx;
        if idx >= self.active_tool_calls.len() {
            self.mode = ChatMode::Chat;
            return;
        }

        let tool_name = self.active_tool_calls[idx].tool_name.clone();
        let arguments = self.active_tool_calls[idx].arguments.clone();

        // 生成 allow 规则并写入 .jcli
        let rule = super::permission::generate_allow_rule(&tool_name, &arguments);
        let mut jcli = (*self.jcli_config).clone();
        jcli.add_allow_rule(&rule);
        self.jcli_config = std::sync::Arc::new(jcli);

        // 执行工具
        self.execute_pending_tool();
    }

    /// 用户确认执行当前待处理工具（spawn 后台线程，立即返回）
    pub fn execute_pending_tool(&mut self) {
        let idx = self.pending_tool_idx;
        if idx >= self.active_tool_calls.len() {
            self.mode = ChatMode::Chat;
            return;
        }

        write_info_log(
            "execute_pending_tool",
            &format!(
                "确认执行 idx={}, tool={}, tools_executing_count={}, tool_exec_tx={}",
                idx,
                self.active_tool_calls[idx].tool_name,
                self.tools_executing_count,
                self.tool_exec_tx.is_some(),
            ),
        );

        {
            let tc_status = &mut self.active_tool_calls[idx];
            tc_status.status = ToolExecStatus::Executing;
        }

        let (tool_name, arguments, tool_call_id) = {
            let tc = &self.active_tool_calls[idx];
            (
                tc.tool_name.clone(),
                tc.arguments.clone(),
                tc.tool_call_id.clone(),
            )
        };

        self.tools_executing_count += 1;

        // 新工具开始前，清除上一批的取消标志
        self.tool_cancelled
            .store(false, std::sync::atomic::Ordering::Relaxed);

        // 复用已有 channel，或创建新的
        let exec_tx = if let Some(ref tx) = self.tool_exec_tx {
            tx.clone()
        } else {
            let (tx, rx) = mpsc::channel::<ToolExecDoneMsg>();
            self.tool_exec_tx = Some(tx.clone());
            self.tool_exec_rx = Some(rx);
            tx
        };

        let registry = Arc::clone(&self.tool_registry);
        let cancelled = Arc::clone(&self.tool_cancelled);
        std::thread::spawn(move || {
            let result = registry.execute(&tool_name, &arguments, &cancelled);
            let _ = exec_tx.send(ToolExecDoneMsg {
                tool_call_id,
                output: result.output,
                is_error: result.is_error,
            });
        });

        // 推进到下一个待确认工具；如果没有更多待确认工具，退出确认模式
        self.advance_tool_confirm();
    }

    /// 用户拒绝执行当前待处理工具，可附带拒绝原因
    pub fn reject_pending_tool(&mut self, reason: &str) {
        let idx = self.pending_tool_idx;
        if idx >= self.active_tool_calls.len() {
            self.mode = ChatMode::Chat;
            return;
        }

        let tool_call_id = self.active_tool_calls[idx].tool_call_id.clone();
        self.active_tool_calls[idx].status = ToolExecStatus::Rejected;

        let reject_msg = if reason.is_empty() {
            "用户拒绝执行该工具".to_string()
        } else {
            format!("用户拒绝执行该工具。用户说: {}", reason)
        };

        if let Some(ref tx) = self.tool_result_tx {
            let _ = tx.send(ToolResultMsg {
                tool_call_id,
                result: reject_msg,
                is_error: true,
            });
        }

        self.advance_tool_confirm();
    }

    /// 推进到下一个待确认工具，或退出确认模式
    fn advance_tool_confirm(&mut self) {
        // 查找下一个 PendingConfirm 状态的工具
        let next = self
            .active_tool_calls
            .iter()
            .enumerate()
            .find(|(_, tc)| matches!(tc.status, ToolExecStatus::PendingConfirm))
            .map(|(i, _)| i);

        if let Some(next_idx) = next {
            self.pending_tool_idx = next_idx;
            self.tool_confirm_entered_at = std::time::Instant::now();
            write_info_log(
                "advance_tool_confirm",
                &format!("推进到 pending_tool_idx={}", next_idx),
            );
            // 继续保持 ToolConfirm 模式
        } else {
            write_info_log(
                "advance_tool_confirm",
                &format!(
                    "所有工具已处理, 退出 ToolConfirm, tools_executing_count={}",
                    self.tools_executing_count,
                ),
            );
            // 没有更多需要确认的工具，退出确认模式
            self.mode = ChatMode::Chat;
        }
    }

    /// 结束加载状态（流式完成或错误）
    fn finish_loading(&mut self, had_error: bool, was_cancelled: bool) {
        self.stream_rx = None;
        self.cancel_token = None;
        self.tool_result_tx = None;
        self.tool_exec_tx = None;
        self.tool_exec_rx = None;
        self.tools_executing_count = 0;
        self.is_loading = false;
        self.last_rendered_streaming_len = 0;
        self.msg_lines_cache = None;
        self.active_tool_calls.clear();

        if was_cancelled {
            // 保留已输出内容，追加 [已取消] 标记
            let content = {
                let sc = self.streaming_content.lock().unwrap();
                sc.clone()
            };
            if !content.is_empty() {
                let cancelled_content = format!("{}\n\n*[已取消]*", content);
                self.session
                    .messages
                    .push(ChatMessage::text("assistant", cancelled_content));
            }
            self.streaming_content.lock().unwrap().clear();
            if self.auto_scroll {
                self.scroll_offset = u16::MAX;
            }
            self.show_toast("已取消", false);
        } else if !had_error {
            let content = {
                let sc = self.streaming_content.lock().unwrap();
                sc.clone()
            };
            if !content.is_empty() {
                self.session
                    .messages
                    .push(ChatMessage::text("assistant", content));
                self.streaming_content.lock().unwrap().clear();
                self.show_toast("回复完成 ✓", false);
            }
            if self.auto_scroll {
                self.scroll_offset = u16::MAX;
            }
        } else {
            self.streaming_content.lock().unwrap().clear();
        }

        let _ = save_chat_session(&self.session);

        // 检查排队的任务，若有则自动启动下一个
        let next_task = {
            let mut tasks = self.queued_tasks.lock().unwrap();
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
        // 只通知 ShellTool kill 子进程，不 drop tool_result_tx
        self.tool_cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.show_toast("工具已取消", false);
    }

    /// 取消当前流式请求
    pub fn cancel_stream(&mut self) {
        if let Some(ref token) = self.cancel_token {
            token.cancel();
        }
        // drop tool_result_tx，使后台 agent loop 里阻塞等待工具结果的 recv() 立即返回 Disconnected
        self.tool_result_tx = None;
        // 通知 ShellTool kill 正在执行的子进程
        self.tool_cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// 清空对话
    pub fn clear_session(&mut self) {
        self.session.messages.clear();
        self.scroll_offset = 0;
        self.msg_lines_cache = None; // 清除缓存
        let _ = save_chat_session(&self.session);
        self.show_toast("对话已清空", false);
    }

    /// 切换模型
    pub fn switch_model(&mut self) {
        if let Some(sel) = self.model_list_state.selected() {
            self.agent_config.active_index = sel;
            let _ = save_agent_config(&self.agent_config);
            let name = self.active_model_name();
            self.show_toast(format!("已切换到: {}", name), false);
        }
        self.mode = ChatMode::Chat;
    }

    /// 向上滚动消息
    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(3);
        // 用户手动上滚，关闭自动滚动
        self.auto_scroll = false;
    }

    /// 向下滚动消息
    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(3);
        // 注意：scroll_offset 可能超过 max_scroll，绘制时会校正。
        // 如果用户滚到了底部（offset >= max_scroll），在绘制时会恢复 auto_scroll。
    }

    // ========== 归档相关方法 ==========

    /// 开始归档确认流程
    pub fn start_archive_confirm(&mut self) {
        use super::archive::generate_default_archive_name;
        self.archive_default_name = generate_default_archive_name();
        self.archive_custom_name = String::new();
        self.archive_editing_name = false;
        self.archive_edit_cursor = 0;
        self.mode = ChatMode::ArchiveConfirm;
    }

    /// 开始还原流程（加载归档列表）
    pub fn start_archive_list(&mut self) {
        use super::archive::list_archives;
        self.archives = list_archives();
        self.archive_list_index = 0;
        self.restore_confirm_needed = false;
        self.mode = ChatMode::ArchiveList;
    }

    /// 执行归档
    pub fn do_archive(&mut self, name: &str) {
        use super::archive::create_archive;

        match create_archive(name, self.session.messages.clone()) {
            Ok(_) => {
                // 归档成功后清空当前会话
                self.clear_session();
                self.show_toast(format!("对话已归档: {}", name), false);
            }
            Err(e) => {
                self.show_toast(e, true);
            }
        }
        self.mode = ChatMode::Chat;
    }

    /// 执行还原归档
    pub fn do_restore(&mut self) {
        use super::archive::restore_archive;

        if let Some(archive) = self.archives.get(self.archive_list_index) {
            match restore_archive(&archive.name) {
                Ok(messages) => {
                    // 清空当前会话
                    self.session.messages = messages;
                    self.scroll_offset = u16::MAX;
                    self.msg_lines_cache = None;
                    self.input.clear();
                    self.cursor_pos = 0;
                    let _ = save_chat_session(&self.session);
                    self.show_toast(format!("已还原归档: {}", archive.name), false);
                }
                Err(e) => {
                    self.show_toast(e, true);
                }
            }
        }
        self.mode = ChatMode::Chat;
    }

    /// 删除选中的归档
    pub fn do_delete_archive(&mut self) {
        use super::archive::delete_archive;

        if let Some(archive) = self.archives.get(self.archive_list_index) {
            match delete_archive(&archive.name) {
                Ok(_) => {
                    self.show_toast(format!("归档已删除: {}", archive.name), false);
                    // 刷新归档列表
                    self.archives = super::archive::list_archives();
                    if self.archive_list_index >= self.archives.len() && self.archive_list_index > 0
                    {
                        self.archive_list_index -= 1;
                    }
                }
                Err(e) => {
                    self.show_toast(e, true);
                }
            }
        }
    }
}
