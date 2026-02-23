use crate::config::YamlConfig;
use crate::{error, info};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseEventKind,
    },
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures::StreamExt;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};

// ========== 数据结构 ==========

/// 单个模型提供方配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProvider {
    /// 显示名称（如 "GPT-4o", "DeepSeek-V3"）
    pub name: String,
    /// API Base URL（如 "https://api.openai.com/v1"）
    pub api_base: String,
    /// API Key
    pub api_key: String,
    /// 模型名称（如 "gpt-4o", "deepseek-chat"）
    pub model: String,
}

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentConfig {
    /// 模型提供方列表
    #[serde(default)]
    pub providers: Vec<ModelProvider>,
    /// 当前选中的 provider 索引
    #[serde(default)]
    pub active_index: usize,
    /// 系统提示词（可选）
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// 是否使用流式输出（默认 true，设为 false 则等回复完整后再显示）
    #[serde(default = "default_stream_mode")]
    pub stream_mode: bool,
}

/// 默认流式输出
fn default_stream_mode() -> bool {
    true
}

/// 对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "user" | "assistant" | "system"
    pub content: String,
}

/// 对话会话
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatSession {
    pub messages: Vec<ChatMessage>,
}

// ========== 文件路径 ==========

/// 获取 agent 数据目录: ~/.jdata/agent/data/
fn agent_data_dir() -> PathBuf {
    let dir = YamlConfig::data_dir().join("agent").join("data");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 获取 agent 配置文件路径
fn agent_config_path() -> PathBuf {
    agent_data_dir().join("agent_config.json")
}

/// 获取对话历史文件路径
fn chat_history_path() -> PathBuf {
    agent_data_dir().join("chat_history.json")
}

// ========== 配置读写 ==========

/// 加载 Agent 配置
fn load_agent_config() -> AgentConfig {
    let path = agent_config_path();
    if !path.exists() {
        return AgentConfig::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            error!("❌ 解析 agent_config.json 失败: {}", e);
            AgentConfig::default()
        }),
        Err(e) => {
            error!("❌ 读取 agent_config.json 失败: {}", e);
            AgentConfig::default()
        }
    }
}

/// 保存 Agent 配置
fn save_agent_config(config: &AgentConfig) -> bool {
    let path = agent_config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(config) {
        Ok(json) => match fs::write(&path, json) {
            Ok(_) => true,
            Err(e) => {
                error!("❌ 保存 agent_config.json 失败: {}", e);
                false
            }
        },
        Err(e) => {
            error!("❌ 序列化 agent 配置失败: {}", e);
            false
        }
    }
}

/// 加载对话历史
fn load_chat_session() -> ChatSession {
    let path = chat_history_path();
    if !path.exists() {
        return ChatSession::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| ChatSession::default()),
        Err(_) => ChatSession::default(),
    }
}

/// 保存对话历史
fn save_chat_session(session: &ChatSession) -> bool {
    let path = chat_history_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(session) {
        Ok(json) => fs::write(&path, json).is_ok(),
        Err(_) => false,
    }
}

// ========== async-openai API 调用 ==========

/// 根据 ModelProvider 配置创建 async-openai Client
fn create_openai_client(provider: &ModelProvider) -> Client<OpenAIConfig> {
    let config = OpenAIConfig::new()
        .with_api_key(&provider.api_key)
        .with_api_base(&provider.api_base);
    Client::with_config(config)
}

/// 将内部 ChatMessage 转换为 async-openai 的请求消息格式
fn to_openai_messages(messages: &[ChatMessage]) -> Vec<ChatCompletionRequestMessage> {
    messages
        .iter()
        .filter_map(|msg| match msg.role.as_str() {
            "system" => ChatCompletionRequestSystemMessageArgs::default()
                .content(msg.content.as_str())
                .build()
                .ok()
                .map(ChatCompletionRequestMessage::System),
            "user" => ChatCompletionRequestUserMessageArgs::default()
                .content(msg.content.as_str())
                .build()
                .ok()
                .map(ChatCompletionRequestMessage::User),
            "assistant" => ChatCompletionRequestAssistantMessageArgs::default()
                .content(msg.content.as_str())
                .build()
                .ok()
                .map(ChatCompletionRequestMessage::Assistant),
            _ => None,
        })
        .collect()
}

/// 使用 async-openai 流式调用 API，通过回调逐步输出
/// 返回完整的助手回复内容
async fn call_openai_stream_async(
    provider: &ModelProvider,
    messages: &[ChatMessage],
    on_chunk: &mut dyn FnMut(&str),
) -> Result<String, String> {
    let client = create_openai_client(provider);
    let openai_messages = to_openai_messages(messages);

    let request = CreateChatCompletionRequestArgs::default()
        .model(&provider.model)
        .messages(openai_messages)
        .build()
        .map_err(|e| format!("构建请求失败: {}", e))?;

    let mut stream = client
        .chat()
        .create_stream(request)
        .await
        .map_err(|e| format!("API 请求失败: {}", e))?;

    let mut full_content = String::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                for choice in &response.choices {
                    if let Some(ref content) = choice.delta.content {
                        full_content.push_str(content);
                        on_chunk(content);
                    }
                }
            }
            Err(e) => {
                return Err(format!("流式响应错误: {}", e));
            }
        }
    }

    Ok(full_content)
}

/// 同步包装：创建 tokio runtime 执行异步流式调用
fn call_openai_stream(
    provider: &ModelProvider,
    messages: &[ChatMessage],
    on_chunk: &mut dyn FnMut(&str),
) -> Result<String, String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("创建异步运行时失败: {}", e))?;
    rt.block_on(call_openai_stream_async(provider, messages, on_chunk))
}

// ========== 命令入口 ==========

/// 处理 chat 命令: j chat [message...]
pub fn handle_chat(content: &[String], _config: &YamlConfig) {
    let agent_config = load_agent_config();

    if agent_config.providers.is_empty() {
        info!("⚠️  尚未配置 LLM 模型提供方。");
        info!("📁 请编辑配置文件: {}", agent_config_path().display());
        info!("📝 配置示例:");
        let example = AgentConfig {
            providers: vec![ModelProvider {
                name: "GPT-4o".to_string(),
                api_base: "https://api.openai.com/v1".to_string(),
                api_key: "sk-your-api-key".to_string(),
                model: "gpt-4o".to_string(),
            }],
            active_index: 0,
            system_prompt: Some("你是一个有用的助手。".to_string()),
            stream_mode: true,
        };
        if let Ok(json) = serde_json::to_string_pretty(&example) {
            println!("{}", json);
        }
        // 自动创建示例配置文件
        if !agent_config_path().exists() {
            let _ = save_agent_config(&example);
            info!(
                "✅ 已自动创建示例配置文件: {}",
                agent_config_path().display()
            );
            info!("📌 请修改其中的 api_key 和其他配置后重新运行 chat 命令");
        }
        return;
    }

    if content.is_empty() {
        // 无参数：进入 TUI 对话界面
        run_chat_tui();
        return;
    }

    // 有参数：快速发送消息并打印回复
    let message = content.join(" ");
    let message = message.trim().to_string();
    if message.is_empty() {
        error!("⚠️ 消息内容为空");
        return;
    }

    let idx = agent_config
        .active_index
        .min(agent_config.providers.len() - 1);
    let provider = &agent_config.providers[idx];

    info!("🤖 [{}] 思考中...", provider.name);

    let mut messages = Vec::new();
    if let Some(sys) = &agent_config.system_prompt {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: sys.clone(),
        });
    }
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: message,
    });

    match call_openai_stream(provider, &messages, &mut |chunk| {
        print!("{}", chunk);
        let _ = io::stdout().flush();
    }) {
        Ok(_) => {
            println!(); // 换行
        }
        Err(e) => {
            error!("\n❌ {}", e);
        }
    }
}

// ========== TUI 界面 ==========

/// 后台线程发送给 TUI 的消息类型
enum StreamMsg {
    /// 收到一个流式文本块
    Chunk,
    /// 流式响应完成
    Done,
    /// 发生错误
    Error(String),
}

/// TUI 应用状态
struct ChatApp {
    /// Agent 配置
    agent_config: AgentConfig,
    /// 当前对话会话
    session: ChatSession,
    /// 输入缓冲区
    input: String,
    /// 光标位置（字符索引）
    cursor_pos: usize,
    /// 当前模式
    mode: ChatMode,
    /// 消息列表滚动偏移
    scroll_offset: u16,
    /// 是否正在等待 AI 回复
    is_loading: bool,
    /// 模型选择列表状态
    model_list_state: ListState,
    /// Toast 通知消息 (内容, 是否错误, 创建时间)
    toast: Option<(String, bool, std::time::Instant)>,
    /// 用于接收后台流式回复的 channel
    stream_rx: Option<mpsc::Receiver<StreamMsg>>,
    /// 当前正在流式接收的 AI 回复内容（实时更新）
    streaming_content: Arc<Mutex<String>>,
    /// 消息渲染行缓存：(消息数, 最后一条消息内容hash, 气泡宽度) → 渲染好的行
    /// 避免每帧都重新解析 Markdown
    msg_lines_cache: Option<MsgLinesCache>,
    /// 消息浏览模式中选中的消息索引
    browse_msg_index: usize,
}

/// 消息渲染行缓存
struct MsgLinesCache {
    /// 会话消息数量
    msg_count: usize,
    /// 最后一条消息的内容长度（用于检测流式更新）
    last_msg_len: usize,
    /// 流式内容长度
    streaming_len: usize,
    /// 是否正在加载
    is_loading: bool,
    /// 气泡最大宽度（窗口变化时需要重算）
    bubble_max_width: usize,
    /// 浏览模式选中索引（None 表示非浏览模式）
    browse_index: Option<usize>,
    /// 缓存的渲染行
    lines: Vec<Line<'static>>,
    /// 每条消息（按 msg_index）的起始行号（用于浏览模式自动滚动）
    msg_start_lines: Vec<(usize, usize)>, // (msg_index, start_line)
}

/// Toast 通知显示时长（秒）
const TOAST_DURATION_SECS: u64 = 4;

#[derive(PartialEq)]
enum ChatMode {
    /// 正常对话模式（焦点在输入框）
    Chat,
    /// 模型选择模式
    SelectModel,
    /// 消息浏览模式（可选中消息并复制）
    Browse,
    /// 帮助
    Help,
}

