use crate::command;
use crate::config::YamlConfig;
use crate::constants::{self, cmd, config_key, rmeta_action, time_function, search_flag, shell, NOTE_CATEGORIES, ALL_SECTIONS, ALIAS_PATH_SECTIONS, LIST_ALL};
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
    FilePath, // 文件系统路径补全
    None,
}

/// 获取命令的补全规则定义
fn command_completion_rules() -> Vec<(&'static [&'static str], Vec<ArgHint>)> {
    vec![
        // 别名管理
        (cmd::SET, vec![ArgHint::Placeholder("<alias>"), ArgHint::FilePath]),
        (cmd::REMOVE, vec![ArgHint::Alias]),
        (cmd::RENAME, vec![ArgHint::Alias, ArgHint::Placeholder("<new_alias>")]),
        (cmd::MODIFY, vec![ArgHint::Alias, ArgHint::FilePath]),
        // 分类
        (cmd::NOTE, vec![ArgHint::Alias, ArgHint::Category]),
        (cmd::DENOTE, vec![ArgHint::Alias, ArgHint::Category]),
        // 列表
        (cmd::LIST, vec![ArgHint::Fixed({
            let mut v: Vec<&'static str> = vec!["", LIST_ALL];
            for s in ALL_SECTIONS { v.push(s); }
            v
        })]),
        // 查找
        (cmd::CONTAIN, vec![ArgHint::Alias, ArgHint::Placeholder("<sections>")]),
        // 系统设置
        (cmd::LOG, vec![ArgHint::Fixed(vec![config_key::MODE]), ArgHint::Fixed(vec![config_key::VERBOSE, config_key::CONCISE])]),
        (cmd::CHANGE, vec![ArgHint::Section, ArgHint::Placeholder("<field>"), ArgHint::Placeholder("<value>")]),
        // 日报系统
        (cmd::REPORT, vec![ArgHint::Placeholder("<content>")]),
        (cmd::REPORTCTL, vec![ArgHint::Fixed(vec![rmeta_action::NEW, rmeta_action::SYNC, rmeta_action::PUSH, rmeta_action::PULL, rmeta_action::SET_URL, rmeta_action::OPEN]), ArgHint::Placeholder("<date|message|url>")]),
        (cmd::CHECK, vec![ArgHint::Placeholder("<line_count>")]),
        (cmd::SEARCH, vec![ArgHint::Placeholder("<line_count|all>"), ArgHint::Placeholder("<target>"), ArgHint::Fixed(vec![search_flag::FUZZY_SHORT, search_flag::FUZZY])]),
        // 脚本
        (cmd::CONCAT, vec![ArgHint::Placeholder("<script_name>"), ArgHint::Placeholder("<script_content>")]),
        // 倒计时
        (cmd::TIME, vec![ArgHint::Fixed(vec![time_function::COUNTDOWN]), ArgHint::Placeholder("<duration>")]),
        // shell 补全
        (cmd::COMPLETION, vec![ArgHint::Fixed(vec!["zsh", "bash"])]),
        // 系统信息
        (cmd::VERSION, vec![]),
        (cmd::HELP, vec![]),
        (cmd::CLEAR, vec![]),
        (cmd::EXIT, vec![]),
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

        // Shell 命令（! 前缀）：对所有参数提供文件路径补全
        if !parts.is_empty() && (parts[0] == "!" || parts[0].starts_with('!')) {
            // ! 后面的所有参数都支持文件路径补全
            let candidates = complete_file_path(current_word);
            return Ok((start_pos, candidates));
        }

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
                        ArgHint::FilePath => {
                            // 文件系统路径补全
                            complete_file_path(current_word)
                        }
                        ArgHint::None => vec![],
                    };
                    return Ok((start_pos, candidates));
                }
                break;
            }
        }

        // 如果第一个词是别名（非命令），根据别名类型智能补全后续参数
        if self.config.alias_exists(cmd) {
            // 编辑器类别名：后续参数补全文件路径（如 vscode ./src<Tab>）
            if self.config.contains(constants::section::EDITOR, cmd) {
                let candidates = complete_file_path(current_word);
                return Ok((start_pos, candidates));
            }

            // 浏览器类别名：后续参数补全 URL 别名 + 文件路径
            if self.config.contains(constants::section::BROWSER, cmd) {
                let mut candidates: Vec<Pair> = self.all_aliases()
                    .into_iter()
                    .filter(|a| a.starts_with(current_word))
                    .map(|a| Pair { display: a.clone(), replacement: a })
                    .collect();
                // 也支持文件路径补全（浏览器打开本地文件）
                candidates.extend(complete_file_path(current_word));
                return Ok((start_pos, candidates));
            }

            // 其他别名（如 CLI 工具）：后续参数补全文件路径 + 别名
            let mut candidates = complete_file_path(current_word);
            candidates.extend(
                self.all_aliases()
                    .into_iter()
                    .filter(|a| a.starts_with(current_word))
                    .map(|a| Pair { display: a.clone(), replacement: a })
            );
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
        .auto_add_history(false) // 手动控制历史记录，report 内容不入历史（隐私保护）
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

    info!("{}", constants::WELCOME_MESSAGE);

    // 进入交互模式时，将所有别名路径注入为当前进程的环境变量
    inject_envs_to_process(config);

    let prompt = format!("{} ", constants::INTERACTIVE_PROMPT.yellow());

    loop {
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim();

                if input.is_empty() {
                    continue;
                }

                // Shell 命令前缀开头：执行 shell 命令
                if input.starts_with(constants::SHELL_PREFIX) {
                    let shell_cmd = &input[1..].trim();
                    execute_shell_command(shell_cmd, config);
                    // Shell 命令记录到历史
                    let _ = rl.add_history_entry(input);
                    println!();
                    continue;
                }

                // 解析并执行 copilot 命令
                let args = parse_input(input);
                if args.is_empty() {
                    continue;
                }

                // 展开参数中的环境变量引用（如 $J_HELLO → 实际路径）
                let args: Vec<String> = args.iter().map(|a| expand_env_vars(a)).collect();

                let verbose = config.is_verbose();
                let start = if verbose {
                    Some(std::time::Instant::now())
                } else {
                    None
                };

                // report 内容不记入历史（隐私保护），其他命令正常记录
                let is_report_cmd = !args.is_empty() && cmd::REPORT.contains(&args[0].as_str());
                if !is_report_cmd {
                    let _ = rl.add_history_entry(input);
                }

                execute_interactive_command(&args, config);

                if let Some(start) = start {
                    let elapsed = start.elapsed();
                    crate::debug_log!(config, "duration: {} ms", elapsed.as_millis());
                }

                // 每次命令执行后刷新补全器中的配置（别名可能已变化）
                if let Some(helper) = rl.helper_mut() {
                    helper.refresh(config);
                }
                // 刷新进程环境变量（别名可能已增删改）
                inject_envs_to_process(config);

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

/// 交互命令解析结果（三态）
enum ParseResult {
    /// 成功解析为内置命令
    Matched(crate::cli::SubCmd),
    /// 是内置命令但参数不足，已打印 usage 提示
    Handled,
    /// 不是内置命令
    NotFound,
}

/// 在交互模式下执行命令
/// 与快捷模式不同，这里从解析后的 args 来分发命令
fn execute_interactive_command(args: &[String], config: &mut YamlConfig) {
    if args.is_empty() {
        return;
    }

    let cmd_str = &args[0];

    // 检查是否是退出命令
    if cmd::EXIT.contains(&cmd_str.as_str()) {
        command::system::handle_exit();
        return;
    }

    // 尝试解析为内置命令
    match parse_interactive_command(args) {
        ParseResult::Matched(subcmd) => {
            command::dispatch(subcmd, config);
        }
        ParseResult::Handled => {
            // 内置命令参数不足，已打印 usage，无需额外处理
        }
        ParseResult::NotFound => {
            // 不是内置命令，尝试作为别名打开
            command::open::handle_open(args, config);
        }
    }
}

/// 从交互模式输入的参数解析出 SubCmd
fn parse_interactive_command(args: &[String]) -> ParseResult {
    use crate::cli::SubCmd;

    if args.is_empty() {
        return ParseResult::NotFound;
    }

    let cmd = args[0].as_str();
    let rest = &args[1..];

    // 使用闭包简化命令匹配：判断 cmd 是否在某个命令常量组中
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
            Some(alias) => ParseResult::Matched(SubCmd::Remove { alias: alias.clone() }),
            None => { crate::usage!("rm <alias>"); ParseResult::Handled }
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

    // 分类标记
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

    // 列表
    } else if is(cmd::LIST) {
        ParseResult::Matched(SubCmd::List {
            part: rest.first().cloned(),
        })

    // 查找
    } else if is(cmd::CONTAIN) {
        if rest.is_empty() {
            crate::usage!("contain <alias> [sections]");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Contain {
            alias: rest[0].clone(),
            containers: rest.get(1).cloned(),
        })

    // 系统设置
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

    // 日报系统
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

    // 脚本创建
    } else if is(cmd::CONCAT) {
        if rest.is_empty() {
            crate::usage!("concat <script_name> [\"<script_content>\"]");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Concat {
            name: rest[0].clone(),
            content: if rest.len() > 1 { rest[1..].to_vec() } else { vec![] },
        })

    // 倒计时
    } else if is(cmd::TIME) {
        if rest.len() < 2 {
            crate::usage!("time countdown <duration>");
            return ParseResult::Handled;
        }
        ParseResult::Matched(SubCmd::Time {
            function: rest[0].clone(),
            arg: rest[1].clone(),
        })

    // 系统信息
    } else if is(cmd::VERSION) {
        ParseResult::Matched(SubCmd::Version)
    } else if is(cmd::HELP) {
        ParseResult::Matched(SubCmd::Help)
    } else if is(cmd::COMPLETION) {
        ParseResult::Matched(SubCmd::Completion {
            shell: rest.first().cloned(),
        })

    // 未匹配到内置命令
    } else {
        ParseResult::NotFound
    }
}

/// 文件系统路径补全
/// 根据用户已输入的部分路径，列出匹配的文件和目录
fn complete_file_path(partial: &str) -> Vec<Pair> {
    let mut candidates = Vec::new();

    // 展开 ~ 为 home 目录
    let expanded = if partial.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            partial.replacen('~', &home.to_string_lossy(), 1)
        } else {
            partial.to_string()
        }
    } else {
        partial.to_string()
    };

    // 解析目录路径和文件名前缀
    let (dir_path, file_prefix) = if expanded.ends_with('/') || expanded.ends_with(std::path::MAIN_SEPARATOR) {
        (std::path::Path::new(&expanded).to_path_buf(), String::new())
    } else {
        let p = std::path::Path::new(&expanded);
        let parent = p.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
        let fp = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        (parent, fp)
    };

    if let Ok(entries) = std::fs::read_dir(&dir_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();

            // 跳过隐藏文件（除非用户已经输入了 .）
            if name.starts_with('.') && !file_prefix.starts_with('.') {
                continue;
            }

            if name.starts_with(&file_prefix) {
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

                // 构建完整路径用于替换
                // 保留用户输入的原始前缀风格（如 ~ 或绝对路径）
                let full_replacement = if partial.ends_with('/') || partial.ends_with(std::path::MAIN_SEPARATOR) {
                    format!("{}{}{}", partial, name, if is_dir { "/" } else { "" })
                } else if partial.contains('/') || partial.contains(std::path::MAIN_SEPARATOR) {
                    // 替换最后一段
                    let last_sep = partial.rfind('/').or_else(|| partial.rfind(std::path::MAIN_SEPARATOR)).unwrap();
                    format!("{}/{}{}", &partial[..last_sep], name, if is_dir { "/" } else { "" })
                } else {
                    format!("{}{}", name, if is_dir { "/" } else { "" })
                };

                let display_name = format!("{}{}", name, if is_dir { "/" } else { "" });

                candidates.push(Pair {
                    display: display_name,
                    replacement: full_replacement,
                });
            }
        }
    }

    // 按名称排序，目录优先
    candidates.sort_by(|a, b| a.display.cmp(&b.display));
    candidates
}

