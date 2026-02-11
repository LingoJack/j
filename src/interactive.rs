use crate::command;
use crate::config::YamlConfig;
use crate::constants::{self, config_key, NOTE_CATEGORIES, ALL_SECTIONS, ALIAS_PATH_SECTIONS};
use crate::{info, error};
use colored::Colorize;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::highlight::CmdKind;
use rustyline::{Cmd, CompletionType, Config, Context, EditMode, Editor, EventHandler, KeyCode, KeyEvent, Modifiers};
use std::borrow::Cow;

// ========== 补全器定义 ==========

/// 自定义补全器：根据上下文提供命令、别名、分类等补全
struct CopilotCompleter {
    config: YamlConfig,
}

impl CopilotCompleter {
    fn new(config: &YamlConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// 刷新配置（别名可能在交互过程中发生变化）
    fn refresh(&mut self, config: &YamlConfig) {
        self.config = config.clone();
    }

    /// 获取所有别名列表（用于补全）
    fn all_aliases(&self) -> Vec<String> {
        let mut aliases = Vec::new();
        for s in ALIAS_PATH_SECTIONS {
            if let Some(map) = self.config.get_section(s) {
                aliases.extend(map.keys().cloned());
            }
        }
        aliases.sort();
        aliases.dedup();
        aliases
    }

    /// 所有 section 名称（用于 ls / change 等补全）
    fn all_sections(&self) -> Vec<String> {
        self.config
            .all_section_names()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// 指定 section 下的所有 key（用于 change 第三个参数补全）
    fn section_keys(&self, section: &str) -> Vec<String> {
        self.config
            .get_section(section)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// 命令定义：(命令名列表, 参数位置补全策略)
/// 参数位置策略: Alias = 别名补全, Category = 分类补全, Section = section补全, File = 文件路径提示, Fixed = 固定选项
#[derive(Clone)]
#[allow(dead_code)]
enum ArgHint {
    Alias,
    Category,
    Section,
    SectionKeys(String), // 依赖上一个参数的 section 名
    Fixed(Vec<&'static str>),
    Placeholder(&'static str),
    None,
}

/// 获取命令的补全规则定义
fn command_completion_rules() -> Vec<(&'static [&'static str], Vec<ArgHint>)> {
    vec![
        // 别名管理
        (&["set", "s"], vec![ArgHint::Placeholder("<alias>"), ArgHint::Placeholder("<path>")]),
        (&["rm", "remove"], vec![ArgHint::Alias]),
        (&["rename", "rn"], vec![ArgHint::Alias, ArgHint::Placeholder("<new_alias>")]),
        (&["mf", "modify"], vec![ArgHint::Alias, ArgHint::Placeholder("<new_path>")]),
        // 分类
        (&["note", "nt"], vec![ArgHint::Alias, ArgHint::Category]),
        (&["denote", "dnt"], vec![ArgHint::Alias, ArgHint::Category]),
        // 列表
        (&["ls", "list"], vec![ArgHint::Fixed({
            let mut v = vec!["", "all"];
            for s in ALL_SECTIONS { v.push(s); }
            v
        })]),
        // 查找
        (&["contain", "find"], vec![ArgHint::Alias, ArgHint::Placeholder("<sections>")]),
        // 系统设置
        (&["log"], vec![ArgHint::Fixed(vec![config_key::MODE]), ArgHint::Fixed(vec![config_key::VERBOSE, config_key::CONCISE])]),
        (&["change", "chg"], vec![ArgHint::Section, ArgHint::Placeholder("<field>"), ArgHint::Placeholder("<value>")]),
        // 日报系统
        (&["report", "r"], vec![ArgHint::Placeholder("<content>")]),
        (&["r-meta"], vec![ArgHint::Fixed(vec!["new", "sync"]), ArgHint::Placeholder("<date>")]),
        (&["check", "c"], vec![ArgHint::Placeholder("<line_count>")]),
        (&["search", "select", "look", "sch"], vec![ArgHint::Placeholder("<line_count|all>"), ArgHint::Placeholder("<target>"), ArgHint::Fixed(vec!["-f", "-fuzzy"])]),
        // 脚本
        (&["concat"], vec![ArgHint::Placeholder("<script_name>"), ArgHint::Placeholder("<script_content>")]),
        // 倒计时
        (&["time"], vec![ArgHint::Fixed(vec!["countdown"]), ArgHint::Placeholder("<duration>")]),
        // 系统信息
        (&["version", "v"], vec![]),
        (&["help", "h"], vec![]),
        (&["clear", "cls"], vec![]),
        (&["exit", "q", "quit"], vec![]),
    ]
}

/// 分类常量（引用全局常量）
const ALL_NOTE_CATEGORIES: &[&str] = NOTE_CATEGORIES;

impl Completer for CopilotCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line_to_cursor = &line[..pos];
        let parts: Vec<&str> = line_to_cursor.split_whitespace().collect();

        // 判断光标处是否在空格之后（即准备输入新 token）
        let trailing_space = line_to_cursor.ends_with(' ');
        let word_index = if trailing_space {
            parts.len()
        } else {
            parts.len().saturating_sub(1)
        };

        let current_word = if trailing_space {
            ""
        } else {
            parts.last().copied().unwrap_or("")
        };

        let start_pos = pos - current_word.len();

        if word_index == 0 {
            // 第一个词：补全命令名 + 别名
            let mut candidates = Vec::new();

            // 内置命令
            let rules = command_completion_rules();
            for (names, _) in &rules {
                for name in *names {
                    if name.starts_with(current_word) {
                        candidates.push(Pair {
                            display: name.to_string(),
                            replacement: name.to_string(),
                        });
                    }
                }
            }

            // 别名（用于 j <alias> 直接打开）
            for alias in self.all_aliases() {
                if alias.starts_with(current_word) && !command::all_command_keywords().contains(&alias.as_str()) {
                    candidates.push(Pair {
                        display: alias.clone(),
                        replacement: alias,
                    });
                }
            }

            return Ok((start_pos, candidates));
        }

        // 后续参数：根据第一个词确定补全策略
        let cmd = parts[0];
        let rules = command_completion_rules();

        for (names, arg_hints) in &rules {
            if names.contains(&cmd) {
                let arg_index = word_index - 1; // 减去命令本身
                if arg_index < arg_hints.len() {
                    let candidates = match &arg_hints[arg_index] {
                        ArgHint::Alias => {
                            self.all_aliases()
                                .into_iter()
                                .filter(|a| a.starts_with(current_word))
                                .map(|a| Pair { display: a.clone(), replacement: a })
                                .collect()
                        }
                        ArgHint::Category => {
                            ALL_NOTE_CATEGORIES
                                .iter()
                                .filter(|c| c.starts_with(current_word))
                                .map(|c| Pair { display: c.to_string(), replacement: c.to_string() })
                                .collect()
                        }
                        ArgHint::Section => {
                            self.all_sections()
                                .into_iter()
                                .filter(|s| s.starts_with(current_word))
                                .map(|s| Pair { display: s.clone(), replacement: s })
                                .collect()
                        }
                        ArgHint::SectionKeys(section) => {
                            self.section_keys(section)
                                .into_iter()
                                .filter(|k| k.starts_with(current_word))
                                .map(|k| Pair { display: k.clone(), replacement: k })
                                .collect()
                        }
                        ArgHint::Fixed(options) => {
                            options
                                .iter()
                                .filter(|o| !o.is_empty() && o.starts_with(current_word))
                                .map(|o| Pair { display: o.to_string(), replacement: o.to_string() })
                                .collect()
                        }
                        ArgHint::Placeholder(_) => {
                            // placeholder 不提供候选项
                            vec![]
                        }
                        ArgHint::None => vec![],
                    };
                    return Ok((start_pos, candidates));
                }
                break;
            }
        }

        // 如果第一个词是别名（非命令），后续参数也可能是别名（比如浏览器 + URL 别名）
        if self.config.alias_exists(cmd) {
            let candidates: Vec<Pair> = self.all_aliases()
                .into_iter()
                .filter(|a| a.starts_with(current_word))
                .map(|a| Pair { display: a.clone(), replacement: a })
                .collect();
            return Ok((start_pos, candidates));
        }

        Ok((start_pos, vec![]))
    }
}

// ========== Hinter：基于历史的自动建议 ==========

struct CopilotHinter {
    history_hinter: HistoryHinter,
}

impl CopilotHinter {
    fn new() -> Self {
        Self {
            history_hinter: HistoryHinter::new(),
        }
    }
}

impl Hinter for CopilotHinter {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.history_hinter.hint(line, pos, ctx)
    }
}

// ========== Highlighter：提示文字灰色显示 ==========

struct CopilotHighlighter;

impl Highlighter for CopilotHighlighter {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        // 灰色显示 hint
        Cow::Owned(format!("\x1b[90m{}\x1b[0m", hint))
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: CmdKind) -> bool {
        // 返回 true 让 highlight_hint 生效
        true
    }
}

// ========== 组合 Helper ==========

struct CopilotHelper {
    completer: CopilotCompleter,
    hinter: CopilotHinter,
    highlighter: CopilotHighlighter,
}

impl CopilotHelper {
    fn new(config: &YamlConfig) -> Self {
        Self {
            completer: CopilotCompleter::new(config),
            hinter: CopilotHinter::new(),
            highlighter: CopilotHighlighter,
        }
    }

