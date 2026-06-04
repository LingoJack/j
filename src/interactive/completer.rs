use crate::command;
use crate::config::YamlConfig;
use crate::constants::{
    self, ALIAS_PATH_SECTIONS, ALL_SECTIONS, LIST_ALL, NOTE_CATEGORIES, cmd, config_key,
    rmeta_action, search_flag, time_function,
};
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::CmdKind;
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};

use rustyline::Context;
use rustyline::validate::Validator;
use std::borrow::Cow;

// ========== 自定义 Hint 类型 ==========

/// REPL 提示类型：区分纯展示的 tip 和可补全的历史命令
pub enum CopilotHint {
    /// 纯展示的使用技巧，不可被按键自动补全到输入行
    Tip(String),
    /// 历史命令建议，可通过右箭头/End 等按键接受补全
    History(String),
}

impl rustyline::hint::Hint for CopilotHint {
    fn display(&self) -> &str {
        match self {
            CopilotHint::Tip(s) | CopilotHint::History(s) => s,
        }
    }

    fn completion(&self) -> Option<&str> {
        match self {
            CopilotHint::Tip(_) => None,
            CopilotHint::History(s) => Some(s),
        }
    }
}

// ========== 补全器定义 ==========

/// 自定义补全器：根据上下文提供命令、别名、分类等补全
#[derive(Debug)]
pub struct CopilotCompleter {
    pub config: YamlConfig,
}

