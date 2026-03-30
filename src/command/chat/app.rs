use super::agent::run_agent_loop;
use super::compact::CompactConfig;
use super::hook::{HookContext, HookEvent, HookManager};
use super::markdown::image_cache::ImageCache;
use super::permission::JcliConfig;
use super::sandbox::Sandbox;
use super::skill::{self, Skill, skills_dir};
use super::storage::{
    AgentConfig, ChatMessage, ChatSession, ModelProvider, SessionEvent, ToolCallItem,
    append_session_event, load_agent_config, memory_path, save_agent_config, save_memory,
    save_soul, save_system_prompt, soul_path, system_prompt_path,
};
use super::theme::Theme;
use super::tools::ToolRegistry;
use super::tools::background::BackgroundManager;
use crate::command::chat::constants::{ROLE_ASSISTANT, ROLE_TOOL, ROLE_USER};
use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS, TOAST_DURATION_SECS};
use crate::util::log::write_info_log;
use crate::util::safe_lock;
use async_openai::types::chat::ChatCompletionTools;
use ratatui::text::Line;
use ratatui::widgets::ListState;
use std::sync::{Arc, Mutex, mpsc};
use tokio_util::sync::CancellationToken;

// ========== 消息类型（跨线程通信）==========

/// 后台线程发送给 TUI 的消息类型
pub enum StreamMsg {
    /// 收到一个流式文本块
    Chunk,
    /// LLM 请求执行工具（附带完整工具调用列表）
    ToolCallRequest(Vec<ToolCallItem>),
    /// Agent loop 中新增的消息（tool_call assistant + tool results），增量推送
    AgentMessages(Vec<super::storage::ChatMessage>),
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

// ========== 前端状态 ==========

/// UI 前端状态：所有与界面展示相关的字段
pub struct UIState {
    /// 输入缓冲区
    pub input: String,
    /// 光标位置（字符索引）
    pub cursor_pos: usize,
    /// 当前模式
    pub mode: ChatMode,
    /// 消息列表滚动偏移
    pub scroll_offset: u16,
    /// 流式输出时是否自动滚动到底部
    pub auto_scroll: bool,
    /// 消息浏览模式中选中的消息索引
    pub browse_msg_index: usize,
    /// 浏览模式下当前消息内部的滚动偏移
    pub browse_scroll_offset: u16,
    /// 模型选择列表状态
    pub model_list_state: ListState,
    /// Toast 通知消息 (内容, 是否错误, 创建时间)
    pub toast: Option<(String, bool, std::time::Instant)>,
    /// 消息渲染行缓存
    pub msg_lines_cache: Option<MsgLinesCache>,
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
    /// 技能补全弹窗是否激活
    pub skill_popup_active: bool,
    /// @skill: 在 input 中的起始字符索引
    pub skill_popup_start_pos: usize,
    /// @skill: 之后的名称过滤文本
    pub skill_popup_filter: String,
    /// 技能弹窗中选中项索引
    pub skill_popup_selected: usize,
    /// 统一交互区：当前选中项索引（0=continue, 1=allow, 2=refuse, 3=type）
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
    /// 配置界面：是否有待处理的 system_prompt 编辑
    pub pending_system_prompt_edit: bool,
    /// 配置界面：是否有待处理的 style 编辑
    pub pending_style_edit: bool,
    /// 图片缓存（渲染终端图片）
    pub image_cache: Arc<Mutex<ImageCache>>,
    /// 工具开关子菜单中选中的索引
    pub tool_toggle_index: usize,
    /// Skill 开关子菜单中选中的索引
    pub skill_toggle_index: usize,
    /// 是否展开工具调用详情（Ctrl+O 切换）
    pub expand_tools: bool,
    /// 配置/工具/技能列表界面的垂直滚动偏移
    pub config_scroll_offset: u16,
}

// ========== 后端状态 ==========

/// Chat 后端数据状态：对话、配置、模型相关
pub struct ChatState {
    /// Agent 配置
    pub agent_config: AgentConfig,
    /// 当前对话会话
    pub session: ChatSession,
    /// 当前正在流式接收的 AI 回复内容（实时更新）
    pub streaming_content: Arc<Mutex<String>>,
    /// 是否正在等待 AI 回复
    pub is_loading: bool,
    /// 已加载的 skills（用于补全和高亮）
    pub loaded_skills: Vec<Skill>,
    /// 排队的任务列表（new_task 工具产生，当前任务完成后自动执行）
    pub queued_tasks: Arc<Mutex<Vec<String>>>,
    /// 用户在 agent loop 期间发送的待处理消息队列
    pub pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,
}

// ========== 工具执行器 ==========

/// 工具执行器：管理工具调用的状态和执行
pub struct ToolExecutor {
    /// 当前活跃的工具调用状态列表
    pub active_tool_calls: Vec<ToolCallStatus>,
    /// ToolConfirm 模式中当前待处理工具的索引
    pub pending_tool_idx: usize,
    /// 进入 ToolConfirm 模式的时间（用于超时自动执行）
    pub tool_confirm_entered_at: std::time::Instant,
    /// 是否有待执行的工具（已设为 Executing 状态但尚未实际调用）
    pub pending_tool_execution: bool,
    /// 当前正在后台执行的工具数量
    pub tools_executing_count: usize,
    /// 工具执行取消标志
    pub tool_cancelled: Arc<std::sync::atomic::AtomicBool>,
    /// 工具后台线程 → 主线程的执行结果 channel（发送端）
    pub tool_exec_tx: Option<mpsc::Sender<ToolExecDoneMsg>>,
    /// 工具后台线程 → 主线程的执行结果 channel
    pub tool_exec_rx: Option<mpsc::Receiver<ToolExecDoneMsg>>,
    /// 工具结果发送通道（主线程 → 后台线程）
    pub tool_result_tx: Option<mpsc::SyncSender<ToolResultMsg>>,
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self {
            active_tool_calls: Vec::new(),
            pending_tool_idx: 0,
            tool_confirm_entered_at: std::time::Instant::now(),
            pending_tool_execution: false,
            tools_executing_count: 0,
            tool_cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            tool_exec_tx: None,
            tool_exec_rx: None,
            tool_result_tx: None,
        }
    }