    fn refresh(&mut self, config: &YamlConfig) {
        self.completer.refresh(config);
    }
}

impl Completer for CopilotHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for CopilotHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for CopilotHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        self.highlighter.highlight_hint(hint)
    }

    fn highlight_char(&self, line: &str, pos: usize, forced: CmdKind) -> bool {
        self.highlighter.highlight_char(line, pos, forced)
    }
}

impl Validator for CopilotHelper {}

impl rustyline::Helper for CopilotHelper {}

// ========== 交互模式入口 ==========

/// 启动交互模式
pub fn run_interactive(config: &mut YamlConfig) {
    let rl_config = Config::builder()
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .auto_add_history(true)
        .build();

    let helper = CopilotHelper::new(config);

    let mut rl: Editor<CopilotHelper, DefaultHistory> =
        Editor::with_config(rl_config).expect("无法初始化编辑器");
    rl.set_helper(Some(helper));

    // Tab 键绑定到补全
    rl.bind_sequence(
        KeyEvent(KeyCode::Tab, Modifiers::NONE),
        EventHandler::Simple(Cmd::Complete),
    );

    // 加载历史记录
    let history_path = history_file_path();
    let _ = rl.load_history(&history_path);

    info!("Welcome to use work copilot 🚀 ~");

    let prompt = format!("{} ", constants::INTERACTIVE_PROMPT.yellow());

    loop {
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim();

                if input.is_empty() {
                    continue;
                }

                // ! 开头：执行 shell 命令
                if input.starts_with('!') {
                    let shell_cmd = &input[1..].trim();
                    execute_shell_command(shell_cmd);
                    println!();
                    continue;
                }

                // 解析并执行 copilot 命令
                let args = parse_input(input);
                if args.is_empty() {
                    continue;
                }

                let verbose = config.is_verbose();
                let start = if verbose {
                    Some(std::time::Instant::now())
                } else {
                    None
                };

                execute_interactive_command(&args, config);

                if let Some(start) = start {
                    let elapsed = start.elapsed();
                    crate::debug_log!(config, "duration: {} ms", elapsed.as_millis());
                }

                // 每次命令执行后刷新补全器中的配置（别名可能已变化）
                if let Some(helper) = rl.helper_mut() {
                    helper.refresh(config);
                }

                println!();
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C
                info!("\nProgram interrupted. Use 'exit' to quit.");
            }
            Err(ReadlineError::Eof) => {
                // Ctrl+D
                info!("\nGoodbye! 👋");
                break;
            }
            Err(err) => {
                error!("读取输入失败: {:?}", err);
                break;
            }
        }
    }

    // 保存历史记录
    let _ = rl.save_history(&history_path);
}