impl CopilotCompleter {
    /// 基于给定配置创建补全器实例
    pub fn new(config: &YamlConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// 刷新补全器的配置快照（配置变更后调用）
    pub fn refresh(&mut self, config: &YamlConfig) {
        self.config = config.clone();
    }

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

    fn all_sections(&self) -> Vec<String> {
        self.config
            .all_section_names()
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn section_keys(&self, section: &str) -> Vec<String> {
        self.config
            .get_section(section)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }
}

/// 命令定义：(命令名列表, 参数位置补全策略)
#[derive(Clone)]
#[allow(dead_code)]
pub enum ArgHint {
    Alias,
    Category,
    Section,
    SectionKeys(String),
    Fixed(Vec<&'static str>),
    /// Flag 补全：当前词以 `-` 开头时触发，可出现在任意参数位置
    Flags(Vec<&'static str>),
    /// 动态补全：根据前面第 N 个 positional 参数作为 section 名，补全该 section 下的 key
    DynamicSectionKeys {
        section_arg_index: usize,
    },
    /// 根据 key 名动态补全 value 的可选值（用于 config 命令第三个参数）
    DynamicValueForkey {
        key_arg_index: usize,
    },
    Placeholder(&'static str),
    FilePath,
    None,
}

/// 获取所有命令的参数补全规则定义
pub fn command_completion_rules() -> Vec<(&'static [&'static str], Vec<ArgHint>)> {
    vec![
        (
            cmd::SET,
            vec![ArgHint::Placeholder("<alias>"), ArgHint::FilePath],
        ),
        (cmd::REMOVE, vec![ArgHint::Alias]),
        (
            cmd::RENAME,
            vec![ArgHint::Alias, ArgHint::Placeholder("<new_alias>")],
        ),
        (cmd::MODIFY, vec![ArgHint::Alias, ArgHint::FilePath]),
        (cmd::TAG, vec![ArgHint::Alias, ArgHint::Category]),
        (cmd::UNTAG, vec![ArgHint::Alias, ArgHint::Category]),
        (
            cmd::LIST,
            vec![ArgHint::Fixed({
                let mut v: Vec<&'static str> = vec!["", LIST_ALL];
                for s in ALL_SECTIONS {
                    v.push(s);
                }
                v
            })],
        ),
        (
            cmd::CONTAIN,
            vec![ArgHint::Alias, ArgHint::Placeholder("<sections>")],
        ),
        (
            cmd::LOG,
            vec![
                ArgHint::Fixed(vec![config_key::MODE]),
                ArgHint::Fixed(vec![config_key::VERBOSE, config_key::CONCISE]),
            ],
        ),
        (
            cmd::CONFIG,
            vec![
                ArgHint::Section,
                ArgHint::DynamicSectionKeys {
                    section_arg_index: 0,
                },
                ArgHint::DynamicValueForkey { key_arg_index: 1 },
            ],
        ),
        (cmd::REPORT, vec![ArgHint::Placeholder("<content>")]),
        (
            cmd::REPORTCTL,
            vec![
                ArgHint::Fixed(vec![
                    rmeta_action::NEW,
                    rmeta_action::SYNC,
                    rmeta_action::PUSH,
                    rmeta_action::PULL,
                    rmeta_action::SET_URL,
                    rmeta_action::OPEN,
                ]),
                ArgHint::Placeholder("<date|message|url>"),
            ],
        ),
        (
            cmd::CHECK,
            vec![ArgHint::Fixed(vec!["open", "<line_count>"])],
        ),
        (
            cmd::SEARCH,
            vec![
                ArgHint::Placeholder("<line_count|all>"),
                ArgHint::Placeholder("<target>"),
                ArgHint::Fixed(vec![search_flag::FUZZY_SHORT, search_flag::FUZZY]),
            ],
        ),
        (
            cmd::TODO,
            vec![
                ArgHint::Fixed(vec!["list", "add"]),
                ArgHint::Placeholder("<content>"),
            ],
        ),
        (
            cmd::CHAT,
            vec![
                ArgHint::Flags(vec!["--continue", "-c", "--session", "--remote"]),
                ArgHint::Placeholder("<message>"),
            ],
        ),
        (
            cmd::SCRIPT,
            vec![
                ArgHint::Placeholder("<script_name>"),
                ArgHint::Placeholder("<script_content>"),
            ],
        ),
        (
            cmd::TIME,
            vec![
                ArgHint::Fixed(vec![time_function::COUNTDOWN]),
                ArgHint::Placeholder("<duration>"),
            ],
        ),
        (cmd::COMPLETION, vec![ArgHint::Fixed(vec!["zsh", "bash"])]),
        (cmd::VERSION, vec![]),
        (cmd::HELP, vec![]),
        (cmd::CLEAR, vec![]),
        (cmd::EXIT, vec![]),
        (cmd::UPDATE, vec![ArgHint::Fixed(vec!["--check"])]),
        (cmd::MD, vec![ArgHint::FilePath]),
        (
            cmd::READ,
            vec![
                ArgHint::Flags(vec!["--port", "--no-open"]),
                ArgHint::FilePath,
            ],
        ),
    ]
}

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

        // Shell 命令（! 前缀）
        if !parts.is_empty() && (parts[0] == "!" || parts[0].starts_with('!')) {
            let candidates = complete_file_path(current_word);
            return Ok((start_pos, candidates));
        }

        if word_index == 0 {
            let mut candidates = Vec::new();
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
            for alias in self.all_aliases() {
                if alias.starts_with(current_word)
                    && !command::all_command_keywords().contains(&alias.as_str())
                {
                    candidates.push(Pair {
                        display: alias.clone(),
                        replacement: alias,
                    });
                }
            }
            return Ok((start_pos, candidates));
        }

        let cmd_str = parts[0];
        let rules = command_completion_rules();

        for (names, arg_hints) in &rules {
            if names.contains(&cmd_str) {
                // 当前词以 `-` 开头时，扫描所有 Flags hint 进行补全（不受位置限制）
                if current_word.starts_with('-') {
                    let flags: Vec<Pair> = arg_hints
                        .iter()
                        .filter_map(|h| {
                            if let ArgHint::Flags(fs) = h {
                                Some(fs)
                            } else {
                                None
                            }
                        })
                        .flatten()
                        .filter(|f| f.starts_with(current_word))
                        .map(|f| Pair {
                            display: f.to_string(),
                            replacement: f.to_string(),
                        })
                        .collect();
                    if !flags.is_empty() {
                        return Ok((start_pos, flags));
                    }
                }

                // 计算非 flag 参数的实际位置（跳过已输入的 flag 和 --session 的值）
                let non_flag_args: Vec<&ArgHint> = arg_hints
                    .iter()
                    .filter(|h| !matches!(h, ArgHint::Flags(_)))
                    .collect();

                // 统计前面参数中已消耗的位置（flag 和 --session <value> 不计入位置索引）
                let preceding = &parts[1..word_index]; // 当前词之前的所有参数
                let mut skip = 0usize;
                let mut i = 0;
                while i < preceding.len() {
                    if preceding[i] == "--session" {
                        skip += 2; // --session 和它的值各占一个位置
                        i += 2;
                    } else if preceding[i].starts_with('-') {
                        skip += 1;
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
                let positional_index = (word_index - 1).saturating_sub(skip);

                // 收集前面已输入的 positional 参数值（跳过 flag）
                let mut positional_values: Vec<&str> = Vec::new();
                {
                    let mut j = 0;
                    let preceding = &parts[1..word_index];
                    while j < preceding.len() {
                        if preceding[j] == "--session" {
                            j += 2;
                        } else if preceding[j].starts_with('-') {
                            j += 1;
                        } else {
                            positional_values.push(preceding[j]);
                            j += 1;
                        }
                    }
                }

                if positional_index < non_flag_args.len() {
                    let candidates = match non_flag_args[positional_index] {
                        ArgHint::Alias => self
                            .all_aliases()
                            .into_iter()
                            .filter(|a| a.starts_with(current_word))
                            .map(|a| Pair {
                                display: a.clone(),
                                replacement: a,
                            })
                            .collect(),
                        ArgHint::Category => ALL_NOTE_CATEGORIES
                            .iter()
                            .filter(|c| c.starts_with(current_word))
                            .map(|c| Pair {
                                display: c.to_string(),
                                replacement: c.to_string(),
                            })
                            .collect(),
                        ArgHint::Section => self
                            .all_sections()
                            .into_iter()
                            .filter(|s| s.starts_with(current_word))
                            .map(|s| Pair {
                                display: s.clone(),
                                replacement: s,
                            })
                            .collect(),
                        ArgHint::SectionKeys(section) => self
                            .section_keys(section)
                            .into_iter()
                            .filter(|k| k.starts_with(current_word))
                            .map(|k| Pair {
                                display: k.clone(),
                                replacement: k,
                            })
                            .collect(),
                        ArgHint::DynamicSectionKeys { section_arg_index } => {
                            if let Some(section_name) = positional_values.get(*section_arg_index) {
                                let mut keys = self.section_keys(section_name);
                                // setting 段额外合并已知可配置 key（即使配置文件中尚未出现）
                                if *section_name == crate::constants::section::SETTING {
                                    let existing: std::collections::HashSet<String> =
                                        keys.iter().cloned().collect();
                                    let extra: Vec<String> = crate::constants::SETTING_KNOWN_KEYS
                                        .iter()
                                        .filter(|k| !existing.contains(**k))
                                        .map(|k| k.to_string())
                                        .collect();
                                    keys.extend(extra);
                                }
                                keys.into_iter()
                                    .filter(|k| k.starts_with(current_word))
                                    .map(|k| Pair {
                                        display: k.clone(),
                                        replacement: k,
                                    })
                                    .collect()
                            } else {
                                vec![]
                            }
                        }
                        ArgHint::DynamicValueForkey { key_arg_index } => {
                            if let Some(key_name) = positional_values.get(*key_arg_index) {
                                if let Some(candidates) =
                                    crate::constants::config_value_candidates(key_name)
                                {
                                    candidates
                                        .iter()
                                        .filter(|c| c.starts_with(current_word))
                                        .map(|c| Pair {
                                            display: c.to_string(),
                                            replacement: c.to_string(),
                                        })
                                        .collect()
                                } else {
                                    vec![Pair {
                                        display: "<value>".to_string(),
                                        replacement: current_word.to_string(),
                                    }]
                                }
                            } else {
                                vec![]
                            }
                        }
                        ArgHint::Fixed(options) => options
                            .iter()
                            .filter(|o| !o.is_empty() && o.starts_with(current_word))
                            .map(|o| Pair {
                                display: o.to_string(),
                                replacement: o.to_string(),
                            })
                            .collect(),
                        ArgHint::Placeholder(hint) => vec![Pair {
                            display: hint.to_string(),
                            replacement: current_word.to_string(),
                        }],
                        ArgHint::Flags(_) => vec![],
                        ArgHint::FilePath => complete_file_path(current_word),
                        ArgHint::None => vec![],
                    };
                    return Ok((start_pos, candidates));
                }
                break;
            }
        }

        // 别名后续参数智能补全
        if self.config.alias_exists(cmd_str) {
            if self.config.contains(constants::section::EDITOR, cmd_str) {
                return Ok((start_pos, complete_file_path(current_word)));
            }
            if self.config.contains(constants::section::BROWSER, cmd_str) {
                let mut candidates: Vec<Pair> = self
                    .all_aliases()
                    .into_iter()
                    .filter(|a| a.starts_with(current_word))
                    .map(|a| Pair {
                        display: a.clone(),
                        replacement: a,
                    })
                    .collect();
                candidates.extend(complete_file_path(current_word));
                return Ok((start_pos, candidates));
            }
            let mut candidates = complete_file_path(current_word);
            candidates.extend(
                self.all_aliases()
                    .into_iter()
                    .filter(|a| a.starts_with(current_word))
                    .map(|a| Pair {
                        display: a.clone(),
                        replacement: a,
                    }),
            );
            return Ok((start_pos, candidates));
        }

        Ok((start_pos, vec![]))
    }
}

// ========== Hinter ==========

/// 基于 rustyline HistoryHinter 的命令历史提示器
/// 空行时展示随机使用技巧，有输入时回退到历史提示
pub struct CopilotHinter {
    history_hinter: HistoryHinter,
    current_tip: String,
}

impl Default for CopilotHinter {
    fn default() -> Self {
        Self::new()
    }
}

impl CopilotHinter {
    /// 创建历史提示器
    pub fn new() -> Self {
        Self {
            history_hinter: HistoryHinter::new(),
            current_tip: pick_random_tip(),
        }
    }

    /// 轮转到下一条随机技巧（每次 prompt 前调用）
    pub fn rotate_tip(&mut self) {
        self.current_tip = pick_random_tip();
    }
}

impl Hinter for CopilotHinter {
    type Hint = CopilotHint;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<CopilotHint> {
        if line.is_empty() {
            // 空行时展示 tip，不可被补全
            return Some(CopilotHint::Tip(self.current_tip.clone()));
        }
        // 有输入时回退到历史命令提示，可被按键接受
        self.history_hinter
            .hint(line, pos, ctx)
            .map(|s| CopilotHint::History(s.to_owned()))
    }
}

// ========== Highlighter ==========

/// 命令行高亮器：将补全提示显示为灰色
pub struct CopilotHighlighter;

impl Highlighter for CopilotHighlighter {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("\x1b[90m{}\x1b[0m", hint))
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: CmdKind) -> bool {
        true
    }
}

// ========== 组合 Helper ==========

/// REPL 辅助组件集合：补全器 + 历史提示器 + 高亮器
pub struct CopilotHelper {
    pub completer: CopilotCompleter,
    hinter: CopilotHinter,
    highlighter: CopilotHighlighter,
}

impl CopilotHelper {
    /// 基于当前配置创建 Helper 组件集合
    pub fn new(config: &YamlConfig) -> Self {
        Self {
            completer: CopilotCompleter::new(config),
            hinter: CopilotHinter::new(),
            highlighter: CopilotHighlighter,
        }
    }

    /// 刷新补全器的配置快照（配置变更后调用）
    pub fn refresh(&mut self, config: &YamlConfig) {
        self.completer.refresh(config);
    }

    /// 轮转使用技巧（每次 prompt 前调用）
    pub fn rotate_tip(&mut self) {
        self.hinter.rotate_tip();
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
    type Hint = CopilotHint;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<CopilotHint> {
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

// ========== 文件路径补全 ==========

/// 文件系统路径补全
pub fn complete_file_path(partial: &str) -> Vec<Pair> {
    let mut candidates = Vec::new();

    let expanded = if partial.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            partial.replacen('~', &home.to_string_lossy(), 1)
        } else {
            partial.to_string()
        }
    } else {
        partial.to_string()
    };

    let (dir_path, file_prefix) =
        if expanded.ends_with('/') || expanded.ends_with(std::path::MAIN_SEPARATOR) {
            (std::path::Path::new(&expanded).to_path_buf(), String::new())
        } else {
            let p = std::path::Path::new(&expanded);
            let parent = p.parent().unwrap_or(std::path::Path::new("."));
            // 空路径视为当前目录
            let parent = if parent.as_os_str().is_empty() {
                std::path::Path::new(".")
            } else {
                parent
            };
            let fp = p
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            (parent.to_path_buf(), fp)
        };

    if let Ok(entries) = std::fs::read_dir(&dir_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') && !file_prefix.starts_with('.') {
                continue;
            }
            if name.starts_with(&file_prefix) {
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let full_replacement =
                    if partial.ends_with('/') || partial.ends_with(std::path::MAIN_SEPARATOR) {
                        format!("{}{}{}", partial, name, if is_dir { "/" } else { "" })
                    } else if partial.contains('/') || partial.contains(std::path::MAIN_SEPARATOR) {
                        let last_sep = partial
                            .rfind('/')
                            .or_else(|| partial.rfind(std::path::MAIN_SEPARATOR))
                            .unwrap_or(0);
                        format!(
                            "{}/{}{}",
                            &partial[..last_sep],
                            name,
                            if is_dir { "/" } else { "" }
                        )
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

    candidates.sort_by(|a, b| a.display.cmp(&b.display));
    candidates
}

// ========== 使用技巧 ==========

use std::sync::atomic::{AtomicUsize, Ordering};

/// 全局轮转计数器，保证每条 tip 依次出现
static TIP_INDEX: AtomicUsize = AtomicUsize::new(0);

/// 从 tips.txt 中轮转选取一条使用技巧
fn pick_random_tip() -> String {
    let tips: Vec<&str> = crate::assets::tips_text()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    if tips.is_empty() {
        return String::new();
    }
    let index = TIP_INDEX.fetch_add(1, Ordering::Relaxed) % tips.len();
    format!("({})", tips[index])
}
