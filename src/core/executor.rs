//! 统一命令执行器（GUI 友好）
//! 接收命令字符串，解析并执行，返回结构化结果
//! 支持与 CLI 完全一致的命令语法

use crate::config::YamlConfig;
use crate::constants;
use crate::core::{alias, open, search};
use serde::Serialize;

/// 输出类型枚举
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputType {
    /// 简单消息（单行）
    Simple,
    /// 列表结果
    List,
    /// 多行文本输出
    Text,
    /// 表格数据
    Table,
}

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
    /// 输出类型
    pub output_type: OutputType,
    /// 原始文本输出（用于多行文本展示）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_output: Option<String>,
}

impl Default for CommandResult {
    fn default() -> Self {
        Self {
            success: false,
            command: String::new(),
            message: String::new(),
            results: None,
            output_type: OutputType::Simple,
            raw_output: None,
        }
    }
}

impl CommandResult {
    /// 创建简单成功结果
    pub fn success(command: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: true,
            command: command.into(),
            message: message.into(),
            output_type: OutputType::Simple,
            ..Default::default()
        }
    }

    /// 创建简单失败结果
    pub fn failure(command: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            success: false,
            command: command.into(),
            message: message.into(),
            output_type: OutputType::Simple,
            ..Default::default()
        }
    }

    /// 创建列表结果
    pub fn list(
        command: impl Into<String>,
        message: impl Into<String>,
        results: Vec<search::SearchResult>,
    ) -> Self {
        Self {
            success: true,
            command: command.into(),
            message: message.into(),
            results: Some(results),
            output_type: OutputType::List,
            ..Default::default()
        }
    }

    /// 创建文本结果
    pub fn text(
        command: impl Into<String>,
        message: impl Into<String>,
        raw_output: impl Into<String>,
    ) -> Self {
        Self {
            success: true,
            command: command.into(),
            message: message.into(),
            output_type: OutputType::Text,
            raw_output: Some(raw_output.into()),
            ..Default::default()
        }
    }
}

/// 解析并执行命令字符串
/// 输入格式与 CLI 完全一致：如 "set chrome /Applications/Google Chrome.app"
pub fn execute_command(input: &str) -> CommandResult {
    let input = input.trim();
    if input.is_empty() {
        return CommandResult::failure("", "请输入命令");
    }

    // 拆分为 args（类似 shell 的 argv）
    let args = shell_split(input);
    if args.is_empty() {
        return CommandResult::failure("", "请输入命令");
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
        "note" | "nt" => handle_note(rest, &mut config),
        "denote" | "dnt" => handle_denote(rest, &mut config),

        // ========== 列表 & 搜索 ==========
        "list" | "ls" => handle_list(rest, &config),
        "contain" | "find" => handle_contain(rest, &config),

        // ========== 日报系统 ==========
        "report" | "r" => handle_report(rest),
        "reportctl" | "rctl" => handle_reportctl(rest),
        "check" | "c" => handle_check(rest),
        "search" | "select" | "look" | "sch" => handle_search(rest),

        // ========== 待办备忘 ==========
        "todo" | "td" => handle_todo(rest),

        // ========== AI 对话 ==========
        "chat" | "ai" => handle_chat(rest),

        // ========== 脚本 ==========
        "concat" => handle_concat(rest, &mut config),

        // ========== 计时器 ==========
        "time" => handle_time(rest),

        // ========== 系统设置 ==========
        "log" => handle_log(rest, &mut config),
        "change" | "chg" => handle_change(rest, &mut config),

        // ========== 更新 ==========
        "update" | "up" => handle_update(rest),

        // ========== 版本 & 帮助 ==========
        "version" | "v" => {
            CommandResult::success("version", format!("j-cli v{}", constants::VERSION))
        }
        "help" | "h" | "?" => handle_help(),

        // ========== 默认：尝试作为别名打开 ==========
        _ => handle_open_alias(&args, &config),
    }
}

// ========== 别名管理 ==========

fn handle_set(args: &[String], config: &mut YamlConfig) -> CommandResult {
    if args.len() < 2 {
        return CommandResult::failure("set", "用法: set <alias> <path>");
    }
    let alias_name = &args[0];
    let path = args[1..].join(" ");
    match alias::set_alias(alias_name, &path, config) {
        Ok(msg) => CommandResult::success("set", msg),
        Err(msg) => CommandResult::failure("set", msg),
    }
}

fn handle_remove(args: &[String], config: &mut YamlConfig) -> CommandResult {
    if args.is_empty() {
        return CommandResult::failure("remove", "用法: rm <alias>");
    }
    match alias::remove_alias(&args[0], config) {
        Ok(msg) => CommandResult::success("remove", msg),
        Err(msg) => CommandResult::failure("remove", msg),
    }
}

