use super::completer::CopilotHelper;
use super::parser::execute_interactive_command;
use super::shell::{
    enter_interactive_shell, execute_shell_command, expand_env_vars, inject_envs_to_process,
};
use crate::config::YamlConfig;
use crate::constants::{
    HISTORY_FILE, INTERACTIVE_PROMPT, SHELL_PREFIX_CN, SHELL_PREFIX_EN, WELCOME_MESSAGE, cmd,
};
use crate::{error, info};
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{
    Cmd, CompletionType, Config, EditMode, Editor, EventHandler, KeyCode, KeyEvent, Modifiers,
};

pub fn run_interactive(config: &mut YamlConfig) {
    let rl_config = Config::builder()
        .completion_type(CompletionType::Circular)
        .edit_mode(EditMode::Emacs)
        .auto_add_history(false)
        .build();

    let helper = CopilotHelper::new(config);

    let mut rl: Editor<CopilotHelper, DefaultHistory> =
        Editor::with_config(rl_config).expect("无法初始化编辑器");
    rl.set_helper(Some(helper));

    rl.bind_sequence(
        KeyEvent(KeyCode::Tab, Modifiers::NONE),
        EventHandler::Simple(Cmd::Complete),
    );
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('q'), Modifiers::CTRL),
        EventHandler::Simple(Cmd::Interrupt),
    );

    let history_path = history_file_path();
    let _ = rl.load_history(&history_path);

    info!("{}", WELCOME_MESSAGE);

    inject_envs_to_process(config);

    let prompt = format!("{} ", INTERACTIVE_PROMPT.yellow());

    loop {
        match rl.readline(&prompt) {
            Ok(line) => {
                let input = line.trim();

                if input.is_empty() {
                    continue;
                }

                if input.starts_with(SHELL_PREFIX_EN) || input.starts_with(SHELL_PREFIX_CN) {
                    let shell_cmd = input.chars().skip(1).collect::<String>();
                    let shell_cmd = shell_cmd.trim();
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

                *config = crate::config::YamlConfig::load();

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
                info!("\nGoodbye! 👋");
                break;
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

fn history_file_path() -> std::path::PathBuf {
    let data_dir = crate::config::YamlConfig::data_dir();
    let _ = std::fs::create_dir_all(&data_dir);
    data_dir.join(HISTORY_FILE)
}

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