    /// 轮询后台工具执行结果，更新状态并转发给 agent loop。
    /// 返回新完成的工具信息 (tool_name, output_summary, is_error)。
    pub fn poll_results(&mut self) -> Vec<(String, String, bool)> {
        let mut exec_done_msgs: Vec<ToolExecDoneMsg> = Vec::new();
        if let Some(ref rx) = self.tool_exec_rx {
            loop {
                match rx.try_recv() {
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
        let mut completed = Vec::new();
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
            // 查找工具名
            let tool_name = self
                .active_tool_calls
                .iter()
                .find(|tc| tc.tool_call_id == done.tool_call_id)
                .map(|tc| tc.tool_name.clone())
                .unwrap_or_default();
            if let Some(tc) = self
                .active_tool_calls
                .iter_mut()
                .find(|tc| tc.tool_call_id == done.tool_call_id)
            {
                tc.status = if done.is_error {
                    ToolExecStatus::Failed(summary.clone())
                } else {
                    ToolExecStatus::Done(summary.clone())
                };
            }
            completed.push((tool_name, summary, done.is_error));
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
                // 本批工具全部完成，重置取消标志
                self.tool_cancelled
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
        }
        completed
    }

    /// 把所有 Executing 状态的工具放到后台线程执行
    pub fn execute_batch(&mut self, registry: &Arc<ToolRegistry>) {
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

        let exec_tx = if let Some(ref tx) = self.tool_exec_tx {
            tx.clone()
        } else {
            let (tx, rx) = mpsc::channel::<ToolExecDoneMsg>();
            self.tool_exec_tx = Some(tx.clone());
            self.tool_exec_rx = Some(rx);
            tx
        };

        for (tool_call_id, tool_name, arguments) in tasks {
            let tx = exec_tx.clone();
            let registry = Arc::clone(registry);
            let cancelled = Arc::clone(&self.tool_cancelled);
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    registry.execute(&tool_name, &arguments, &cancelled)
                }));
                match result {
                    Ok(exec_result) => {
                        let _ = tx.send(ToolExecDoneMsg {
                            tool_call_id,
                            output: exec_result.output,
                            is_error: exec_result.is_error,
                        });
                    }
                    Err(panic_info) => {
                        let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                            s.to_string()
                        } else if let Some(s) = panic_info.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "unknown panic".to_string()
                        };
                        let _ = tx.send(ToolExecDoneMsg {
                            tool_call_id,
                            output: format!("[Tool panic] {}", msg),
                            is_error: true,
                        });
                    }
                }
            });
        }
    }

    /// 用户确认执行当前待处理工具 → 返回 Some(ChatMode) 表示需要切换模式
    pub fn execute_current(&mut self, registry: &Arc<ToolRegistry>) -> Option<ChatMode> {
        let idx = self.pending_tool_idx;
        if idx >= self.active_tool_calls.len() {
            return Some(ChatMode::Chat);
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

        self.active_tool_calls[idx].status = ToolExecStatus::Executing;

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

        let exec_tx = if let Some(ref tx) = self.tool_exec_tx {
            tx.clone()
        } else {
            let (tx, rx) = mpsc::channel::<ToolExecDoneMsg>();
            self.tool_exec_tx = Some(tx.clone());
            self.tool_exec_rx = Some(rx);
            tx
        };

        let registry = Arc::clone(registry);
        let cancelled = Arc::clone(&self.tool_cancelled);
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                registry.execute(&tool_name, &arguments, &cancelled)
            }));
            match result {
                Ok(exec_result) => {
                    let _ = exec_tx.send(ToolExecDoneMsg {
                        tool_call_id,
                        output: exec_result.output,
                        is_error: exec_result.is_error,
                    });
                }
                Err(panic_info) => {
                    let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_info.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic".to_string()
                    };
                    let _ = exec_tx.send(ToolExecDoneMsg {
                        tool_call_id,
                        output: format!("[Tool panic] {}", msg),
                        is_error: true,
                    });
                }
            }
        });

        self.advance()
    }

    /// 用户拒绝执行当前待处理工具 → 返回 Some(ChatMode) 表示需要切换模式
    pub fn reject_current(&mut self, reason: &str) -> Option<ChatMode> {
        let idx = self.pending_tool_idx;
        if idx >= self.active_tool_calls.len() {
            return Some(ChatMode::Chat);
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

        self.advance()
    }

    /// 用户选择 "允许并记住" → 返回 Some(ChatMode) 表示需要切换模式
    pub fn allow_and_execute(
        &mut self,
        registry: &Arc<ToolRegistry>,
        jcli_config: &mut Arc<JcliConfig>,
    ) -> Option<ChatMode> {
        let idx = self.pending_tool_idx;
        if idx >= self.active_tool_calls.len() {
            return Some(ChatMode::Chat);
        }

        let tool_name = self.active_tool_calls[idx].tool_name.clone();
        let arguments = self.active_tool_calls[idx].arguments.clone();

        // 生成 allow 规则并写入 .jcli/permissions.yaml
        let rule = super::permission::generate_allow_rule(&tool_name, &arguments);
        let mut jcli = (**jcli_config).clone();
        jcli.add_allow_rule(&rule);
        *jcli_config = Arc::new(jcli);

        // 执行工具
        self.execute_current(registry)
    }

    /// 是否还有待确认的工具
    pub fn has_pending_confirm(&self) -> bool {
        self.active_tool_calls
            .iter()
            .any(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))
    }

    /// 推进到下一个待确认工具，或返回 Some(ChatMode::Chat) 退出确认模式
    pub fn advance(&mut self) -> Option<ChatMode> {
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
            None // 继续保持 ToolConfirm 模式
        } else {
            write_info_log(
                "advance_tool_confirm",
                &format!(
                    "所有工具已处理, 退出 ToolConfirm, tools_executing_count={}",
                    self.tools_executing_count,
                ),
            );
            Some(ChatMode::Chat)
        }
    }

    /// 只取消工具执行，不终止 agent loop
    pub fn cancel(&mut self) {
        self.tool_cancelled
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// 重置所有工具状态（新消息发送时调用）
    pub fn reset(&mut self) {
        self.active_tool_calls.clear();
        self.pending_tool_idx = 0;
        self.tool_exec_tx = None;
        self.tool_exec_rx = None;
        self.tools_executing_count = 0;
        self.pending_tool_execution = false;
        self.tool_cancelled
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

// ========== Agent 生命周期句柄 ==========

/// Agent 生命周期管理：封装 stream channel、取消令牌等
pub struct AgentHandle {
    /// 用于接收后台流式回复的 channel
    pub stream_rx: mpsc::Receiver<StreamMsg>,
    /// 流式请求取消令牌
    pub cancel_token: CancellationToken,
}

impl AgentHandle {
    /// 启动一个 agent loop，返回 (AgentHandle, tool_result_tx)
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        provider: ModelProvider,
        api_messages: Vec<ChatMessage>,
        tools: Vec<ChatCompletionTools>,
        system_prompt_fn: Box<dyn FnOnce() -> Option<String> + Send>,
        use_stream: bool,
        streaming_content: Arc<Mutex<String>>,
        max_tool_rounds: usize,
        pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,
        background_manager: Arc<BackgroundManager>,
        compact_config: CompactConfig,
        hook_manager: super::hook::HookManager,
        todo_manager: Arc<super::tools::todo::TodoManager>,
    ) -> (Self, mpsc::SyncSender<ToolResultMsg>) {
        let (stream_tx, stream_rx) = mpsc::channel::<StreamMsg>();
        let (tool_result_tx, tool_result_rx) = mpsc::sync_channel::<ToolResultMsg>(16);

        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();

        std::thread::spawn(move || {
            // 保留一个 stream_tx 副本，用于 panic 后向主线程发送错误消息
            let stream_tx_panic = stream_tx.clone();

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                // 在后台线程里执行文件 IO（resolve_system_prompt），避免阻塞主线程
                let system_prompt = system_prompt_fn();

                let runtime = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ =
                            stream_tx.send(StreamMsg::Error(format!("创建异步运行时失败: {}", e)));
                        return;
                    }
                };

                runtime.block_on(run_agent_loop(
                    provider,
                    api_messages,
                    tools,
                    system_prompt,
                    use_stream,
                    streaming_content,
                    stream_tx,
                    tool_result_rx,
                    max_tool_rounds,
                    cancel_token_clone,
                    pending_user_messages,
                    background_manager,
                    compact_config,
                    hook_manager,
                    todo_manager,
                ));
            }));

            if let Err(panic_info) = result {
                // 尝试提取 panic 信息
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    format!("Agent 线程 panic: {}", s)
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    format!("Agent 线程 panic: {}", s)
                } else {
                    "Agent 线程发生未知 panic".to_string()
                };
                crate::util::log::write_error_log("AgentHandle::spawn", &panic_msg);
                // 通知主线程，避免 loading 状态永久卡住
                let _ = stream_tx_panic.send(StreamMsg::Error(panic_msg));
            }
        });

        // 这里是一个表达式
        (
            AgentHandle {
                stream_rx,
                cancel_token,
            },
            tool_result_tx,
        )
    }

    /// 取消当前流式请求
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    /// 非阻塞地获取所有可用的流式消息
    pub fn poll(&self) -> Vec<StreamMsg> {
        let mut msgs = Vec::new();
        loop {
            match self.stream_rx.try_recv() {
                Ok(msg) => msgs.push(msg),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    // channel 断开，标记为完成
                    msgs.push(StreamMsg::Done);
                    break;
                }
            }
        }
        msgs
    }
}

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
    pub todo_manager: Arc<super::tools::todo::TodoManager>,
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
    pub ws_bridge: Option<super::remote::bridge::WsBridge>,
    /// 远程客户端是否已连接
    pub remote_connected: bool,
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
    /// 缓存的总行数（历史消息 + 流式内容）
    pub total_line_count: usize,
    /// 每条消息（按 msg_index）的起始行号（用于浏览模式自动滚动）
    pub msg_start_lines: Vec<(usize, usize)>, // (msg_index, start_line)
    /// 按消息粒度缓存：每条历史消息的渲染行（key: 消息索引）
    pub per_msg_lines: Vec<PerMsgCache>,
    /// 流式内容 + tool confirm + 末尾留白的渲染行（与历史消息分开存储）
    pub streaming_lines: Vec<Line<'static>>,
    /// 流式增量渲染缓存：已完成段落的渲染行
    pub streaming_stable_lines: Vec<Line<'static>>,
    /// 流式增量渲染缓存：已缓存到 streaming_content 的字节偏移
    pub streaming_stable_offset: usize,
    /// 工具展开状态（缓存时记录，变化时需重建）
    pub expand_tools: bool,
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

