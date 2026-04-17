//! Chat 模块常量定义
//! 集中管理所有魔法数字，便于维护和调整

// ========== Chat 消息角色 ==========

pub const ROLE_SYSTEM: &str = "system";
pub const ROLE_USER: &str = "user";
pub const ROLE_ASSISTANT: &str = "assistant";
pub const ROLE_TOOL: &str = "tool";

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

/// Compact 保留最近消息数
pub const COMPACT_KEEP_RECENT: usize = 10;

/// Auto compact 后技能附件总 token 预算（~25K tokens）
pub const COMPACT_SKILL_TOKEN_BUDGET: usize = 25_000;

/// Auto compact 后单个技能的 token 预算（~5K tokens，保留头部使用说明）
pub const COMPACT_SKILL_PER_SKILL_TOKEN_BUDGET: usize = 5_000;

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
