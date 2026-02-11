/// 项目全局常量定义
/// 所有魔法字符串和可复用常量统一在此维护

// ========== 版本信息 ==========

/// 内核版本号（唯一定义，所有需要版本号的地方引用此常量）
pub const VERSION: &str = "11.0.0";

/// 项目名称
pub const APP_NAME: &str = "work-copilot";

/// 作者
pub const AUTHOR: &str = "lingojack";

/// 邮箱
pub const EMAIL: &str = "lingojack@qq.com";

// ========== Section 名称 ==========

/// 配置文件中的 section 名称常量
pub mod section {
    pub const PATH: &str = "path";
    pub const INNER_URL: &str = "inner_url";
    pub const OUTER_URL: &str = "outer_url";
    pub const EDITOR: &str = "editor";
    pub const BROWSER: &str = "browser";
    pub const VPN: &str = "vpn";
    pub const SCRIPT: &str = "script";
    pub const VERSION: &str = "version";
    pub const SETTING: &str = "setting";
    pub const LOG: &str = "log";
    pub const REPORT: &str = "report";
}

/// 所有 section 名称列表（有序）
pub const ALL_SECTIONS: &[&str] = &[
    section::PATH,
    section::INNER_URL,
    section::OUTER_URL,
    section::EDITOR,
    section::BROWSER,
    section::VPN,
    section::SCRIPT,
    section::VERSION,
    section::SETTING,
    section::LOG,
    section::REPORT,
];

/// 默认展示的 section（ls 命令无参数时使用）
pub const DEFAULT_DISPLAY_SECTIONS: &[&str] = &[
    section::PATH,
    section::INNER_URL,
    section::OUTER_URL,
    section::EDITOR,
    section::BROWSER,
    section::VPN,
    section::SCRIPT,
];

/// contain 命令默认搜索的 section
pub const CONTAIN_SEARCH_SECTIONS: &[&str] = &[
    section::PATH,
    section::SCRIPT,
    section::BROWSER,
    section::EDITOR,
    section::VPN,
    section::INNER_URL,
    section::OUTER_URL,
];

// ========== 分类标记 ==========

/// 可标记的分类列表（note/denote 命令使用）
pub const NOTE_CATEGORIES: &[&str] = &[
    section::BROWSER,
    section::EDITOR,
    section::VPN,
    section::OUTER_URL,
    section::SCRIPT,
];

// ========== 别名查找 section ==========

/// 用于查找别名路径的 section 列表（按优先级排列）
pub const ALIAS_PATH_SECTIONS: &[&str] = &[
    section::PATH,
    section::INNER_URL,
    section::OUTER_URL,
];

/// 用于判断别名是否存在的 section 列表
pub const ALIAS_EXISTS_SECTIONS: &[&str] = &[
    section::PATH,
    section::INNER_URL,
    section::OUTER_URL,
    section::SCRIPT,
    section::BROWSER,
    section::EDITOR,
    section::VPN,
];

/// modify 命令需要检查并更新的 section 列表
pub const MODIFY_SECTIONS: &[&str] = &[
    section::PATH,
    section::INNER_URL,
    section::OUTER_URL,
    section::EDITOR,
    section::BROWSER,
    section::VPN,
];

/// remove 时需要同步清理的 category section
pub const REMOVE_CLEANUP_SECTIONS: &[&str] = &[
    section::EDITOR,
    section::VPN,
    section::BROWSER,
    section::SCRIPT,
];

/// rename 时需要同步重命名的 category section
pub const RENAME_SYNC_SECTIONS: &[&str] = &[
    section::BROWSER,
    section::EDITOR,
    section::VPN,
    section::SCRIPT,
];

// ========== 配置 key ==========

/// 配置 key 名称常量
pub mod config_key {
    pub const MODE: &str = "mode";
    pub const VERBOSE: &str = "verbose";
    pub const CONCISE: &str = "concise";
    pub const SEARCH_ENGINE: &str = "search-engine";
    pub const WEEK_REPORT: &str = "week_report";
    pub const WEEK_NUM: &str = "week_num";
    pub const LAST_DAY: &str = "last_day";
}

// ========== 搜索引擎 ==========

/// 默认搜索引擎
pub const DEFAULT_SEARCH_ENGINE: &str = "bing";

/// 搜索引擎 URL 模板
pub mod search_engine {
    pub const GOOGLE: &str = "https://www.google.com/search?q={}";
    pub const BING: &str = "https://www.bing.com/search?q={}";
    pub const BAIDU: &str = "https://www.baidu.com/s?wd={}";
}

// ========== 日报相关 ==========

/// 日报日期格式
pub const REPORT_DATE_FORMAT: &str = "%Y.%m.%d";

/// 日报简短日期格式
pub const REPORT_SIMPLE_DATE_FORMAT: &str = "%Y/%m/%d";

/// check 命令默认行数
pub const DEFAULT_CHECK_LINES: usize = 5;

// ========== 交互模式 ==========

/// 交互模式提示符
pub const INTERACTIVE_PROMPT: &str = "copilot >";

/// 历史记录文件名
pub const HISTORY_FILE: &str = "history.txt";

/// 配置文件名
pub const CONFIG_FILE: &str = "config.yaml";

/// 脚本目录名
pub const SCRIPTS_DIR: &str = "scripts";

/// 数据根目录名
pub const DATA_DIR: &str = ".jdata";

/// 数据路径环境变量名
pub const DATA_PATH_ENV: &str = "J_DATA_PATH";
