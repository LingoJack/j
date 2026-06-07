use super::completer::CopilotHelper;
use super::parser::execute_interactive_command;
use super::shell::{
    enter_interactive_shell, execute_shell_command, expand_env_vars, inject_envs_to_process,
};
use crate::config::YamlConfig;
use crate::constants::{HISTORY_FILE, SHELL_PREFIX_CN, SHELL_PREFIX_EN, WELCOME_MESSAGE, cmd};
use crate::{error, info};
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{
    Cmd, CompletionType, Config, EditMode, Editor, EventHandler, KeyCode, KeyEvent, Modifiers,
};

/// 启动交互式命令行循环
pub fn run_interactive(config: &mut YamlConfig) {
    let rl_config = Config::builder()
        .completion_type(CompletionType::Circular)
        .edit_mode(EditMode::Emacs)
        .auto_add_history(false)
        .build();

    let helper = CopilotHelper::new(config);

    let mut rl: Editor<CopilotHelper, DefaultHistory> = Editor::with_config(rl_config)
        // SAFETY: 编辑器初始化失败仅可能发生在终端不支持等极罕见情况，
        // 此交互模式无法运行，panic 是合理的终止方式。
        .expect("无法初始化编辑器");
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

    loop {
        let cwd = format_cwd();
        println!("{}", format!("work dir: {}", cwd).dimmed());
        if let Some(helper) = rl.helper_mut() {
            helper.rotate_tip();
        }
        match rl.readline(&format!("{} ", "j >".yellow())) {
            Ok(line) => {
                let input = line.trim();

                if input.is_empty() {
                    println!();
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

                execute_interactive_command(&args, input, config);

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

/// cwd 显示最大字符数（超出时中间省略）
const CWD_MAX_LEN: usize = 30;

/// 格式化当前工作目录，HOME 缩写为 ~，过长时中间用 ... 省略
fn format_cwd() -> String {
    let path = std::env::current_dir()
        .map(|p| {
            let home = std::env::var("HOME").unwrap_or_default();
            if !home.is_empty() && p.starts_with(&home) {
                let rest = p.strip_prefix(&home).unwrap_or(&p);
                let rest_str = rest.display().to_string();
                if rest_str.is_empty() {
                    "~".to_string()
                } else {
                    format!("~/{}", rest_str.trim_start_matches('/'))
                }
            } else {
                format!("{}", p.display())
            }
        })
        .unwrap_or_else(|_| ".".to_string());

    if path.len() <= CWD_MAX_LEN {
        return path;
    }

    // 保留首尾，中间用 ... 连接
    let sep = "...";
    let keep = (CWD_MAX_LEN - sep.len()) / 2;
    let prefix: String = path.chars().take(keep).collect();
    let suffix: String = path.chars().skip(path.chars().count() - keep).collect();
    format!("{}{}{}", prefix, sep, suffix)
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