/// 执行 shell 命令（交互模式下 ! 前缀触发）
/// 自动注入所有别名路径为环境变量（J_<ALIAS_UPPER>）
fn execute_shell_command(cmd: &str, config: &YamlConfig) {
    if cmd.is_empty() {
        return;
    }

    let os = std::env::consts::OS;
    let mut command = if os == shell::WINDOWS_OS {
        let mut c = std::process::Command::new(shell::WINDOWS_CMD);
        c.args([shell::WINDOWS_CMD_FLAG, cmd]);
        c
    } else {
        let mut c = std::process::Command::new(shell::BASH_PATH);
        c.args([shell::BASH_CMD_FLAG, cmd]);
        c
    };

    // 注入别名环境变量
    for (key, value) in config.collect_alias_envs() {
        command.env(&key, &value);
    }

    let result = command.status();

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

/// 将所有别名路径注入为当前进程的环境变量
/// 这样在交互模式下，参数中的 $J_XXX 可以被正确展开
fn inject_envs_to_process(config: &YamlConfig) {
    for (key, value) in config.collect_alias_envs() {
        // SAFETY: 交互模式为单线程，set_var 不会引起数据竞争
        unsafe {
            std::env::set_var(&key, &value);
        }
    }
}

/// 展开字符串中的环境变量引用
/// 支持 $VAR_NAME 和 ${VAR_NAME} 两种格式
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
                        // 环境变量不存在，保留原文
                        result.push_str(&input[i..i + 3 + end]);
                    }
                    i = i + 3 + end;
                    continue;
                }
            }
            // $VAR_NAME 格式（变量名由字母、数字、下划线组成）
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
                    // 环境变量不存在，保留原文
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