/// Redux-like Action 枚举：所有用户输入和系统事件都转化为 Action
///
/// 设计原则：
/// 1. 完备性：覆盖所有操作入口（处理程序、流式事件、UI 状态变化）
/// 2. 原语性：Actions 是操作的原语，不包含复杂的条件逻辑
/// 3. 单向流向：KeyEvent/StreamMsg → Action → update() → ChatApp 状态变化 → 渲染
///
/// 按分类组织：
/// - Chat 模式：消息输入、文本编辑、弹窗交互
/// - 流式生命周期：流式块、工具调用请求、完成/错误/取消
/// - 工具执行：工具结果回调、工具确认/拒绝、Ask 工具交互
/// - 导航：模式切换、滚动、列表选择
/// - 配置：字段编辑、开关切换、保存
/// - 数据：清空会话、归档操作、主题切换
/// - UI 管理：Toast 展示、窗口 Tick
#[allow(dead_code)]
pub enum Action {
    // ========== Chat 输入和文本编辑 ==========
    /// 发送消息（当前输入框内容）
    SendMessage,
    /// 在光标位置插入字符
    InsertChar(char),
    /// 删除光标前的字符（Backspace）
    DeleteChar,
    /// 删除光标后的字符（Delete）
    DeleteForward,
    /// 移动光标
    MoveCursor(CursorDirection),
    /// 清空输入框
    ClearInput,

