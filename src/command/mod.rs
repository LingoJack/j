pub mod alias;
pub mod category;
pub mod list;
pub mod open;
pub mod report;
pub mod script;
pub mod system;
pub mod time;

use crate::cli::SubCmd;
use crate::config::YamlConfig;
use crate::constants;

/// 所有内置命令的关键字列表（用于判断别名冲突）
/// 统一从 constants::cmd 模块获取，避免多处重复定义
pub fn all_command_keywords() -> Vec<&'static str> {
    constants::cmd::all_keywords()
}

// ========== 命令执行结果 ==========

/// 命令执行结果
/// 用于支持管道和多命令组合
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// 成功完成，可选输出内容
    Success {
        output: Option<String>,
    },
    /// 执行失败，带错误信息
    Error {
        message: String,
    },
    /// 需要退出交互模式
    Exit,
}

impl CommandResult {
    /// 创建成功结果（无输出）
    pub fn ok() -> Self {
        CommandResult::Success { output: None }
    }

    /// 创建成功结果（带输出）
    pub fn with_output(output: impl Into<String>) -> Self {
        CommandResult::Success {
            output: Some(output.into()),
        }
    }

    /// 创建错误结果
    pub fn error(message: impl Into<String>) -> Self {
        CommandResult::Error {
            message: message.into(),
        }
    }

    /// 是否成功
    pub fn is_success(&self) -> bool {
        matches!(self, CommandResult::Success { .. })
    }

    /// 是否有输出
    pub fn has_output(&self) -> bool {
        matches!(self, CommandResult::Success { output: Some(_) })
    }

    /// 获取输出内容
    pub fn output(&self) -> Option<&str> {
        match self {
            CommandResult::Success { output } => output.as_deref(),
            _ => None,
        }
    }

    /// 获取错误信息
    pub fn error_message(&self) -> Option<&str> {
        match self {
            CommandResult::Error { message } => Some(message),
            _ => None,
        }
    }
}

// ========== 管道和多命令解析 ==========

/// 命令链中的单个命令片段
#[derive(Debug, Clone)]
pub struct CommandSegment {
    /// 命令文本
    pub text: String,
    /// 是否是 shell 命令（以 ! 开头）
    pub is_shell: bool,
}

/// 操作符类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Operator {
    /// 管道：前一命令的 stdout 传给后一命令的 stdin
    Pipe,
    /// 逻辑与：前一命令成功才执行后一命令
    And,
    /// 分号：无论前一命令结果，都执行后一命令
    Sequential,
}

/// 解析后的命令链
#[derive(Debug, Clone)]
pub struct CommandChain {
    /// 命令片段列表
    pub segments: Vec<CommandSegment>,
    /// 操作符列表（segments.len() - 1 个）
    pub operators: Vec<Operator>,
}

