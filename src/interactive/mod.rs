pub mod completer;
pub mod parser;
pub mod shell;

use crate::config::YamlConfig;
use crate::constants::{
    HISTORY_FILE, INTERACTIVE_PROMPT, SHELL_PREFIX_CN, SHELL_PREFIX_EN, WELCOME_MESSAGE, cmd,
};
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
    // Ctrl+Q 快捷退出
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

                // 每条命令执行前重新读取最新配置，保证内存与磁盘一致
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
                // 空行按 Escape 触发，直接退出
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

/// 获取历史文件路径: ~/.jdata/history.txt
fn history_file_path() -> std::path::PathBuf {
    let data_dir = crate::config::YamlConfig::data_dir();
    let _ = std::fs::create_dir_all(&data_dir);
    data_dir.join(HISTORY_FILE)
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