    // ========== 弹窗交互（@ 补全、文件补全、Ask） ==========
    /// 激活 @ 补全弹窗（在 "@" 之后）
    AtPopupActivate,
    /// 关闭 @ 补全弹窗
    AtPopupClose,
    /// 更新 @ 补全过滤文本
    AtPopupFilter(String),
    /// 在 @ 补全中导航（向上/向下）
    AtPopupNavigate(CursorDirection),
    /// 在 @ 补全中确认选择（插入技能名称）
    AtPopupConfirm,

    /// 激活文件补全弹窗（在 "@file:" 之后）
    FilePopupActivate,
    /// 关闭文件补全弹窗
    FilePopupClose,
    /// 更新文件补全过滤路径
    FilePopupFilter(String),
    /// 在文件补全中导航
    FilePopupNavigate(CursorDirection),
    /// 在文件补全中确认（插入文件路径）
    FilePopupConfirm,

    /// 激活技能补全弹窗（在 "@skill:" 之后）
    SkillPopupActivate,
    /// 关闭技能补全弹窗
    SkillPopupClose,
    /// 更新技能补全过滤文本
    SkillPopupFilter(String),
    /// 在技能补全中导航
    SkillPopupNavigate(CursorDirection),
    /// 在技能补全中确认（插入技能名称）
    SkillPopupConfirm,

    // ========== 流式生命周期（来自后台 Agent） ==========
    /// 收到一个流式文本块（实时回复）
    StreamChunk,
    /// LLM 请求执行工具（包含完整工具调用列表）
    ToolCallRequest(Vec<ToolCallItem>),
    /// 流式完成（正常结束）
    StreamDone,
    /// 流式错误
    StreamError(String),
    /// 流式被用户取消
    StreamCancelled,

    // ========== 工具执行和确认 ==========
    /// 执行当前待处理工具（用户确认）
    ExecutePendingTool,
    /// 拒绝当前待处理工具（无原因）
    RejectPendingTool,
    /// 拒绝当前待处理工具（带拒绝原因）
    RejectPendingToolWithReason(String),
    /// 允许并执行当前工具（记住规则到 .jcli/）
    AllowAndExecutePendingTool,
    /// 工具后台执行完成
    ToolExecDone(ToolExecDoneMsg),