impl CommandChain {
    /// 解析输入为命令链
    /// 支持：|（管道）、&&（逻辑与）、;（顺序执行）
    pub fn parse(input: &str) -> Self {
        let mut segments = Vec::new();
        let mut operators = Vec::new();

        // 状态机解析
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;
        let len = chars.len();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';

        while i < len {
            let ch = chars[i];

            // 引号处理
            if (ch == '"' || ch == '\'') && !in_quotes {
                in_quotes = true;
                quote_char = ch;
                current.push(ch);
                i += 1;
                continue;
            }
            if in_quotes && ch == quote_char {
                in_quotes = false;
                current.push(ch);
                i += 1;
                continue;
            }

            // 引号内不解析操作符
            if in_quotes {
                current.push(ch);
                i += 1;
                continue;
            }

            // 检测操作符
            if ch == '|' && i + 1 < len && chars[i + 1] == '|' {
                // || 不支持，跳过
                current.push(ch);
                i += 1;
                continue;
            }

            if ch == '|' {
                // 管道 |
                if !current.trim().is_empty() {
                    let trimmed = current.trim();
                    segments.push(CommandSegment {
                        text: trimmed.to_string(),
                        is_shell: trimmed.starts_with('!'),
                    });
                }
                operators.push(Operator::Pipe);
                current.clear();
                i += 1;
                continue;
            }

            if ch == '&' && i + 1 < len && chars[i + 1] == '&' {
                // 逻辑与 &&
                if !current.trim().is_empty() {
                    let trimmed = current.trim();
                    segments.push(CommandSegment {
                        text: trimmed.to_string(),
                        is_shell: trimmed.starts_with('!'),
                    });
                }
                operators.push(Operator::And);
                current.clear();
                i += 2;
                continue;
            }

            if ch == ';' {
                // 顺序执行 ;
                if !current.trim().is_empty() {
                    let trimmed = current.trim();
                    segments.push(CommandSegment {
                        text: trimmed.to_string(),
                        is_shell: trimmed.starts_with('!'),
                    });
                }
                operators.push(Operator::Sequential);
                current.clear();
                i += 1;
                continue;
            }

            current.push(ch);
            i += 1;
        }

        // 添加最后一个片段
        if !current.trim().is_empty() {
            let trimmed = current.trim();
            segments.push(CommandSegment {
                text: trimmed.to_string(),
                is_shell: trimmed.starts_with('!'),
            });
        }

        CommandChain {
            segments,
            operators,
        }
    }

    /// 是否是多命令/管道
    pub fn is_chain(&self) -> bool {
        !self.operators.is_empty()
    }

    /// 是否只有单个命令
    pub fn is_single(&self) -> bool {
        self.operators.is_empty() && self.segments.len() == 1
    }

    /// 获取单个命令（如果不是单个命令返回 None）
    pub fn single_command(&self) -> Option<&str> {
        if self.is_single() {
            self.segments.first().map(|s| s.text.as_str())
        } else {
            None
        }
    }
}

/// 执行命令链
/// 返回最后一个命令的结果
pub fn execute_chain(chain: &CommandChain, config: &mut YamlConfig) -> CommandResult {
    if chain.segments.is_empty() {
        return CommandResult::ok();
    }

    // 单个命令直接执行
    if chain.is_single() {
        let segment = &chain.segments[0];
        if segment.is_shell {
            // Shell 命令
            let shell_cmd = segment.text.trim_start_matches('!').trim();
            execute_shell_in_chain(shell_cmd, None, config)
        } else {
            // j 内部命令
            execute_j_command(&segment.text, None, config)
        }
    } else {
        // 多命令链
        let mut last_output: Option<String> = None;
        let mut last_success = true;

        for (i, segment) in chain.segments.iter().enumerate() {
            // 检查操作符条件
            if i > 0 {
                let op = chain.operators.get(i - 1);
                match op {
                    Some(Operator::And) if !last_success => {
                        // && 但前一命令失败，跳过
                        break;
                    }
                    Some(Operator::Pipe) => {
                        // 管道：前一命令的输出作为 stdin
                    }
                    Some(Operator::Sequential) | None => {
                        // ; 或第一个命令：正常执行
                    }
                    _ => {}
                }
            }

            let result = if segment.is_shell {
                let shell_cmd = segment.text.trim_start_matches('!').trim();
                execute_shell_in_chain(shell_cmd, last_output.as_deref(), config)
            } else {
                execute_j_command(&segment.text, last_output.as_deref(), config)
            };

            // 更新状态
            last_success = result.is_success();

            // 管道：传递输出
            let op = chain.operators.get(i);
            if op == Some(&Operator::Pipe) {
                last_output = result.output().map(|s| s.to_string());
            } else {
                last_output = result.output().map(|s| s.to_string());
            }

            // 最后一个命令的结果直接返回
            if i == chain.segments.len() - 1 {
                return result;
            }
        }

        CommandResult::ok()
    }
}

