// 项目全局常量定义
// 所有魔法字符串和可复用常量统一在此维护

// ========== 版本信息 ==========

/// 内核版本号（自动从 Cargo.toml 读取，编译时确定）
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 项目名称
pub const APP_NAME: &str = "j-cli";

/// 作者
pub const AUTHOR: &str = "lingojack";

/// 邮箱
pub const EMAIL: &str = "lingojack@qq.com";

/// 安装来源（编译时嵌入）
/// - "github": 从 GitHub Release 安装
/// - "cargo": 从 crates.io 安装（默认）
///
/// GitHub Release 构建时通过环境变量 INSTALL_SOURCE=github 设置
pub const INSTALL_SOURCE: &str = match option_env!("INSTALL_SOURCE") {
    Some(s) => s,
    None => "cargo",
};

/// 配置编辑界面的字段列表
pub const CONFIG_FIELDS: &[&str] = &["name", "api_base", "api_key", "model", "supports_vision"];
/// 全局配置字段（旧版，保留向后兼容）
#[allow(dead_code)]
pub const CONFIG_GLOBAL_FIELDS: &[&str] = &[
    "system_prompt",
    "style",
    "max_history_messages",
    "theme",
    "tools_enabled",
    "max_tool_rounds",
    "tool_confirm_timeout",
    "skills_enabled",
];

/// 全局配置字段（Tab 分页版，去掉 tools_enabled 和 skills_enabled）
pub const CONFIG_GLOBAL_FIELDS_TAB: &[&str] = &[
    "system_prompt",
    "style",
    "max_history_messages",
    "theme",
    "max_tool_rounds",
    "tool_confirm_timeout",
    "auto_restore_session",
];

/// Toast 通知显示时长（秒）
pub const TOAST_DURATION_SECS: u64 = 4;

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
pub const ALIAS_PATH_SECTIONS: &[&str] = &[section::PATH, section::INNER_URL, section::OUTER_URL];

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
    pub const GIT_REPO: &str = "git_repo";
    #[cfg_attr(not(feature = "browser_cdp"), allow(dead_code))]
    pub const BROWSER_HEADLESS: &str = "browser_headless";
}

// ========== 搜索引擎 ==========

/// 默认搜索引擎
pub const DEFAULT_SEARCH_ENGINE: &str = "bing";

/// 新窗口执行标志
pub const NEW_WINDOW_FLAG: &str = "-w";
pub const NEW_WINDOW_FLAG_LONG: &str = "--new-window";

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
pub const DEFAULT_CHECK_LINES: usize = 10;

// ========== 命令名常量 ==========

/// 所有内置命令的名称和别名，统一在此维护
/// interactive.rs 的补全规则 / parse_interactive_command 和 command/mod.rs 的 all_command_keywords 共同引用
pub mod cmd {
    // 别名管理
    pub const SET: &[&str] = &["set", "s"];
    pub const REMOVE: &[&str] = &["rm", "remove"];
    pub const RENAME: &[&str] = &["rename", "rn"];
    pub const MODIFY: &[&str] = &["mf", "modify"];

    // 分类标记
    pub const NOTE: &[&str] = &["note", "nt"];
    pub const DENOTE: &[&str] = &["denote", "dnt"];

    // 列表 & 查找
    pub const LIST: &[&str] = &["ls", "list"];
    pub const CONTAIN: &[&str] = &["contain", "find"];

    // 日报系统
    pub const REPORT: &[&str] = &["report", "r"];
    pub const REPORTCTL: &[&str] = &["reportctl", "rctl"];
    pub const CHECK: &[&str] = &["check", "c"];
    pub const SEARCH: &[&str] = &["search", "select", "look", "sch"];

    // 待办备忘录
    pub const TODO: &[&str] = &["todo", "td"];

    // 脚本
    pub const CONCAT: &[&str] = &["concat"];

    // 倒计时
    pub const TIME: &[&str] = &["time"];

    // 系统设置
    pub const LOG: &[&str] = &["log"];
    pub const CHANGE: &[&str] = &["change", "chg"];
    pub const CLEAR: &[&str] = &["clear", "cls"];

    // 系统信息
    pub const VERSION: &[&str] = &["version", "v"];
    pub const HELP: &[&str] = &["help", "h"];
    pub const EXIT: &[&str] = &["exit", "q", "quit"];

    // shell 补全
    pub const COMPLETION: &[&str] = &["completion"];

    // AI 对话
    pub const CHAT: &[&str] = &["chat", "ai"];

    // agent（预留）
    pub const AGENT: &[&str] = &["agent"];
    pub const SYSTEM: &[&str] = &["system", "ps"];

    // 自更新
    pub const UPDATE: &[&str] = &["update", "up"];

    // Markdown 编辑器
    pub const MD: &[&str] = &["md", "markdown"];

    // 笔记本
    pub const NOTEBOOK: &[&str] = &["notebook", "nb"];

    /// 获取所有内置命令关键字的扁平列表（用于判断别名冲突等）
    pub fn all_keywords() -> Vec<&'static str> {
        let groups: &[&[&str]] = &[
            SET, REMOVE, RENAME, MODIFY, NOTE, DENOTE, LIST, CONTAIN, REPORT, REPORTCTL, CHECK,
            SEARCH, TODO, CHAT, CONCAT, TIME, LOG, CHANGE, CLEAR, VERSION, HELP, EXIT, COMPLETION,
            AGENT, SYSTEM, UPDATE, MD, NOTEBOOK,
        ];
        groups.iter().flat_map(|g| g.iter().copied()).collect()
    }
}