    // ========== Ask 工具交互 ==========
    /// Ask 工具问题导航（上一题/下一题）
    AskNavigate(CursorDirection),
    /// Ask 工具选项导航（上下移动选项/输入框）
    AskOptionNavigate(CursorDirection),
    /// Ask 工具单选确认（选中当前选项）
    AskSingleSelect,
    /// Ask 工具多选勾选（切换当前选项的选中状态）
    AskToggleMultiSelect,
    /// Ask 工具自由文本输入
    AskInputChar(char),
    /// Ask 工具自由文本删除字符
    AskDeleteChar,
    /// Ask 工具提交答案（当前问题）
    AskSubmitAnswer,
    /// Ask 工具取消（放弃所有问题）
    AskCancel,

    // ========== 工具交互区（统一交互 UI） ==========
    /// 工具交互区选项导航（Continue → Allow → Refuse → Type Reason）
    ToolInteractNavigate(CursorDirection),
    /// 工具交互区拒绝原因输入
    ToolInteractInputChar(char),
    /// 工具交互区拒绝原因删除字符
    ToolInteractDeleteChar,
    /// 工具交互区确认当前选项（执行/允许/拒绝）
    ToolInteractConfirm,

    // ========== 模式切换和导航 ==========
    /// 进入指定模式
    EnterMode(ChatMode),
    /// 返回到 Chat 模式
    ExitToChat,
    /// 滚动消息（向上/向下）
    Scroll(CursorDirection),
    /// 分页滚动消息（Page Up/Page Down）
    PageScroll(CursorDirection),
    /// 消息浏览模式：选择上一条/下一条消息
    BrowseNavigate(CursorDirection),
    /// 消息浏览模式：微调滚动（某条消息内的细粒度滚动）
    BrowseFineScroll(CursorDirection),
    /// 消息浏览模式：复制选中消息到剪贴板
    BrowseCopyMessage,

    // ========== 配置编辑 ==========
    /// 配置界面：选择上一个/下一个字段
    ConfigNavigate(CursorDirection),
    /// 配置界面：切换上一个/下一个 provider
    ConfigSwitchProvider(CursorDirection),
    /// 配置界面：开始编辑当前字段或触发特殊操作
    ConfigEnter,
    /// 配置编辑模式：输入字符
    ConfigEditChar(char),
    /// 配置编辑模式：删除字符
    ConfigEditDelete,
    /// 配置编辑模式：移动光标
    ConfigEditMoveCursor(CursorDirection),
    /// 配置编辑模式：提交编辑
    ConfigEditSubmit,
    /// 配置界面：添加新 Provider
    ConfigAddProvider,
    /// 配置界面：删除当前 Provider
    ConfigDeleteProvider,
    /// 配置界面：设置当前 Provider 为活跃
    ConfigSetActiveProvider,
    /// 进入工具开关子菜单
    EnterToolToggleMenu,
    /// 进入 Skill 开关子菜单
    EnterSkillToggleMenu,
    /// 工具/Skill 开关：导航
    ToggleMenuNavigate(CursorDirection),
    /// 工具/Skill 开关：切换当前项
    ToggleMenuToggle,
    /// 工具/Skill 开关：全部启用
    ToggleMenuEnableAll,
    /// 工具/Skill 开关：全部禁用
    ToggleMenuDisableAll,

    // ========== 模型选择 ==========
    /// 模型选择模式：导航
    ModelSelectNavigate(CursorDirection),
    /// 模型选择模式：确认切换
    ModelSelectConfirm,

    // ========== 归档管理 ==========
    /// 启动归档确认流程
    StartArchiveConfirm,
    /// 归档确认：编辑自定义名称
    ArchiveConfirmEditName,
    /// 归档确认：编辑光标移动
    ArchiveConfirmMoveCursor(CursorDirection),
    /// 归档确认：编辑字符输入
    ArchiveConfirmInputChar(char),
    /// 归档确认：编辑字符删除
    ArchiveConfirmDeleteChar,
    /// 归档确认：使用默认名称保存
    ArchiveWithDefault,
    /// 归档确认：使用自定义名称保存
    ArchiveWithCustom,
    /// 清空当前会话（不归档）
    ClearSession,

    /// 启动还原流程（加载归档列表）
    StartArchiveList,
    /// 归档列表：导航
    ArchiveListNavigate(CursorDirection),
    /// 归档列表：还原选中的归档
    RestoreArchive,
    /// 归档列表：删除选中的归档
    DeleteArchive,

    // ========== 模型和主题切换 ==========
    /// 进入模型选择模式（Ctrl+T）
    SwitchModel,
    /// 切换主题
    SwitchTheme,
    /// 切换流式 vs 批处理模式
    ToggleStreamMode,

    // ========== 流式控制 ==========
    /// 用户取消当前流式请求（Esc）
    CancelStream,
    /// 只取消工具执行，不中断 Agent Loop
    CancelToolsOnly,

    // ========== UI 管理 ==========
    /// Toast 通知（消息内容, 是否为错误）
    ShowToast(String, bool),
    /// 定时器 Tick（检查 Toast 过期）
    TickToast,
    /// 保存配置（Esc 离开配置屏）
    SaveConfig,

