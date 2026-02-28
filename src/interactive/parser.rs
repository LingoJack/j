use crate::cli::SubCmd;
use crate::command;
use crate::config::YamlConfig;
use crate::constants::cmd;

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
pub fn execute_interactive_command(args: &[String], config: &mut YamlConfig) {
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
            command::open::handle_open(args, config);
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
    } else if is(cmd::NOTE) {
        if rest.len() < 2 {
            crate::usage!("note <alias> <category>");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Note {
            alias: rest[0].clone(),
            category: rest[1].clone(),
        })
    } else if is(cmd::DENOTE) {
        if rest.len() < 2 {
            crate::usage!("denote <alias> <category>");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Denote {
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
    } else if is(cmd::CHANGE) {
        if rest.len() < 3 {
            crate::usage!("change <part> <field> <value>");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Change {
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
            crate::usage!("reportctl <new|sync|push|pull|set-url> [date|message|url]");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Reportctl {
            action: rest[0].clone(),
            arg: rest.get(1).cloned(),
        })
    } else if is(cmd::CHECK) {
        ParseResult::Matched(SubCmd::Check {
            line_count: rest.first().cloned(),
        })
    } else if is(cmd::SEARCH) {
        if rest.len() < 2 {
            crate::usage!("search <line_count|all> <target> [-f|-fuzzy]");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Search {
            line_count: rest[0].clone(),
            target: rest[1].clone(),
            fuzzy: rest.get(2).cloned(),
        })
    } else if is(cmd::TODO) {
        let list_flag = rest.iter().any(|s| s == "-l" || s == "--list");
        let content: Vec<String> = rest
            .iter()
            .filter(|s| *s != "-l" && *s != "--list")
            .cloned()
            .collect();
        ParseResult::Matched(SubCmd::Todo {
            list: list_flag,
            content,
        })
    } else if is(cmd::CHAT) {
        ParseResult::Matched(SubCmd::Chat {
            content: rest.to_vec(),
        })
    } else if is(cmd::CONCAT) {
        if rest.is_empty() {
            crate::usage!("concat <script_name> [\"<script_content>\"]");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Concat {
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
    } else if is(cmd::VOICE) {
        ParseResult::Matched(SubCmd::Voice {
            action: rest.first().cloned().unwrap_or_default(),
            copy: rest.contains(&"-c".to_string()),
            model: rest
                .iter()
                .position(|a| a == "-m" || a == "--model")
                .and_then(|i| rest.get(i + 1).cloned()),
        })
    } else {
        ParseResult::NotFound
    }
}
