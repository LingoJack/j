pub mod alias;
pub mod category;
pub mod chat;
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
    match subcmd {
        // 别名管理
        SubCmd::Set { alias, path } => alias::handle_set(&alias, &path, config),
        SubCmd::Remove { alias } => alias::handle_remove(&alias, config),
        SubCmd::Rename { alias, new_alias } => alias::handle_rename(&alias, &new_alias, config),
        SubCmd::Modify { alias, path } => alias::handle_modify(&alias, &path, config),

        // 分类标记
        SubCmd::Note { alias, category } => category::handle_note(&alias, &category, config),
        SubCmd::Denote { alias, category } => category::handle_denote(&alias, &category, config),

        // 列表 & 查找
        SubCmd::List { part } => list::handle_list(part.as_deref(), config),
        SubCmd::Contain { alias, containers } => {
            system::handle_contain(&alias, containers.as_deref(), config)
        }

        // 日报系统
        SubCmd::Report { content } => report::handle_report("report", &content, config),
        SubCmd::Reportctl { action, arg } => {
            let mut args = vec![action];
            if let Some(a) = arg {
                args.push(a);
            }
            report::handle_report("reportctl", &args, config);
        }
        SubCmd::Check { line_count } => report::handle_check(line_count.as_deref(), config),
        SubCmd::Search {
            line_count,
            target,
            fuzzy,
        } => {
            report::handle_search(&line_count, &target, fuzzy.as_deref(), config);
        }

        // 待办备忘录
        SubCmd::Todo { content } => todo::handle_todo(&content, config),

        // AI 对话
        SubCmd::Chat { content } => chat::handle_chat(&content, config),

        // 脚本
        SubCmd::Concat { name, content } => script::handle_concat(&name, &content, config),

        // 倒计时
        SubCmd::Time { function, arg } => time::handle_time(&function, &arg),

        // 系统设置
        SubCmd::Log { key, value } => system::handle_log(&key, &value, config),
        SubCmd::Change { part, field, value } => {
            system::handle_change(&part, &field, &value, config)
        }
        SubCmd::Clear => system::handle_clear(),

        // 系统信息
        SubCmd::Version => system::handle_version(config),
        SubCmd::Help => system::handle_help(),
        SubCmd::Exit => system::handle_exit(),
        SubCmd::Completion { shell } => system::handle_completion(shell.as_deref(), config),

        // 语音转文字
        SubCmd::Voice {
            action,
            copy,
            model,
        } => voice::handle_voice(&action, copy, model.as_deref(), config),
    }
}