    // ========== 快速操作 ==========
    /// 复制最后一条 AI 回复（Ctrl+Y）
    CopyLastAiReply,
    /// 显示帮助（F1 或 "?"）
    ShowHelp,
    /// 打开日志窗口（Ctrl+G）
    OpenLogWindows,

    // ========== 应用控制 ==========
    /// 正常退出（Ctrl+C）
    Quit,
    /// 切换工具详情展开/折叠（Ctrl+O）
    ToggleExpandTools,
}

#[derive(Debug, Clone, Copy)]
pub enum CursorDirection {
    Up,
    Down,
}

/// 所有字段数 = provider 字段 + 全局字段
pub fn config_total_fields() -> usize {
    CONFIG_FIELDS.len() + CONFIG_GLOBAL_FIELDS.len()
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
        let (ask_req_tx, ask_req_rx) = mpsc::channel::<AskRequest>();
        let queued_tasks: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let background_manager = Arc::new(BackgroundManager::new());
        let task_manager = Arc::new(super::tools::task::TaskManager::new());
        let hook_manager = Arc::new(Mutex::new(HookManager::load()));
        let tool_registry = ToolRegistry::new(
            loaded_skills.clone(),
            ask_req_tx,
            Arc::clone(&background_manager),
            Arc::clone(&task_manager),
            Arc::clone(&hook_manager),
        );
        let todo_manager = Arc::clone(&tool_registry.todo_manager);
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
                tool_toggle_index: 0,
                skill_toggle_index: 0,
                expand_tools: false,
                config_scroll_offset: 0,
            },
            state: ChatState {
                agent_config,
                session,
                streaming_content: Arc::new(Mutex::new(String::new())),
                is_loading: false,
                loaded_skills,
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
                if self.ui.input.len() < 16384 {
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
                    self.broadcast_ws(super::remote::protocol::WsOutbound::StreamChunk { content });
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
                        self.broadcast_ws(super::remote::protocol::WsOutbound::Message {
                            role: "assistant".to_string(),
                            content: last_msg.content.clone(),
                        });
                    }
                    self.broadcast_ws(super::remote::protocol::WsOutbound::Status {
                        state: "idle".to_string(),
                    });
                }
                self.finish_loading(false, false);
            }
            Action::StreamError(ref e) => {
                self.broadcast_ws(super::remote::protocol::WsOutbound::Error {
                    message: format!("请求失败: {}", e),
                });
                self.broadcast_ws(super::remote::protocol::WsOutbound::Status {
                    state: "idle".to_string(),
                });
                self.show_toast(format!("请求失败: {}", e), true);
                self.finish_loading(true, false);
            }
            Action::StreamCancelled => {
                self.broadcast_ws(super::remote::protocol::WsOutbound::Status {
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
            Action::ToolExecDone(ref _msg) => {
                // Will delegate to helper (existing ToolExecutor::poll_results logic)
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
                    let tools: Vec<super::remote::protocol::ToolConfirmInfo> = self
                        .tool_executor
                        .active_tool_calls
                        .iter()
                        .filter(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))
                        .map(|tc| super::remote::protocol::ToolConfirmInfo {
                            id: tc.tool_call_id.clone(),
                            name: tc.tool_name.clone(),
                            arguments: tc.arguments.clone(),
                            confirm_message: tc.confirm_message.clone(),
                        })
                        .collect();
                    if !tools.is_empty() {
                        self.broadcast_ws(
                            super::remote::protocol::WsOutbound::ToolConfirmRequest { tools },
                        );
                    }
                    self.broadcast_ws(super::remote::protocol::WsOutbound::Status {
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
                use super::render_cache::copy_to_clipboard;
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
                let total_fields = config_total_fields();
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
                use super::ui_helpers::config_field_raw_value;
                use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS};
                let total_provider = CONFIG_FIELDS.len();
                if self.ui.config_field_idx < total_provider
                    && self.state.agent_config.providers.is_empty()
                {
                    self.show_toast("还没有 Provider，按 a 新增", true);
                    return;
                }
                let gi = self.ui.config_field_idx.checked_sub(total_provider);
                if let Some(gi) = gi {
                    if CONFIG_GLOBAL_FIELDS[gi] == "stream_mode" {
                        self.state.agent_config.stream_mode = !self.state.agent_config.stream_mode;
                        return;
                    }
                    if CONFIG_GLOBAL_FIELDS[gi] == "tools_enabled" {
                        self.ui.tool_toggle_index = 0;
                        self.ui.mode = ChatMode::ToolToggle;
                        return;
                    }
                    if CONFIG_GLOBAL_FIELDS[gi] == "skills_enabled" {
                        self.ui.skill_toggle_index = 0;
                        self.ui.mode = ChatMode::SkillToggle;
                        return;
                    }
                    if CONFIG_GLOBAL_FIELDS[gi] == "theme" {
                        self.switch_theme();
                        return;
                    }
                    if CONFIG_GLOBAL_FIELDS[gi] == "system_prompt" {
                        self.ui.pending_system_prompt_edit = true;
                        return;
                    }
                    if CONFIG_GLOBAL_FIELDS[gi] == "style" {
                        self.ui.pending_style_edit = true;
                        return;
                    }
                }
                self.ui.config_edit_buf = config_field_raw_value(self, self.ui.config_field_idx);
                self.ui.config_edit_cursor = self.ui.config_edit_buf.chars().count();
                self.ui.config_editing = true;
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
                use super::ui_helpers::config_field_set;
                let val = self.ui.config_edit_buf.clone();
                config_field_set(self, self.ui.config_field_idx, &val);
                self.ui.config_editing = false;
            }
            Action::ConfigAddProvider => {
                let new_provider = ModelProvider {
                    name: format!("Provider-{}", self.state.agent_config.providers.len() + 1),
                    api_base: "https://api.openai.com/v1".to_string(),
                    api_key: String::new(),
                    model: String::new(),
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
            Action::EnterToolToggleMenu => {
                self.ui.mode = ChatMode::ToolToggle;
                self.ui.tool_toggle_index = 0;
                self.ui.config_scroll_offset = 0;
            }
            Action::EnterSkillToggleMenu => {
                self.ui.mode = ChatMode::SkillToggle;
                self.ui.skill_toggle_index = 0;
                self.ui.config_scroll_offset = 0;
            }
            Action::ToggleMenuNavigate(dir) => {
                // Used by both ToolToggle and SkillToggle modes
                // The total count is determined by mode in the handler
                // Here we just navigate generically
                match dir {
                    CursorDirection::Up => {
                        if self.ui.mode == ChatMode::ToolToggle {
                            let total = self.tool_registry.tool_names().len();
                            if total > 0 {
                                if self.ui.tool_toggle_index == 0 {
                                    self.ui.tool_toggle_index = total - 1;
                                } else {
                                    self.ui.tool_toggle_index -= 1;
                                }
                            }
                        } else {
                            let total = self.state.loaded_skills.len();
                            if total > 0 {
                                if self.ui.skill_toggle_index == 0 {
                                    self.ui.skill_toggle_index = total - 1;
                                } else {
                                    self.ui.skill_toggle_index -= 1;
                                }
                            }
                        }
                    }
                    CursorDirection::Down => {
                        if self.ui.mode == ChatMode::ToolToggle {
                            let total = self.tool_registry.tool_names().len();
                            if total > 0 {
                                self.ui.tool_toggle_index = (self.ui.tool_toggle_index + 1) % total;
                            }
                        } else {
                            let total = self.state.loaded_skills.len();
                            if total > 0 {
                                self.ui.skill_toggle_index =
                                    (self.ui.skill_toggle_index + 1) % total;
                            }
                        }
                    }
                }
            }
            Action::ToggleMenuToggle => {
                if self.ui.mode == ChatMode::ToolToggle {
                    let tool_names = self.tool_registry.tool_names();
                    if let Some(name) = tool_names.get(self.ui.tool_toggle_index) {
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
                } else if let Some(skill) = self.state.loaded_skills.get(self.ui.skill_toggle_index)
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
                }
            }
            Action::ToggleMenuEnableAll => {
                if self.ui.mode == ChatMode::ToolToggle {
                    self.state.agent_config.disabled_tools.clear();
                    self.show_toast("已启用全部工具", false);
                } else {
                    self.state.agent_config.disabled_skills.clear();
                    self.show_toast("已启用全部 Skills", false);
                }
            }
            Action::ToggleMenuDisableAll => {
                if self.ui.mode == ChatMode::ToolToggle {
                    self.state.agent_config.disabled_tools = self
                        .tool_registry
                        .tool_names()
                        .iter()
                        .map(|n| n.to_string())
                        .collect();
                    self.show_toast("已禁用全部工具", false);
                } else {
                    self.state.agent_config.disabled_skills = self
                        .state
                        .loaded_skills
                        .iter()
                        .map(|s| s.frontmatter.name.clone())
                        .collect();
                    self.show_toast("已禁用全部 Skills", false);
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
            Action::ToggleStreamMode => {
                self.state.agent_config.stream_mode = !self.state.agent_config.stream_mode;
                let mode_name = if self.state.agent_config.stream_mode {
                    "流式"
                } else {
                    "批处理"
                };
                self.show_toast(format!("已切换为{}模式", mode_name), false);
                let _ = save_agent_config(&self.state.agent_config);
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
                use super::render_cache::copy_to_clipboard;
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
    pub fn broadcast_ws(&self, msg: super::remote::protocol::WsOutbound) {
        if let Some(ref ws) = self.ws_bridge {
            ws.broadcast(msg);
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
        let use_stream = self.state.agent_config.stream_mode;
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
        let disabled_skills = self.state.agent_config.disabled_skills.clone();
        let disabled_tools = self.state.agent_config.disabled_tools.clone();
        let tool_registry = Arc::clone(&self.tool_registry);
        let system_prompt_fn: Box<dyn FnOnce() -> Option<String> + Send> = Box::new(move || {
            use super::storage::{load_memory, load_soul, load_style, load_system_prompt};
            let template = load_system_prompt()?;
            let skills_summary = skill::build_skills_summary(&loaded_skills, &disabled_skills);
            let tools_summary = tool_registry.build_tools_summary(&disabled_tools);
            let style_text = load_style().unwrap_or_else(|| "（未设置）".to_string());
            let memory_text = load_memory().unwrap_or_default();
            let soul_text = load_soul().unwrap_or_default();
            let current_dir = std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string());
            let skill_dir = skills_dir().to_string_lossy().to_string();
            let resolved = template
                .replace("{{.current_dir}}", &current_dir)
                .replace("{{.skills}}", &skills_summary)
                .replace("{{.skill_dir}}", &skill_dir)
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

        // 启动 agent handle
        let (handle, tool_result_tx) = AgentHandle::spawn(
            provider,
            api_messages,
            tools,
            system_prompt_fn,
            use_stream,
            streaming_content,
            max_tool_rounds,
            pending_user_messages,
            background_manager,
            compact_config,
            hook_manager_clone,
            todo_manager,
        );

        self.agent = Some(handle);
        self.tool_executor.tool_result_tx = Some(tool_result_tx);
    }

    /// 处理后台流式消息（在主循环中每帧调用）
    /// 轮询后台流式消息并收集 Actions（Step 6: collect + dispatch 分离）
    ///
    /// 该方法完成以下职责：
    /// 1. 轮询工具执行结果（ToolExecutor 内部状态更新）
    /// 2. 轮询 Ask 工具请求（初始化 ask mode）
    /// 3. 处理延迟工具执行（pending_tool_execution）
    /// 4. 轮询 Agent StreamMsg 并映射为 Action
    ///
    /// 返回需要通过 update() 分发的 Actions 列表
    pub fn poll_stream_actions(&mut self) -> Vec<Action> {
        let mut actions = Vec::new();

        if self.agent.is_none() {
            return actions;
        }

        // 如果在 ToolConfirm 模式，仍然需要轮询工具执行结果（但暂停流式消息轮询）
        if self.ui.mode == ChatMode::ToolConfirm {
            let completed = self.tool_executor.poll_results();
            for (name, output, is_error) in completed {
                self.broadcast_ws(super::remote::protocol::WsOutbound::ToolResult {
                    name,
                    output,
                    is_error,
                });
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
                    self.broadcast_ws(super::remote::protocol::WsOutbound::ToolCall {
                        name: tc.tool_name.clone(),
                        arguments: tc.arguments.clone(),
                    });
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
            self.broadcast_ws(super::remote::protocol::WsOutbound::ToolResult {
                name,
                output,
                is_error,
            });
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

        // 轮询 Agent StreamMsg 并映射为 Action
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
                    StreamMsg::AgentMessages(new_msgs) => {
                        // 增量推送：agent loop 中产生的 tool_call + tool_result 消息
                        for msg in new_msgs {
                            self.state.session.messages.push(msg);
                        }
                        self.ui.msg_lines_cache = None;
                        // 不 break，继续处理后续消息
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
            let questions: Vec<super::remote::protocol::AskQuestionInfo> = ask_req
                .questions
                .iter()
                .map(|q| super::remote::protocol::AskQuestionInfo {
                    question: q.question.clone(),
                    header: q.header.clone(),
                    options: q
                        .options
                        .iter()
                        .map(|o| super::remote::protocol::AskOptionInfo {
                            label: o.label.clone(),
                            description: o.description.clone(),
                        })
                        .collect(),
                    multi_select: q.multi_select,
                })
                .collect();
            self.broadcast_ws(super::remote::protocol::WsOutbound::AskRequest { questions });
            self.broadcast_ws(super::remote::protocol::WsOutbound::Status {
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
        self.tool_executor.tool_exec_tx = None;
        self.tool_executor.tool_exec_rx = None;
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
            ) = super::render_cache::build_message_lines_incremental(
                self,
                inner_width,
                bubble_max_width,
                old_cache.as_ref(),
            );
            let total_line_count: usize = new_per_msg.iter().map(|p| p.lines.len()).sum::<usize>()
                + new_streaming_lines.len();
            self.ui.msg_lines_cache = Some(MsgLinesCache {
                msg_count,
                last_msg_len,
                streaming_len,
                is_loading: self.state.is_loading,
                bubble_max_width,
                browse_index: current_browse_index,
                tool_confirm_idx: current_tool_confirm_idx,
                total_line_count,
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
        use super::archive::generate_default_archive_name;
        self.ui.archive_default_name = generate_default_archive_name();
        self.ui.archive_custom_name = String::new();
        self.ui.archive_editing_name = false;
        self.ui.archive_edit_cursor = 0;
        self.ui.mode = ChatMode::ArchiveConfirm;
    }

    /// 开始还原流程（加载归档列表）
    pub fn start_archive_list(&mut self) {
        use super::archive::list_archives;
        self.ui.archives = list_archives();
        self.ui.archive_list_index = 0;
        self.ui.restore_confirm_needed = false;
        self.ui.mode = ChatMode::ArchiveList;
    }

    /// 执行归档
    pub fn do_archive(&mut self, name: &str) {
        use super::archive::create_archive;

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
        use super::archive::restore_archive;

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
        use super::archive::delete_archive;

        if let Some(archive) = self.ui.archives.get(self.ui.archive_list_index) {
            match delete_archive(&archive.name) {
                Ok(_) => {
                    self.show_toast(format!("归档已删除: {}", archive.name), false);
                    self.ui.archives = super::archive::list_archives();
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