fn handle_rename(args: &[String], config: &mut YamlConfig) -> CommandResult {
    if args.len() < 2 {
        return CommandResult::failure("rename", "用法: rn <alias> <new_alias>");
    }
    match alias::rename_alias(&args[0], &args[1], config) {
        Ok(msg) => CommandResult::success("rename", msg),
        Err(msg) => CommandResult::failure("rename", msg),
    }
}

fn handle_modify(args: &[String], config: &mut YamlConfig) -> CommandResult {
    if args.len() < 2 {
        return CommandResult::failure("modify", "用法: mf <alias> <new_path>");
    }
    let path = args[1..].join(" ");
    match alias::modify_alias(&args[0], &path, config) {
        Ok(msg) => CommandResult::success("modify", msg),
        Err(msg) => CommandResult::failure("modify", msg),
    }
}

fn handle_note(args: &[String], config: &mut YamlConfig) -> CommandResult {
    if args.len() < 2 {
        return CommandResult::failure(
            "note",
            format!(
                "用法: note <alias> <category>\n可选分类: {}",
                constants::NOTE_CATEGORIES.join(", ")
            ),
        );
    }
    // 调用 category 模块的 handle_note
    crate::command::category::handle_note(&args[0], &args[1], config);
    CommandResult::success("note", format!("已将 {} 标记为 {}", args[0], args[1]))
}

fn handle_denote(args: &[String], config: &mut YamlConfig) -> CommandResult {
    if args.len() < 2 {
        return CommandResult::failure(
            "denote",
            format!(
                "用法: denote <alias> <category>\n可选分类: {}",
                constants::NOTE_CATEGORIES.join(", ")
            ),
        );
    }
    crate::command::category::handle_denote(&args[0], &args[1], config);
    CommandResult::success("denote", format!("已将 {} 从 {} 移除", args[0], args[1]))
}

// ========== 列表 & 搜索 ==========

fn handle_list(args: &[String], config: &YamlConfig) -> CommandResult {
    let query = if args.is_empty() { "" } else { &args[0] };

    // 处理特殊参数 "all"
    if !args.is_empty() && args[0] == "all" {
        let results = search::search_aliases("", config);
        return CommandResult::list("list", format!("共 {} 个别名", results.len()), results);
    }

    let results = search::search_aliases(query, config);
    if query.is_empty() {
        CommandResult::list("list", format!("共 {} 个别名", results.len()), results)
    } else {
        CommandResult::list("list", format!("找到 {} 个匹配", results.len()), results)
    }
}

fn handle_contain(args: &[String], config: &YamlConfig) -> CommandResult {
    if args.is_empty() {
        return CommandResult::failure("contain", "用法: contain <alias> [containers]");
    }

    let alias = &args[0];
    let containers = if args.len() > 1 {
        args[1]
            .split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>()
    } else {
        constants::DEFAULT_DISPLAY_SECTIONS
            .iter()
            .map(|s| s.to_string())
            .collect()
    };

    let mut found_in = Vec::new();
    for container in &containers {
        if config.contains(container, alias) {
            found_in.push(container.clone());
        }
    }

    if found_in.is_empty() {
        CommandResult::success("contain", format!("别名 {} 不在任何分类中", alias))
    } else {
        CommandResult::success(
            "contain",
            format!("别名 {} 在以下分类中: {}", alias, found_in.join(", ")),
        )
    }
}

// ========== 日报系统 ==========

fn handle_report(args: &[String]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::failure(
            "report",
            "用法: report <content>\n提示: 在 GUI 中暂不支持 TUI 编辑器，请直接输入内容",
        );
    }

    let content = args.join(" ");
    match crate::command::report::write_report(&content) {
        Ok(msg) => CommandResult::success("report", msg),
        Err(msg) => CommandResult::failure("report", msg),
    }
}

fn handle_reportctl(args: &[String]) -> CommandResult {
    if args.is_empty() {
        return CommandResult::failure(
            "reportctl",
            "用法: reportctl <action> [arg]\n操作: new / sync / push / pull",
        );
    }

    let action = &args[0];
    let arg = args.get(1).map(|s| s.as_str());

    match crate::command::report::handle_reportctl(action, arg) {
        Ok(msg) => CommandResult::success("reportctl", msg),
        Err(msg) => CommandResult::failure("reportctl", msg),
    }
}

fn handle_check(args: &[String]) -> CommandResult {
    let line_count = if args.is_empty() {
        5
    } else {
        args[0].parse().unwrap_or(5)
    };

    match crate::command::report::check_report(line_count) {
        Ok(content) => {
            if content.is_empty() {
                CommandResult::text("check", "日报为空", "")
            } else {
                CommandResult::text("check", format!("最近 {} 行日报", line_count), content)
            }
        }
        Err(msg) => CommandResult::failure("check", msg),
    }
}