/// 执行 j 内部命令
fn execute_j_command(input: &str, stdin: Option<&str>, config: &mut YamlConfig) -> CommandResult {
    // 如果有 stdin 输入，追加到命令参数中
    let input = if let Some(stdin_data) = stdin {
        format!("{} {}", input, stdin_data)
    } else {
        input.to_string()
    };

    // 解析参数
    let args = parse_command_args(&input);
    if args.is_empty() {
        return CommandResult::ok();
    }

    // 展开环境变量
    let args: Vec<String> = args.iter().map(|a| expand_env_vars(a)).collect();

    // 解析为 SubCmd 并执行
    match parse_to_subcmd(&args) {
        ParsedCommand::Matched(subcmd) => dispatch_with_result(subcmd, config),
        ParsedCommand::Handled => CommandResult::ok(),
        ParsedCommand::Exit => CommandResult::Exit,
        ParsedCommand::NotFound => {
            // 尝试作为别名打开
            dispatch_open_with_result(&args, config)
        }
    }
}

/// 执行 shell 命令（支持管道输入）
fn execute_shell_in_chain(cmd: &str, stdin: Option<&str>, config: &YamlConfig) -> CommandResult {
    use std::process::{Command, Stdio};

    let os = std::env::consts::OS;
    let mut command = if os == "windows" {
        let mut c = Command::new("cmd");
        c.args(["/c", cmd]);
        c
    } else {
        let mut c = Command::new("/bin/sh");
        c.args(["-c", cmd]);
        c
    };

    // 注入别名环境变量
    for (key, value) in config.collect_alias_envs() {
        command.env(&key, &value);
    }

    // 设置 stdin
    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    match command.spawn() {
        Ok(mut child) => {
            // 写入 stdin
            if let Some(stdin_data) = stdin {
                use std::io::Write;
                if let Some(mut child_stdin) = child.stdin.take() {
                    let _ = child_stdin.write_all(stdin_data.as_bytes());
                }
            }

            match child.wait_with_output() {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                    if output.status.success() {
                        if stdout.is_empty() {
                            CommandResult::ok()
                        } else {
                            // 输出到终端并返回
                            print!("{}", stdout);
                            CommandResult::with_output(stdout)
                        }
                    } else {
                        if !stderr.is_empty() {
                            eprint!("{}", stderr);
                        }
                        CommandResult::error(format!("退出码: {}", output.status))
                    }
                }
                Err(e) => CommandResult::error(format!("等待命令完成失败: {}", e)),
            }
        }
        Err(e) => CommandResult::error(format!("执行 shell 命令失败: {}", e)),
    }
}

