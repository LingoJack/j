/// 默认远程连接端口
const DEFAULT_REMOTE_PORT: u16 = 9390;

use crate::cli::SubCmd;
use crate::command;
use crate::config::YamlConfig;
use crate::constants::cmd;
use crate::info;
use colored::Colorize;

/// 交互命令解析结果（三态）
pub enum ParseResult {
    /// 成功解析为内置命令
    Matched(SubCmd),
    /// 是内置命令但参数不足，已打印 usage 提示
    Handled,
    /// 不是内置命令
    NotFound,
}

/// 在交互模式下执行命令
pub fn execute_interactive_command(
    args: &[String],
    raw_input: &str,
    config: &mut YamlConfig,
    shell_session: &mut Option<super::shell::ShellSession>,
) {
    if args.is_empty() {
        return;
    }

    let cmd_str = &args[0];

    if cmd::EXIT.contains(&cmd_str.as_str()) {
        command::system::handle_exit();
        return;
    }

    match parse_interactive_command(args) {
        ParseResult::Matched(subcmd) => {
            command::dispatch(subcmd, config);
        }
        ParseResult::Handled => {}
        ParseResult::NotFound => {
            let matched = command::open::handle_open(args, config);
            if !matched {
                info!(
                    "{} {}",
                    "(shell)".green(),
                    super::shell::highlight_shell_command(raw_input)
                );
                if shell_session.is_none() {
                    *shell_session = super::shell::ShellSession::new(config);
                }
                if let Some(session) = shell_session.as_mut() {
                    if !session.execute(raw_input) {
                        *shell_session = super::shell::ShellSession::new(config);
                    }
                } else {
                    let _ = super::shell::execute_shell_once(raw_input, config);
                }
            }
        }
    }
}