// ========== reportctl 子命令 ==========

pub mod rmeta_action {
    pub const NEW: &str = "new";
    pub const SYNC: &str = "sync";
    pub const PUSH: &str = "push";
    pub const PULL: &str = "pull";
    pub const SET_URL: &str = "set-url";
    pub const OPEN: &str = "open";
}

// ========== notebook 子命令 ==========

pub mod notebook_action {
    pub const LIST: &str = "list";
    pub const SEARCH: &str = "search";
    pub const DELETE: &str = "delete";
    pub const OPEN: &str = "open";
    pub const RENAME: &str = "rename";
}

// ========== time 子命令 ==========

pub mod time_function {
    pub const COUNTDOWN: &str = "countdown";
}

// ========== search 标记 ==========

pub mod search_flag {
    pub const FUZZY_SHORT: &str = "-f";
    pub const FUZZY: &str = "-fuzzy";
}

// ========== ls 补全固定选项 ==========

pub const LIST_ALL: &str = "all";

// ========== 交互模式 ==========

/// 欢迎语
pub const WELCOME_MESSAGE: &str = r###"
  ╔═══════════════════════════════════════════════════════════════════════════╗
  ║                                                                           ║
  ║                        ██╗        ██████╗ ██╗      ██╗                    ║
  ║                        ██║       ██╔════╝ ██║      ██║                    ║
  ║                        ██║ █████╗██║      ██║      ██║                    ║
  ║                    ██  ██║ ╚════╝██║      ██║      ██║                    ║
  ║                    ╚█████╔╝      ╚██████╗ ███████╗ ██║                    ║
  ║                      ╚════╝        ╚═════╝ ╚══════╝ ╚═╝                   ║
  ║                                                                           ║
  ╠═══════════════════════════════════════════════════════════════════════════╣
  ║                                                                           ║
  ║   💻 Recommended Terminal: Kitty                                          ║
  ║      https://sw.kovidgoyal.net/kitty                                      ║
  ║                                                                           ║
  ║   🎨 Recommended Theme:                                                   ║
  ║      https://github.com/LingoJack/kitty_conf                              ║
  ║                                                                           ║
  ║   🚉 Github Repo:                                                         ║
  ║      https://github.com/LingoJack/j.git                                   ║
  ║                                                                           ║
  ╠═══════════════════════════════════════════════════════════════════════════╣
  ║                                                                           ║
  ║   📦 Quick Setup:                                                         ║
  ║                                                                           ║
  ║     1. Install Kitty                                                      ║
  ║        curl -L https://sw.kovidgoyal.net/kitty/installer.sh | sh          ║
  ║                                                                           ║
  ║     2. Apply Theme                                                        ║
  ║        cd ~/.config/kitty &&                                              ║
  ║        git clone github.com/LingoJack/kitty_conf .                        ║
  ║                                                                           ║
  ║     3. Reload Kitty                                                       ║
  ║                                                                           ║
  ╚═══════════════════════════════════════════════════════════════════════════╝
"###;

/// Shell 命令前缀字符
pub const SHELL_PREFIX_EN: char = '!';
pub const SHELL_PREFIX_CN: char = '！';

/// 交互模式提示符
pub const INTERACTIVE_PROMPT: &str = "j >";

/// 历史记录文件名
pub const HISTORY_FILE: &str = "history.txt";

/// 配置文件名
pub const CONFIG_FILE: &str = "config.yaml";

/// 脚本目录名
pub const SCRIPTS_DIR: &str = "scripts";

/// 日报目录名
pub const REPORT_DIR: &str = "report";

/// agent 目录名
pub const AGENT_DIR: &str = "agent";

/// agent 日志目录名
pub const AGENT_LOG_DIR: &str = "logs";

/// agent 日志文件名
pub const AGENT_LOG_INFO: &str = "info.log";
pub const AGENT_LOG_ERROR: &str = "error.log";

/// 日报默认文件名
pub const REPORT_DEFAULT_FILE: &str = "week_report.md";

/// 数据根目录名
pub const DATA_DIR: &str = ".jdata";

/// 数据路径环境变量名
pub const DATA_PATH_ENV: &str = "J_DATA_PATH";

// ========== Shell 命令 ==========

pub mod shell {
    pub const BASH_PATH: &str = "/bin/bash";
    pub const WINDOWS_CMD: &str = "cmd";
    pub const WINDOWS_CMD_FLAG: &str = "/c";
    pub const BASH_CMD_FLAG: &str = "-c";
    pub const WINDOWS_OS: &str = "windows";
    pub const MACOS_OS: &str = "macos";
}

// ========== Todo 过滤状态 ==========

/// Todo 过滤模式常量
pub mod todo_filter {
    /// 显示全部待办项
    pub const ALL: usize = 0;
    /// 只显示未完成的待办项
    pub const UNDONE: usize = 1;
    /// 只显示已完成的待办项
    pub const DONE: usize = 2;
    /// 过滤模式总数
    pub const COUNT: usize = 3;

    /// 获取过滤模式标签
    pub fn label(filter: usize) -> &'static str {
        match filter {
            UNDONE => "未完成",
            DONE => "已完成",
            _ => "全部",
        }
    }

    /// 默认过滤模式（未完成）
    pub const DEFAULT: usize = UNDONE;
}