fn handle_search(args: &[String]) -> CommandResult {
    if args.len() < 2 {
        return CommandResult::failure("search", "用法: search <line_count> <keyword> [-fuzzy]");
    }

    let line_count = args[0].parse().unwrap_or(10);
    let target = &args[1];
    let fuzzy = args.len() > 2 && (args[2] == "-f" || args[2] == "-fuzzy");

    match crate::command::report::search_report(line_count, target, fuzzy) {
        Ok(content) => {
            if content.is_empty() {
                CommandResult::success("search", format!("未找到匹配 '{}' 的内容", target))
            } else {
                CommandResult::text("search", format!("搜索 '{}' 结果", target), content)
            }
        }
        Err(msg) => CommandResult::failure("search", msg),
    }
}

// ========== 待办备忘 ==========

fn handle_todo(args: &[String]) -> CommandResult {
    if args.is_empty() {
        // 返回待办列表
        match crate::command::todo::list_todos() {
            Ok(todos) => {
                if todos.is_empty() {
                    CommandResult::success("todo", "暂无待办事项")
                } else {
                    let output = todos
                        .iter()
                        .enumerate()
                        .map(|(i, t)| format!("{}. {}", i + 1, t))
                        .collect::<Vec<_>>()
                        .join("\n");
                    CommandResult::text("todo", format!("共 {} 个待办", todos.len()), output)
                }
            }
            Err(msg) => CommandResult::failure("todo", msg),
        }
    } else {
        // 解析子命令
        match args[0].as_str() {
            "list" | "ls" => match crate::command::todo::list_todos() {
                Ok(todos) => {
                    if todos.is_empty() {
                        CommandResult::success("todo", "暂无待办事项")
                    } else {
                        let output = todos
                            .iter()
                            .enumerate()
                            .map(|(i, t)| format!("{}. {}", i + 1, t))
                            .collect::<Vec<_>>()
                            .join("\n");
                        CommandResult::text("todo", format!("共 {} 个待办", todos.len()), output)
                    }
                }
                Err(msg) => CommandResult::failure("todo", msg),
            },
            "add" | "a" => {
                if args.len() < 2 {
                    return CommandResult::failure("todo", "用法: todo add <content>");
                }
                let content = args[1..].join(" ");
                match crate::command::todo::add_todo(&content) {
                    Ok(msg) => CommandResult::success("todo", msg),
                    Err(msg) => CommandResult::failure("todo", msg),
                }
            }
            "done" | "d" | "complete" => {
                if args.len() < 2 {
                    return CommandResult::failure("todo", "用法: todo done <index>");
                }
                let index: usize = args[1].parse().unwrap_or(0);
                if index == 0 {
                    return CommandResult::failure("todo", "索引必须是正整数");
                }
                match crate::command::todo::complete_todo(index) {
                    Ok(msg) => CommandResult::success("todo", msg),
                    Err(msg) => CommandResult::failure("todo", msg),
                }
            }
            "remove" | "rm" | "delete" => {
                if args.len() < 2 {
                    return CommandResult::failure("todo", "用法: todo rm <index>");
                }
                let index: usize = args[1].parse().unwrap_or(0);
                if index == 0 {
                    return CommandResult::failure("todo", "索引必须是正整数");
                }
                match crate::command::todo::remove_todo(index) {
                    Ok(msg) => CommandResult::success("todo", msg),
                    Err(msg) => CommandResult::failure("todo", msg),
                }
            }
            _ => CommandResult::failure(
                "todo",
                format!("未知子命令: {}\n可用: list, add, done, rm", args[0]),
            ),
        }
    }
}

// ========== AI 对话 ==========

fn handle_chat(args: &[String]) -> CommandResult {
    // 检查是否有 --continue 或 -c 标志
    let has_continue = args.iter().any(|a| a == "--continue" || a == "-c");
    let has_session = args.iter().position(|a| a == "--session");

    // 过滤掉标志参数
    let content_args: Vec<String> = args
        .iter()
        .filter(|a| !["--continue", "-c"].contains(&a.as_str()) && a.as_str() != "--session")
        .filter(|a| {
            if let Some(idx) = has_session {
                // 排除 --session 及其参数
                return args.iter().position(|x| x == *a) != Some(idx)
                    && args.iter().position(|x| x == *a) != Some(idx + 1);
            }
            true
        })
        .map(|s| s.clone())
        .collect();

    if content_args.is_empty() {
        return CommandResult::failure(
            "chat",
            "用法: chat <message>\n提示: 在 GUI 中暂不支持 TUI 界面，请直接输入问题",
        );
    }

    let content = content_args.join(" ");
    let session_id = if let Some(idx) = has_session {
        args.get(idx + 1).map(|s| s.as_str())
    } else {
        None
    };

    match crate::command::chat::chat_oneshot(&content, has_continue, session_id) {
        Ok(response) => CommandResult::text("chat", "AI 回复", response),
        Err(msg) => CommandResult::failure("chat", msg),
    }
}

