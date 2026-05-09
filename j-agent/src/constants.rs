//! j-cli-core 核心常量
//!
//! 与 j-cli 的 constants.rs 共享的常量定义。

/// 用户数据根目录名
pub const DATA_DIR: &str = ".jdata";

/// Agent 子目录
pub const AGENT_DIR: &str = "agent";

/// Agent 日志子目录
pub const AGENT_LOG_DIR: &str = "logs";

/// 信息日志文件名
pub const AGENT_LOG_INFO: &str = "info.log";

/// 错误日志文件名
pub const AGENT_LOG_ERROR: &str = "error.log";

/// Agent 数据子目录
pub const AGENT_DATA_DIR: &str = "data";

/// 默认数据路径环境变量
pub const DATA_PATH_ENV: &str = "J_DATA_PATH";

/// 获取数据根目录路径
pub fn data_root() -> std::path::PathBuf {
    if let Ok(p) = std::env::var(DATA_PATH_ENV) {
        std::path::PathBuf::from(p)
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(DATA_DIR)
    }
}

// ========== Chat 模块常量 ==========

// ========== 工具执行相关 ==========

/// 工具输出摘要最大长度（字符数）
pub const TOOL_OUTPUT_SUMMARY_MAX_LEN: usize = 60;

/// 输入缓冲区最大长度
pub const INPUT_BUFFER_MAX_LEN: usize = 16384;

// ========== Shell 工具 ==========

/// Shell 命令默认超时时间（秒）
pub const SHELL_DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Shell 命令最大超时时间（秒）
pub const SHELL_MAX_TIMEOUT_SECS: u64 = 600;

/// Shell 命令轮询间隔（毫秒）
pub const SHELL_POLL_INTERVAL_MS: u64 = 50;

/// 前台 Shell 命令自动升级为后台任务的阈值（秒）
/// 超过此时间未结束，自动 adopt 到 BackgroundManager 并返回 task_id
pub const SHELL_AUTO_BG_SECS: u64 = 30;

/// 自动升级前的"疑似交互式"静默阈值（秒）
/// 进程最近这么久都没有任何 stdout/stderr 输出，视为可能在等待交互输入，不升级
/// 设为 180s 以避免误杀构建类命令（cargo build --release、docker build 等编译阶段可能数分钟无输出）
pub const SHELL_INTERACTIVE_SILENCE_SECS: u64 = 180;

// ========== Web 请求相关 ==========

/// Web 请求默认超时时间（秒）
pub const WEB_REQUEST_TIMEOUT_SECS: u64 = 15;

/// Web 响应最大字节数（1MB）
pub const WEB_RESPONSE_MAX_BYTES: usize = 1024 * 1024;

/// Web 响应默认最大字符数
pub const WEB_RESPONSE_DEFAULT_MAX_CHARS: usize = 50000;

/// Web 搜索结果数量上限
pub const WEB_SEARCH_MAX_COUNT: usize = 10;

/// Web 搜索默认结果数量
pub const WEB_SEARCH_DEFAULT_COUNT: usize = 5;

/// Web 搜索摘要最大字符数
pub const WEB_SEARCH_HIGHLIGHTS_MAX_CHARS: usize = 4000;

// ========== Agent 相关 ==========

/// Todo 提醒间隔轮数
pub const TODO_NAG_INTERVAL_ROUNDS: u32 = 15;

/// 默认历史消息数量限制
pub const DEFAULT_MAX_HISTORY_MESSAGES: usize = 100;

/// 默认上下文 token 预算（0 = 不限制，否则单位为 K tokens，如 100 = 100K tokens）
pub const DEFAULT_MAX_CONTEXT_TOKENS: usize = 0;

/// 默认工具调用最大轮数
pub const DEFAULT_MAX_TOOL_ROUNDS: usize = 1000;

// ========== Compact 相关 ==========

/// Micro compact 字节数阈值
pub const MICRO_COMPACT_BYTES_THRESHOLD: usize = 800;

/// Compact token 阈值（256 * 800）
pub const COMPACT_TOKEN_THRESHOLD: usize = 256 * 800;

/// Compact 保留最近消息数（micro_compact 保留的 tool result 数量）
pub const COMPACT_KEEP_RECENT: usize = 10;

/// Auto compact 后保留最近几条 user 消息原文（不限于未回复的）
pub const COMPACT_KEEP_RECENT_USER_MESSAGES: usize = 5;

/// Auto compact 后技能附件总 token 预算（~25K tokens）
pub const COMPACT_SKILL_TOKEN_BUDGET: usize = 25_000;

