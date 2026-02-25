use super::api::{create_openai_client, to_openai_messages};
use super::model::{
    AgentConfig, ChatMessage, ChatSession, ModelProvider, load_agent_config, load_chat_session,
    save_agent_config, save_chat_session,
};
use super::theme::Theme;
use crate::util::log::write_error_log;
use async_openai::types::chat::CreateChatCompletionRequestArgs;
use futures::StreamExt;
use ratatui::text::Line;
use ratatui::widgets::ListState;
use std::sync::{Arc, Mutex, mpsc};

// ========== TUI 界面 ==========

/// 后台线程发送给 TUI 的消息类型
pub enum StreamMsg {
    /// 收到一个流式文本块
    Chunk,
    /// 流式响应完成
    Done,
    /// 发生错误
    Error(String),
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
}

/// Toast 通知显示时长（秒）
pub const TOAST_DURATION_SECS: u64 = 4;

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
}

/// 配置编辑界面的字段列表
pub const CONFIG_FIELDS: &[&str] = &["name", "api_base", "api_key", "model"];
/// 全局配置字段
pub const CONFIG_GLOBAL_FIELDS: &[&str] = &[
    "system_prompt",
    "stream_mode",
    "max_history_messages",
    "theme",
];
/// 所有字段数 = provider 字段 + 全局字段
pub fn config_total_fields() -> usize {
    CONFIG_FIELDS.len() + CONFIG_GLOBAL_FIELDS.len()
}

impl ChatApp {
    pub fn new() -> Self {
        let agent_config = load_agent_config();
        let session = load_chat_session();
        let mut model_list_state = ListState::default();
        if !agent_config.providers.is_empty() {
            model_list_state.select(Some(agent_config.active_index));
        }
        let theme = Theme::from_name(&agent_config.theme);
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
            last_rendered_streaming_len: 0,
            last_stream_render_time: std::time::Instant::now(),
            config_provider_idx: 0,
            config_field_idx: 0,
            config_editing: false,
            config_edit_buf: String::new(),
            config_edit_cursor: 0,
            auto_scroll: true,
            theme,
        }
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
        if let Some((_, _, created)) = &self.toast {
            if created.elapsed().as_secs() >= TOAST_DURATION_SECS {
                self.toast = None;
            }
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
        let mut messages = Vec::new();
        if let Some(sys) = &self.agent_config.system_prompt {
            messages.push(ChatMessage {
                role: "system".to_string(),
                content: sys.clone(),
            });
        }

        // 只取最近的 N 条历史消息，避免 token 消耗过大
        let max_history = self.agent_config.max_history_messages;
        let history_messages: Vec<_> = if self.session.messages.len() > max_history {
            self.session.messages[self.session.messages.len() - max_history..].to_vec()
        } else {
            self.session.messages.clone()
        };

        for msg in history_messages {
            messages.push(msg);
        }
        messages
    }

    /// 发送消息（非阻塞，启动后台线程流式接收）
    pub fn send_message(&mut self) {
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
                            let error_msg = format!("API 请求失败: {}", e);
                            write_error_log("Chat API 流式请求创建", &error_msg);
                            let _ = tx.send(StreamMsg::Error(error_msg));
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
                                let error_str = format!("{}", e);
                                write_error_log("Chat API 流式响应", &error_str);
                                let _ = tx.send(StreamMsg::Error(error_str));
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
                            let error_msg = format!("API 请求失败: {}", e);
                            write_error_log("Chat API 非流式请求", &error_msg);
                            let _ = tx.send(StreamMsg::Error(error_msg));
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
    pub fn poll_stream(&mut self) {
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
                        // 内容已经通过 Arc<Mutex<String>> 更新
                        // 只有在用户没有手动滚动的情况下才自动滚到底部
                        if self.auto_scroll {
                            self.scroll_offset = u16::MAX;
                        }
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
            // 重置流式节流状态
            self.last_rendered_streaming_len = 0;
            // 清除缓存，流式结束后需要完整重建（新消息已加入 session）
            self.msg_lines_cache = None;

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
                if self.auto_scroll {
                    self.scroll_offset = u16::MAX;
                }
            } else {
                // 错误时也清空流式缓冲
                self.streaming_content.lock().unwrap().clear();
            }

            // 自动保存对话历史
            let _ = save_chat_session(&self.session);
        }
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
}