impl ChatApp {
    fn new() -> Self {
        let agent_config = load_agent_config();
        let session = load_chat_session();
        let mut model_list_state = ListState::default();
        if !agent_config.providers.is_empty() {
            model_list_state.select(Some(agent_config.active_index));
        }
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
        }
    }

    /// 显示一条 toast 通知
    fn show_toast(&mut self, msg: impl Into<String>, is_error: bool) {
        self.toast = Some((msg.into(), is_error, std::time::Instant::now()));
    }

    /// 清理过期的 toast
    fn tick_toast(&mut self) {
        if let Some((_, _, created)) = &self.toast {
            if created.elapsed().as_secs() >= TOAST_DURATION_SECS {
                self.toast = None;
            }
        }
    }

    /// 获取当前活跃的 provider
    fn active_provider(&self) -> Option<&ModelProvider> {
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
    fn active_model_name(&self) -> String {
        self.active_provider()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "未配置".to_string())
    }

    /// 构建发送给 API 的消息列表
    fn build_api_messages(&self) -> Vec<ChatMessage> {
        let mut messages = Vec::new();
        if let Some(sys) = &self.agent_config.system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: sys.clone(),
            });
        }
        for msg in &self.session.messages {
            messages.push(msg.clone());
        }
        messages
    }

    /// 发送消息（非阻塞，启动后台线程流式接收）
    fn send_message(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return;
        }

        // 添加用户消息
        self.session.messages.push(ChatMessage {
            role: "user".to_string(),
            content: text,
        });
        self.input.clear();
        self.cursor_pos = 0;
        // 自动滚动到底部
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

        let api_messages = self.build_api_messages();

        // 清空流式内容缓冲
        {
            let mut sc = self.streaming_content.lock().unwrap();
            sc.clear();
        }

        // 创建 channel 用于后台线程 -> TUI 通信
        let (tx, rx) = mpsc::channel::<StreamMsg>();
        self.stream_rx = Some(rx);

        let streaming_content = Arc::clone(&self.streaming_content);

        let use_stream = self.agent_config.stream_mode;

        // 启动后台线程执行 API 调用
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = tx.send(StreamMsg::Error(format!("创建异步运行时失败: {}", e)));
                    return;
                }
            };

            rt.block_on(async {
                let client = create_openai_client(&provider);
                let openai_messages = to_openai_messages(&api_messages);

                let request = match CreateChatCompletionRequestArgs::default()
                    .model(&provider.model)
                    .messages(openai_messages)
                    .build()
                {
                    Ok(req) => req,
                    Err(e) => {
                        let _ = tx.send(StreamMsg::Error(format!("构建请求失败: {}", e)));
                        return;
                    }
                };

                if use_stream {
                    // 流式输出模式
                    let mut stream = match client.chat().create_stream(request).await {
                        Ok(s) => s,
                        Err(e) => {
                            let _ = tx.send(StreamMsg::Error(format!("API 请求失败: {}", e)));
                            return;
                        }
                    };

                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(response) => {
                                for choice in &response.choices {
                                    if let Some(ref content) = choice.delta.content {
                                        // 更新共享缓冲
                                        {
                                            let mut sc = streaming_content.lock().unwrap();
                                            sc.push_str(content);
                                        }
                                        let _ = tx.send(StreamMsg::Chunk);
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(StreamMsg::Error(format!("流式响应错误: {}", e)));
                                return;
                            }
                        }
                    }
                } else {
                    // 非流式输出模式：等待完整响应后一次性返回
                    match client.chat().create(request).await {
                        Ok(response) => {
                            if let Some(choice) = response.choices.first() {
                                if let Some(ref content) = choice.message.content {
                                    {
                                        let mut sc = streaming_content.lock().unwrap();
                                        sc.push_str(content);
                                    }
                                    let _ = tx.send(StreamMsg::Chunk);
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(StreamMsg::Error(format!("API 请求失败: {}", e)));
                            return;
                        }
                    }
                }

                let _ = tx.send(StreamMsg::Done);

                let _ = tx.send(StreamMsg::Done);
            });
        });
    }

    /// 处理后台流式消息（在主循环中每帧调用）
    fn poll_stream(&mut self) {
        if self.stream_rx.is_none() {
            return;
        }

        let mut finished = false;
        let mut had_error = false;

        // 非阻塞地取出所有可用的消息
        if let Some(ref rx) = self.stream_rx {
            loop {
                match rx.try_recv() {
                    Ok(StreamMsg::Chunk) => {
                        // 内容已经通过 Arc<Mutex<String>> 更新，这里只确保滚到底部
                        self.scroll_offset = u16::MAX;
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
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        finished = true;
                        break;
                    }
                }
            }
        }

        if finished {
            self.stream_rx = None;
            self.is_loading = false;

            if !had_error {
                // 将流式内容作为完整回复添加到会话
                let content = {
                    let sc = self.streaming_content.lock().unwrap();
                    sc.clone()
                };
                if !content.is_empty() {
                    self.session.messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content,
                    });
                    // 清空流式缓冲
                    self.streaming_content.lock().unwrap().clear();
                    self.show_toast("回复完成 ✓", false);
                }
                self.scroll_offset = u16::MAX;
            } else {
                // 错误时也清空流式缓冲
                self.streaming_content.lock().unwrap().clear();
            }

            // 自动保存对话历史
            let _ = save_chat_session(&self.session);
        }
    }

    /// 清空对话
    fn clear_session(&mut self) {
        self.session.messages.clear();
        self.scroll_offset = 0;
        let _ = save_chat_session(&self.session);
        self.show_toast("对话已清空", false);
    }

    /// 切换模型
    fn switch_model(&mut self) {
        if let Some(sel) = self.model_list_state.selected() {
            self.agent_config.active_index = sel;
            let _ = save_agent_config(&self.agent_config);
            let name = self.active_model_name();
            self.show_toast(format!("已切换到: {}", name), false);
        }
        self.mode = ChatMode::Chat;
    }

    /// 向上滚动消息
    fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(3);
    }

    /// 向下滚动消息
    fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(3);
    }
}

/// 启动 TUI 对话界面
fn run_chat_tui() {
    match run_chat_tui_internal() {
        Ok(_) => {}
        Err(e) => {
            error!("❌ Chat TUI 启动失败: {}", e);
        }
    }
}

fn run_chat_tui_internal() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = ChatApp::new();

    if app.agent_config.providers.is_empty() {
        terminal::disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        info!("⚠️  尚未配置 LLM 模型提供方，请先运行 j chat 查看配置说明。");
        return Ok(());
    }

    let mut needs_redraw = true; // 首次必须绘制

    loop {
        // 清理过期 toast（如果有 toast 被清理，需要重绘）
        let had_toast = app.toast.is_some();
        app.tick_toast();
        if had_toast && app.toast.is_none() {
            needs_redraw = true;
        }

        // 非阻塞地处理后台流式消息
        let was_loading = app.is_loading;
        app.poll_stream();
        // 流式加载中每帧都需要重绘（内容在更新）
        if app.is_loading || (was_loading && !app.is_loading) {
            needs_redraw = true;
        }

        // 只在状态发生变化时才重绘，大幅降低 CPU 占用
        if needs_redraw {
            terminal.draw(|f| draw_chat_ui(f, &mut app))?;
            needs_redraw = false;
        }

        // 等待事件：加载中用短间隔以刷新流式内容，空闲时用长间隔节省 CPU
        let poll_timeout = if app.is_loading {
            std::time::Duration::from_millis(150)
        } else {
            std::time::Duration::from_millis(1000)
        };

        if event::poll(poll_timeout)? {
            // 批量消费所有待处理事件，避免快速滚动/打字时事件堆积
            let mut should_break = false;
            loop {
                let evt = event::read()?;
                match evt {
                    Event::Key(key) => {
                        needs_redraw = true;
                        match app.mode {
                            ChatMode::Chat => {
                                if handle_chat_mode(&mut app, key) {
                                    should_break = true;
                                    break;
                                }
                            }
                            ChatMode::SelectModel => handle_select_model(&mut app, key),
                            ChatMode::Browse => handle_browse_mode(&mut app, key),
                            ChatMode::Help => {
                                app.mode = ChatMode::Chat;
                            }
                        }
                    }
                    Event::Mouse(mouse) => match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            app.scroll_up();
                            needs_redraw = true;
                        }
                        MouseEventKind::ScrollDown => {
                            app.scroll_down();
                            needs_redraw = true;
                        }
                        _ => {}
                    },
                    Event::Resize(_, _) => {
                        needs_redraw = true;
                    }
                    _ => {}
                }
                // 继续消费剩余事件（非阻塞，Duration::ZERO）
                if !event::poll(std::time::Duration::ZERO)? {
                    break;
                }
            }
            if should_break {
                break;
            }
        }
    }

    // 保存对话历史
    let _ = save_chat_session(&app.session);

    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}

/// 绘制 TUI 界面
fn draw_chat_ui(f: &mut ratatui::Frame, app: &mut ChatApp) {
    let size = f.area();

    // 整体背景
    let bg = Block::default().style(Style::default().bg(Color::Rgb(22, 22, 30)));
    f.render_widget(bg, size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 标题栏
            Constraint::Min(5),    // 消息区
            Constraint::Length(5), // 输入区
            Constraint::Length(1), // 操作提示栏（始终可见）
        ])
        .split(size);

    // ========== 标题栏 ==========
    draw_title_bar(f, chunks[0], app);

    // ========== 消息区 ==========
    if app.mode == ChatMode::Help {
        draw_help(f, chunks[1]);
    } else if app.mode == ChatMode::SelectModel {
        draw_model_selector(f, chunks[1], app);
    } else {
        draw_messages(f, chunks[1], app);
    }

    // ========== 输入区 ==========
    draw_input(f, chunks[2], app);

    // ========== 底部操作提示栏（始终可见）==========
    draw_hint_bar(f, chunks[3], app);

    // ========== Toast 弹窗覆盖层（右上角）==========
    draw_toast(f, size, app);
}

