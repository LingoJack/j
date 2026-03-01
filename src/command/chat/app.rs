use super::api::{build_request_with_tools, create_openai_client};
use super::model::{
    AgentConfig, ChatMessage, ChatSession, ModelProvider, ToolCallItem, load_agent_config,
    load_chat_session, load_style, load_system_prompt, save_agent_config, save_chat_session,
    save_system_prompt, system_prompt_path,
};
use super::skill::{self, Skill};
use super::theme::Theme;
use super::tools::ToolRegistry;
use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS, TOAST_DURATION_SECS};
use crate::util::log::write_error_log;
use async_openai::types::chat::ChatCompletionTools;
use futures::StreamExt;
use ratatui::text::Line;
use ratatui::widgets::ListState;
use std::sync::{Arc, Mutex, mpsc};
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
    pub tool_registry: ToolRegistry,
    /// 当前活跃的工具调用状态列表
    pub active_tool_calls: Vec<ToolCallStatus>,
    /// ToolConfirm 模式中当前待处理工具的索引
    pub pending_tool_idx: usize,
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
    /// 配置界面：是否有待处理的 style 编辑（需弹出全屏编辑器）
    pub pending_style_edit: bool,
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
    /// 工具调用确认模式（弹出确认框）
    ToolConfirm,
}

/// 所有字段数 = provider 字段 + 全局字段
pub fn config_total_fields() -> usize {
    CONFIG_FIELDS.len() + CONFIG_GLOBAL_FIELDS.len()
}

/// 默认系统提示词模板（编译时嵌入）
const DEFAULT_SYSTEM_PROMPT: &str = include_str!("../../../assets/system_prompt_default.md");

