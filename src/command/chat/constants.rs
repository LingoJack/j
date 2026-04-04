// ========== Chat 消息角色 ==========

pub const ROLE_SYSTEM: &str = "system";
pub const ROLE_USER: &str = "user";
pub const ROLE_ASSISTANT: &str = "assistant";
pub const ROLE_TOOL: &str = "tool";

// ========== 工具执行相关 ==========

/// 工具输出摘要最大长度（字符数）
pub const TOOL_OUTPUT_SUMMARY_MAX_LEN: usize = 60;

/// 工具调用结果 channel 缓冲区大小
pub const TOOL_RESULT_CHANNEL_BUFFER: usize = 16;

/// 输入缓冲区最大长度
pub const INPUT_BUFFER_MAX_LEN: usize = 16384;

// ========== 工具确认超时 ==========

/// 工具确认模式默认超时时间（秒）
pub const TOOL_CONFIRM_TIMEOUT_SECS: u64 = 10;

/// Hook 默认超时时间（秒）
pub const HOOK_DEFAULT_TIMEOUT_SECS: u64 = 10;

// ========== Shell 工具 ==========

/// Shell 命令默认超时时间（秒）
pub const SHELL_DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Shell 命令最大超时时间（秒）
pub const SHELL_MAX_TIMEOUT_SECS: u64 = 600;

/// Shell 命令轮询间隔（毫秒）
pub const SHELL_POLL_INTERVAL_MS: u64 = 50;

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

// ========== Browser 工具 ==========

/// Browser 页面文本预览最大长度
pub const BROWSER_TEXT_PREVIEW_MAX_LEN: usize = 500;

/// Browser 元素文本最大长度
pub const BROWSER_ELEMENT_TEXT_MAX_LEN: usize = 80;

/// Browser 元素最大数量
pub const BROWSER_MAX_ELEMENTS: usize = 50;

/// Browser 表单最大数量
pub const BROWSER_MAX_FORMS: usize = 20;

/// Browser 重定向最大次数
pub const BROWSER_MAX_REDIRECTS: usize = 10;

// ========== 文件工具 ==========

/// Glob 结果默认数量上限
pub const GLOB_DEFAULT_LIMIT: usize = 100;

/// Glob 结果最大数量上限
pub const GLOB_MAX_LIMIT: usize = 1000;

// ========== Agent 相关 ==========

/// 子代理最大轮数
pub const AGENT_MAX_TOOL_ROUNDS: usize = 30;

/// Todo 提醒间隔轮数
pub const TODO_NAG_INTERVAL_ROUNDS: u32 = 15;

/// 默认历史消息数量限制
pub const DEFAULT_MAX_HISTORY_MESSAGES: usize = 20;

/// 默认工具调用最大轮数
pub const DEFAULT_MAX_TOOL_ROUNDS: usize = 10;

/// 默认 tool_confirm 超时（秒）
pub const DEFAULT_TOOL_CONFIRM_TIMEOUT_SECS: u64 = 100;

// ========== Compact 相关 ==========

/// Micro compact 字节数阈值
pub const MICRO_COMPACT_BYTES_THRESHOLD: usize = 800;

/// Compact token 阈值（256 * 800）
pub const COMPACT_TOKEN_THRESHOLD: usize = 256 * 800;

/// Compact 保留最近消息数
pub const COMPACT_KEEP_RECENT: usize = 10;

/// Compact 最大字符数
pub const COMPACT_MAX_CHARS: usize = 80000;

// ========== UI 渲染相关 ==========

/// 消息气泡最大宽度百分比（相对于可用宽度）
pub const BUBBLE_MAX_WIDTH_PERCENT: usize = 85;

/// 消息气泡最小宽度
pub const BUBBLE_MIN_WIDTH: usize = 20;

/// Toast 最小宽度
pub const TOAST_MIN_WIDTH: usize = 16;

/// Toast 内边距
pub const TOAST_PADDING: usize = 10;

/// 帮助键宽度
pub const HELP_KEY_WIDTH: usize = 15;

/// 弹窗最小宽度
pub const POPUP_MIN_WIDTH: usize = 30;

/// 弹窗最大宽度
pub const POPUP_MAX_WIDTH: usize = 50;

/// 弹窗列表最大显示项数
pub const POPUP_MAX_ITEMS: usize = 15;

/// 文本截断预览长度
pub const TEXT_PREVIEW_TRUNCATE_LEN: usize = 40;

/// 长命令显示截断长度
pub const COMMAND_DISPLAY_TRUNCATE_LEN: usize = 40;

/// 工具确认信息最大行数
pub const TOOL_CONFIRM_MAX_LINES: usize = 10;