/// 绘制标题栏
fn draw_title_bar(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let model_name = app.active_model_name();
    let msg_count = app.session.messages.len();
    let loading = if app.is_loading {
        " ⏳ 思考中..."
    } else {
        ""
    };

    let title_spans = vec![
        Span::styled(" 💬 ", Style::default().fg(Color::Rgb(120, 180, 255))),
        Span::styled(
            "AI Chat",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(Color::Rgb(60, 60, 80))),
        Span::styled("🤖 ", Style::default()),
        Span::styled(
            model_name,
            Style::default()
                .fg(Color::Rgb(160, 220, 160))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(Color::Rgb(60, 60, 80))),
        Span::styled(
            format!("📨 {} 条消息", msg_count),
            Style::default().fg(Color::Rgb(180, 180, 200)),
        ),
        Span::styled(
            loading,
            Style::default()
                .fg(Color::Rgb(255, 200, 80))
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let title_block = Paragraph::new(Line::from(title_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(80, 100, 140)))
            .style(Style::default().bg(Color::Rgb(28, 28, 40))),
    );
    f.render_widget(title_block, area);
}

/// 绘制消息区
fn draw_messages(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(50, 55, 70)))
        .title(Span::styled(
            " 对话记录 ",
            Style::default()
                .fg(Color::Rgb(140, 140, 170))
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(ratatui::layout::Alignment::Left)
        .style(Style::default().bg(Color::Rgb(22, 22, 30)));

    // 空消息时显示欢迎界面
    if app.session.messages.is_empty() && !app.is_loading {
        let welcome_lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "  ╭──────────────────────────────────────╮",
                Style::default().fg(Color::Rgb(60, 70, 90)),
            )),
            Line::from(Span::styled(
                "  │                                      │",
                Style::default().fg(Color::Rgb(60, 70, 90)),
            )),
            Line::from(vec![
                Span::styled("  │     ", Style::default().fg(Color::Rgb(60, 70, 90))),
                Span::styled(
                    "Hi! What can I help you?  ",
                    Style::default().fg(Color::Rgb(120, 140, 180)),
                ),
                Span::styled("     │", Style::default().fg(Color::Rgb(60, 70, 90))),
            ]),
            Line::from(Span::styled(
                "  │                                      │",
                Style::default().fg(Color::Rgb(60, 70, 90)),
            )),
            Line::from(Span::styled(
                "  │     Type a message, press Enter      │",
                Style::default().fg(Color::Rgb(80, 90, 110)),
            )),
            Line::from(Span::styled(
                "  │                                      │",
                Style::default().fg(Color::Rgb(60, 70, 90)),
            )),
            Line::from(Span::styled(
                "  ╰──────────────────────────────────────╯",
                Style::default().fg(Color::Rgb(60, 70, 90)),
            )),
        ];
        let empty = Paragraph::new(welcome_lines).block(block);
        f.render_widget(empty, area);
        return;
    }

    // 内部可用宽度（减去边框和左右各1的 padding）
    let inner_width = area.width.saturating_sub(4) as usize;
    // 消息内容最大宽度为可用宽度的 75%
    let bubble_max_width = (inner_width * 75 / 100).max(20);

    // 计算缓存 key：消息数 + 最后一条消息长度 + 流式内容长度 + is_loading + 气泡宽度 + 浏览模式索引
    let msg_count = app.session.messages.len();
    let last_msg_len = app
        .session
        .messages
        .last()
        .map(|m| m.content.len())
        .unwrap_or(0);
    let streaming_len = app.streaming_content.lock().unwrap().len();
    let current_browse_index = if app.mode == ChatMode::Browse {
        Some(app.browse_msg_index)
    } else {
        None
    };
    let cache_hit = if let Some(ref cache) = app.msg_lines_cache {
        cache.msg_count == msg_count
            && cache.last_msg_len == last_msg_len
            && cache.streaming_len == streaming_len
            && cache.is_loading == app.is_loading
            && cache.bubble_max_width == bubble_max_width
            && cache.browse_index == current_browse_index
    } else {
        false
    };

    if !cache_hit {
        // 缓存未命中，重新构建渲染行并存入缓存
        let (new_lines, new_msg_start_lines) =
            build_message_lines(app, inner_width, bubble_max_width);
        app.msg_lines_cache = Some(MsgLinesCache {
            msg_count,
            last_msg_len,
            streaming_len,
            is_loading: app.is_loading,
            bubble_max_width,
            browse_index: current_browse_index,
            lines: new_lines,
            msg_start_lines: new_msg_start_lines,
        });
    }

    // 从缓存中借用 lines（零拷贝）
    let cached = app.msg_lines_cache.as_ref().unwrap();
    let all_lines = &cached.lines;
    let total_lines = all_lines.len() as u16;

    // 渲染边框
    f.render_widget(block, area);

    // 计算内部区域（去掉边框）
    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    let visible_height = inner.height;
    let max_scroll = total_lines.saturating_sub(visible_height);

    // 自动滚动到底部（非浏览模式下）
    if app.mode != ChatMode::Browse {
        if app.scroll_offset == u16::MAX || app.scroll_offset > max_scroll {
            app.scroll_offset = max_scroll;
        }
    } else {
        // 浏览模式：自动滚动到选中消息的位置
        if let Some(target_line) = cached
            .msg_start_lines
            .iter()
            .find(|(idx, _)| *idx == app.browse_msg_index)
            .map(|(_, line)| *line as u16)
        {
            // 确保选中消息在可视区域内
            if target_line < app.scroll_offset {
                app.scroll_offset = target_line;
            } else if target_line >= app.scroll_offset + visible_height {
                app.scroll_offset = target_line.saturating_sub(visible_height / 3);
            }
            // 限制滚动范围
            if app.scroll_offset > max_scroll {
                app.scroll_offset = max_scroll;
            }
        }
    }

    // 填充内部背景色（避免空白行没有背景）
    let bg_fill = Block::default().style(Style::default().bg(Color::Rgb(22, 22, 30)));
    f.render_widget(bg_fill, inner);

    // 只渲染可见区域的行（逐行借用缓存，clone 单行开销极小）
    let start = app.scroll_offset as usize;
    let end = (start + visible_height as usize).min(all_lines.len());
    for (i, line_idx) in (start..end).enumerate() {
        let line = &all_lines[line_idx];
        let y = inner.y + i as u16;
        let line_area = Rect::new(inner.x, y, inner.width, 1);
        // 使用 Paragraph 渲染单行（clone 单行开销很小）
        let p = Paragraph::new(line.clone());
        f.render_widget(p, line_area);
    }
}