impl ChatApp {
    pub fn new() -> Self {
        let mut agent_config = load_agent_config();
        // 加载 system_prompt
        if let Some(file_prompt) = load_system_prompt() {
            agent_config.system_prompt = Some(file_prompt);
        } else if !system_prompt_path().exists() {
            if let Some(config_prompt) = agent_config.system_prompt.clone() {
                let _ = save_system_prompt(&config_prompt);
            } else {
                // 首次运行：写入默认系统提示词
                let _ = save_system_prompt(DEFAULT_SYSTEM_PROMPT);
                agent_config.system_prompt = Some(DEFAULT_SYSTEM_PROMPT.to_string());
            }
        }
        // 加载 style
        if let Some(s) = load_style() {
            agent_config.style = Some(s);
        }
        let session = load_chat_session();
        let mut model_list_state = ListState::default();
        if !agent_config.providers.is_empty() {
            model_list_state.select(Some(agent_config.active_index));
        }
        let theme = Theme::from_name(&agent_config.theme);
        let loaded_skills = skill::load_all_skills();
        let tool_registry = ToolRegistry::new(loaded_skills.clone());
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
            active_tool_calls: Vec::new(),
            pending_tool_idx: 0,
            pending_system_prompt_edit: false,
            loaded_skills,
            at_popup_active: false,
            at_popup_filter: String::new(),
            at_popup_start_pos: 0,
            at_popup_selected: 0,
            pending_style_edit: false,
        }
    }

    /// 解析系统提示词模板，替换 {{.skills}}、{{.tools}}、{{.style}} 占位符
    pub fn resolve_system_prompt(&self) -> Option<String> {
        let template = self.agent_config.system_prompt.as_ref()?;
        let skills_summary = skill::build_skills_summary(&self.loaded_skills);
        let tools_summary = self.tool_registry.build_tools_summary();
        let style_text = self.agent_config.style.as_deref().unwrap_or("（未设置）");

        let resolved = template
            .replace("{{.skills}}", &skills_summary)
            .replace("{{.tools}}", &tools_summary)
            .replace("{{.style}}", style_text);
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

        // 添加用户消息
        self.session.messages.push(ChatMessage::text("user", text));
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
        self.active_tool_calls.clear();
        self.pending_tool_idx = 0;

        let api_messages = self.build_api_messages();

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
            self.tool_registry.to_openai_tools()
        } else {
            vec![]
        };

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
            ));
        });
    }

    /// 处理后台流式消息（在主循环中每帧调用）
    pub fn poll_stream(&mut self) {
        if self.stream_rx.is_none() {
            return;
        }

        // 如果在 ToolConfirm 模式，暂停轮询（等待用户操作）
        if self.mode == ChatMode::ToolConfirm {
            return;
        }

        let mut finished = false;
        let mut had_error = false;

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
                            let confirm_msg = if let Some(tool) = self.tool_registry.get(&tc.name) {
                                tool.confirmation_message(&tc.arguments)
                            } else {
                                format!("调用工具 {} 参数: {}", tc.name, tc.arguments)
                            };
                            let needs_confirm = self
                                .tool_registry
                                .get(&tc.name)
                                .map(|t| t.requires_confirmation())
                                .unwrap_or(false);
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

                        // 找第一个需要确认的工具
                        let first_confirm_idx = self
                            .active_tool_calls
                            .iter()
                            .position(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm));

                        if let Some(idx) = first_confirm_idx {
                            self.pending_tool_idx = idx;
                            self.mode = ChatMode::ToolConfirm;
                            // 直接执行不需要确认的工具（在弹出确认框前）
                            // 注意：确认框出现后，需要等用户按键，由 execute_pending_tool / reject_pending_tool 驱动
                        } else {
                            // 全部不需要确认，直接执行所有工具
                            self.execute_all_tools_no_confirm();
                        }
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
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        finished = true;
                        break;
                    }
                }
            }
        }

        if finished {
            self.finish_loading(had_error);
        }
    }

    /// 执行所有不需要确认的工具
    fn execute_all_tools_no_confirm(&mut self) {
        for tc_status in self.active_tool_calls.iter_mut() {
            if matches!(tc_status.status, ToolExecStatus::Executing) {
                let result = if let Some(tool) = self.tool_registry.get(&tc_status.tool_name) {
                    tool.execute(&tc_status.arguments)
                } else {
                    super::tools::ToolResult {
                        output: format!("未知工具: {}", tc_status.tool_name),
                        is_error: true,
                    }
                };
                let summary = if result.output.len() > 60 {
                    let mut end = 60;
                    while !result.output.is_char_boundary(end) {
                        end -= 1;
                    }
                    format!("{}...", &result.output[..end])
                } else {
                    result.output.clone()
                };
                let is_error = result.is_error;
                if let Some(ref tx) = self.tool_result_tx {
                    let _ = tx.send(ToolResultMsg {
                        tool_call_id: tc_status.tool_call_id.clone(),
                        result: result.output,
                        is_error,
                    });
                }
                tc_status.status = if is_error {
                    ToolExecStatus::Failed(summary)
                } else {
                    ToolExecStatus::Done(summary)
                };
            }
        }
    }

    /// 用户确认执行当前待处理工具
    pub fn execute_pending_tool(&mut self) {
        let idx = self.pending_tool_idx;
        if idx >= self.active_tool_calls.len() {
            self.mode = ChatMode::Chat;
            return;
        }

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

        let result = if let Some(tool) = self.tool_registry.get(&tool_name) {
            tool.execute(&arguments)
        } else {
            super::tools::ToolResult {
                output: format!("未知工具: {}", tool_name),
                is_error: true,
            }
        };

        let summary = if result.output.len() > 60 {
            let mut end = 60;
            while !result.output.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &result.output[..end])
        } else {
            result.output.clone()
        };
        let is_error = result.is_error;

        if let Some(ref tx) = self.tool_result_tx {
            let _ = tx.send(ToolResultMsg {
                tool_call_id,
                result: result.output,
                is_error,
            });
        }

        {
            let tc_status = &mut self.active_tool_calls[idx];
            tc_status.status = if is_error {
                ToolExecStatus::Failed(summary)
            } else {
                ToolExecStatus::Done(summary)
            };
        }

        // 处理下一个待确认工具
        self.advance_tool_confirm();
    }

    /// 用户拒绝执行当前待处理工具
    pub fn reject_pending_tool(&mut self) {
        let idx = self.pending_tool_idx;
        if idx >= self.active_tool_calls.len() {
            self.mode = ChatMode::Chat;
            return;
        }

        let tool_call_id = self.active_tool_calls[idx].tool_call_id.clone();
        self.active_tool_calls[idx].status = ToolExecStatus::Rejected;

        if let Some(ref tx) = self.tool_result_tx {
            let _ = tx.send(ToolResultMsg {
                tool_call_id,
                result: "用户拒绝执行该工具".to_string(),
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
            // 继续保持 ToolConfirm 模式
        } else {
            // 没有更多需要确认的工具，处理剩余 Executing 状态的工具
            self.execute_all_tools_no_confirm();
            // 退出确认模式，恢复轮询
            self.mode = ChatMode::Chat;
        }
    }

    /// 结束加载状态（流式完成或错误）
    fn finish_loading(&mut self, had_error: bool) {
        self.stream_rx = None;
        self.tool_result_tx = None;
        self.is_loading = false;
        self.last_rendered_streaming_len = 0;
        self.msg_lines_cache = None;
        self.active_tool_calls.clear();

        if !had_error {
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

// ========== Agent 循环（后台异步函数）==========

/// 后台 Agent 循环：支持多轮工具调用
async fn run_agent_loop(
    provider: ModelProvider,
    mut messages: Vec<ChatMessage>,
    tools: Vec<ChatCompletionTools>,
    system_prompt: Option<String>,
    use_stream: bool,
    streaming_content: Arc<Mutex<String>>,
    tx: mpsc::Sender<StreamMsg>,
    tool_result_rx: mpsc::Receiver<ToolResultMsg>,
    max_tool_rounds: usize,
) {
    let client = create_openai_client(&provider);

    for _round in 0..max_tool_rounds {
        // 清空流式内容缓冲（每轮开始时）
        {
            let mut sc = streaming_content.lock().unwrap();
            sc.clear();
        }

        let request = match build_request_with_tools(
            &provider,
            &messages,
            tools.clone(),
            system_prompt.as_deref(),
        ) {
            Ok(req) => req,
            Err(e) => {
                let _ = tx.send(StreamMsg::Error(format!("构建请求失败: {}", e)));
                return;
            }
        };

        if use_stream {
            // 流式模式
            let mut stream = match client.chat().create_stream(request.clone()).await {
                Ok(s) => s,
                Err(e) => {
                    let error_msg = format!("API 请求失败: {}", e);
                    write_error_log("Chat API 流式请求创建", &error_msg);
                    let _ = tx.send(StreamMsg::Error(error_msg));
                    return;
                }
            };

            let mut finish_reason: Option<async_openai::types::chat::FinishReason> = None;
            let mut assistant_text = String::new();
            // 手动收集 tool_calls：按 index 聚合 (id, name, arguments)
            let mut raw_tool_calls: std::collections::BTreeMap<u32, (String, String, String)> =
                std::collections::BTreeMap::new();
            let mut stream_had_deserialize_error = false;

            'stream: while let Some(result) = stream.next().await {
                match result {
                    Ok(response) => {
                        for choice in &response.choices {
                            if let Some(ref content) = choice.delta.content {
                                assistant_text.push_str(content);
                                let mut sc = streaming_content.lock().unwrap();
                                sc.push_str(content);
                                drop(sc);
                                let _ = tx.send(StreamMsg::Chunk);
                            }
                            // 尝试直接读取 tool_calls（若 async-openai 能反序列化）
                            if let Some(ref tc_chunks) = choice.delta.tool_calls {
                                for chunk in tc_chunks {
                                    let entry =
                                        raw_tool_calls.entry(chunk.index).or_insert_with(|| {
                                            (
                                                chunk.id.clone().unwrap_or_default(),
                                                String::new(),
                                                String::new(),
                                            )
                                        });
                                    if entry.0.is_empty() {
                                        if let Some(ref id) = chunk.id {
                                            entry.0 = id.clone();
                                        }
                                    }
                                    if let Some(ref func) = chunk.function {
                                        if let Some(ref name) = func.name {
                                            entry.1.push_str(name);
                                        }
                                        if let Some(ref args) = func.arguments {
                                            entry.2.push_str(args);
                                        }
                                    }
                                }
                            }
                            if let Some(ref fr) = choice.finish_reason {
                                finish_reason = Some(fr.clone());
                            }
                        }
                    }
                    Err(e) => {
                        let error_str = format!("{}", e);
                        // 检测是否是 tool_calls 反序列化错误（Gemini 等不返回 chunk index）
                        if error_str.contains("missing field `index`")
                            || error_str.contains("tool_calls")
                        {
                            // 标记需要用非流式重做，跳出流式循环
                            stream_had_deserialize_error = true;
                            break 'stream;
                        }
                        write_error_log("Chat API 流式响应", &error_str);
                        let _ = tx.send(StreamMsg::Error(error_str));
                        return;
                    }
                }
            }

            // 如果流式遇到 tool_calls 反序列化错误，fallback 到非流式获取完整响应
            if stream_had_deserialize_error {
                // 清空流式内容（切换到非流式）
                {
                    let mut sc = streaming_content.lock().unwrap();
                    sc.clear();
                }
                // 重新构建请求（不带 stream）
                match client.chat().create(request).await {
                    Ok(response) => {
                        if let Some(choice) = response.choices.first() {
                            let is_tool_calls = matches!(
                                choice.finish_reason,
                                Some(async_openai::types::chat::FinishReason::ToolCalls)
                            );
                            if is_tool_calls {
                                if let Some(ref tc_list) = choice.message.tool_calls {
                                    let tool_items: Vec<ToolCallItem> = tc_list
                                        .iter()
                                        .filter_map(|tc| {
                                            if let async_openai::types::chat::ChatCompletionMessageToolCalls::Function(f) = tc {
                                                Some(ToolCallItem {
                                                    id: f.id.clone(),
                                                    name: f.function.name.clone(),
                                                    arguments: f.function.arguments.clone(),
                                                })
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();

                                    if tool_items.is_empty() {
                                        break;
                                    }

                                    let assistant_text =
                                        choice.message.content.clone().unwrap_or_default();
                                    messages.push(ChatMessage {
                                        role: "assistant".to_string(),
                                        content: assistant_text,
                                        tool_calls: Some(tool_items.clone()),
                                        tool_call_id: None,
                                    });

                                    if tx
                                        .send(StreamMsg::ToolCallRequest(tool_items.clone()))
                                        .is_err()
                                    {
                                        return;
                                    }

                                    let mut tool_results: Vec<ToolResultMsg> = Vec::new();
                                    for _ in &tool_items {
                                        match tool_result_rx
                                            .recv_timeout(std::time::Duration::from_secs(60))
                                        {
                                            Ok(result) => tool_results.push(result),
                                            Err(_) => {
                                                let _ = tx.send(StreamMsg::Error(
                                                    "等待工具执行超时".to_string(),
                                                ));
                                                return;
                                            }
                                        }
                                    }

                                    for result in tool_results {
                                        messages.push(ChatMessage {
                                            role: "tool".to_string(),
                                            content: result.result,
                                            tool_calls: None,
                                            tool_call_id: Some(result.tool_call_id),
                                        });
                                    }
                                    continue;
                                }
                            }
                            // 普通文本回复
                            if let Some(ref content) = choice.message.content {
                                let mut sc = streaming_content.lock().unwrap();
                                sc.push_str(content);
                                drop(sc);
                                let _ = tx.send(StreamMsg::Chunk);
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("API 请求失败(fallback): {}", e);
                        write_error_log("Chat API fallback 非流式", &error_msg);
                        let _ = tx.send(StreamMsg::Error(error_msg));
                        return;
                    }
                }
                break;
            }

            // 检查流式模式下的 tool_calls finish_reason
            let is_tool_calls = matches!(
                finish_reason,
                Some(async_openai::types::chat::FinishReason::ToolCalls)
            );

            if is_tool_calls && !raw_tool_calls.is_empty() {
                let tool_items: Vec<ToolCallItem> = raw_tool_calls
                    .into_values()
                    .map(|(id, name, arguments)| ToolCallItem {
                        id,
                        name,
                        arguments,
                    })
                    .collect();

                if tool_items.is_empty() {
                    break;
                }

                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: assistant_text,
                    tool_calls: Some(tool_items.clone()),
                    tool_call_id: None,
                });

                if tx
                    .send(StreamMsg::ToolCallRequest(tool_items.clone()))
                    .is_err()
                {
                    return;
                }

                let mut tool_results: Vec<ToolResultMsg> = Vec::new();
                for _ in &tool_items {
                    match tool_result_rx.recv_timeout(std::time::Duration::from_secs(60)) {
                        Ok(result) => tool_results.push(result),
                        Err(_) => {
                            let _ = tx.send(StreamMsg::Error("等待工具执行超时".to_string()));
                            return;
                        }
                    }
                }

                for result in tool_results {
                    messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: result.result,
                        tool_calls: None,
                        tool_call_id: Some(result.tool_call_id),
                    });
                }

                continue;
            } else {
                // 正常结束
                break;
            }
        } else {
            // 非流式模式
            match client.chat().create(request).await {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        let is_tool_calls = matches!(
                            choice.finish_reason,
                            Some(async_openai::types::chat::FinishReason::ToolCalls)
                        );

                        if is_tool_calls {
                            if let Some(ref tc_list) = choice.message.tool_calls {
                                let tool_items: Vec<ToolCallItem> = tc_list
                                    .iter()
                                    .filter_map(|tc| {
                                        if let async_openai::types::chat::ChatCompletionMessageToolCalls::Function(f) = tc {
                                            Some(ToolCallItem {
                                                id: f.id.clone(),
                                                name: f.function.name.clone(),
                                                arguments: f.function.arguments.clone(),
                                            })
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();

                                if tool_items.is_empty() {
                                    break;
                                }

                                let assistant_text =
                                    choice.message.content.clone().unwrap_or_default();
                                messages.push(ChatMessage {
                                    role: "assistant".to_string(),
                                    content: assistant_text,
                                    tool_calls: Some(tool_items.clone()),
                                    tool_call_id: None,
                                });

                                if tx
                                    .send(StreamMsg::ToolCallRequest(tool_items.clone()))
                                    .is_err()
                                {
                                    return;
                                }

                                let mut tool_results: Vec<ToolResultMsg> = Vec::new();
                                for _ in &tool_items {
                                    match tool_result_rx
                                        .recv_timeout(std::time::Duration::from_secs(60))
                                    {
                                        Ok(result) => tool_results.push(result),
                                        Err(_) => {
                                            let _ = tx.send(StreamMsg::Error(
                                                "等待工具执行超时".to_string(),
                                            ));
                                            return;
                                        }
                                    }
                                }

                                for result in tool_results {
                                    messages.push(ChatMessage {
                                        role: "tool".to_string(),
                                        content: result.result,
                                        tool_calls: None,
                                        tool_call_id: Some(result.tool_call_id),
                                    });
                                }

                                continue;
                            }
                        }

                        // 正常文本回复
                        if let Some(ref content) = choice.message.content {
                            let mut sc = streaming_content.lock().unwrap();
                            sc.push_str(content);
                            drop(sc);
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
            break;
        }
    }

    let _ = tx.send(StreamMsg::Done);
}
