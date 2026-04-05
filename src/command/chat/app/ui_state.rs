use super::types::{AskAnswer, AskQuestion};
use crate::command::chat::archive::ChatArchive;
use crate::command::chat::markdown::image_cache::ImageCache;
use crate::command::chat::storage::SessionMeta;
use crate::command::chat::theme::Theme;
use ratatui::text::Line;
use ratatui::widgets::ListState;
use std::sync::{Arc, Mutex};

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
    /// @mention 范围缓存：(input 内容, 范围列表)，仅 input 变化时重算
    pub cached_mention_ranges: Option<(String, Vec<(usize, usize)>)>,
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
    pub archives: Vec<ChatArchive>,
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
    /// 命令补全弹窗是否激活
    pub command_popup_active: bool,
    /// @command: 在 input 中的起始字符索引
    pub command_popup_start_pos: usize,
    /// @command: 之后的名称过滤文本
    pub command_popup_filter: String,
    /// 命令弹窗中选中项索引
    pub command_popup_selected: usize,
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
    /// 是否展开工具调用详情（Ctrl+O 切换）
    pub expand_tools: bool,
    /// 配置/工具/技能列表界面的垂直滚动偏移
    pub config_scroll_offset: u16,
    /// 配置面板当前 Tab
    pub config_tab: ConfigTab,
    /// 会话列表（缓存）
    pub session_list: Vec<SessionMeta>,
    /// 会话列表选中索引
    pub session_list_index: usize,
    /// 会话恢复确认模式（当前有消息时需要确认）
    pub session_restore_confirm: bool,
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
    /// 历史消息的总行数（预计算，避免每帧重复求和）
    pub history_line_count: usize,
    /// 每条消息（按 msg_index）的起始行号（用于浏览模式自动滚动）
    pub msg_start_lines: Vec<(usize, usize)>, // (msg_index, start_line)
    /// 按消息粒度缓存：每条历史消息的渲染行（key: 消息索引）
    pub per_msg_lines: Vec<PerMsgCache>,
    /// 流式内容 + tool confirm + 末尾留白的渲染行（与历史消息分开存储）
    pub streaming_lines: Vec<Line<'static>>,
    /// 流式增量渲染缓存：已完成段落的渲染行
    pub streaming_stable_lines: Arc<Vec<Line<'static>>>,
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
}

/// 配置面板 Tab 分页枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigTab {
    /// 模型配置
    Model,
    /// 全局配置
    Global,
    /// 工具开关
    Tools,
    /// 技能开关
    Skills,
    /// Hooks（占位）
    Hooks,
    /// 自定义命令（占位）
    Commands,
    /// 会话管理
    Session,
    /// 归档管理
    Archive,
}

impl ConfigTab {
    /// 返回 Tab 显示标签
    pub fn label(&self) -> &'static str {
        match self {
            ConfigTab::Model => "Model",
            ConfigTab::Session => "Session",
            ConfigTab::Global => "Global",
            ConfigTab::Tools => "Tools",
            ConfigTab::Skills => "Skills",
            ConfigTab::Hooks => "Hooks",
            ConfigTab::Commands => "Commands",
            ConfigTab::Archive => "归档",
        }
    }

    /// 返回 Tab 索引
    pub fn index(&self) -> usize {
        match self {
            ConfigTab::Model => 0,
            ConfigTab::Session => 1,
            ConfigTab::Global => 2,
            ConfigTab::Tools => 3,
            ConfigTab::Skills => 4,
            ConfigTab::Hooks => 5,
            ConfigTab::Commands => 6,
            ConfigTab::Archive => 7,
        }
    }

    /// 从索引创建 Tab
    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => ConfigTab::Model,
            1 => ConfigTab::Session,
            2 => ConfigTab::Global,
            3 => ConfigTab::Tools,
            4 => ConfigTab::Skills,
            5 => ConfigTab::Hooks,
            6 => ConfigTab::Commands,
            _ => ConfigTab::Archive,
        }
    }

    /// Tab 总数
    pub const COUNT: usize = 8;

    /// 切换到下一个 Tab
    pub fn next(&self) -> Self {
        ConfigTab::from_index((self.index() + 1) % Self::COUNT)
    }

    /// 切换到上一个 Tab
    pub fn prev(&self) -> Self {
        ConfigTab::from_index((self.index() + Self::COUNT - 1) % Self::COUNT)
    }
}
