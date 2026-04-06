//! 统一命令执行器（GUI 友好）
//! 接收命令字符串，解析并执行，返回结构化结果

use crate::config::YamlConfig;
use crate::core::{alias, open, search};
use serde::Serialize;

/// 命令执行结果
#[derive(Debug, Clone, Serialize)]
pub struct CommandResult {
    /// 是否成功
    pub success: bool,
    /// 执行的命令类型
    pub command: String,
    /// 结果消息
    pub message: String,
    /// 搜索结果（如果是搜索/列表类命令）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<search::SearchResult>>,
}

/// 解析并执行命令字符串
/// 输入格式与 CLI 完全一致：如 "set chrome /Applications/Google Chrome.app"
pub fn execute_command(input: &str) -> CommandResult {
    let input = input.trim();
    if input.is_empty() {
        return CommandResult {
            success: false,
            command: String::new(),
            message: "请输入命令".to_string(),
            results: None,
        };
    }

    // 拆分为 args（类似 shell 的 argv）
    let args = shell_split(input);
    if args.is_empty() {
        return CommandResult {
            success: false,
            command: String::new(),
            message: "请输入命令".to_string(),
            results: None,
        };
    }

    let cmd = args[0].to_lowercase();
    let rest = &args[1..];
    let mut config = YamlConfig::load();

    match cmd.as_str() {
        // ========== 别名管理 ==========
        "set" | "s" => handle_set(rest, &mut config),
        "remove" | "rm" => handle_remove(rest, &mut config),
        "rename" | "rn" => handle_rename(rest, &mut config),
        "modify" | "mf" => handle_modify(rest, &mut config),

        // ========== 列表 & 搜索 ==========
        "list" | "ls" => handle_list(rest, &config),

        // ========== 版本 & 帮助 ==========
        "version" | "v" => CommandResult {
            success: true,
            command: "version".to_string(),
            message: format!("j-cli v{}", crate::constants::VERSION),
            results: None,
        },
        "help" | "h" => CommandResult {
            success: true,
            command: "help".to_string(),
            message: get_help_text(),
            results: None,
        },

        // ========== 默认：尝试作为别名打开 ==========
        _ => handle_open_alias(&args, &config),
    }
}

fn handle_set(args: &[String], config: &mut YamlConfig) -> CommandResult {
    if args.len() < 2 {
        return CommandResult {
            success: false,
            command: "set".to_string(),
            message: "用法: set <alias> <path>".to_string(),
            results: None,
        };
    }
    let alias_name = &args[0];
    let path = args[1..].join(" ");
    match alias::set_alias(alias_name, &path, config) {
        Ok(msg) => CommandResult {
            success: true,
            command: "set".to_string(),
            message: msg,
            results: None,
        },
        Err(msg) => CommandResult {
            success: false,
            command: "set".to_string(),
            message: msg,
            results: None,
        },
    }
}

fn handle_remove(args: &[String], config: &mut YamlConfig) -> CommandResult {
    if args.is_empty() {
        return CommandResult {
            success: false,
            command: "remove".to_string(),
            message: "用法: rm <alias>".to_string(),
            results: None,
        };
    }
    match alias::remove_alias(&args[0], config) {
        Ok(msg) => CommandResult {
            success: true,
            command: "remove".to_string(),
            message: msg,
            results: None,
        },
        Err(msg) => CommandResult {
            success: false,
            command: "remove".to_string(),
            message: msg,
            results: None,
        },
    }
}

fn handle_rename(args: &[String], config: &mut YamlConfig) -> CommandResult {
    if args.len() < 2 {
        return CommandResult {
            success: false,
            command: "rename".to_string(),
            message: "用法: rn <alias> <new_alias>".to_string(),
            results: None,
        };
    }
    match alias::rename_alias(&args[0], &args[1], config) {
        Ok(msg) => CommandResult {
            success: true,
            command: "rename".to_string(),
            message: msg,
            results: None,
        },
        Err(msg) => CommandResult {
            success: false,
            command: "rename".to_string(),
            message: msg,
            results: None,
        },
    }
}

fn handle_modify(args: &[String], config: &mut YamlConfig) -> CommandResult {
    if args.len() < 2 {
        return CommandResult {
            success: false,
            command: "modify".to_string(),
            message: "用法: mf <alias> <new_path>".to_string(),
            results: None,
        };
    }
    let path = args[1..].join(" ");
    match alias::modify_alias(&args[0], &path, config) {
        Ok(msg) => CommandResult {
            success: true,
            command: "modify".to_string(),
            message: msg,
            results: None,
        },
        Err(msg) => CommandResult {
            success: false,
            command: "modify".to_string(),
            message: msg,
            results: None,
        },
    }
}

fn handle_list(args: &[String], config: &YamlConfig) -> CommandResult {
    let query = if args.is_empty() { "" } else { &args[0] };
    let results = search::search_aliases(query, config);
    CommandResult {
        success: true,
        command: "list".to_string(),
        message: format!("找到 {} 个别名", results.len()),
        results: Some(results),
    }
}

fn handle_open_alias(args: &[String], config: &YamlConfig) -> CommandResult {
    match open::open_alias_silent(&args[0], args, config) {
        Ok(msg) => CommandResult {
            success: true,
            command: "open".to_string(),
            message: msg,
            results: None,
        },
        Err(msg) => CommandResult {
            success: false,
            command: "open".to_string(),
            message: msg,
            results: None,
        },
    }
}

/// 简单的 shell 字符串分割（支持引号）
fn shell_split(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    for ch in input.chars() {
        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

fn get_help_text() -> String {
    [
        "j-cli GUI 命令帮助",
        "",
        "别名管理:",
        "  set <alias> <path>     添加别名",
        "  rm <alias>             删除别名",
        "  rn <old> <new>         重命名别名",
        "  mf <alias> <path>      修改别名路径",
        "",
        "查询:",
        "  ls [filter]            列出别名",
        "",
        "其他:",
        "  <alias> [args...]      打开别名",
        "  version                版本信息",
        "  help                   帮助信息",
    ]
    .join("\n")
}
