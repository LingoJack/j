pub mod completer;
pub mod parser;
pub mod shell;

use crate::config::YamlConfig;
use crate::constants::{self, cmd};
use crate::{error, info};
use colored::Colorize;
use completer::CopilotHelper;
use parser::execute_interactive_command;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{
    Cmd, CompletionType, Config, EditMode, Editor, EventHandler, KeyCode, KeyEvent, Modifiers,
};
use shell::{
    enter_interactive_shell, execute_shell_command, expand_env_vars, inject_envs_to_process,
};

/// 启动交互模式
pub fn run_interactive(config: &mut YamlConfig) {
    let rl_config = Config::builder()
        .completion_type(CompletionType::Circular)
        .edit_mode(EditMode::Emacs)
        .auto_add_history(false) // 手动控制历史记录，report 内容不入历史（隐私保护）
        .build();

    let helper = CopilotHelper::new(config);

    let mut rl: Editor<CopilotHelper, DefaultHistory> =
        Editor::with_config(rl_config).expect("无法初始化编辑器");
    rl.set_helper(Some(helper));

    rl.bind_sequence(
        KeyEvent(KeyCode::Tab, Modifiers::NONE),
        EventHandler::Simple(Cmd::Complete),
    );

    let history_path = history_file_path();
    let _ = rl.load_history(&history_path);

    info!("{}", constants::WELCOME_MESSAGE);

    inject_envs_to_process(config);

    let prompt = format!("{} ", constants::INTERACTIVE_PROMPT.yellow());

    loop {
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim();

                if input.is_empty() {
                    continue;
                }

                if input.starts_with(constants::SHELL_PREFIX) {
                    let shell_cmd = &input[1..].trim();
                    if shell_cmd.is_empty() {
                        enter_interactive_shell(config);
                    } else {
                        execute_shell_command(shell_cmd, config);
                    }
                    let _ = rl.add_history_entry(input);
                    println!();
                    continue;
                }

                let args = parse_input(input);
                if args.is_empty() {
                    continue;
                }

                let args: Vec<String> = args.iter().map(|a| expand_env_vars(a)).collect();

                let verbose = config.is_verbose();
                let start = if verbose {
                    Some(std::time::Instant::now())
                } else {
                    None
                };

                let is_report_cmd = !args.is_empty() && cmd::REPORT.contains(&args[0].as_str());
                if !is_report_cmd {
                    let _ = rl.add_history_entry(input);
                }

                execute_interactive_command(&args, config);

                if let Some(start) = start {
                    let elapsed = start.elapsed();
                    crate::debug_log!(config, "duration: {} ms", elapsed.as_millis());
                }

                if let Some(helper) = rl.helper_mut() {
                    helper.refresh(config);
                }
                inject_envs_to_process(config);

                println!();
            }
            Err(ReadlineError::Interrupted) => {
                info!("\nProgram interrupted. Use 'exit' to quit.");
            }
            Err(ReadlineError::Eof) => {
                info!("\nGoodbye! 👋");
                break;
            }
            Err(err) => {
                error!("读取输入失败: {:?}", err);
                break;
            }
        }
    }

    let _ = rl.save_history(&history_path);
}

/// 获取历史文件路径: ~/.jdata/history.txt
fn history_file_path() -> std::path::PathBuf {
    let data_dir = crate::config::YamlConfig::data_dir();
    let _ = std::fs::create_dir_all(&data_dir);
    data_dir.join(constants::HISTORY_FILE)
}

/// 解析用户输入为参数列表（支持双引号包裹带空格的参数）
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