/// 获取历史文件路径: ~/.jdata/history.txt
fn history_file_path() -> std::path::PathBuf {
    let data_dir = crate::config::YamlConfig::data_dir();
    // 确保目录存在
    let _ = std::fs::create_dir_all(&data_dir);
    data_dir.join(constants::HISTORY_FILE)
}

/// 解析用户输入为参数列表
/// 支持双引号包裹带空格的参数，与 Java 版保持一致
fn parse_input(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in input.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' if !in_quotes => {
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

/// 在交互模式下执行命令
/// 与快捷模式不同，这里从解析后的 args 来分发命令
fn execute_interactive_command(args: &[String], config: &mut YamlConfig) {
    if args.is_empty() {
        return;
    }

    let cmd_str = &args[0];

    // 检查是否是退出命令
    if matches!(cmd_str.as_str(), "exit" | "q" | "quit") {
        command::system::handle_exit();
        return;
    }

    // 尝试解析为内置命令
    if let Some(subcmd) = parse_interactive_command(args) {
        command::dispatch(subcmd, config);
    } else {
        // 不是内置命令，尝试作为别名打开
        command::open::handle_open(args, config);
    }
}

/// 从交互模式输入的参数解析出 SubCmd
fn parse_interactive_command(args: &[String]) -> Option<crate::cli::SubCmd> {
    use crate::cli::SubCmd;

    if args.is_empty() {
        return None;
    }

    let cmd = args[0].as_str();
    let rest = &args[1..];

    match cmd {
        // 别名管理
        "set" | "s" => {
            if rest.is_empty() {
                crate::usage!("set <alias> <path>");
                return None;
            }
            Some(SubCmd::Set {
                alias: rest[0].clone(),
                path: rest[1..].to_vec(),
            })
        }
        "rm" | "remove" => {
            rest.first().map(|alias| SubCmd::Remove { alias: alias.clone() })
                .or_else(|| { crate::usage!("rm <alias>"); None })
        }
        "rename" | "rn" => {
            if rest.len() < 2 {
                crate::usage!("rename <alias> <new_alias>");
                return None;
            }
            Some(SubCmd::Rename {
                alias: rest[0].clone(),
                new_alias: rest[1].clone(),
            })
        }
        "mf" | "modify" => {
            if rest.is_empty() {
                crate::usage!("mf <alias> <new_path>");
                return None;
            }
            Some(SubCmd::Modify {
                alias: rest[0].clone(),
                path: rest[1..].to_vec(),
            })
        }

        // 分类标记
        "note" | "nt" => {
            if rest.len() < 2 {
                crate::usage!("note <alias> <category>");
                return None;
            }
            Some(SubCmd::Note {
                alias: rest[0].clone(),
                category: rest[1].clone(),
            })
        }
        "denote" | "dnt" => {
            if rest.len() < 2 {
                crate::usage!("denote <alias> <category>");
                return None;
            }
            Some(SubCmd::Denote {
                alias: rest[0].clone(),
                category: rest[1].clone(),
            })
        }

        // 列表
        "ls" | "list" => Some(SubCmd::List {
            part: rest.first().cloned(),
        }),

        // 查找
        "contain" | "find" => {
            if rest.is_empty() {
                crate::usage!("contain <alias> [sections]");
                return None;
            }
            Some(SubCmd::Contain {
                alias: rest[0].clone(),
                containers: rest.get(1).cloned(),
            })
        }

        // 系统设置
        "log" => {
            if rest.len() < 2 {
                crate::usage!("log mode <verbose|concise>");
                return None;
            }
            Some(SubCmd::Log {
                key: rest[0].clone(),
                value: rest[1].clone(),
            })
        }
        "change" | "chg" => {
            if rest.len() < 3 {
                crate::usage!("change <part> <field> <value>");
                return None;
            }
            Some(SubCmd::Change {
                part: rest[0].clone(),
                field: rest[1].clone(),
                value: rest[2].clone(),
            })
        }
        "clear" | "cls" => Some(SubCmd::Clear),

        // 日报系统
        "report" | "r" => {
            if rest.is_empty() {
                crate::usage!("report <content>");
                return None;
            }
            Some(SubCmd::Report {
                content: rest.to_vec(),
            })
        }
        "r-meta" => {
            if rest.is_empty() {
                crate::usage!("r-meta <new|sync> [date]");
                return None;
            }
            Some(SubCmd::RMeta {
                action: rest[0].clone(),
                date: rest.get(1).cloned(),
            })
        }
        "check" | "c" => Some(SubCmd::Check {
            line_count: rest.first().cloned(),
        }),
        "search" | "select" | "look" | "sch" => {
            if rest.len() < 2 {
                crate::usage!("search <line_count|all> <target> [-f|-fuzzy]");
                return None;
            }
            Some(SubCmd::Search {
                line_count: rest[0].clone(),
                target: rest[1].clone(),
                fuzzy: rest.get(2).cloned(),
            })
        }

        // 脚本创建
        "concat" => {
            if rest.len() < 2 {
                crate::usage!("concat <script_name> \"<script_content>\"");
                return None;
            }
            Some(SubCmd::Concat {
                name: rest[0].clone(),
                content: rest[1..].join(" "),
            })
        }

        // 倒计时
        "time" => {
            if rest.len() < 2 {
                crate::usage!("time countdown <duration>");
                return None;
            }
            Some(SubCmd::Time {
                function: rest[0].clone(),
                arg: rest[1].clone(),
            })
        }

        // 系统信息
        "version" | "v" => Some(SubCmd::Version),
        "help" | "h" => Some(SubCmd::Help),

        // 未匹配到内置命令
        _ => None,
    }
}

/// 执行 shell 命令
fn execute_shell_command(cmd: &str) {
    if cmd.is_empty() {
        return;
    }

    let os = std::env::consts::OS;
    let result = if os == "windows" {
        std::process::Command::new("cmd")
            .args(["/c", cmd])
            .status()
    } else {
        std::process::Command::new("/bin/bash")
            .args(["-c", cmd])
            .status()
    };

    match result {
        Ok(status) => {
            if !status.success() {
                if let Some(code) = status.code() {
                    error!("命令退出码: {}", code);
                }
            }
        }
        Err(e) => {
            error!("执行命令失败: {}", e);
        }
    }
}
