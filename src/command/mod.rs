pub mod alias;
pub mod category;
pub mod chat;
pub mod handler;
pub mod list;
pub mod open;
pub mod report;
pub mod script;
pub mod system;
pub mod time;
pub mod todo;
pub mod voice;

use crate::cli::SubCmd;
use crate::config::YamlConfig;
use crate::constants;

/// 所有内置命令的关键字列表（用于判断别名冲突）
/// 统一从 constants::cmd 模块获取，避免多处重复定义
pub fn all_command_keywords() -> Vec<&'static str> {
    constants::cmd::all_keywords()
}

/// 命令分发执行
pub fn dispatch(subcmd: SubCmd, config: &mut YamlConfig) {
    subcmd.into_handler().execute(config);
}