/// 构建所有消息的渲染行（独立函数，用于缓存）
/// 返回 (渲染行列表, 消息起始行号映射)
fn build_message_lines(
    app: &ChatApp,
    inner_width: usize,
    bubble_max_width: usize,
) -> (Vec<Line<'static>>, Vec<(usize, usize)>) {
    struct RenderMsg {
        role: String,
        content: String,
        msg_index: Option<usize>, // 对应 session.messages 的索引（流式消息为 None）
    }
    let mut render_msgs: Vec<RenderMsg> = app
        .session
        .messages
        .iter()
        .enumerate()
        .map(|(i, m)| RenderMsg {
            role: m.role.clone(),
            content: m.content.clone(),
            msg_index: Some(i),
        })
        .collect();

    // 如果正在流式接收，添加一条临时的 assistant 消息
    if app.is_loading {
        let streaming = app.streaming_content.lock().unwrap().clone();
        if !streaming.is_empty() {
            render_msgs.push(RenderMsg {
                role: "assistant".to_string(),
                content: streaming,
                msg_index: None,
            });
        } else {
            // 正在等待首个 chunk，显示占位
            render_msgs.push(RenderMsg {
                role: "assistant".to_string(),
                content: "◍".to_string(),
                msg_index: None,
            });
        }
    }

    // 构建所有消息行
    let is_browse_mode = app.mode == ChatMode::Browse;
    let mut lines: Vec<Line> = Vec::new();
    let mut msg_start_lines: Vec<(usize, usize)> = Vec::new(); // (msg_index, start_line)
    for msg in &render_msgs {
        // 判断当前消息是否在浏览模式下被选中
        let is_selected = is_browse_mode
            && msg.msg_index.is_some()
            && msg.msg_index.unwrap() == app.browse_msg_index;

        // 记录消息起始行号
        if let Some(idx) = msg.msg_index {
            msg_start_lines.push((idx, lines.len()));
        }

        match msg.role.as_str() {
            "user" => {
                // 用户消息：右对齐，蓝色系
                lines.push(Line::from(""));
                // 用户标签（浏览模式选中时加 ▶ 指示器）
                let label = if is_selected { "▶ You " } else { "You " };
                let pad = inner_width.saturating_sub(display_width(label) + 2);
                lines.push(Line::from(vec![
                    Span::raw(" ".repeat(pad)),
                    Span::styled(
                        label,
                        Style::default()
                            .fg(if is_selected {
                                Color::Rgb(255, 200, 80)
                            } else {
                                Color::Rgb(100, 160, 255)
                            })
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                // 消息内容（右对齐气泡效果）
                let user_bg = if is_selected {
                    Color::Rgb(55, 85, 140)
                } else {
                    Color::Rgb(40, 70, 120)
                };
                let user_pad_lr = 3usize; // 左右内边距
                let user_content_w = bubble_max_width.saturating_sub(user_pad_lr * 2);

                // 先预计算所有换行后的内容行，以便确定实际气泡宽度
                let mut all_wrapped_lines: Vec<String> = Vec::new();
                for content_line in msg.content.lines() {
                    let wrapped = wrap_text(content_line, user_content_w);
                    all_wrapped_lines.extend(wrapped);
                }
                // 如果消息为空，至少保留一行空行
                if all_wrapped_lines.is_empty() {
                    all_wrapped_lines.push(String::new());
                }

                // 根据实际内容宽度动态计算气泡宽度（不超过 bubble_max_width）
                let actual_content_w = all_wrapped_lines
                    .iter()
                    .map(|l| display_width(l))
                    .max()
                    .unwrap_or(0);
                let actual_bubble_w = (actual_content_w + user_pad_lr * 2)
                    .min(bubble_max_width)
                    .max(user_pad_lr * 2 + 1);
                let actual_inner_content_w = actual_bubble_w.saturating_sub(user_pad_lr * 2);

                // 上边距
                {
                    let bubble_text = " ".repeat(actual_bubble_w);
                    let pad = inner_width.saturating_sub(actual_bubble_w);
                    lines.push(Line::from(vec![
                        Span::raw(" ".repeat(pad)),
                        Span::styled(bubble_text, Style::default().bg(user_bg)),
                    ]));
                }

                for wl in &all_wrapped_lines {
                    let wl_width = display_width(wl);
                    let fill = actual_inner_content_w.saturating_sub(wl_width);
                    let text = format!(
                        "{}{}{}{}",
                        " ".repeat(user_pad_lr),
                        wl,
                        " ".repeat(fill),
                        " ".repeat(user_pad_lr),
                    );
                    let text_width = display_width(&text);
                    let pad = inner_width.saturating_sub(text_width);
                    lines.push(Line::from(vec![
                        Span::raw(" ".repeat(pad)),
                        Span::styled(text, Style::default().fg(Color::White).bg(user_bg)),
                    ]));
                }

                // 下边距
                {
                    let bubble_text = " ".repeat(actual_bubble_w);
                    let pad = inner_width.saturating_sub(actual_bubble_w);
                    lines.push(Line::from(vec![
                        Span::raw(" ".repeat(pad)),
                        Span::styled(bubble_text, Style::default().bg(user_bg)),
                    ]));
                }
            }
            "assistant" => {
                // AI 消息：左对齐，使用 Markdown 渲染
                lines.push(Line::from(""));
                let ai_label = if is_selected { "  ▶ AI" } else { "  AI" };
                lines.push(Line::from(Span::styled(
                    ai_label,
                    Style::default()
                        .fg(if is_selected {
                            Color::Rgb(255, 200, 80)
                        } else {
                            Color::Rgb(120, 220, 160)
                        })
                        .add_modifier(Modifier::BOLD),
                )));

                // 使用 pulldown-cmark 解析 Markdown 内容并渲染
                let bubble_bg = if is_selected {
                    Color::Rgb(48, 48, 68)
                } else {
                    Color::Rgb(38, 38, 52)
                };
                let pad_left = "   "; // 左内边距 3 字符
                let pad_right = "   "; // 右内边距 3 字符
                let pad_left_w = 3usize;
                let pad_right_w = 3usize;
                // 内容区最大宽度要减去左右内边距
                let md_content_w = bubble_max_width.saturating_sub(pad_left_w + pad_right_w);
                let md_lines = markdown_to_lines(&msg.content, md_content_w + 2); // +2 因为 markdown_to_lines 内部还会减2

                // 气泡总宽度 = pad_left + 内容填充到统一宽度 + pad_right
                let bubble_total_w = bubble_max_width;

                // 上边距：一行空白行（带背景色）
                {
                    let mut top_spans: Vec<Span> = Vec::new();
                    top_spans.push(Span::styled(
                        " ".repeat(bubble_total_w),
                        Style::default().bg(bubble_bg),
                    ));
                    lines.push(Line::from(top_spans));
                }

                for md_line in md_lines {
                    // 给每行添加左侧内边距，并应用 AI 消息背景色
                    let mut styled_spans: Vec<Span> = Vec::new();
                    styled_spans.push(Span::styled(pad_left, Style::default().bg(bubble_bg)));
                    // 计算内容区宽度
                    let mut content_w: usize = 0;
                    for span in md_line.spans {
                        let sw = display_width(&span.content);
                        content_w += sw;
                        // 保留 Markdown 渲染的前景色/修饰符，叠加背景色
                        let merged_style = span.style.bg(bubble_bg);
                        styled_spans.push(Span::styled(span.content.to_string(), merged_style));
                    }
                    // 用空格填充到统一宽度，再加右内边距
                    let target_content_w = bubble_total_w.saturating_sub(pad_left_w + pad_right_w);
                    let fill = target_content_w.saturating_sub(content_w);
                    if fill > 0 {
                        styled_spans.push(Span::styled(
                            " ".repeat(fill),
                            Style::default().bg(bubble_bg),
                        ));
                    }
                    styled_spans.push(Span::styled(pad_right, Style::default().bg(bubble_bg)));
                    lines.push(Line::from(styled_spans));
                }

                // 下边距：一行空白行（带背景色）
                {
                    let mut bottom_spans: Vec<Span> = Vec::new();
                    bottom_spans.push(Span::styled(
                        " ".repeat(bubble_total_w),
                        Style::default().bg(bubble_bg),
                    ));
                    lines.push(Line::from(bottom_spans));
                }
            }
            "system" => {
                // 系统消息：居中，淡色
                lines.push(Line::from(""));
                let wrapped = wrap_text(&msg.content, inner_width.saturating_sub(8));
                for wl in wrapped {
                    lines.push(Line::from(Span::styled(
                        format!("    {}  {}", "sys", wl),
                        Style::default().fg(Color::Rgb(100, 100, 120)),
                    )));
                }
            }
            _ => {}
        }
    }
    // 末尾留白
    lines.push(Line::from(""));

    (lines, msg_start_lines)
}

/// 将 Markdown 文本解析为 ratatui 的 Line 列表
/// 支持：标题（去掉 # 标记）、加粗、斜体、行内代码、代码块（语法高亮）、列表、分隔线
/// content_width：内容区可用宽度（不含外层 "  " 缩进和右侧填充）
fn markdown_to_lines(md: &str, max_width: usize) -> Vec<Line<'static>> {
    use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

    // 内容区宽度 = max_width - 2（左侧 "  " 缩进由外层负责）
    let content_width = max_width.saturating_sub(2);

    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    let parser = Parser::new_ext(md, options);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default().fg(Color::Rgb(220, 220, 230))];
    let mut in_code_block = false;
    let mut code_block_content = String::new();
    let mut code_block_lang = String::new();
    let mut list_depth: usize = 0;
    let mut ordered_index: Option<u64> = None;
    let mut heading_level: Option<u8> = None;
    // 跟踪是否在引用块中
    let mut in_blockquote = false;
    // 表格相关状态
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new(); // 收集所有行（含表头）
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    let mut table_alignments: Vec<pulldown_cmark::Alignment> = Vec::new();

    let base_style = Style::default().fg(Color::Rgb(220, 220, 230));

    let flush_line = |current_spans: &mut Vec<Span<'static>>, lines: &mut Vec<Line<'static>>| {
        if !current_spans.is_empty() {
            lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
        }
    };

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_line(&mut current_spans, &mut lines);
                heading_level = Some(level as u8);
                if !lines.is_empty() {
                    lines.push(Line::from(""));
                }
                // 根据标题级别使用不同的颜色
                let heading_style = match level as u8 {
                    1 => Style::default()
                        .fg(Color::Rgb(100, 180, 255))
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    2 => Style::default()
                        .fg(Color::Rgb(130, 190, 255))
                        .add_modifier(Modifier::BOLD),
                    3 => Style::default()
                        .fg(Color::Rgb(160, 200, 255))
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default()
                        .fg(Color::Rgb(180, 210, 255))
                        .add_modifier(Modifier::BOLD),
                };
                style_stack.push(heading_style);
            }
            Event::End(TagEnd::Heading(level)) => {
                flush_line(&mut current_spans, &mut lines);
                // h1/h2 下方加分隔线（完整填充 content_width）
                if (level as u8) <= 2 {
                    let sep_char = if (level as u8) == 1 { "━" } else { "─" };
                    lines.push(Line::from(Span::styled(
                        sep_char.repeat(content_width),
                        Style::default().fg(Color::Rgb(60, 70, 100)),
                    )));
                }
                style_stack.pop();
                heading_level = None;
            }
            Event::Start(Tag::Strong) => {
                let current = *style_stack.last().unwrap_or(&base_style);
                style_stack.push(current.add_modifier(Modifier::BOLD));
            }
            Event::End(TagEnd::Strong) => {
                style_stack.pop();
            }
            Event::Start(Tag::Emphasis) => {
                let current = *style_stack.last().unwrap_or(&base_style);
                style_stack.push(current.add_modifier(Modifier::ITALIC));
            }
            Event::End(TagEnd::Emphasis) => {
                style_stack.pop();
            }
            Event::Start(Tag::Strikethrough) => {
                let current = *style_stack.last().unwrap_or(&base_style);
                style_stack.push(current.add_modifier(Modifier::CROSSED_OUT));
            }
            Event::End(TagEnd::Strikethrough) => {
                style_stack.pop();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_line(&mut current_spans, &mut lines);
                in_code_block = true;
                code_block_content.clear();
                code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                // 代码块上方边框（自适应宽度）
                let label = if code_block_lang.is_empty() {
                    " code ".to_string()
                } else {
                    format!(" {} ", code_block_lang)
                };
                let label_w = display_width(&label);
                let border_fill = content_width.saturating_sub(2 + label_w);
                let top_border = format!("┌─{}{}", label, "─".repeat(border_fill));
                lines.push(Line::from(Span::styled(
                    top_border,
                    Style::default().fg(Color::Rgb(80, 90, 110)),
                )));
            }
            Event::End(TagEnd::CodeBlock) => {
                // 渲染代码块内容（带语法高亮）
                let code_inner_w = content_width.saturating_sub(4); // "│ " 前缀 + 右侧 " │" 后缀占4
                for code_line in code_block_content.lines() {
                    let wrapped = wrap_text(code_line, code_inner_w);
                    for wl in wrapped {
                        let highlighted = highlight_code_line(&wl, &code_block_lang);
                        let text_w: usize =
                            highlighted.iter().map(|s| display_width(&s.content)).sum();
                        let fill = code_inner_w.saturating_sub(text_w);
                        let mut spans_vec = Vec::new();
                        spans_vec.push(Span::styled(
                            "│ ",
                            Style::default().fg(Color::Rgb(80, 90, 110)),
                        ));
                        for hs in highlighted {
                            spans_vec.push(Span::styled(
                                hs.content.to_string(),
                                hs.style.bg(Color::Rgb(30, 30, 42)),
                            ));
                        }
                        spans_vec.push(Span::styled(
                            format!("{} │", " ".repeat(fill)),
                            Style::default()
                                .fg(Color::Rgb(80, 90, 110))
                                .bg(Color::Rgb(30, 30, 42)),
                        ));
                        lines.push(Line::from(spans_vec));
                    }
                }
                let bottom_border = format!("└{}", "─".repeat(content_width.saturating_sub(1)));
                lines.push(Line::from(Span::styled(
                    bottom_border,
                    Style::default().fg(Color::Rgb(80, 90, 110)),
                )));
                in_code_block = false;
                code_block_content.clear();
                code_block_lang.clear();
            }
            Event::Code(text) => {
                if in_table {
                    // 表格中的行内代码也收集到当前单元格
                    current_cell.push('`');
                    current_cell.push_str(&text);
                    current_cell.push('`');
                } else {
                    // 行内代码
                    current_spans.push(Span::styled(
                        format!(" {} ", text),
                        Style::default()
                            .fg(Color::Rgb(230, 190, 120))
                            .bg(Color::Rgb(45, 45, 60)),
                    ));
                }
            }
            Event::Start(Tag::List(start)) => {
                flush_line(&mut current_spans, &mut lines);
                list_depth += 1;
                ordered_index = start;
            }
            Event::End(TagEnd::List(_)) => {
                flush_line(&mut current_spans, &mut lines);
                list_depth = list_depth.saturating_sub(1);
                ordered_index = None;
            }
            Event::Start(Tag::Item) => {
                flush_line(&mut current_spans, &mut lines);
                let indent = "  ".repeat(list_depth);
                let bullet = if let Some(ref mut idx) = ordered_index {
                    let s = format!("{}{}. ", indent, idx);
                    *idx += 1;
                    s
                } else {
                    format!("{}- ", indent)
                };
                current_spans.push(Span::styled(
                    bullet,
                    Style::default().fg(Color::Rgb(160, 180, 220)),
                ));
            }
            Event::End(TagEnd::Item) => {
                flush_line(&mut current_spans, &mut lines);
            }
            Event::Start(Tag::Paragraph) => {
                if !lines.is_empty() && !in_code_block && heading_level.is_none() {
                    let last_empty = lines.last().map(|l| l.spans.is_empty()).unwrap_or(false);
                    if !last_empty {
                        lines.push(Line::from(""));
                    }
                }
            }
            Event::End(TagEnd::Paragraph) => {
                flush_line(&mut current_spans, &mut lines);
            }
            Event::Start(Tag::BlockQuote(_)) => {
                flush_line(&mut current_spans, &mut lines);
                in_blockquote = true;
                style_stack.push(Style::default().fg(Color::Rgb(150, 160, 180)));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                flush_line(&mut current_spans, &mut lines);
                in_blockquote = false;
                style_stack.pop();
            }
            Event::Text(text) => {
                if in_code_block {
                    code_block_content.push_str(&text);
                } else if in_table {
                    // 表格中的文本收集到当前单元格
                    current_cell.push_str(&text);
                } else {
                    let style = *style_stack.last().unwrap_or(&base_style);
                    let text_str = text.to_string();

                    // 标题：添加可视化符号前缀代替 # 标记
                    if let Some(level) = heading_level {
                        let (prefix, prefix_style) = match level {
                            1 => (
                                ">> ",
                                Style::default()
                                    .fg(Color::Rgb(100, 180, 255))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            2 => (
                                ">> ",
                                Style::default()
                                    .fg(Color::Rgb(130, 190, 255))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            3 => (
                                "> ",
                                Style::default()
                                    .fg(Color::Rgb(160, 200, 255))
                                    .add_modifier(Modifier::BOLD),
                            ),
                            _ => (
                                "> ",
                                Style::default()
                                    .fg(Color::Rgb(180, 210, 255))
                                    .add_modifier(Modifier::BOLD),
                            ),
                        };
                        current_spans.push(Span::styled(prefix.to_string(), prefix_style));
                        heading_level = None; // 只加一次前缀
                    }

                    // 计算 current_spans 已有的显示宽度
                    let existing_w: usize = current_spans
                        .iter()
                        .map(|s| display_width(&s.content))
                        .sum();

                    // 引用块：加左侧竖线
                    let effective_prefix_w = if in_blockquote { 2 } else { 0 }; // "| " 宽度
                    let wrap_w = content_width.saturating_sub(effective_prefix_w + existing_w);

                    for (i, line) in text_str.split('\n').enumerate() {
                        if i > 0 {
                            flush_line(&mut current_spans, &mut lines);
                            if in_blockquote {
                                current_spans.push(Span::styled(
                                    "| ".to_string(),
                                    Style::default().fg(Color::Rgb(80, 100, 140)),
                                ));
                            }
                        }
                        if !line.is_empty() {
                            // 第一行使用减去已有 span 宽度的 wrap_w，后续行使用完整 content_width
                            let effective_wrap = if i == 0 {
                                wrap_w
                            } else {
                                content_width.saturating_sub(effective_prefix_w)
                            };
                            let wrapped = wrap_text(line, effective_wrap);
                            for (j, wl) in wrapped.iter().enumerate() {
                                if j > 0 {
                                    flush_line(&mut current_spans, &mut lines);
                                    if in_blockquote {
                                        current_spans.push(Span::styled(
                                            "| ".to_string(),
                                            Style::default().fg(Color::Rgb(80, 100, 140)),
                                        ));
                                    }
                                }
                                current_spans.push(Span::styled(wl.clone(), style));
                            }
                        }
                    }
                }
            }
            Event::SoftBreak => {
                if in_table {
                    current_cell.push(' ');
                } else {
                    current_spans.push(Span::raw(" "));
                }
            }
            Event::HardBreak => {
                if in_table {
                    current_cell.push(' ');
                } else {
                    flush_line(&mut current_spans, &mut lines);
                }
            }
            Event::Rule => {
                flush_line(&mut current_spans, &mut lines);
                lines.push(Line::from(Span::styled(
                    "─".repeat(content_width),
                    Style::default().fg(Color::Rgb(70, 75, 90)),
                )));
            }
            // ===== 表格支持 =====
            Event::Start(Tag::Table(alignments)) => {
                flush_line(&mut current_spans, &mut lines);
                in_table = true;
                table_rows.clear();
                table_alignments = alignments;
            }
            Event::End(TagEnd::Table) => {
                // 表格结束：计算列宽，渲染完整表格
                flush_line(&mut current_spans, &mut lines);
                in_table = false;

                if !table_rows.is_empty() {
                    let num_cols = table_rows.iter().map(|r| r.len()).max().unwrap_or(0);
                    if num_cols > 0 {
                        // 计算每列最大宽度
                        let mut col_widths: Vec<usize> = vec![0; num_cols];
                        for row in &table_rows {
                            for (i, cell) in row.iter().enumerate() {
                                let w = display_width(cell);
                                if w > col_widths[i] {
                                    col_widths[i] = w;
                                }
                            }
                        }

                        // 限制总宽度不超过 content_width，等比缩放
                        let sep_w = num_cols + 1; // 竖线占用
                        let pad_w = num_cols * 2; // 每列左右各1空格
                        let avail = content_width.saturating_sub(sep_w + pad_w);
                        // 单列最大宽度限制（避免一列过宽）
                        let max_col_w = avail * 2 / 3;
                        for cw in col_widths.iter_mut() {
                            if *cw > max_col_w {
                                *cw = max_col_w;
                            }
                        }
                        let total_col_w: usize = col_widths.iter().sum();
                        if total_col_w > avail && total_col_w > 0 {
                            // 等比缩放
                            let mut remaining = avail;
                            for (i, cw) in col_widths.iter_mut().enumerate() {
                                if i == num_cols - 1 {
                                    // 最后一列取剩余宽度，避免取整误差
                                    *cw = remaining.max(1);
                                } else {
                                    *cw = ((*cw) * avail / total_col_w).max(1);
                                    remaining = remaining.saturating_sub(*cw);
                                }
                            }
                        }

                        let table_style = Style::default().fg(Color::Rgb(180, 180, 200));
                        let header_style = Style::default()
                            .fg(Color::Rgb(120, 180, 255))
                            .add_modifier(Modifier::BOLD);
                        let border_style = Style::default().fg(Color::Rgb(60, 70, 100));

                        // 渲染顶边框 ┌─┬─┐
                        let mut top = String::from("┌");
                        for (i, cw) in col_widths.iter().enumerate() {
                            top.push_str(&"─".repeat(cw + 2));
                            if i < num_cols - 1 {
                                top.push('┬');
                            }
                        }
                        top.push('┐');
                        lines.push(Line::from(Span::styled(top, border_style)));

                        for (row_idx, row) in table_rows.iter().enumerate() {
                            // 数据行 │ cell │ cell │
                            let mut row_spans: Vec<Span> = Vec::new();
                            row_spans.push(Span::styled("│", border_style));
                            for (i, cw) in col_widths.iter().enumerate() {
                                let cell_text = row.get(i).map(|s| s.as_str()).unwrap_or("");
                                let cell_w = display_width(cell_text);
                                let text = if cell_w > *cw {
                                    // 截断
                                    let mut t = String::new();
                                    let mut w = 0;
                                    for ch in cell_text.chars() {
                                        let chw = char_width(ch);
                                        if w + chw > *cw {
                                            break;
                                        }
                                        t.push(ch);
                                        w += chw;
                                    }
                                    let fill = cw.saturating_sub(w);
                                    format!(" {}{} ", t, " ".repeat(fill))
                                } else {
                                    // 根据对齐方式填充
                                    let fill = cw.saturating_sub(cell_w);
                                    let align = table_alignments
                                        .get(i)
                                        .copied()
                                        .unwrap_or(pulldown_cmark::Alignment::None);
                                    match align {
                                        pulldown_cmark::Alignment::Center => {
                                            let left = fill / 2;
                                            let right = fill - left;
                                            format!(
                                                " {}{}{} ",
                                                " ".repeat(left),
                                                cell_text,
                                                " ".repeat(right)
                                            )
                                        }
                                        pulldown_cmark::Alignment::Right => {
                                            format!(" {}{} ", " ".repeat(fill), cell_text)
                                        }
                                        _ => {
                                            format!(" {}{} ", cell_text, " ".repeat(fill))
                                        }
                                    }
                                };
                                let style = if row_idx == 0 {
                                    header_style
                                } else {
                                    table_style
                                };
                                row_spans.push(Span::styled(text, style));
                                row_spans.push(Span::styled("│", border_style));
                            }
                            lines.push(Line::from(row_spans));

                            // 表头行后加分隔线 ├─┼─┤
                            if row_idx == 0 {
                                let mut sep = String::from("├");
                                for (i, cw) in col_widths.iter().enumerate() {
                                    sep.push_str(&"─".repeat(cw + 2));
                                    if i < num_cols - 1 {
                                        sep.push('┼');
                                    }
                                }
                                sep.push('┤');
                                lines.push(Line::from(Span::styled(sep, border_style)));
                            }
                        }

                        // 底边框 └─┴─┘
                        let mut bottom = String::from("└");
                        for (i, cw) in col_widths.iter().enumerate() {
                            bottom.push_str(&"─".repeat(cw + 2));
                            if i < num_cols - 1 {
                                bottom.push('┴');
                            }
                        }
                        bottom.push('┘');
                        lines.push(Line::from(Span::styled(bottom, border_style)));
                    }
                }
                table_rows.clear();
                table_alignments.clear();
            }
            Event::Start(Tag::TableHead) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                table_rows.push(current_row.clone());
                current_row.clear();
            }
            Event::Start(Tag::TableRow) => {
                current_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                table_rows.push(current_row.clone());
                current_row.clear();
            }
            Event::Start(Tag::TableCell) => {
                current_cell.clear();
            }
            Event::End(TagEnd::TableCell) => {
                current_row.push(current_cell.clone());
                current_cell.clear();
            }
            _ => {}
        }
    }

    // 刷新最后一行
    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    // 如果解析结果为空，至少返回原始文本
    if lines.is_empty() {
        let wrapped = wrap_text(md, content_width);
        for wl in wrapped {
            lines.push(Line::from(Span::styled(wl, base_style)));
        }
    }

    lines
}

