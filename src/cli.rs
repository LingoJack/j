use clap::{Parser, Subcommand};

/// work-copilot (j) - 快捷命令行工具 🚀
#[derive(Parser, Debug)]
#[command(name = "j", version = "11.0.0", about = "快捷命令行工具", long_about = None)]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<SubCmd>,

    /// 当没有匹配到子命令时，收集所有剩余参数（用于别名打开）
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum SubCmd {
    // ========== 别名管理 ==========
    /// 设置别名（路径/URL）
    #[command(alias = "s")]
    Set {
        /// 别名
        alias: String,
        /// 路径或 URL（支持空格，多个参数会拼接）
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        path: Vec<String>,
    },

    /// 删除别名
    #[command(alias = "remove")]
    Rm {
        /// 要删除的别名
        alias: String,
    },

    /// 重命名别名
    #[command(alias = "rn")]
    Rename {
        /// 原别名
        alias: String,
        /// 新别名
        new_alias: String,
    },

    /// 修改别名对应的路径
    #[command(alias = "modify")]
    Mf {
        /// 别名
        alias: String,
        /// 新路径或 URL
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        path: Vec<String>,
    },

    // ========== 分类标记 ==========
    /// 标记别名为指定分类（browser/editor/vpn/outer_url/script）
    #[command(alias = "nt")]
    Note {
        /// 别名
        alias: String,
        /// 分类: browser, editor, vpn, outer_url, script
        category: String,
    },

    /// 解除别名的分类标记
    #[command(alias = "dnt")]
    Denote {
        /// 别名
        alias: String,
        /// 分类: browser, editor, vpn, outer_url, script
        category: String,
    },

    // ========== 列表 ==========
    /// 列出别名
    #[command(alias = "list")]
    Ls {
        /// 指定 section（可选，如 path/inner_url/all 等）
        part: Option<String>,
    },

    /// 在指定分类中查找别名
    #[command(alias = "find")]
    Contain {
        /// 要搜索的别名
        alias: String,
        /// 可选的分类列表（逗号分隔，如 path,browser,vpn）
        containers: Option<String>,
    },

    // ========== 系统设置 ==========
    /// 日志模式设置
    Log {
        /// 设置项名称（如 mode）
        key: String,
        /// 设置值（如 verbose/concise）
        value: String,
    },

    /// 直接修改配置文件中的某个字段
    #[command(alias = "chg")]
    Change {
        /// section 名称
        part: String,
        /// 字段名
        field: String,
        /// 新值
        value: String,
    },

    /// 清屏
    #[command(alias = "cls")]
    Clear,

    // ========== 系统信息 ==========
    /// 版本信息
    #[command(alias = "v")]
    Version,

    /// 帮助信息
    #[command(alias = "h")]
    Help,

    /// 退出（交互模式）
    #[command(aliases = ["q", "quit"])]
    Exit,
}