/// Auto compact 后单个技能的 token 预算（~5K tokens，保留头部使用说明）
pub const COMPACT_SKILL_PER_SKILL_TOKEN_BUDGET: usize = 5_000;

/// Window 时间保底系数：最近 keep_recent * 此系数 个 unit 无条件保留
pub const WINDOW_KEEP_RECENT_MULTIPLIER: usize = 2;

/// Window 各优先级 tier 的预算配额比例（User / AssistantText / ToolGroup）
/// 三者之和应为 1.0；System 无配额（始终保留）
pub const WINDOW_QUOTA_USER: f32 = 0.35;
pub const WINDOW_QUOTA_ASST_TEXT: f32 = 0.25;
pub const WINDOW_QUOTA_TOOL_GROUP: f32 = 0.40;

// ========== 存储相关 ==========

/// 消息预览最大长度
pub const MESSAGE_PREVIEW_MAX_LEN: usize = 50;

// ========== 分类工具 ==========

/// 分类文本截断长度
pub const CLASSIFY_TRUNCATE_LEN: usize = 50;

/// 分类标题截断长度
pub const CLASSIFY_TITLE_TRUNCATE_LEN: usize = 30;

/// 分类文件大小阈值（字节）
pub const CLASSIFY_SIZE_THRESHOLD_BYTES: usize = 1024;

/// 分类文件大小阈值（字符）
pub const CLASSIFY_SIZE_THRESHOLD_CHARS: usize = 100;

// ========== 渲染相关 ==========

/// API 错误 body 截断最大长度（字符数）
pub const API_ERROR_BODY_MAX_LEN: usize = 500;

/// 确认消息最大显示行数
pub const CONFIRM_MSG_MAX_LINES: usize = 10;

/// 工具参数预览最大字符数
pub const TOOL_ARG_PREVIEW_MAX_CHARS: usize = 60;

/// 错误结果最大显示行数
pub const ERROR_RESULT_MAX_LINES: usize = 20;

/// 正常结果最大显示行数
pub const NORMAL_RESULT_MAX_LINES: usize = 100;

/// Agent 结果最大显示行数
pub const AGENT_RESULT_MAX_LINES: usize = 30;

/// Agent tool call request 展开模式下 prompt 最大显示行数
pub const AGENT_CALL_PROMPT_MAX_LINES: usize = 15;

/// Bash 输出最大显示行数
pub const BASH_OUTPUT_MAX_LINES: usize = 100;

/// 思考脉冲周期（毫秒）
pub const THINKING_PULSE_PERIOD_MS: u64 = 1500;

/// 思考脉冲最低亮度因子
pub const THINKING_PULSE_MIN_FACTOR: f64 = 0.3;

// ========== UI 交互相关 ==========

/// 分页滚动行数
pub const PAGE_SCROLL_LINES: usize = 10;

/// 精细滚动行数
pub const FINE_SCROLL_LINES: usize = 3;

/// 工具交互选项上限
pub const TOOL_INTERACT_MAX_OPTIONS: usize = 3;

// ========== Hook 相关 ==========

/// Shell hook 默认超时（秒）
pub const HOOK_DEFAULT_TIMEOUT_SECS: u64 = 10;

/// LLM hook 默认超时（秒）
pub const HOOK_DEFAULT_LLM_TIMEOUT_SECS: u64 = 30;

/// LLM hook 最大生成 token 数
pub const HOOK_LLM_MAX_TOKENS: u64 = 2048;
/// LLM hook 默认温度值（0.0 = 确定性输出）
pub const HOOK_LLM_TEMPERATURE: f32 = 0.0;

// ========== 后台任务相关 ==========

/// 后台任务默认超时（毫秒）
pub const BG_TASK_DEFAULT_TIMEOUT_MS: u64 = 30_000;

/// 后台任务最大超时（毫秒，10 分钟）
pub const BG_TASK_MAX_TIMEOUT_MS: u64 = 600_000;

/// 后台任务命令显示截断长度
pub const BG_TASK_CMD_DISPLAY_MAX_CHARS: usize = 77;

// ========== Glob 工具 ==========

/// Glob 默认返回数量限制
pub const GLOB_DEFAULT_LIMIT: usize = 100;

// ========== Computer Use 工具 ==========

/// 拖拽操作默认持续时间（毫秒）
pub const DRAG_DEFAULT_DURATION_MS: u64 = 500;

// ========== Compact 相关（续）==========

/// Compact 摘要最大 token 数
pub const COMPACT_SUMMARY_MAX_TOKENS: u32 = 20000;

/// Compact 截断字符数（80K 字符）
pub const COMPACT_TRUNCATE_MAX_CHARS: usize = 80_000;

// ========== Browser 工具 ==========

