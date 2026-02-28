use crate::command;
use crate::config::YamlConfig;
use crate::constants::{
    self, ALIAS_PATH_SECTIONS, ALL_SECTIONS, LIST_ALL, NOTE_CATEGORIES, cmd, config_key,
    rmeta_action, search_flag, time_function, voice as vc,
};
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::CmdKind;
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};

use rustyline::Context;
use rustyline::validate::Validator;
use std::borrow::Cow;

// ========== 补全器定义 ==========

/// 自定义补全器：根据上下文提供命令、别名、分类等补全
pub struct CopilotCompleter {
    pub config: YamlConfig,
}

impl CopilotCompleter {
    pub fn new(config: &YamlConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

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
    Placeholder(&'static str),
    FilePath,
    None,
}

/// 获取命令的补全规则定义
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
        (cmd::NOTE, vec![ArgHint::Alias, ArgHint::Category]),
        (cmd::DENOTE, vec![ArgHint::Alias, ArgHint::Category]),
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
            cmd::CHANGE,
            vec![
                ArgHint::Section,
                ArgHint::Placeholder("<field>"),
                ArgHint::Placeholder("<value>"),
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
        (cmd::CHECK, vec![ArgHint::Placeholder("<line_count>")]),
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
        (cmd::CHAT, vec![ArgHint::Placeholder("<message>")]),
        (cmd::VOICE, vec![ArgHint::Fixed(vec![vc::ACTION_DOWNLOAD])]),
        (
            cmd::CONCAT,
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
                let arg_index = word_index - 1;
                if arg_index < arg_hints.len() {
                    let candidates = match &arg_hints[arg_index] {
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
                        ArgHint::Fixed(options) => options
                            .iter()
                            .filter(|o| !o.is_empty() && o.starts_with(current_word))
                            .map(|o| Pair {
                                display: o.to_string(),
                                replacement: o.to_string(),
                            })
                            .collect(),
                        ArgHint::Placeholder(_) => vec![],
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

pub struct CopilotHinter {
    history_hinter: HistoryHinter,
}

impl CopilotHinter {
    pub fn new() -> Self {
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

// ========== Highlighter ==========

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

pub struct CopilotHelper {
    pub completer: CopilotCompleter,
    hinter: CopilotHinter,
    highlighter: CopilotHighlighter,
}

impl CopilotHelper {
    pub fn new(config: &YamlConfig) -> Self {
        Self {
            completer: CopilotCompleter::new(config),
            hinter: CopilotHinter::new(),
            highlighter: CopilotHighlighter,
        }
    }

    pub fn refresh(&mut self, config: &YamlConfig) {
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
            let parent = p
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_path_buf();
            let fp = p
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            (parent, fp)
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
                            .unwrap();
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
