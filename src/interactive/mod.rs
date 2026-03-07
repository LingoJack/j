pub mod completer;
pub mod parser;
pub mod shell;

use crate::command::voice::do_voice_record_for_interactive;
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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// ========== Voice 快捷键状态 ==========

/// Ctrl+V 语音输入的共享状态
struct VoiceState {
    /// 是否由 Ctrl+V 触发（区分 Ctrl+C）
    triggered: AtomicBool,
    /// 触发时保存的行内容
    saved_line: Mutex<String>,
    /// 触发时保存的光标位置
    saved_pos: Mutex<usize>,
}

impl VoiceState {
    fn new() -> Self {
        Self {
            triggered: AtomicBool::new(false),
            saved_line: Mutex::new(String::new()),
            saved_pos: Mutex::new(0),
        }
    }

    fn reset(&self) {
        self.triggered.store(false, Ordering::SeqCst);
        *self.saved_line.lock().unwrap() = String::new();
        *self.saved_pos.lock().unwrap() = 0;
    }
}

/// Ctrl+V 按键处理器
struct VoiceKeyHandler {
    state: Arc<VoiceState>,
}

impl rustyline::ConditionalEventHandler for VoiceKeyHandler {
    fn handle(
        &self,
        _evt: &rustyline::Event,
        _n: rustyline::RepeatCount,
        _positive: bool,
        ctx: &rustyline::EventContext,
    ) -> Option<Cmd> {
        // 保存当前行内容和光标位置
        *self.state.saved_line.lock().unwrap() = ctx.line().to_string();
        *self.state.saved_pos.lock().unwrap() = ctx.pos();
        self.state.triggered.store(true, Ordering::SeqCst);
        // 返回 Interrupt 跳出 readline
        Some(Cmd::Interrupt)
    }
}

// ========== 交互模式主循环 ==========

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

    // 绑定 Ctrl+V 到语音输入处理器
    let voice_state = Arc::new(VoiceState::new());
    let handler = VoiceKeyHandler {
        state: voice_state.clone(),
    };
    rl.bind_sequence(
        KeyEvent(KeyCode::Char('v'), Modifiers::CTRL),
        EventHandler::Conditional(Box::new(handler)),
    );

    let history_path = history_file_path();
    let _ = rl.load_history(&history_path);

    info!("{}", constants::WELCOME_MESSAGE);

    inject_envs_to_process(config);

    let prompt = format!("{} ", constants::INTERACTIVE_PROMPT.yellow());

    loop {
        // 每次循环重置 voice 状态
        voice_state.reset();

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
                if voice_state.triggered.load(Ordering::SeqCst) {
                    // Ctrl+V 触发的语音输入
                    let saved_line = voice_state.saved_line.lock().unwrap().clone();
                    let saved_pos = *voice_state.saved_pos.lock().unwrap();

                    println!();
                    let text = do_voice_record_for_interactive();

                    if !text.is_empty() {
                        // 将转写文字插入到光标位置
                        let left = &saved_line[..saved_pos];
                        let right = &saved_line[saved_pos..];
                        let new_left = format!("{}{}", left, text);

                        // 用 readline_with_initial 回填
                        match rl.readline_with_initial(&prompt, (&new_left, right)) {
                            Ok(line) => {
                                let input = line.trim();
                                if !input.is_empty() {
                                    let args = parse_input(input);
                                    if !args.is_empty() {
                                        let args: Vec<String> =
                                            args.iter().map(|a| expand_env_vars(a)).collect();
                                        let is_report_cmd = !args.is_empty()
                                            && cmd::REPORT.contains(&args[0].as_str());
                                        if !is_report_cmd {
                                            let _ = rl.add_history_entry(input);
                                        }
                                        execute_interactive_command(&args, config);
                                        if let Some(helper) = rl.helper_mut() {
                                            helper.refresh(config);
                                        }
                                        inject_envs_to_process(config);
                                    }
                                }
                                println!();
                            }
                            Err(ReadlineError::Interrupted) => {
                                // readline_with_initial 被 Ctrl+C 中断
                                // 检查是否又触发了 Ctrl+V
                                if voice_state.triggered.load(Ordering::SeqCst) {
                                    // 嵌套 voice 不处理，简单忽略
                                }
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
                    } else {
                        // 录音无结果，恢复之前的输入
                        if !saved_line.is_empty() {
                            match rl.readline_with_initial(&prompt, (&saved_line, "")) {
                                Ok(line) => {
                                    let input = line.trim();
                                    if !input.is_empty() {
                                        let args = parse_input(input);
                                        if !args.is_empty() {
                                            let args: Vec<String> =
                                                args.iter().map(|a| expand_env_vars(a)).collect();
                                            let is_report_cmd = !args.is_empty()
                                                && cmd::REPORT.contains(&args[0].as_str());
                                            if !is_report_cmd {
                                                let _ = rl.add_history_entry(input);
                                            }
                                            execute_interactive_command(&args, config);
                                            if let Some(helper) = rl.helper_mut() {
                                                helper.refresh(config);
                                            }
                                            inject_envs_to_process(config);
                                        }
                                    }
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
                    }
                } else {
                    info!("\nProgram interrupted. Use 'exit' to quit.");
                }
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