/// 浏览器页面正文最大字符数
pub const BROWSER_TEXT_MAX_CHARS: usize = 50_000;

/// 浏览器快照元素最大数量
pub const BROWSER_SNAPSHOT_MAX_ELEMENTS: usize = 50;

/// 浏览器 Lite 模式链接最大数量
#[cfg(not(feature = "browser_cdp"))]
pub const BROWSER_LITE_MAX_LINKS: usize = 50;

/// 浏览器 Lite 模式表单最大数量
#[cfg(not(feature = "browser_cdp"))]
pub const BROWSER_LITE_MAX_FORMS: usize = 20;

/// 浏览器 Lite 模式文本预览截断长度
#[cfg(not(feature = "browser_cdp"))]
pub const BROWSER_LITE_TEXT_PREVIEW_MAX_CHARS: usize = 500;

// ========== Computer Use 工具 ==========

/// 无障碍树输出最大字符数
pub const AX_TREE_OUTPUT_MAX_CHARS: usize = 20_000;

/// 应用聚焦等待时间（毫秒）
pub const APP_FOCUS_WAIT_MS: u64 = 300;

/// 鼠标双击间隔等待时间（毫秒）
pub const MOUSE_DOUBLE_CLICK_WAIT_MS: u64 = 50;

// ========== Agent Team ==========

// ========== Worktree ==========

/// Worktree 名称最大长度
pub const WORKTREE_NAME_MAX_LEN: usize = 64;

// ========== Archive ==========

/// 归档名称最大长度
pub const ARCHIVE_NAME_MAX_LEN: usize = 50;

// ========== Hook 相关（续）==========

/// Hook prompt 预览最大长度
pub const HOOK_PROMPT_PREVIEW_MAX_LEN: usize = 80;

// ========== TUI 输入线程 ==========

/// 输入线程暂停等待时间（毫秒）
pub const INPUT_THREAD_PAUSE_WAIT_MS: u64 = 50;

/// 输入线程 poll 超时（毫秒）
pub const INPUT_THREAD_POLL_MS: u64 = 50;

/// 输入线程暂停后等待时间（毫秒，确保线程退出当前 poll 周期）
pub const INPUT_THREAD_PAUSE_SETTLE_MS: u64 = 120;
/// 输入线程读取/poll 出错后重试休眠间隔（毫秒）
pub const INPUT_THREAD_RETRY_SLEEP_MS: u64 = 10;

// ========== TUI 主循环 ==========

/// TUI 主循环加载中轮询超时（毫秒）
pub const TUI_LOADING_POLL_MS: u64 = 100;

/// TUI 主循环空闲轮询超时（毫秒）
pub const TUI_IDLE_POLL_MS: u64 = 500;

// ========== Computer Use 输入事件 ==========

/// 按键按下-释放间隔延迟（毫秒）
pub const KEY_PRESS_DELAY_MS: u64 = 10;

// ========== Browser Lite 模式 ==========

/// Browser Lite HTTP 请求超时（秒）
#[cfg(not(feature = "browser_cdp"))]
pub const BROWSER_LITE_HTTP_TIMEOUT_SECS: u64 = 15;

/// Browser Lite HTTP 最大重定向次数
#[cfg(not(feature = "browser_cdp"))]
pub const BROWSER_LITE_MAX_REDIRECTS: usize = 10;

// ========== Hook 日志 ==========

/// Hook 日志描述截断最大长度
pub const HOOK_LOG_DESC_MAX_LEN: usize = 60;

// ========== Teammate 创建 ==========

/// Teammate 日志结果截断最大长度
pub const TEAMMATE_LOG_RESULT_MAX_CHARS: usize = 200;

/// Teammate prompt 预览截断最大长度
pub const TEAMMATE_PROMPT_PREVIEW_MAX_CHARS: usize = 100;

/// 配置字段列表
pub const CONFIG_FIELDS: &[&str] = &["name", "api_base", "api_key", "model", "supports_vision"];

/// 全局配置字段（Tab 分页版，去掉 tools_enabled 和 skills_enabled）
pub const CONFIG_GLOBAL_FIELDS_TAB: &[&str] = &[
    "system_prompt",
    "agent_md",
    "style",
    "max_history_messages",
    "max_context_tokens",
    "max_tool_rounds",
    "tool_confirm_timeout",
    "theme",
    "auto_restore_session",
    "thinking_style",
    "flat_bubble",
    "compact_enabled",
    "compact_token_threshold",
    "compact_keep_recent",
    "compact_exempt_tools",
];

/// Toast 通知显示时长（秒）
pub const TOAST_DURATION_SECS: u64 = 4;
