use super::types::{AskAnswer, AskQuestion};
use crate::infra::archive::ChatArchive;
use crate::infra::command::CommandSource;
use crate::permission::queue::PendingAgentPerm;
use crate::storage::SessionMeta;
use crate::tools::plan::PendingPlanApproval;
use crate::markdown::image_cache::ImageCache;
use crate::theme::Theme;
use crate::tui::editor_core::text_buffer::TextBuffer;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::ListState;
use std::sync::{Arc, Mutex};

// ========== 鼠标选区 ==========

/// 鼠标选区状态（用于消息区拖拽选择文字）
#[derive(Clone, Debug)]
pub struct MouseSelection {
    /// 选区起点（全局行号，行内字符偏移）
    pub anchor: (usize, usize),
    /// 选区当前位置（全局行号，行内字符偏移）
    pub current: (usize, usize),
}

// ========== 右键上下文菜单 ==========

/// 右键上下文菜单状态
#[derive(Clone, Debug)]
pub struct ContextMenu {
    /// 右键点击时的全局行号（用于定位所属消息）
    pub global_line: usize,
    /// 菜单显示的屏幕坐标 (col, row)
    pub screen_pos: (u16, u16),
}

// ========== 前端状态 ==========

/// UI 前端状态：所有与界面展示相关的字段
pub struct UIState {
    /// 输入缓冲区（多行迷你编辑器）
    pub input_buffer: TextBuffer,
    /// 当前模式
    pub mode: ChatMode,
    /// 消息列表滚动偏移（usize 支持超过 65535 行）
    pub scroll_offset: usize,
    /// 流式输出时是否自动滚动到底部
    pub auto_scroll: bool,
    /// 消息浏览模式中选中的消息索引
    pub browse_msg_index: usize,
    /// 浏览模式下当前消息内部的滚动偏移
    pub browse_scroll_offset: usize,
    /// 浏览模式关键词过滤
    pub browse_filter: String,
    /// 浏览模式角色过滤: None=全部, Some("ai"), Some("user")
    pub browse_role_filter: Option<String>,
    /// 模型选择列表状态
    pub model_list_state: ListState,
    /// 主题选择列表状态
    pub theme_list_state: ListState,
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
    /// / 斜杠命令弹窗是否激活
    pub slash_popup_active: bool,
    /// / 之后的过滤文本
    pub slash_popup_filter: String,
    /// 弹窗中选中项索引
    pub slash_popup_selected: usize,
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
    /// ask 自由输入草稿缓存（每个问题对应一个 String，暂存自由输入内容）
    pub tool_ask_drafts: Vec<String>,
    /// 配置界面：是否有待处理的 system_prompt 编辑
    pub pending_system_prompt_edit: bool,
    /// 配置界面：是否有待处理的 agent_md 编辑
    pub pending_agent_md_edit: bool,
    /// 配置界面：是否有待处理的 style 编辑
    pub pending_style_edit: bool,
    /// 图片缓存（渲染终端图片）
    pub image_cache: Arc<Mutex<ImageCache>>,
    /// 是否展开工具调用详情（Ctrl+O 切换）
    pub expand_tools: bool,
    /// 配置/工具/技能列表界面的垂直滚动偏移
    pub config_scroll_offset: u16,
    /// 配置面板 Provider 子标签水平滚动偏移（可见窗口起始索引）
    pub config_provider_scroll_offset: usize,
    /// 配置面板当前 Tab
    pub config_tab: ConfigTab,
    /// 会话列表（缓存）
    pub session_list: Vec<SessionMeta>,
    /// 会话列表选中索引
    pub session_list_index: usize,
    /// 会话恢复确认模式（当前有消息时需要确认）
    pub session_restore_confirm: bool,
    /// Teammate Dashboard 中选中的 teammate 索引
    pub teammate_list_index: usize,
    /// 欢迎界面诗句索引（每次进入 chat 时随机选定）
    pub quote_idx: usize,
    /// 输入区视觉折行宽度（由 draw_input 每帧更新，handler 用于判断视觉折行）
    pub input_wrap_width: usize,
    /// 来自子 agent 的待决权限请求（Some 时进入 AgentPermConfirm 模式）
    pub pending_agent_perm: Option<Arc<PendingAgentPerm>>,
    /// 来自 teammate 的待决 Plan 审批请求（Some 时进入 PlanApprovalConfirm 模式）
    pub pending_plan_approval: Option<Arc<PendingPlanApproval>>,
    /// 是否在 Global tab 的"豁免压缩工具"子列表中
    pub compact_exempt_sublist: bool,
    /// 豁免压缩工具子列表选中索引
    pub compact_exempt_idx: usize,
    /// Tools tab：是否在选项层级（选中工具的启用/defer 选项）
    pub tools_in_options: bool,
    /// Tools tab：选项层级中的焦点索引（0=启用, 1=defer）
    pub tools_option_idx: usize,
    /// 是否自动批准所有操作（bypass 模式，per-session 持久化）
    pub auto_approve: bool,
    /// 配置界面：Commands Tab 的当前模式
    pub commands_mode: CommandsMode,
    /// 配置界面：Commands 创建时的来源选择索引（0=用户级, 1=项目级）
    pub commands_source_idx: usize,
    /// 配置界面：是否有待处理的命令创建
    pub pending_command_create: bool,
    /// 配置界面：命令创建的目标级别
    pub command_create_source: CommandSource,
    /// 鼠标选区状态（拖拽选择文字时使用）
    pub mouse_selection: Option<MouseSelection>,
    /// 消息区域 inner rect（用于鼠标坐标映射，每次渲染时更新）
    pub msg_area_inner: Option<Rect>,
    /// 右键上下文菜单状态
    pub context_menu: Option<ContextMenu>,
    /// 配置面板列表区域（chunks[1]），用于鼠标点击检测
    pub config_list_area: Option<Rect>,
    /// 配置面板 Tab 栏的全局 Y 坐标（屏幕行号）
    pub config_tab_bar_y: Option<u16>,
    /// 配置面板可交互项的行号列表（每帧更新，与 field_line_indices 同步）
    pub config_field_lines: Vec<usize>,
    /// 配置面板 Tab 点击区域（每帧更新）
    pub config_tab_hitboxes: Vec<ConfigTabHitBox>,
}