/// 工具调用显示最大行数
pub const TOOL_CALL_DISPLAY_MAX_LINES: usize = 100;

/// 工具调用显示最大行数（错误）
pub const TOOL_CALL_ERROR_MAX_LINES: usize = 20;

/// 工具调用显示最大行数（完成）
pub const TOOL_CALL_DONE_MAX_LINES: usize = 30;

// ========== 滚动相关 ==========

/// 单次滚动行数
pub const SCROLL_LINES: u16 = 3;

/// 分页滚动行数
pub const PAGE_SCROLL_LINES: usize = 10;

/// 微调滚动行数
pub const FINE_SCROLL_LINES: u16 = 3;

// ========== 渲染节流 ==========

/// 渲染帧间隔（毫秒，~30fps）
pub const RENDER_INTERVAL_MS: u64 = 33;

/// 流式渲染字节数阈值
pub const STREAM_RENDER_BYTES_THRESHOLD: usize = 200;

/// 流式渲染时间阈值（毫秒）
pub const STREAM_RENDER_TIME_THRESHOLD_MS: u64 = 150;

/// 流式渲染动画周期（毫秒）
pub const STREAM_ANIMATION_CYCLE_MS: u64 = 1500;

/// 流式渲染最小亮度百分比
pub const STREAM_ANIMATION_MIN_BRIGHTNESS: f64 = 0.3;

// ========== 远程控制相关 ==========

/// WebSocket ping 间隔（秒）
pub const WS_PING_INTERVAL_SECS: u64 = 15;

/// WebSocket pong 超时（秒）
pub const WS_PONG_TIMEOUT_SECS: u64 = 30;

/// WebSocket 密钥交换超时（秒）
pub const WS_KEY_EXCHANGE_TIMEOUT_SECS: u64 = 10;

/// WebSocket 连接重试次数
pub const WS_CONNECT_MAX_RETRIES: usize = 20;

/// WebSocket 连接重试间隔（毫秒）
pub const WS_CONNECT_RETRY_INTERVAL_MS: u64 = 100;

/// WebSocket channel 缓冲区大小
pub const WS_CHANNEL_BUFFER: usize = 256;

/// WebSocket 读取缓冲区大小
pub const WS_READ_BUFFER_SIZE: usize = 4096;

/// 远程启动等待间隔（毫秒）
pub const REMOTE_START_WAIT_MS: u64 = 500;

/// Socket listen backlog
pub const SOCKET_LISTEN_BACKLOG: i32 = 128;

// ========== 加密相关 ==========

/// AES-256-GCM nonce 长度
pub const AES_GCM_NONCE_LEN: usize = 12;

/// AES-256-GCM tag 长度
pub const AES_GCM_TAG_LEN: usize = 16;

/// AES-256 密钥长度
pub const AES_256_KEY_LEN: usize = 32;

/// P-256 公钥未压缩格式长度
pub const P256_PUBLIC_KEY_LEN: usize = 65;

// ========== 自动补全相关 ==========

/// 自动补全最大显示项数
pub const AUTOCOMPLETE_MAX_ITEMS: usize = 20;

/// 文件补全最大显示项数
pub const FILE_AUTOCOMPLETE_MAX_ITEMS: usize = 10;

/// Skill 补全最大显示项数
pub const SKILL_AUTOCOMPLETE_MAX_ITEMS: usize = 15;

/// Command 补全最大显示项数
pub const COMMAND_AUTOCOMPLETE_MAX_ITEMS: usize = 15;

/// 补全评分乘数
pub const AUTOCOMPLETE_SCORE_MULTIPLIER: i32 = 10;

/// 开头匹配加分
pub const AUTOCOMPLETE_START_MATCH_BONUS: i32 = -50;

/// 包含匹配加分
pub const AUTOCOMPLETE_CONTAINS_MATCH_BONUS: i32 = -20;

/// 代码文件扩展名加分
pub const AUTOCOMPLETE_CODE_EXT_BONUS: i32 = -15;

/// 配置文件扩展名加分
pub const AUTOCOMPLETE_CONFIG_EXT_BONUS: i32 = -10;

// ========== 输入线程相关 ==========

/// 输入线程轮询间隔（毫秒）
pub const INPUT_POLL_INTERVAL_MS: u64 = 50;

/// 输入线程退出等待（毫秒）
pub const INPUT_EXIT_WAIT_MS: u64 = 120;

/// 输入去抖间隔（毫秒）
pub const INPUT_DEBOUNCE_MS: u64 = 10;

// ========== Computer Use 相关 ==========

/// SoM 过期时间（秒）
pub const SOM_STALE_SECS: u64 = 30;