/// 简单的代码语法高亮（无需外部依赖）
/// 根据语言类型对常见关键字、字符串、注释、数字进行着色
fn highlight_code_line<'a>(line: &'a str, lang: &str) -> Vec<Span<'static>> {
    let lang_lower = lang.to_lowercase();
    let keywords: &[&str] = match lang_lower.as_str() {
        "rust" | "rs" => &[
            "fn", "let", "mut", "pub", "use", "mod", "struct", "enum", "impl", "trait", "for",
            "while", "loop", "if", "else", "match", "return", "self", "Self", "where", "async",
            "await", "move", "ref", "type", "const", "static", "crate", "super", "as", "in",
            "true", "false", "Some", "None", "Ok", "Err",
        ],
        "python" | "py" => &[
            "def", "class", "return", "if", "elif", "else", "for", "while", "import", "from", "as",
            "with", "try", "except", "finally", "raise", "pass", "break", "continue", "yield",
            "lambda", "and", "or", "not", "in", "is", "True", "False", "None", "global",
            "nonlocal", "assert", "del", "async", "await", "self", "print",
        ],
        "javascript" | "js" | "typescript" | "ts" | "jsx" | "tsx" => &[
            "function",
            "const",
            "let",
            "var",
            "return",
            "if",
            "else",
            "for",
            "while",
            "class",
            "new",
            "this",
            "import",
            "export",
            "from",
            "default",
            "async",
            "await",
            "try",
            "catch",
            "finally",
            "throw",
            "typeof",
            "instanceof",
            "true",
            "false",
            "null",
            "undefined",
            "of",
            "in",
            "switch",
            "case",
        ],
        "go" | "golang" => &[
            "func",
            "package",
            "import",
            "return",
            "if",
            "else",
            "for",
            "range",
            "struct",
            "interface",
            "type",
            "var",
            "const",
            "defer",
            "go",
            "chan",
            "select",
            "case",
            "switch",
            "default",
            "break",
            "continue",
            "map",
            "true",
            "false",
            "nil",
            "make",
            "append",
            "len",
            "cap",
        ],
        "java" | "kotlin" | "kt" => &[
            "public",
            "private",
            "protected",
            "class",
            "interface",
            "extends",
            "implements",
            "return",
            "if",
            "else",
            "for",
            "while",
            "new",
            "this",
            "import",
            "package",
            "static",
            "final",
            "void",
            "int",
            "String",
            "boolean",
            "true",
            "false",
            "null",
            "try",
            "catch",
            "throw",
            "throws",
            "fun",
            "val",
            "var",
            "when",
            "object",
            "companion",
        ],
        "sh" | "bash" | "zsh" | "shell" => &[
            "if",
            "then",
            "else",
            "elif",
            "fi",
            "for",
            "while",
            "do",
            "done",
            "case",
            "esac",
            "function",
            "return",
            "exit",
            "echo",
            "export",
            "local",
            "readonly",
            "set",
            "unset",
            "shift",
            "source",
            "in",
            "true",
            "false",
            "read",
            "declare",
            "typeset",
            "trap",
            "eval",
            "exec",
            "test",
            "select",
            "until",
            "break",
            "continue",
            "printf",
            // Go 命令
            "go",
            "build",
            "run",
            "test",
            "fmt",
            "vet",
            "mod",
            "get",
            "install",
            "clean",
            "doc",
            "list",
            "version",
            "env",
            "generate",
            "tool",
            "proxy",
            "GOPATH",
            "GOROOT",
            "GOBIN",
            "GOMODCACHE",
            "GOPROXY",
            "GOSUMDB",
            // Cargo 命令
            "cargo",
            "new",
            "init",
            "add",
            "remove",
            "update",
            "check",
            "clippy",
            "rustfmt",
            "rustc",
            "rustup",
            "publish",
            "install",
            "uninstall",
            "search",
            "tree",
            "locate_project",
            "metadata",
            "audit",
            "watch",
            "expand",
        ],
        "c" | "cpp" | "c++" | "h" | "hpp" => &[
            "int",
            "char",
            "float",
            "double",
            "void",
            "long",
            "short",
            "unsigned",
            "signed",
            "const",
            "static",
            "extern",
            "struct",
            "union",
            "enum",
            "typedef",
            "sizeof",
            "return",
            "if",
            "else",
            "for",
            "while",
            "do",
            "switch",
            "case",
            "break",
            "continue",
            "default",
            "goto",
            "auto",
            "register",
            "volatile",
            "class",
            "public",
            "private",
            "protected",
            "virtual",
            "override",
            "template",
            "namespace",
            "using",
            "new",
            "delete",
            "try",
            "catch",
            "throw",
            "nullptr",
            "true",
            "false",
            "this",
            "include",
            "define",
            "ifdef",
            "ifndef",
            "endif",
        ],
        "sql" => &[
            "SELECT",
            "FROM",
            "WHERE",
            "INSERT",
            "UPDATE",
            "DELETE",
            "CREATE",
            "DROP",
            "ALTER",
            "TABLE",
            "INDEX",
            "INTO",
            "VALUES",
            "SET",
            "AND",
            "OR",
            "NOT",
            "NULL",
            "JOIN",
            "LEFT",
            "RIGHT",
            "INNER",
            "OUTER",
            "ON",
            "GROUP",
            "BY",
            "ORDER",
            "ASC",
            "DESC",
            "HAVING",
            "LIMIT",
            "OFFSET",
            "UNION",
            "AS",
            "DISTINCT",
            "COUNT",
            "SUM",
            "AVG",
            "MIN",
            "MAX",
            "LIKE",
            "IN",
            "BETWEEN",
            "EXISTS",
            "CASE",
            "WHEN",
            "THEN",
            "ELSE",
            "END",
            "BEGIN",
            "COMMIT",
            "ROLLBACK",
            "PRIMARY",
            "KEY",
            "FOREIGN",
            "REFERENCES",
            "select",
            "from",
            "where",
            "insert",
            "update",
            "delete",
            "create",
            "drop",
            "alter",
            "table",
            "index",
            "into",
            "values",
            "set",
            "and",
            "or",
            "not",
            "null",
            "join",
            "left",
            "right",
            "inner",
            "outer",
            "on",
            "group",
            "by",
            "order",
            "asc",
            "desc",
            "having",
            "limit",
            "offset",
            "union",
            "as",
            "distinct",
            "count",
            "sum",
            "avg",
            "min",
            "max",
            "like",
            "in",
            "between",
            "exists",
            "case",
            "when",
            "then",
            "else",
            "end",
            "begin",
            "commit",
            "rollback",
            "primary",
            "key",
            "foreign",
            "references",
        ],
        "yaml" | "yml" => &["true", "false", "null", "yes", "no", "on", "off"],
        "toml" => &[
            "true",
            "false",
            "true",
            "false",
            // Cargo.toml 常用
            "name",
            "version",
            "edition",
            "authors",
            "dependencies",
            "dev-dependencies",
            "build-dependencies",
            "features",
            "workspace",
            "members",
            "exclude",
            "include",
            "path",
            "git",
            "branch",
            "tag",
            "rev",
            "package",
            "lib",
            "bin",
            "example",
            "test",
            "bench",
            "doc",
            "profile",
            "release",
            "debug",
            "opt-level",
            "lto",
            "codegen-units",
            "panic",
            "strip",
            "default",
            "features",
            "optional",
            // 常见配置项
            "repository",
            "homepage",
            "documentation",
            "license",
            "license-file",
            "keywords",
            "categories",
            "readme",
            "description",
            "resolver",
        ],
        "css" | "scss" | "less" => &[
            "color",
            "background",
            "border",
            "margin",
            "padding",
            "display",
            "position",
            "width",
            "height",
            "font",
            "text",
            "flex",
            "grid",
            "align",
            "justify",
            "important",
            "none",
            "auto",
            "inherit",
            "initial",
            "unset",
        ],
        "dockerfile" | "docker" => &[
            "FROM",
            "RUN",
            "CMD",
            "LABEL",
            "EXPOSE",
            "ENV",
            "ADD",
            "COPY",
            "ENTRYPOINT",
            "VOLUME",
            "USER",
            "WORKDIR",
            "ARG",
            "ONBUILD",
            "STOPSIGNAL",
            "HEALTHCHECK",
            "SHELL",
            "AS",
        ],
        "ruby" | "rb" => &[
            "def", "end", "class", "module", "if", "elsif", "else", "unless", "while", "until",
            "for", "do", "begin", "rescue", "ensure", "raise", "return", "yield", "require",
            "include", "attr", "self", "true", "false", "nil", "puts", "print",
        ],
        _ => &[
            "fn", "function", "def", "class", "return", "if", "else", "for", "while", "import",
            "export", "const", "let", "var", "true", "false", "null", "nil", "None", "self",
            "this",
        ],
    };

    let comment_prefix = match lang_lower.as_str() {
        "python" | "py" | "sh" | "bash" | "zsh" | "shell" | "ruby" | "rb" | "yaml" | "yml"
        | "toml" | "dockerfile" | "docker" => "#",
        "sql" => "--",
        "css" | "scss" | "less" => "/*",
        _ => "//",
    };

    // 默认代码颜色
    let code_style = Style::default().fg(Color::Rgb(200, 200, 210));
    // 关键字颜色
    let kw_style = Style::default().fg(Color::Rgb(198, 120, 221));
    // 字符串颜色
    let str_style = Style::default().fg(Color::Rgb(152, 195, 121));
    // 注释颜色
    let comment_style = Style::default()
        .fg(Color::Rgb(92, 99, 112))
        .add_modifier(Modifier::ITALIC);
    // 数字颜色
    let num_style = Style::default().fg(Color::Rgb(209, 154, 102));
    // 类型/大写开头标识符
    let type_style = Style::default().fg(Color::Rgb(229, 192, 123));

    let trimmed = line.trim_start();

    // 注释行
    if trimmed.starts_with(comment_prefix) {
        return vec![Span::styled(line.to_string(), comment_style)];
    }

    // 逐词解析
    let mut spans = Vec::new();
    let mut chars = line.chars().peekable();
    let mut buf = String::new();

    while let Some(&ch) = chars.peek() {
        // 字符串
        if ch == '"' || ch == '\'' || ch == '`' {
            // 先刷新 buf
            if !buf.is_empty() {
                spans.extend(colorize_tokens(
                    &buf, keywords, code_style, kw_style, num_style, type_style,
                ));
                buf.clear();
            }
            let quote = ch;
            let mut s = String::new();
            s.push(ch);
            chars.next();
            while let Some(&c) = chars.peek() {
                s.push(c);
                chars.next();
                if c == quote && !s.ends_with("\\\\") {
                    break;
                }
            }
            spans.push(Span::styled(s, str_style));
            continue;
        }
        // Shell 变量 ($VAR, ${VAR}, $1 等)
        if ch == '$'
            && matches!(
                lang_lower.as_str(),
                "sh" | "bash" | "zsh" | "shell" | "dockerfile" | "docker"
            )
        {
            if !buf.is_empty() {
                spans.extend(colorize_tokens(
                    &buf, keywords, code_style, kw_style, num_style, type_style,
                ));
                buf.clear();
            }
            let var_style = Style::default().fg(Color::Rgb(86, 182, 194));
            let mut var = String::new();
            var.push(ch);
            chars.next();
            if let Some(&next_ch) = chars.peek() {
                if next_ch == '{' {
                    // ${VAR}
                    var.push(next_ch);
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        var.push(c);
                        chars.next();
                        if c == '}' {
                            break;
                        }
                    }
                } else if next_ch == '(' {
                    // $(cmd)
                    var.push(next_ch);
                    chars.next();
                    let mut depth = 1;
                    while let Some(&c) = chars.peek() {
                        var.push(c);
                        chars.next();
                        if c == '(' {
                            depth += 1;
                        }
                        if c == ')' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                } else if next_ch.is_alphanumeric()
                    || next_ch == '_'
                    || next_ch == '@'
                    || next_ch == '#'
                    || next_ch == '?'
                    || next_ch == '!'
                {
                    // $VAR, $1, $@, $#, $? 等
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            var.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                }
            }
            spans.push(Span::styled(var, var_style));
            continue;
        }
        // 行内注释
        if ch == '/' || ch == '#' {
            let rest: String = chars.clone().collect();
            if rest.starts_with(comment_prefix) {
                if !buf.is_empty() {
                    spans.extend(colorize_tokens(
                        &buf, keywords, code_style, kw_style, num_style, type_style,
                    ));
                    buf.clear();
                }
                spans.push(Span::styled(rest, comment_style));
                break;
            }
        }
        buf.push(ch);
        chars.next();
    }

    if !buf.is_empty() {
        spans.extend(colorize_tokens(
            &buf, keywords, code_style, kw_style, num_style, type_style,
        ));
    }

    if spans.is_empty() {
        spans.push(Span::styled(line.to_string(), code_style));
    }

    spans
}

