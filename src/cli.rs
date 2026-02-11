use clap::{Parser, Subcommand};
use crate::constants;

/// work-copilot (j) - 快捷命令行工具 🚀
#[derive(Parser, Debug)]
#[command(name = "j", version = constants::VERSION, about = "快捷命令行工具", long_about = None)]
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
    #[command(alias = "rm")]
    Remove {
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
    #[command(alias = "mf")]
    Modify {
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
    #[command(alias = "ls")]
    List {
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

    // ========== 日报系统 ==========
    /// 写入日报
    #[command(aliases = ["r"])]
    Report {
        /// 日报内容（支持多个参数拼接）
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        content: Vec<String>,
    },

    /// 日报元数据操作（new/sync/push/pull）
    #[command(name = "r-meta")]
    RMeta {
        /// 操作: new / sync / push / pull
        action: String,
        /// 可选参数（new/sync 时为日期，push 时为 commit message）
        arg: Option<String>,
    },

    /// 查看日报最近 N 行
    #[command(alias = "c")]
    Check {
        /// 行数（默认 5）
        line_count: Option<String>,
    },

    /// 在日报中搜索关键字
    #[command(aliases = ["select", "look", "sch"])]
    Search {
        /// 行数或 "all"
        line_count: String,
        /// 搜索关键字
        target: String,
        /// 可选: -f 或 -fuzzy 启用模糊匹配
        #[arg(allow_hyphen_values = true)]
        fuzzy: Option<String>,
    },

    // ========== 脚本 ==========
    /// 创建脚本
    Concat {
        /// 脚本名称
        name: String,
        /// 脚本内容
        content: String,
    },

    // ========== 计时器 ==========
    /// 倒计时器
    Time {
        /// 功能名称（目前支持: countdown）
        function: String,
        /// 参数（时长，如 30s、5m、1h）
        arg: String,
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