/// 鼠标点击后等待时间（毫秒）
pub const MOUSE_CLICK_WAIT_MS: u64 = 10;

/// 鼠标拖拽后等待时间（毫秒）
pub const MOUSE_DRAG_WAIT_MS: u64 = 50;

/// 键盘输入间隔（毫秒）
pub const KEYBOARD_TYPE_DELAY_MS: u64 = 10;

/// 滚动动画默认时长（毫秒）
pub const SCROLL_DURATION_DEFAULT_MS: u64 = 500;

/// 默认键盘输入延迟（毫秒）
pub const TYPE_DELAY_DEFAULT_MS: u64 = 10;

/// Accessibility tree JSON 输出最大长度
pub const AX_TREE_JSON_MAX_LEN: usize = 20000;

/// 文本预览最大长度
pub const AX_TEXT_PREVIEW_MAX_LEN: usize = 15;

/// 文本显示最大长度
pub const AX_TEXT_DISPLAY_MAX_LEN: usize = 30;

/// 点击指示符显示时间（毫秒）
pub const CLICK_INDICATOR_DURATION_MS: u64 = 300;

// ========== 归档相关 ==========

/// 归档名称最大长度
pub const ARCHIVE_NAME_MAX_LEN: usize = 50;

// ========== 存储相关 ==========

/// 消息预览最大长度
pub const MESSAGE_PREVIEW_MAX_LEN: usize = 50;

/// 会话预览最大长度
pub const SESSION_PREVIEW_MAX_LEN: usize = 40;

/// 会话列表名称截断长度
pub const SESSION_NAME_TRUNCATE_LEN: usize = 40;

// ========== 配置显示相关 ==========

/// Hook 命令显示截断长度
pub const HOOK_CMD_DISPLAY_TRUNCATE_LEN: usize = 40;

/// Hook 事件名称显示宽度
pub const HOOK_EVENT_DISPLAY_WIDTH: usize = 22;

// ========== 分类工具 ==========

/// 分类文本截断长度
pub const CLASSIFY_TRUNCATE_LEN: usize = 50;

/// 分类标题截断长度
pub const CLASSIFY_TITLE_TRUNCATE_LEN: usize = 30;

/// 分类路径短格式长度
pub const CLASSIFY_SHORT_PATH_LEN: usize = 40;

/// 分类文件大小阈值（字节）
pub const CLASSIFY_SIZE_THRESHOLD_BYTES: usize = 1024;

/// 分类文件大小阈值（字符）
pub const CLASSIFY_SIZE_THRESHOLD_CHARS: usize = 100;

/// 分类毫秒阈值
pub const CLASSIFY_MS_THRESHOLD: u64 = 1000;

/// 分类秒阈值
pub const CLASSIFY_SEC_THRESHOLD: u64 = 60000;

// ========== 用户代理 ==========

/// 默认 User-Agent
pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

// ========== 状态字符串 ==========

pub const STATUS_IDLE: &str = "idle";
pub const STATUS_LOADING: &str = "loading";
pub const STATUS_TOOL_CONFIRM: &str = "tool_confirm";
pub const STATUS_ASK: &str = "ask";

// ========== 时间格式 ==========

/// 时间格式：秒
pub const TIME_FORMAT_SEC_THRESHOLD: u64 = 60;
/// 时间格式：分钟
pub const TIME_FORMAT_MIN_THRESHOLD: u64 = 3600;
/// 时间格式：小时
pub const TIME_FORMAT_HOUR_THRESHOLD: u64 = 86400;
/// 时间格式：天
pub const TIME_FORMAT_DAY_THRESHOLD: u64 = 86400 * 30;

/// Unix 纪元起始年份
pub const UNIX_EPOCH_START_YEAR: i32 = 1970;

/// 闰年天数
pub const DAYS_IN_LEAP_YEAR: u32 = 366;
/// 平年天数
pub const DAYS_IN_NORMAL_YEAR: u32 = 365;

// ========== 默认配置值 ==========

/// 默认终端宽度
pub const DEFAULT_TERMINAL_WIDTH: u16 = 80;

/// 默认终端高度
pub const DEFAULT_TERMINAL_HEIGHT: u16 = 20;

/// 默认重连等待（毫秒）
pub const DEFAULT_RECONNECT_WAIT_MS: u64 = 300;

/// 默认重连最大等待（毫秒）
pub const DEFAULT_RECONNECT_MAX_WAIT_MS: u64 = 500;

/// 分页滚动次数
pub const PAGE_SCROLL_ITERATIONS: usize = 10;

// ========== Ask 工具 ==========

/// Ask 问题标签最大长度
pub const ASK_HEADER_MAX_LEN: usize = 12;