/// 将文本按照 word boundary 拆分并对关键字、数字、类型名着色
fn colorize_tokens<'a>(
    text: &str,
    keywords: &[&str],
    default_style: Style,
    kw_style: Style,
    num_style: Style,
    type_style: Style,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current_word = String::new();
    let mut current_non_word = String::new();

    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            if !current_non_word.is_empty() {
                spans.push(Span::styled(current_non_word.clone(), default_style));
                current_non_word.clear();
            }
            current_word.push(ch);
        } else {
            if !current_word.is_empty() {
                let style = if keywords.contains(&current_word.as_str()) {
                    kw_style
                } else if current_word
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                {
                    num_style
                } else if current_word
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    type_style
                } else {
                    default_style
                };
                spans.push(Span::styled(current_word.clone(), style));
                current_word.clear();
            }
            current_non_word.push(ch);
        }
    }

    // 刷新剩余
    if !current_non_word.is_empty() {
        spans.push(Span::styled(current_non_word, default_style));
    }
    if !current_word.is_empty() {
        let style = if keywords.contains(&current_word.as_str()) {
            kw_style
        } else if current_word
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
        {
            num_style
        } else if current_word
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            type_style
        } else {
            default_style
        };
        spans.push(Span::styled(current_word, style));
    }

    spans
}