/// 从交互模式输入的参数解析出 SubCmd
pub fn parse_interactive_command(args: &[String]) -> ParseResult {
    if args.is_empty() {
        return ParseResult::NotFound;
    }

    let cmd = args[0].as_str();
    let rest = &args[1..];

    let is = |names: &[&str]| names.contains(&cmd);

    if is(cmd::SET) {
        if rest.is_empty() {
            crate::usage!("set <alias> <path>");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Set {
            alias: rest[0].clone(),
            path: rest[1..].to_vec(),
        })
    } else if is(cmd::REMOVE) {
        match rest.first() {
            Some(alias) => ParseResult::Matched(SubCmd::Remove {
                alias: alias.clone(),
            }),
            None => {
                crate::usage!("rm <alias>");
                ParseResult::Handled
            }
        }
    } else if is(cmd::RENAME) {
        if rest.len() < 2 {
            crate::usage!("rename <alias> <new_alias>");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Rename {
            alias: rest[0].clone(),
            new_alias: rest[1].clone(),
        })
    } else if is(cmd::MODIFY) {
        if rest.is_empty() {
            crate::usage!("mf <alias> <new_path>");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Modify {
            alias: rest[0].clone(),
            path: rest[1..].to_vec(),
        })
    } else if is(cmd::TAG) {
        if rest.len() < 2 {
            crate::usage!("tag <alias> <category>");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Tag {
            alias: rest[0].clone(),
            category: rest[1].clone(),
        })
    } else if is(cmd::UNTAG) {
        if rest.len() < 2 {
            crate::usage!("untag <alias> <category>");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Untag {
            alias: rest[0].clone(),
            category: rest[1].clone(),
        })
    } else if is(cmd::LIST) {
        ParseResult::Matched(SubCmd::List {
            part: rest.first().cloned(),
        })
    } else if is(cmd::CONTAIN) {
        if rest.is_empty() {
            crate::usage!("contain <alias> [sections]");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Contain {
            alias: rest[0].clone(),
            containers: rest.get(1).cloned(),
        })
    } else if is(cmd::LOG) {
        if rest.len() < 2 {
            crate::usage!("log mode <verbose|concise>");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Log {
            key: rest[0].clone(),
            value: rest[1].clone(),
        })
    } else if is(cmd::CONFIG) {
        if rest.len() < 3 {
            crate::usage!("config <part> <field> <value>");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Config {
            part: rest[0].clone(),
            field: rest[1].clone(),
            value: rest[2].clone(),
        })
    } else if is(cmd::CLEAR) {
        ParseResult::Matched(SubCmd::Clear)
    } else if is(cmd::REPORT) {
        ParseResult::Matched(SubCmd::Report {
            content: rest.to_vec(),
        })
    } else if is(cmd::REPORTCTL) {
        if rest.is_empty() {
            crate::usage!("reportctl <new|sync|push|pull|set-url|use> [date|message|url|backend]");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Reportctl {
            action: rest[0].clone(),
            args: rest[1..].to_vec(),
        })
    } else if is(cmd::CHECK) {
        ParseResult::Matched(SubCmd::Check {
            line_count: rest.first().cloned(),
        })
    } else if is(cmd::SEARCH) {
        if rest.len() < 2 {
            crate::usage!("search <line_count|all> <target...> [-f|--fuzzy]");
            return ParseResult::Handled;
        }
        let mut is_fuzzy = false;
        let mut target_parts: Vec<String> = Vec::new();
        for part in &rest[1..] {
            if part == "-f" || part == "--fuzzy" {
                is_fuzzy = true;
            } else {
                target_parts.push(part.clone());
            }
        }
        if target_parts.is_empty() {
            crate::usage!("search <line_count|all> <target...> [-f|--fuzzy]");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Search {
            line_count: rest[0].clone(),
            target: target_parts,
            fuzzy: is_fuzzy,
        })
    } else if is(cmd::TODO) {
        ParseResult::Matched(SubCmd::Todo {
            content: rest.to_vec(),
        })
    } else if is(cmd::CHAT) {
        let mut cont = false;
        let mut session: Option<String> = None;
        let mut remote = false;
        let mut port: u16 = DEFAULT_REMOTE_PORT;
        let mut bypass = false;
        let mut no_render = false;
        let mut content: Vec<String> = Vec::new();
        let mut i = 0;
        while i < rest.len() {
            match rest[i].as_str() {
                "--continue" | "-c" => {
                    cont = true;
                    i += 1;
                }
                "--session" => {
                    i += 1;
                    if i < rest.len() {
                        session = Some(rest[i].clone());
                        i += 1;
                    }
                }
                "--remote" => {
                    remote = true;
                    i += 1;
                }
                "--port" => {
                    i += 1;
                    if i < rest.len() {
                        port = rest[i].parse().unwrap_or(DEFAULT_REMOTE_PORT);
                        i += 1;
                    }
                }
                "--bypass" => {
                    bypass = true;
                    i += 1;
                }
                "--no-render" => {
                    no_render = true;
                    i += 1;
                }
                _ => {
                    content.push(rest[i].clone());
                    i += 1;
                }
            }
        }
        ParseResult::Matched(SubCmd::Chat {
            cont,
            session,
            content,
            remote,
            port,
            bypass,
            no_render,
        })
    } else if is(cmd::SCRIPT) {
        if rest.is_empty() {
            crate::usage!("script <script_name> [\"<script_content>\"]");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Script {
            name: rest[0].clone(),
            content: if rest.len() > 1 {
                rest[1..].to_vec()
            } else {
                vec![]
            },
        })
    } else if is(cmd::TIME) {
        if rest.len() < 2 {
            crate::usage!("time countdown <duration>");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Time {
            function: rest[0].clone(),
            arg: rest[1].clone(),
        })
    } else if is(cmd::VERSION) {
        ParseResult::Matched(SubCmd::Version)
    } else if is(cmd::HELP) {
        ParseResult::Matched(SubCmd::Help)
    } else if is(cmd::COMPLETION) {
        ParseResult::Matched(SubCmd::Completion {
            shell: rest.first().cloned(),
        })
    } else if is(cmd::UPDATE) {
        ParseResult::Matched(SubCmd::Update {
            check: rest
                .first()
                .map(|s| s == "--check" || s == "-c")
                .unwrap_or(false),
            interactive: true,
        })
    } else if is(cmd::MD) {
        ParseResult::Matched(SubCmd::Md {
            args: rest.to_vec(),
        })
    } else if is(cmd::NOTEBOOK) {
        ParseResult::Matched(SubCmd::Notebook {
            args: rest.to_vec(),
        })
    } else if is(cmd::LOCK) {
        if rest.is_empty() {
            crate::usage!("lock <password> [file_or_dir]");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Lock {
            password: rest[0].clone(),
            target: rest.get(1).cloned(),
        })
    } else if is(cmd::UNLOCK) {
        if rest.is_empty() {
            crate::usage!("unlock <password> [file_or_dir]");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Unlock {
            password: rest[0].clone(),
            target: rest.get(1).cloned(),
        })
    } else if is(cmd::READ) {
        let mut file_path: Option<String> = None;
        for arg in rest {
            if file_path.is_none() {
                file_path = Some(arg.to_string());
            } else {
                crate::error!("✖️ read 命令仅接受一个文件或目录路径，收到额外参数：{arg}");
                return ParseResult::Handled;
            }
        }
        let Some(path) = file_path else {
            crate::usage!("read <file_or_dir_path>");
            return ParseResult::Handled;
        };
        ParseResult::Matched(SubCmd::Read { file_path: path })
    } else {
        ParseResult::NotFound
    }
}