/// 解析命令参数（支持引号）
fn parse_command_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    for ch in input.chars() {
        if !in_quotes && (ch == '"' || ch == '\'') {
            in_quotes = true;
            quote_char = ch;
            continue;
        }
        if in_quotes && ch == quote_char {
            in_quotes = false;
            continue;
        }
        if !in_quotes && ch == ' ' {
            if !current.is_empty() {
                args.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}

/// 展开环境变量
fn expand_env_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '$' && i + 1 < len {
            // ${VAR_NAME} 格式
            if chars[i + 1] == '{' {
                if let Some(end) = chars[i + 2..].iter().position(|&c| c == '}') {
                    let var_name: String = chars[i + 2..i + 2 + end].iter().collect();
                    if let Ok(val) = std::env::var(&var_name) {
                        result.push_str(&val);
                    } else {
                        result.push_str(&input[i..i + 3 + end]);
                    }
                    i = i + 3 + end;
                    continue;
                }
            }
            // $VAR_NAME 格式
            let start = i + 1;
            let mut end = start;
            while end < len && (chars[end].is_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            if end > start {
                let var_name: String = chars[start..end].iter().collect();
                if let Ok(val) = std::env::var(&var_name) {
                    result.push_str(&val);
                } else {
                    let original: String = chars[i..end].iter().collect();
                    result.push_str(&original);
                }
                i = end;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

// ========== 子命令解析（从 interactive.rs 复用） ==========

/// 解析结果
enum ParsedCommand {
    Matched(SubCmd),
    Handled,
    Exit,
    NotFound,
}

/// 从参数解析为 SubCmd
fn parse_to_subcmd(args: &[String]) -> ParsedCommand {
    use crate::constants::cmd;

    if args.is_empty() {
        return ParsedCommand::NotFound;
    }

    let cmd = args[0].as_str();
    let rest = &args[1..];

    let is = |names: &[&str]| names.contains(&cmd);

    // 退出命令
    if is(cmd::EXIT) {
        return ParsedCommand::Exit;
    }

    if is(cmd::SET) {
        if rest.is_empty() {
            return ParsedCommand::Handled;
        }
        ParsedCommand::Matched(SubCmd::Set {
            alias: rest[0].clone(),
            path: rest[1..].to_vec(),
        })
    } else if is(cmd::REMOVE) {
        match rest.first() {
            Some(alias) => ParsedCommand::Matched(SubCmd::Remove { alias: alias.clone() }),
            None => ParsedCommand::Handled,
        }
    } else if is(cmd::RENAME) {
        if rest.len() < 2 {
            return ParsedCommand::Handled;
        }
        ParsedCommand::Matched(SubCmd::Rename {
            alias: rest[0].clone(),
            new_alias: rest[1].clone(),
        })
    } else if is(cmd::MODIFY) {
        if rest.is_empty() {
            return ParsedCommand::Handled;
        }
        ParsedCommand::Matched(SubCmd::Modify {
            alias: rest[0].clone(),
            path: rest[1..].to_vec(),
        })
    } else if is(cmd::NOTE) {
        if rest.len() < 2 {
            return ParsedCommand::Handled;
        }
        ParsedCommand::Matched(SubCmd::Note {
            alias: rest[0].clone(),
            category: rest[1].clone(),
        })
    } else if is(cmd::DENOTE) {
        if rest.len() < 2 {
            return ParsedCommand::Handled;
        }
        ParsedCommand::Matched(SubCmd::Denote {
            alias: rest[0].clone(),
            category: rest[1].clone(),
        })
    } else if is(cmd::LIST) {
        ParsedCommand::Matched(SubCmd::List {
            part: rest.first().cloned(),
        })
    } else if is(cmd::CONTAIN) {
        if rest.is_empty() {
            return ParsedCommand::Handled;
        }
        ParsedCommand::Matched(SubCmd::Contain {
            alias: rest[0].clone(),
            containers: rest.get(1).cloned(),
        })
    } else if is(cmd::LOG) {
        if rest.len() < 2 {
            return ParsedCommand::Handled;
        }
        ParsedCommand::Matched(SubCmd::Log {
            key: rest[0].clone(),
            value: rest[1].clone(),
        })
    } else if is(cmd::CHANGE) {
        if rest.len() < 3 {
            return ParsedCommand::Handled;
        }
        ParsedCommand::Matched(SubCmd::Change {
            part: rest[0].clone(),
            field: rest[1].clone(),
            value: rest[2].clone(),
        })
    } else if is(cmd::CLEAR) {
        ParsedCommand::Matched(SubCmd::Clear)
    } else if is(cmd::REPORT) {
        ParsedCommand::Matched(SubCmd::Report {
            content: rest.to_vec(),
        })
    } else if is(cmd::REPORTCTL) {
        if rest.is_empty() {
            return ParsedCommand::Handled;
        }
        ParsedCommand::Matched(SubCmd::Reportctl {
            action: rest[0].clone(),
            arg: rest.get(1).cloned(),
        })
    } else if is(cmd::CHECK) {
        ParsedCommand::Matched(SubCmd::Check {
            line_count: rest.first().cloned(),
        })
    } else if is(cmd::SEARCH) {
        if rest.len() < 2 {
            return ParsedCommand::Handled;
        }
        ParsedCommand::Matched(SubCmd::Search {
            line_count: rest[0].clone(),
            target: rest[1].clone(),
            fuzzy: rest.get(2).cloned(),
        })
    } else if is(cmd::CONCAT) {
        if rest.is_empty() {
            return ParsedCommand::Handled;
        }
        ParsedCommand::Matched(SubCmd::Concat {
            name: rest[0].clone(),
            content: if rest.len() > 1 { rest[1..].to_vec() } else { vec![] },
        })
    } else if is(cmd::TIME) {
        if rest.len() < 2 {
            return ParsedCommand::Handled;
        }
        ParsedCommand::Matched(SubCmd::Time {
            function: rest[0].clone(),
            arg: rest[1].clone(),
        })
    } else if is(cmd::VERSION) {
        ParsedCommand::Matched(SubCmd::Version)
    } else if is(cmd::HELP) {
        ParsedCommand::Matched(SubCmd::Help)
    } else if is(cmd::COMPLETION) {
        ParsedCommand::Matched(SubCmd::Completion {
            shell: rest.first().cloned(),
        })
    } else {
        ParsedCommand::NotFound
    }
}

// ========== 带结果返回的分发函数 ==========

/// 分发命令并返回结果（用于管道支持）
pub fn dispatch_with_result(subcmd: SubCmd, config: &mut YamlConfig) -> CommandResult {
    match subcmd {
        // 别名管理
        SubCmd::Set { alias, path } => alias::handle_set_with_result(&alias, &path, config),
        SubCmd::Remove { alias } => alias::handle_remove_with_result(&alias, config),
        SubCmd::Rename { alias, new_alias } => alias::handle_rename_with_result(&alias, &new_alias, config),
        SubCmd::Modify { alias, path } => alias::handle_modify_with_result(&alias, &path, config),

        // 分类标记
        SubCmd::Note { alias, category } => category::handle_note_with_result(&alias, &category, config),
        SubCmd::Denote { alias, category } => category::handle_denote_with_result(&alias, &category, config),

        // 列表 & 查找
        SubCmd::List { part } => list::handle_list_with_result(part.as_deref(), config),
        SubCmd::Contain { alias, containers } => system::handle_contain_with_result(&alias, containers.as_deref(), config),

        // 日报系统
        SubCmd::Report { content } => report::handle_report_with_result("report", &content, config),
        SubCmd::Reportctl { action, arg } => {
            let mut args = vec![action];
            if let Some(a) = arg {
                args.push(a);
            }
            report::handle_report_with_result("reportctl", &args, config)
        }
        SubCmd::Check { line_count } => report::handle_check_with_result(line_count.as_deref(), config),
        SubCmd::Search { line_count, target, fuzzy } => {
            report::handle_search_with_result(&line_count, &target, fuzzy.as_deref(), config)
        }

        // 脚本
        SubCmd::Concat { name, content } => script::handle_concat_with_result(&name, &content, config),

        // 倒计时
        SubCmd::Time { function, arg } => time::handle_time_with_result(&function, &arg),

        // 系统设置
        SubCmd::Log { key, value } => system::handle_log_with_result(&key, &value, config),
        SubCmd::Change { part, field, value } => system::handle_change_with_result(&part, &field, &value, config),
        SubCmd::Clear => system::handle_clear_with_result(),

        // 系统信息
        SubCmd::Version => system::handle_version_with_result(config),
        SubCmd::Help => system::handle_help_with_result(),
        SubCmd::Exit => CommandResult::Exit,
        SubCmd::Completion { shell } => system::handle_completion_with_result(shell.as_deref(), config),
    }
}

/// 打开别名并返回结果
fn dispatch_open_with_result(args: &[String], config: &YamlConfig) -> CommandResult {
    open::handle_open_with_result(args, config)
}

// ========== 原有分发函数（保持向后兼容） ==========

/// 命令分发执行
pub fn dispatch(subcmd: SubCmd, config: &mut YamlConfig) {
    let result = dispatch_with_result(subcmd, config);
    // 输出结果
    output_result(&result);
}

/// 输出命令结果（Markdown 渲染）
pub fn output_result(result: &CommandResult) {
    match result {
        CommandResult::Success { output } => {
            if let Some(text) = output {
                // 使用 Markdown 渲染输出
                crate::md!("{}", text);
            }
        }
        CommandResult::Error { message } => {
            crate::error!("{}", message);
        }
        CommandResult::Exit => {
            // 退出由调用方处理
        }
    }
}