/// 简单文本自动换行
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
        let ch_width = char_width(ch);
        if current_width + ch_width > max_width && !current_line.is_empty() {
            result.push(current_line.clone());
            current_line.clear();
            current_width = 0;
        }
        current_line.push(ch);
        current_width += ch_width;
    }
    if !current_line.is_empty() {
        result.push(current_line);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

/// 计算字符串的显示宽度
/// 使用 unicode-width 规则：CJK 字符宽度2，其他（含 Box Drawing、符号等）宽度1
fn display_width(s: &str) -> usize {
    s.chars().map(|c| char_width(c)).sum()
}

/// 计算单个字符的显示宽度
fn char_width(c: char) -> usize {
    if c.is_ascii() {
        return 1;
    }
    // CJK Unified Ideographs 及扩展
    let cp = c as u32;
    if (0x4E00..=0x9FFF).contains(&cp)    // CJK Unified Ideographs
        || (0x3400..=0x4DBF).contains(&cp) // CJK Unified Ideographs Extension A
        || (0x20000..=0x2A6DF).contains(&cp) // Extension B
        || (0x2A700..=0x2B73F).contains(&cp) // Extension C
        || (0x2B740..=0x2B81F).contains(&cp) // Extension D
        || (0xF900..=0xFAFF).contains(&cp)   // CJK Compatibility Ideographs
        || (0x2F800..=0x2FA1F).contains(&cp)  // CJK Compatibility Ideographs Supplement
        // CJK 标点和符号
        || (0x3000..=0x303F).contains(&cp)    // CJK Symbols and Punctuation
        || (0xFF01..=0xFF60).contains(&cp)    // Fullwidth Forms
        || (0xFFE0..=0xFFE6).contains(&cp)    // Fullwidth Signs
        // 日韩
        || (0x3040..=0x309F).contains(&cp)    // Hiragana
        || (0x30A0..=0x30FF).contains(&cp)    // Katakana
        || (0xAC00..=0xD7AF).contains(&cp)    // Hangul Syllables
        // Emoji（常见范围）
        || (0x1F300..=0x1F9FF).contains(&cp)
        || (0x2600..=0x26FF).contains(&cp)
        || (0x2700..=0x27BF).contains(&cp)
    {
        2
    } else {
        1
    }
}

/// 绘制输入区
fn draw_input(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    // 输入区可用宽度（减去边框2 + prompt 4）
    let usable_width = area.width.saturating_sub(2 + 4) as usize;

    let chars: Vec<char> = app.input.chars().collect();

    // 计算光标之前文本的显示宽度，决定是否需要水平滚动
    let before_all: String = chars[..app.cursor_pos].iter().collect();
    let before_width = display_width(&before_all);

    // 如果光标超出可视范围，从光标附近开始显示
    let scroll_offset_chars = if before_width >= usable_width {
        // 往回找到一个合适的起始字符位置
        let target_width = before_width.saturating_sub(usable_width / 2);
        let mut w = 0;
        let mut skip = 0;
        for (i, &ch) in chars.iter().enumerate() {
            if w >= target_width {
                skip = i;
                break;
            }
            w += char_width(ch);
        }
        skip
    } else {
        0
    };

    // 截取可见部分的字符
    let visible_chars = &chars[scroll_offset_chars..];
    let cursor_in_visible = app.cursor_pos - scroll_offset_chars;

    let before: String = visible_chars[..cursor_in_visible].iter().collect();
    let cursor_ch = if cursor_in_visible < visible_chars.len() {
        visible_chars[cursor_in_visible].to_string()
    } else {
        " ".to_string()
    };
    let after: String = if cursor_in_visible < visible_chars.len() {
        visible_chars[cursor_in_visible + 1..].iter().collect()
    } else {
        String::new()
    };

    let prompt_style = if app.is_loading {
        Style::default().fg(Color::Rgb(255, 200, 80))
    } else {
        Style::default().fg(Color::Rgb(100, 200, 130))
    };
    let prompt_text = if app.is_loading { " .. " } else { " >  " };

    // 构建多行输入显示（手动换行）
    let full_visible = format!("{}{}{}", before, cursor_ch, after);
    let inner_height = area.height.saturating_sub(2) as usize; // 减去边框
    let wrapped_lines = wrap_text(&full_visible, usable_width);

    // 找到光标所在的行索引
    let before_len = before.chars().count();
    let cursor_len = cursor_ch.chars().count();
    let cursor_global_pos = before_len; // 光标在全部可见字符中的位置
    let mut cursor_line_idx: usize = 0;
    {
        let mut cumulative = 0usize;
        for (li, wl) in wrapped_lines.iter().enumerate() {
            let line_char_count = wl.chars().count();
            if cumulative + line_char_count > cursor_global_pos {
                cursor_line_idx = li;
                break;
            }
            cumulative += line_char_count;
            cursor_line_idx = li; // 光标恰好在最后一行末尾
        }
    }

    // 计算行滚动：确保光标所在行在可见区域内
    let line_scroll = if wrapped_lines.len() <= inner_height {
        0
    } else if cursor_line_idx < inner_height {
        0
    } else {
        // 让光标行显示在可见区域的最后一行
        cursor_line_idx.saturating_sub(inner_height - 1)
    };

    // 构建带光标高亮的行
    let mut display_lines: Vec<Line> = Vec::new();
    let mut char_offset: usize = 0;
    // 跳过滚动行的字符数
    for wl in wrapped_lines.iter().take(line_scroll) {
        char_offset += wl.chars().count();
    }

    for (_line_idx, wl) in wrapped_lines
        .iter()
        .skip(line_scroll)
        .enumerate()
        .take(inner_height.max(1))
    {
        let mut spans: Vec<Span> = Vec::new();
        if _line_idx == 0 && line_scroll == 0 {
            spans.push(Span::styled(prompt_text, prompt_style));
        } else {
            spans.push(Span::styled("    ", Style::default())); // 对齐 prompt
        }

        // 对该行的每个字符分配样式
        let line_chars: Vec<char> = wl.chars().collect();
        let mut seg_start = 0;
        for (ci, &ch) in line_chars.iter().enumerate() {
            let global_idx = char_offset + ci;
            let is_cursor = global_idx >= before_len && global_idx < before_len + cursor_len;

            if is_cursor {
                // 先把 cursor 前的部分输出
                if ci > seg_start {
                    let seg: String = line_chars[seg_start..ci].iter().collect();
                    spans.push(Span::styled(seg, Style::default().fg(Color::White)));
                }
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default()
                        .fg(Color::Rgb(22, 22, 30))
                        .bg(Color::Rgb(200, 210, 240)),
                ));
                seg_start = ci + 1;
            }
        }
        // 输出剩余部分
        if seg_start < line_chars.len() {
            let seg: String = line_chars[seg_start..].iter().collect();
            spans.push(Span::styled(seg, Style::default().fg(Color::White)));
        }

        char_offset += line_chars.len();
        display_lines.push(Line::from(spans));
    }

    if display_lines.is_empty() {
        display_lines.push(Line::from(vec![
            Span::styled(prompt_text, prompt_style),
            Span::styled(
                " ",
                Style::default()
                    .fg(Color::Rgb(22, 22, 30))
                    .bg(Color::Rgb(200, 210, 240)),
            ),
        ]));
    }

    let input_widget = Paragraph::new(display_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(if app.is_loading {
                Style::default().fg(Color::Rgb(120, 100, 50))
            } else {
                Style::default().fg(Color::Rgb(60, 100, 80))
            })
            .title(Span::styled(
                " 输入消息 ",
                Style::default().fg(Color::Rgb(140, 140, 170)),
            ))
            .style(Style::default().bg(Color::Rgb(26, 26, 38))),
    );

    f.render_widget(input_widget, area);

    // 设置终端光标位置，确保中文输入法 IME 候选窗口在正确位置
    // 计算光标在渲染后的坐标
    if !app.is_loading {
        let prompt_w: u16 = 4; // prompt 宽度
        let border_left: u16 = 1; // 左边框

        // 光标在当前显示行中的列偏移
        let cursor_col_in_line = {
            let mut col = 0usize;
            let mut char_count = 0usize;
            // 跳过 line_scroll 之前的字符
            let mut skip_chars = 0usize;
            for wl in wrapped_lines.iter().take(line_scroll) {
                skip_chars += wl.chars().count();
            }
            // 找到光标在当前行的列
            for wl in wrapped_lines.iter().skip(line_scroll) {
                let line_len = wl.chars().count();
                if skip_chars + char_count + line_len > cursor_global_pos {
                    // 光标在这一行
                    let pos_in_line = cursor_global_pos - (skip_chars + char_count);
                    col = wl.chars().take(pos_in_line).map(|c| char_width(c)).sum();
                    break;
                }
                char_count += line_len;
            }
            col as u16
        };

        // 光标在显示行中的行偏移
        let cursor_row_in_display = (cursor_line_idx - line_scroll) as u16;

        let cursor_x = area.x + border_left + prompt_w + cursor_col_in_line;
        let cursor_y = area.y + 1 + cursor_row_in_display; // +1 跳过上边框

        // 确保光标在区域内
        if cursor_x < area.x + area.width && cursor_y < area.y + area.height {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

/// 绘制底部操作提示栏（始终可见）
fn draw_hint_bar(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let hints = match app.mode {
        ChatMode::Chat => {
            vec![
                ("Enter", "发送"),
                ("↑↓", "滚动"),
                ("Ctrl+T", "切换模型"),
                ("Ctrl+L", "清空"),
                ("Ctrl+Y", "复制"),
                ("Ctrl+B", "浏览"),
                ("Ctrl+S", "流式切换"),
                ("?/F1", "帮助"),
                ("Esc", "退出"),
            ]
        }
        ChatMode::SelectModel => {
            vec![("↑↓/jk", "移动"), ("Enter", "确认"), ("Esc", "取消")]
        }
        ChatMode::Browse => {
            vec![("↑↓", "选择消息"), ("y/Enter", "复制"), ("Esc", "返回")]
        }
        ChatMode::Help => {
            vec![("任意键", "返回")]
        }
    };

    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(" ", Style::default()));
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                "  │  ",
                Style::default().fg(Color::Rgb(50, 50, 65)),
            ));
        }
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(Color::Rgb(22, 22, 30))
                .bg(Color::Rgb(100, 110, 140)),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(Color::Rgb(120, 120, 150)),
        ));
    }

    let hint_bar =
        Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Rgb(22, 22, 30)));
    f.render_widget(hint_bar, area);
}

