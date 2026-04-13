use super::ui_state::ChatMode;
use crate::command::chat::error::ChatError;
use crate::command::chat::storage::ToolCallItem;

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
    StreamError(ChatError),
    /// 流式被用户取消
    StreamCancelled,
    /// 正在重试（展示重试提示，不中断加载状态）
    StreamRetrying {
        attempt: u32,
        max_attempts: u32,
        delay_ms: u64,
        error: String,
    },

    // ========== 工具执行和确认 ==========
    /// 执行当前待处理工具（用户确认）
    ExecutePendingTool,
    /// 拒绝当前待处理工具（无原因）
    RejectPendingTool,
    /// 拒绝当前待处理工具（带拒绝原因）
    RejectPendingToolWithReason(String),
    /// 允许并执行当前工具（记住规则到 .jcli/）
    AllowAndExecutePendingTool,

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
    /// 配置编辑模式：删除字符（退格）
    ConfigEditDelete,
    /// 配置编辑模式：删除光标后的字符（Delete键）
    ConfigEditDeleteForward,
    /// 配置编辑模式：移动光标
    ConfigEditMoveCursor(CursorDirection),
    /// 配置编辑模式：光标移到行首
    ConfigEditMoveHome,
    /// 配置编辑模式：光标移到行尾
    ConfigEditMoveEnd,
    /// 配置编辑模式：清空整行
    ConfigEditClearLine,
    /// 配置编辑模式：提交编辑
    ConfigEditSubmit,
    /// 配置界面：添加新 Provider
    ConfigAddProvider,
    /// 配置界面：删除当前 Provider
    ConfigDeleteProvider,
    /// 配置界面：设置当前 Provider 为活跃
    ConfigSetActiveProvider,
    /// 配置界面：切换 Tab 分页（Left/Right）
    ConfigSwitchTab(CursorDirection),
    /// 工具/Skill 开关：导航（统一使用 config_field_idx）
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

    // ========== 主题选择 ==========
    /// 主题选择模式：导航
    ThemeSelectNavigate(CursorDirection),
    /// 主题选择模式：确认切换
    ThemeSelectConfirm,

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

    /// 列出所有会话（远程）
    ListSessions,
    /// 切换到指定会话（远程）
    SwitchSession { session_id: String },
    /// 新建会话（远程）
    NewSession,

    /// 加载 session 列表（进入 Session tab 时调用）
    LoadSessionList,
    /// Session 列表导航
    SessionListNavigate(CursorDirection),
    /// 恢复选中的 session
    RestoreSession,
    /// 删除选中的 session
    DeleteSession,
    /// 新建空 session（从 session 列表中）
    NewSessionFromList,

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