/// 配置面板 Tab 点击检测区域
#[derive(Clone, Debug)]
pub struct ConfigTabHitBox {
    /// Tab 类型
    pub tab: ConfigTab,
    /// 起始列号（屏幕坐标）
    pub start_col: u16,
    /// 结束列号（屏幕坐标，不含）
    pub end_col: u16,
}

/// 消息渲染行缓存
pub struct MsgLinesCache {
    /// 会话消息数量
    pub msg_count: usize,
    /// 最后一条消息的内容长度（用于检测流式更新）
    pub last_msg_len: usize,
    /// 流式内容长度
    pub streaming_len: usize,
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
    /// 渲染布局版本（气泡样式变化时递增，强制缓存失效）
    pub render_version: u32,
}

/// 单条消息的渲染缓存
#[derive(Debug, Clone)]
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

/// 聊天界面模式枚举，定义 TUI 的不同交互状态
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
    /// 主题选择模式
    SelectTheme,
    /// 子 agent 权限请求确认模式（y 批准 / n 拒绝）
    AgentPermConfirm,
    /// Teammate Plan 审批确认模式（y 批准 / n 拒绝）
    PlanApprovalConfirm,
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
    /// Teammate 状态面板
    Teammates,
    /// 会话管理
    Session,
    /// 归档管理
    Archive,
}

/// Commands Tab 的模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommandsMode {
    /// 正常列表浏览
    #[default]
    Normal,
    /// 选择保存级别
    SelectSource,
}

impl ConfigTab {
    /// 返回 Tab 显示标签
    pub fn label(&self) -> &'static str {
        match self {
            ConfigTab::Model => "模型",
            ConfigTab::Session => "会话",
            ConfigTab::Global => "全局",
            ConfigTab::Tools => "工具",
            ConfigTab::Skills => "技能",
            ConfigTab::Hooks => "钩子",
            ConfigTab::Commands => "命令",
            ConfigTab::Teammates => "协作者",
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
            ConfigTab::Teammates => 7,
            ConfigTab::Archive => 8,
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
            7 => ConfigTab::Teammates,
            _ => ConfigTab::Archive,
        }
    }

    /// Tab 总数
    pub const COUNT: usize = 9;

    /// 切换到下一个 Tab
    pub fn next(&self) -> Self {
        ConfigTab::from_index((self.index() + 1) % Self::COUNT)
    }

    /// 切换到上一个 Tab
    pub fn prev(&self) -> Self {
        ConfigTab::from_index((self.index() + Self::COUNT - 1) % Self::COUNT)
    }
}

impl UIState {
    /// 获取输入文本（用于发送、autocomplete 等）
    pub fn input_text(&self) -> String {
        self.input_buffer.to_string()
    }

    /// 获取扁平字符索引（用于 autocomplete 的 char slicing）
    pub fn cursor_char_idx(&self) -> usize {
        let (row, col) = self.input_buffer.cursor();
        let mut idx = 0;
        for i in 0..row {
            idx += self
                .input_buffer
                .line(i)
                .map(|l: &String| l.chars().count())
                .unwrap_or(0);
            idx += 1; // \n
        }
        idx += col;
        idx
    }

    /// 从扁平字符索引设置光标（用于 autocomplete 替换后）
    pub fn set_cursor_from_char_idx(&mut self, char_idx: usize) {
        let mut remaining = char_idx;
        let lines = self.input_buffer.lines();
        for (row, line) in lines.iter().enumerate() {
            let line_len: usize = line.chars().count();
            if remaining <= line_len {
                self.input_buffer.set_cursor(row, remaining);
                return;
            }
            remaining -= line_len + 1; // +1 for \n
        }
        // 超出范围，放到最后
        let last_row = lines.len().saturating_sub(1);
        let last_col = lines[last_row].chars().count();
        self.input_buffer.set_cursor(last_row, last_col);
    }

    /// 替换整个输入文本（用于 autocomplete 替换 @mention）
    pub fn set_input_text(&mut self, text: &str, cursor_char_idx: usize) {
        self.input_buffer = TextBuffer::from_content(text);
        self.set_cursor_from_char_idx(cursor_char_idx);
    }

    /// 清空输入
    pub fn clear_input(&mut self) {
        self.input_buffer = TextBuffer::new();
    }

    /// 输入是否为空
    pub fn is_input_empty(&self) -> bool {
        self.input_buffer
            .lines()
            .iter()
            .all(|l: &String| l.is_empty())
            && self.input_buffer.line_count() <= 1
    }
}