/// 绘制 Toast 弹窗（右上角浮层）
fn draw_toast(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    if let Some((ref msg, is_error, _)) = app.toast {
        let text_width = display_width(msg);
        // toast 宽度 = 文字宽度 + 左右 padding(各2) + emoji(2) + border(2)
        let toast_width = (text_width + 10).min(area.width as usize).max(16) as u16;
        let toast_height: u16 = 3;

        // 定位到右上角
        let x = area.width.saturating_sub(toast_width + 1);
        let y: u16 = 1;

        if x + toast_width <= area.width && y + toast_height <= area.height {
            let toast_area = Rect::new(x, y, toast_width, toast_height);

            // 先清空区域背景
            let clear = Block::default().style(Style::default().bg(if is_error {
                Color::Rgb(60, 20, 20)
            } else {
                Color::Rgb(20, 50, 30)
            }));
            f.render_widget(clear, toast_area);

            let (icon, border_color, text_color) = if is_error {
                ("❌", Color::Rgb(200, 70, 70), Color::Rgb(255, 130, 130))
            } else {
                ("✅", Color::Rgb(60, 160, 80), Color::Rgb(140, 230, 160))
            };

            let toast_widget = Paragraph::new(Line::from(vec![
                Span::styled(format!(" {} ", icon), Style::default()),
                Span::styled(msg.as_str(), Style::default().fg(text_color)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .style(Style::default().bg(if is_error {
                        Color::Rgb(50, 18, 18)
                    } else {
                        Color::Rgb(18, 40, 25)
                    })),
            );
            f.render_widget(toast_widget, toast_area);
        }
    }
}

/// 绘制模型选择界面
fn draw_model_selector(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let items: Vec<ListItem> = app
        .agent_config
        .providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_active = i == app.agent_config.active_index;
            let marker = if is_active { " ● " } else { " ○ " };
            let style = if is_active {
                Style::default()
                    .fg(Color::Rgb(120, 220, 160))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(180, 180, 200))
            };
            let detail = format!("{}{}  ({})", marker, p.name, p.model);
            ListItem::new(Line::from(Span::styled(detail, style)))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(180, 160, 80)))
                .title(Span::styled(
                    " 🔄 选择模型 ",
                    Style::default()
                        .fg(Color::Rgb(230, 210, 120))
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(Color::Rgb(28, 28, 40))),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(50, 55, 80))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("  ▸ ");

    f.render_stateful_widget(list, area, &mut app.model_list_state);
}

/// 绘制帮助界面
fn draw_help(f: &mut ratatui::Frame, area: Rect) {
    let separator = Line::from(Span::styled(
        "  ─────────────────────────────────────────",
        Style::default().fg(Color::Rgb(50, 55, 70)),
    ));

    let help_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  📖 快捷键帮助",
            Style::default()
                .fg(Color::Rgb(120, 180, 255))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        separator.clone(),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Enter        ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("发送消息", Style::default().fg(Color::Rgb(200, 200, 220))),
        ]),
        Line::from(vec![
            Span::styled(
                "  ↑ / ↓        ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "滚动对话记录",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  ← / →        ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "移动输入光标",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+T       ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("切换模型", Style::default().fg(Color::Rgb(200, 200, 220))),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+L       ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "清空对话历史",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+Y       ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "复制最后一条 AI 回复",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+B       ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "浏览消息 (↑↓选择, y/Enter复制)",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+S       ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "切换流式/整体输出",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Esc / Ctrl+C ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("退出对话", Style::default().fg(Color::Rgb(200, 200, 220))),
        ]),
        Line::from(vec![
            Span::styled(
                "  ? / F1       ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "显示 / 关闭此帮助",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(""),
        separator,
        Line::from(""),
        Line::from(Span::styled(
            "  📁 配置文件:",
            Style::default()
                .fg(Color::Rgb(120, 180, 255))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("     {}", agent_config_path().display()),
            Style::default().fg(Color::Rgb(100, 100, 130)),
        )),
    ];

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(80, 100, 140)))
        .title(Span::styled(
            " 帮助 (按任意键返回) ",
            Style::default().fg(Color::Rgb(140, 140, 170)),
        ))
        .style(Style::default().bg(Color::Rgb(24, 24, 34)));
    let help_widget = Paragraph::new(help_lines).block(help_block);
    f.render_widget(help_widget, area);
}

/// 对话模式按键处理，返回 true 表示退出
fn handle_chat_mode(app: &mut ChatApp, key: KeyEvent) -> bool {
    // Ctrl+C 强制退出
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }

    // Ctrl+T 切换模型（替代 Ctrl+M，因为 Ctrl+M 在终端中等于 Enter）
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('t') {
        if !app.agent_config.providers.is_empty() {
            app.mode = ChatMode::SelectModel;
            app.model_list_state
                .select(Some(app.agent_config.active_index));
        }
        return false;
    }

    // Ctrl+L 清空对话
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('l') {
        app.clear_session();
        return false;
    }

    // Ctrl+Y 复制最后一条 AI 回复
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('y') {
        if let Some(last_ai) = app
            .session
            .messages
            .iter()
            .rev()
            .find(|m| m.role == "assistant")
        {
            if copy_to_clipboard(&last_ai.content) {
                app.show_toast("已复制最后一条 AI 回复", false);
            } else {
                app.show_toast("复制到剪切板失败", true);
            }
        } else {
            app.show_toast("暂无 AI 回复可复制", true);
        }
        return false;
    }

    // Ctrl+B 进入消息浏览模式（可选中历史消息并复制）
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
        if !app.session.messages.is_empty() {
            // 默认选中最后一条消息
            app.browse_msg_index = app.session.messages.len() - 1;
            app.mode = ChatMode::Browse;
            app.msg_lines_cache = None; // 清除缓存以触发高亮重绘
        } else {
            app.show_toast("暂无消息可浏览", true);
        }
        return false;
    }

    // Ctrl+S 切换流式/非流式输出
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        app.agent_config.stream_mode = !app.agent_config.stream_mode;
        let _ = save_agent_config(&app.agent_config);
        let mode_str = if app.agent_config.stream_mode {
            "流式输出"
        } else {
            "整体输出"
        };
        app.show_toast(&format!("已切换为: {}", mode_str), false);
        return false;
    }

    let char_count = app.input.chars().count();

    match key.code {
        KeyCode::Esc => return true,

        KeyCode::Enter => {
            if !app.is_loading {
                app.send_message();
            }
        }

        // 滚动消息
        KeyCode::Up => app.scroll_up(),
        KeyCode::Down => app.scroll_down(),
        KeyCode::PageUp => {
            for _ in 0..10 {
                app.scroll_up();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                app.scroll_down();
            }
        }

        // 光标移动
        KeyCode::Left => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
            }
        }
        KeyCode::Right => {
            if app.cursor_pos < char_count {
                app.cursor_pos += 1;
            }
        }
        KeyCode::Home => app.cursor_pos = 0,
        KeyCode::End => app.cursor_pos = char_count,

        // 删除
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                let start = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let end = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
                app.input.drain(start..end);
                app.cursor_pos -= 1;
            }
        }
        KeyCode::Delete => {
            if app.cursor_pos < char_count {
                let start = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
                let end = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos + 1)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
                app.input.drain(start..end);
            }
        }

        // F1 任何时候都能唤起帮助
        KeyCode::F(1) => {
            app.mode = ChatMode::Help;
        }
        // 输入框为空时，? 也可唤起帮助
        KeyCode::Char('?') if app.input.is_empty() => {
            app.mode = ChatMode::Help;
        }
        KeyCode::Char(c) => {
            let byte_idx = app
                .input
                .char_indices()
                .nth(app.cursor_pos)
                .map(|(i, _)| i)
                .unwrap_or(app.input.len());
            app.input.insert_str(byte_idx, &c.to_string());
            app.cursor_pos += 1;
        }

        _ => {}
    }

    false
}

/// 消息浏览模式按键处理：↑↓ 选择消息，y/Enter 复制选中消息，Esc 退出
fn handle_browse_mode(app: &mut ChatApp, key: KeyEvent) {
    let msg_count = app.session.messages.len();
    if msg_count == 0 {
        app.mode = ChatMode::Chat;
        app.msg_lines_cache = None;
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.mode = ChatMode::Chat;
            app.msg_lines_cache = None; // 退出浏览模式时清除缓存，去掉高亮
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.browse_msg_index > 0 {
                app.browse_msg_index -= 1;
                app.msg_lines_cache = None; // 选中变化时清缓存
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.browse_msg_index < msg_count - 1 {
                app.browse_msg_index += 1;
                app.msg_lines_cache = None; // 选中变化时清缓存
            }
        }
        KeyCode::Enter | KeyCode::Char('y') => {
            // 复制选中消息的原始内容到剪切板
            if let Some(msg) = app.session.messages.get(app.browse_msg_index) {
                let content = msg.content.clone();
                let role_label = if msg.role == "assistant" {
                    "AI"
                } else if msg.role == "user" {
                    "用户"
                } else {
                    "系统"
                };
                if copy_to_clipboard(&content) {
                    app.show_toast(
                        &format!("已复制第 {} 条{}消息", app.browse_msg_index + 1, role_label),
                        false,
                    );
                } else {
                    app.show_toast("复制到剪切板失败", true);
                }
            }
        }
        _ => {}
    }
}

/// 模型选择模式按键处理
fn handle_select_model(app: &mut ChatApp, key: KeyEvent) {
    let count = app.agent_config.providers.len();
    match key.code {
        KeyCode::Esc => {
            app.mode = ChatMode::Chat;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if count > 0 {
                let i = app
                    .model_list_state
                    .selected()
                    .map(|i| if i == 0 { count - 1 } else { i - 1 })
                    .unwrap_or(0);
                app.model_list_state.select(Some(i));
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if count > 0 {
                let i = app
                    .model_list_state
                    .selected()
                    .map(|i| if i >= count - 1 { 0 } else { i + 1 })
                    .unwrap_or(0);
                app.model_list_state.select(Some(i));
            }
        }
        KeyCode::Enter => {
            app.switch_model();
        }
        _ => {}
    }
}

/// 复制内容到系统剪切板
fn copy_to_clipboard(content: &str) -> bool {
    use std::process::{Command, Stdio};

    let (cmd, args): (&str, Vec<&str>) = if cfg!(target_os = "macos") {
        ("pbcopy", vec![])
    } else if cfg!(target_os = "linux") {
        if Command::new("which")
            .arg("xclip")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            ("xclip", vec!["-selection", "clipboard"])
        } else {
            ("xsel", vec!["--clipboard", "--input"])
        }
    } else {
        return false;
    };

    let child = Command::new(cmd).args(&args).stdin(Stdio::piped()).spawn();

    match child {
        Ok(mut child) => {
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(content.as_bytes());
            }
            child.wait().map(|s| s.success()).unwrap_or(false)
        }
        Err(_) => false,
    }
}