// ========== 脚本 ==========

fn handle_concat(args: &[String], config: &mut YamlConfig) -> CommandResult {
    if args.is_empty() {
        return CommandResult::failure(
            "concat",
            "用法: concat <name> [content]\n提示: 在 GUI 中暂不支持 TUI 编辑器",
        );
    }

    let name = &args[0];
    let content = if args.len() > 1 {
        args[1..].join(" ")
    } else {
        return CommandResult::failure(
            "concat",
            "用法: concat <name> <content>\n提示: 在 GUI 中必须提供脚本内容",
        );
    };

    crate::command::script::handle_concat_with_content(name, &content, config);
    CommandResult::success("concat", format!("脚本 {} 创建成功", name))
}

// ========== 计时器 ==========

fn handle_time(args: &[String]) -> CommandResult {
    if args.len() < 2 {
        return CommandResult::failure(
            "time",
            "用法: time countdown <duration>\n时长格式: 30s, 5m, 1h",
        );
    }

    let function = &args[0];
    if function != "countdown" {
        return CommandResult::failure("time", "目前仅支持 countdown 功能");
    }

    // GUI 中不支持倒计时进度条，返回提示
    let duration = &args[1];
    CommandResult::success(
        "time",
        format!(
            "倒计时 {} 已启动\n提示: 在 GUI 中不支持进度条显示，请使用终端运行 `j time countdown {}`",
            duration, duration
        ),
    )
}

// ========== 系统设置 ==========

fn handle_log(args: &[String], config: &mut YamlConfig) -> CommandResult {
    if args.len() < 2 {
        return CommandResult::failure(
            "log",
            "用法: log <key> <value>\n可用: log mode verbose/concise",
        );
    }

    crate::command::system::handle_log(&args[0], &args[1], config);
    CommandResult::success("log", format!("已设置 {} = {}", args[0], args[1]))
}

fn handle_change(args: &[String], config: &mut YamlConfig) -> CommandResult {
    if args.len() < 3 {
        return CommandResult::failure("change", "用法: change <section> <field> <value>");
    }

    crate::command::system::handle_change(&args[0], &args[1], &args[2], config);
    CommandResult::success(
        "change",
        format!("已修改 {}.{} = {}", args[0], args[1], args[2]),
    )
}

// ========== 更新 ==========

fn handle_update(args: &[String]) -> CommandResult {
    let check_only = args.contains(&"--check".to_string()) || args.contains(&"-c".to_string());

    if check_only {
        // 仅检查更新
        match crate::command::update::check_update() {
            Ok(msg) => CommandResult::success("update", msg),
            Err(msg) => CommandResult::failure("update", msg),
        }
    } else {
        // GUI 中不建议直接更新，给出提示
        CommandResult::success(
            "update",
            format!(
                "当前版本: v{}\n提示: 在 GUI 中暂不支持自动更新，请使用终端运行 `j update`",
                constants::VERSION
            ),
        )
    }
}

// ========== 帮助 ==========

fn handle_help() -> CommandResult {
    let help_text = get_help_text();
    CommandResult::text("help", "j-cli GUI 命令帮助", help_text)
}

// ========== 打开别名 ==========

fn handle_open_alias(args: &[String], config: &YamlConfig) -> CommandResult {
    match open::open_alias_silent(&args[0], args, config) {
        Ok(msg) => CommandResult::success("open", msg),
        Err(msg) => CommandResult::failure("open", msg),
    }
}

// ========== 工具函数 ==========

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
        "别名管理:",
        "  set <alias> <path>        添加别名",
        "  rm <alias>                删除别名",
        "  rn <old> <new>            重命名别名",
        "  mf <alias> <path>         修改别名路径",
        "  note <alias> <category>   标记分类",
        "  denote <alias> <category> 移除分类标记",
        "",
        "列表查询:",
        "  ls [filter]               列出别名",
        "  contain <alias>           查找别名所在分类",
        "",
        "日报系统:",
        "  report <content>          写入日报",
        "  check [n]                 查看最近 n 行",
        "  search <n> <keyword>      搜索日报",
        "  reportctl <action>        日报操作",
        "",
        "待办备忘:",
        "  todo list                 列出待办",
        "  todo add <content>        添加待办",
        "  todo done <index>         完成待办",
        "  todo rm <index>           删除待办",
        "",
        "AI 对话:",
        "  chat <message>            快速提问",
        "  chat -c <message>         延续会话",
        "",
        "其他:",
        "  concat <name> <content>   创建脚本",
        "  version                   版本信息",
        "  update --check            检查更新",
        "  <alias> [args...]         打开别名",
    ]
    .join("\n")
}
