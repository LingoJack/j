use crate::command::chat::agent::config::{AgentLoopConfig, AgentLoopSharedState};
use crate::command::chat::app::AskRequest;
use crate::command::chat::app::MainAgentHandle;
use crate::command::chat::app::build_system_prompt_fn;
use crate::command::chat::app::types::{PlanDecision, StreamMsg, ToolResultMsg};
use crate::command::chat::constants::{THINKING_PULSE_MIN_FACTOR, THINKING_PULSE_PERIOD_MS};
use crate::command::chat::context::compact::new_invoked_skills_map;
use crate::command::chat::context::window::select_messages;
use crate::command::chat::handler::run_chat_tui;
use crate::command::chat::infra::hook::{HookContext, HookEvent, HookManager};
use crate::command::chat::infra::skill;
use crate::command::chat::permission::{JcliConfig, generate_allow_rule};
use crate::command::chat::storage::config::ThinkingStyle;
use crate::command::chat::storage::{
    AgentConfig, ChatMessage, MessageRole, ModelProvider, SessionEvent, ToolCallItem,
    append_session_event, find_latest_session_id, load_agent_config, load_session,
};
use crate::command::chat::tools::ToolRegistry;
use crate::command::chat::tools::background::BackgroundManager;
use crate::command::chat::tools::classification::{ToolCategory, get_result_summary_for_tool};
use crate::command::chat::tools::task::TaskManager;
use crate::command::chat::tools::todo::TodoManager;
use crate::config::YamlConfig;
use crate::theme::Theme;
use crate::{error, info};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ─────────────────────────────────────────────────────────────
//  oneshot UI 辅助函数（与 TUI render/cache.rs 对齐）
// ─────────────────────────────────────────────────────────────

/// 工具调用参数最大预览长度（与 TUI TOOL_ARG_PREVIEW_MAX_CHARS 对齐）
const TOOL_ARG_PREVIEW_MAX_CHARS: usize = 60;

/// 从工具调用参数 JSON 中提取描述信息
fn extract_tool_desc(tool_name: &str, arguments: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;
    match tool_name {
        "Bash" | "Shell" => parsed.get("description")?.as_str().map(|s| s.to_string()),
        "Read" | "Write" | "Edit" | "Glob" | "Grep" => parsed
            .get("path")
            .or_else(|| parsed.get("file_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "Agent" | "AgentTeam" => parsed
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "Ask" => parsed
            .get("header")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

/// 从 Bash 参数中提取命令
fn extract_bash_command(arguments: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;
    parsed
        .get("command")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// 生成截断后的参数预览
fn make_args_preview(arguments: &str) -> String {
    let total_len = arguments.chars().count();
    if total_len <= TOOL_ARG_PREVIEW_MAX_CHARS {
        return arguments.to_string();
    }
    let closing_bracket = arguments.chars().next().and_then(|c| match c {
        '{' => Some('}'),
        '[' => Some(']'),
        _ => None,
    });
    let preview_len = if closing_bracket.is_some() {
        TOOL_ARG_PREVIEW_MAX_CHARS - 4
    } else {
        TOOL_ARG_PREVIEW_MAX_CHARS
    };
    let preview: String = arguments.chars().take(preview_len).collect();
    if let Some(bracket) = closing_bracket {
        format!("{}...{}", preview, bracket)
    } else {
        format!("{}…", preview)
    }
}

/// 获取终端宽度
fn term_width() -> usize {
    crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)
}

/// 计算交互框宽度
fn box_width() -> usize {
    term_width().saturating_sub(4).clamp(20, 56)
}

/// 将 ratatui Color 映射为 colored 可用的颜色字符串
fn ratatui_color_to_colored(color: ratatui::style::Color) -> String {
    use ratatui::style::Color;
    match color {
        Color::Rgb(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b),
        Color::Blue => "blue".to_string(),
        Color::Cyan => "cyan".to_string(),
        Color::Green => "green".to_string(),
        Color::Yellow => "yellow".to_string(),
        Color::Red => "red".to_string(),
        Color::Magenta => "magenta".to_string(),
        Color::White => "white".to_string(),
        Color::DarkGray => "bright black".to_string(),
        Color::LightBlue => "bright blue".to_string(),
        Color::LightCyan => "bright cyan".to_string(),
        Color::LightGreen => "bright green".to_string(),
        Color::LightYellow => "bright yellow".to_string(),
        Color::LightRed => "bright red".to_string(),
        Color::LightMagenta => "bright magenta".to_string(),
        _ => "white".to_string(),
    }
}

/// 思考动画脉冲颜色（与 TUI thinking_pulse_color 对齐）
fn thinking_pulse_color_rgb() -> (u8, u8, u8) {
    use std::time::{SystemTime, UNIX_EPOCH};

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let period = THINKING_PULSE_PERIOD_MS as f64;
    let phase = (millis % period as u128) as f64 / period;
    let t = (phase * std::f64::consts::TAU).sin() * 0.5 + 0.5;

    let theme = Theme::terminal();
    if let ratatui::style::Color::Rgb(r, g, b) = theme.label_ai {
        let min_factor = THINKING_PULSE_MIN_FACTOR;
        let factor = min_factor + (1.0 - min_factor) * t;
        (
            (r as f64 * factor).round().min(255.0) as u8,
            (g as f64 * factor).round().min(255.0) as u8,
            (b as f64 * factor).round().min(255.0) as u8,
        )
    } else {
        (120, 220, 160) // 默认 label_ai 色
    }
}

/// 打印工具调用行（与 TUI 折叠模式对齐: `  {icon} {tool_name}  {desc}`）
fn print_tool_call_line(tool_name: &str, arguments: &str) {
    use colored::Colorize;

    let category = ToolCategory::from_name(tool_name);
    let icon = category.icon();
    let theme = Theme::terminal();
    let tool_color = ratatui_color_to_colored(category.color(&theme));

    let desc = if let Some(d) = extract_tool_desc(tool_name, arguments) {
        d
    } else if !arguments.is_empty() {
        make_args_preview(arguments)
    } else {
        String::new()
    };

    let desc_colored = if desc.is_empty() {
        String::new()
    } else {
        format!("  {}", desc.dimmed())
    };

    eprintln!(
        "  {} {} {}",
        icon,
        tool_name.color(tool_color).bold(),
        desc_colored
    );
}

/// 打印工具执行结果行（与 TUI tool_result 对齐）
fn print_tool_result_line(tool_name: &str, is_error: bool, summary: &str, elapsed: &str) {
    use colored::Colorize;

    let category = ToolCategory::from_name(tool_name);
    let theme = Theme::terminal();
    let tool_color = ratatui_color_to_colored(category.color(&theme));

    let status_icon = if is_error { "✗" } else { "✓" };
    let status_style = if is_error { "red" } else { "green" };

    eprintln!(
        "  🔧 {} {}{} {}",
        tool_name.color(tool_color).bold(),
        status_icon.color(status_style),
        summary.dimmed(),
        elapsed.dimmed(),
    );
}

/// 启动思考动画线程，返回停止标志
fn start_thinking_animation(thinking_style: ThinkingStyle) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);

    std::thread::spawn(move || {
        use colored::Colorize;
        use crossterm::{cursor, execute, terminal};

        let mut tick: u64 = 0;
        let mut stdout = io::stdout();

        while !stop_clone.load(Ordering::Relaxed) {
            let (r, g, b) = thinking_pulse_color_rgb();
            let frame = thinking_style.frame(tick);
            let line = format!("  {} 思考中...", frame);
            let colored_line = line.truecolor(r, g, b).bold().to_string();

            let _ = execute!(
                stdout,
                cursor::MoveToColumn(0),
                terminal::Clear(terminal::ClearType::CurrentLine),
                crossterm::style::Print(&colored_line),
            );
            let _ = stdout.flush();

            tick += 1;
            std::thread::sleep(Duration::from_millis(100));
        }

        // 清除动画行
        let _ = execute!(
            stdout,
            cursor::MoveToColumn(0),
            terminal::Clear(terminal::ClearType::CurrentLine),
        );
        let _ = stdout.flush();
    });

    stop
}

/// 停止思考动画并清除行
fn stop_thinking_animation(stop_flag: &Arc<AtomicBool>) {
    stop_flag.store(true, Ordering::Relaxed);
    // 给动画线程时间清除行
    std::thread::sleep(Duration::from_millis(50));
}

// ─────────────────────────────────────────────────────────────

fn generate_oneshot_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    let pid = std::process::id();
    format!("{:x}-{:x}", ts, pid)
}

fn persist_messages(session_id: &str, messages: &[ChatMessage], start_idx: usize) {
    for msg in messages.iter().skip(start_idx) {
        append_session_event(session_id, &SessionEvent::msg(msg.clone()));
    }
}

/// 处理 chat 子命令入口
pub fn handle_chat(
    content: &[String],
    cont: bool,
    session_id_opt: Option<&str>,
    remote: bool,
    port: u16,
    bypass: bool,
    _config: &YamlConfig,
) {
    let agent_config = load_agent_config();

    if remote
        || content.is_empty() && !cont && session_id_opt.is_none()
        || agent_config.providers.is_empty()
    {
        run_chat_tui(remote, port);
        return;
    }

    let message = content.join(" ");
    let message = message.trim().to_string();
    if message.is_empty() && !cont && session_id_opt.is_none() {
        error!("消息内容为空");
        return;
    }
    if message.is_empty() {
        error!("消息内容为空（--continue / --session 需要附带消息内容）");
        return;
    }

    let session_id = if let Some(id) = session_id_opt {
        id.to_string()
    } else if cont {
        find_latest_session_id().unwrap_or_else(generate_oneshot_session_id)
    } else {
        generate_oneshot_session_id()
    };

    let prior_messages = if cont || session_id_opt.is_some() {
        let loaded = load_session(&session_id).messages;
        if !loaded.is_empty() {
            info!("延续会话 {} （{} 条历史消息）", session_id, loaded.len());
        } else if session_id_opt.is_some() {
            info!("会话 {} 不存在或为空，开始新对话", session_id);
        }
        loaded
    } else {
        vec![]
    };

    let idx = agent_config
        .active_index
        .min(agent_config.providers.len() - 1);
    let provider = &agent_config.providers[idx];

    if agent_config.tools_enabled {
        run_oneshot_agent(
            provider,
            &agent_config,
            message,
            prior_messages,
            &session_id,
            bypass,
        );
    } else {
        run_oneshot_no_tools(
            provider,
            &agent_config,
            message,
            prior_messages,
            &session_id,
        );
    }
}

/// 无工具模式
fn run_oneshot_no_tools(
    provider: &ModelProvider,
    agent_config: &AgentConfig,
    message: String,
    prior_messages: Vec<ChatMessage>,
    session_id: &str,
) {
    use crate::command::chat::agent::api::call_llm_stream;
    use colored::Colorize;

    let user_msg = ChatMessage::text(MessageRole::User, message.clone());
    let mut messages = prior_messages.clone();
    messages.push(user_msg.clone());

    let thinking_style = agent_config.thinking_style;
    let _stop_anim = start_thinking_animation(thinking_style);

    let tw = term_width();
    let mut cur_col: usize = 0;
    let mut raw_lines: usize = 0;
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted2 = Arc::clone(&interrupted);
    let _ = ctrlc::set_handler(move || {
        interrupted2.store(true, Ordering::Relaxed);
    });

    let send_messages = select_messages(
        &messages,
        agent_config.max_history_messages,
        agent_config.max_context_tokens,
        agent_config.compact.keep_recent,
        &agent_config.compact.micro_compact_exempt_tools,
    );

    match call_llm_stream(
        provider,
        &send_messages,
        crate::command::chat::storage::load_system_prompt().as_deref(),
        &mut |chunk| {
            if interrupted.load(Ordering::Relaxed) {
                return;
            }
            print!("{}", chunk);
            let _ = io::stdout().flush();
            for ch in chunk.chars() {
                if ch == '\n' {
                    raw_lines += 1;
                    cur_col = 0;
                } else {
                    cur_col += 1;
                    if cur_col >= tw {
                        raw_lines += 1;
                        cur_col = 0;
                    }
                }
            }
        },
    ) {
        Ok(full_text) => {
            if !full_text.is_empty() {
                redraw_markdown(raw_lines, cur_col, &full_text);
                persist_messages(session_id, &[user_msg], 0);
                persist_messages(
                    session_id,
                    &[ChatMessage::text(MessageRole::Assistant, &full_text)],
                    0,
                );
                eprintln!("{} {}", "会话 ID:".dimmed(), session_id.dimmed());
            }
        }
        Err(e) => {
            error!("\n{}", e.display_message());
        }
    }
}

/// 回退 raw 文本，用 markdown 重绘
fn redraw_markdown(raw_lines: usize, cur_col: usize, text: &str) {
    use crossterm::{cursor, execute, terminal};
    let total_raw_lines = if cur_col > 0 {
        raw_lines + 1
    } else {
        raw_lines
    };
    let mut stdout = io::stdout();
    if total_raw_lines > 0 {
        let _ = execute!(stdout, cursor::MoveToColumn(0));
        if total_raw_lines > 1 {
            let _ = execute!(stdout, cursor::MoveUp((total_raw_lines - 1) as u16));
        }
        let _ = execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));
    }
    crate::util::md_render::render_md(text);
}

/// 交互式工具确认（crossterm raw mode + `┌──┐` 直角边框）
fn interactive_confirm(
    tool_name: &str,
    arguments: &str,
    options: &[&str],
    initial: usize,
) -> Option<usize> {
    use colored::Colorize;
    use crossterm::event::{self, Event, KeyCode};
    use crossterm::{cursor, execute, terminal};

    let bw = box_width();
    let category = ToolCategory::from_name(tool_name);
    let icon = category.icon();
    let theme = Theme::terminal();
    let border_color_str = ratatui_color_to_colored(theme.tool_confirm_border);

    let mut stdout = io::stdout();
    let mut cursor_pos = initial;

    // 参数描述
    let desc = if tool_name == "Bash" || tool_name == "Shell" {
        if let Some(cmd) = extract_bash_command(arguments) {
            format!("$ {}", cmd)
        } else {
            make_args_preview(arguments)
        }
    } else {
        make_args_preview(arguments)
    };

    // 布局: 顶边框 1 + 描述 1 + 空行 1 + 每选项 1 + 空行 1 + 提示 1 + 底边框 1
    let inner_height = 1 + 1 + options.len() + 1 + 1;
    let total_lines = (inner_height + 2) as u16; // +2 for top/bottom border

    let draw = |stdout: &mut io::Stdout,
                cursor_pos: usize,
                first: bool,
                total_lines: u16|
     -> io::Result<()> {
        if !first {
            let _ = execute!(stdout, cursor::MoveUp(total_lines));
        }
        let _ = execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));

        // 顶边框: ┌─ {icon} {tool_name} 需要确认 ─────┐
        let title = format!("{} {} 需要确认", icon, tool_name);
        let title_len = title.chars().count() + 2; // +2 for "─ " prefix
        let inner_w = bw.saturating_sub(2); // ┌ + ┐ = 2
        let dash_fill = inner_w.saturating_sub(title_len + 1); // +1 for trailing ┐

        let top_border = format!(
            "  {} {}{}{}",
            "┌",
            "─".color(border_color_str.clone()),
            format!(" {} ", title)
                .color(border_color_str.clone())
                .bold(),
            format!("{}┐", "─".repeat(dash_fill)).color(border_color_str.clone()),
        );
        writeln!(stdout, "{}\r", top_border)?;

        // 描述行
        let desc_display = format!(
            "  │  {}",
            if desc.len() > bw.saturating_sub(8) {
                format!("{}...", &desc[..bw.saturating_sub(11)])
            } else {
                desc.clone()
            }
        )
        .color(border_color_str.clone())
        .to_string();
        writeln!(stdout, "{}\r", desc_display)?;

        // 空行
        writeln!(
            stdout,
            "{}\r",
            format!("  │{}", " ".repeat(bw.saturating_sub(4))).color(border_color_str.clone())
        )?;

        // 选项列表
        for (i, opt) in options.iter().enumerate() {
            let pointer = if cursor_pos == i { "❯" } else { " " };
            let content = format!("  │  {} {}", pointer, opt);
            if cursor_pos == i {
                writeln!(stdout, "{}\r", content.cyan().bold())?;
            } else {
                writeln!(stdout, "{}\r", content.dimmed())?;
            }
        }

        // 空行
        writeln!(
            stdout,
            "{}\r",
            format!("  │{}", " ".repeat(bw.saturating_sub(4))).color(border_color_str.clone())
        )?;

        // 操作提示
        writeln!(
            stdout,
            "{}\r",
            format!("  │  {} ↑↓ 移动  {} 确认", "•", "Enter").dimmed()
        )?;

        // 底边框
        writeln!(
            stdout,
            "{}\r",
            format!("  └{}┘", "─".repeat(bw.saturating_sub(3))).color(border_color_str.clone())
        )?;

        stdout.flush()?;
        Ok(())
    };

    if terminal::enable_raw_mode().is_err() {
        return None;
    }

    let _ = draw(&mut stdout, cursor_pos, true, total_lines);

    let result = loop {
        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    cursor_pos = cursor_pos.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if cursor_pos + 1 < options.len() {
                        cursor_pos += 1;
                    }
                }
                KeyCode::Enter => break Some(cursor_pos),
                KeyCode::Esc | KeyCode::Char('q') => break None,
                _ => continue,
            }
            let _ = draw(&mut stdout, cursor_pos, false, total_lines);
        }
    };

    let _ = terminal::disable_raw_mode();
    {
        let _ = execute!(stdout, cursor::MoveUp(total_lines));
        let _ = execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));
    }
    result
}

/// 流式文本回退 + markdown 重绘
fn redraw_streaming_as_markdown(
    streaming_content: &Arc<Mutex<String>>,
    raw_lines: &mut usize,
    cur_col: &mut usize,
) {
    let content = streaming_content.lock().unwrap();
    if content.is_empty() {
        return;
    }
    let tw = term_width();
    let mut rl: usize = 0;
    let mut cc: usize = 0;
    for ch in content.chars() {
        if ch == '\n' {
            rl += 1;
            cc = 0;
        } else {
            cc += 1;
            if cc >= tw {
                rl += 1;
                cc = 0;
            }
        }
    }
    *raw_lines = rl;
    *cur_col = cc;
    redraw_markdown(*raw_lines, *cur_col, &content);
}

fn run_oneshot_agent(
    provider: &ModelProvider,
    agent_config: &AgentConfig,
    message: String,
    prior_messages: Vec<ChatMessage>,
    session_id: &str,
    bypass: bool,
) {
    use colored::Colorize;

    let thinking_style = agent_config.thinking_style;

    let hook_manager_loaded = HookManager::load();
    let hook_manager_for_end = hook_manager_loaded.clone();
    let disabled_hooks: Vec<String> = vec![];

    // ★ SessionStart hook
    {
        if hook_manager_loaded.has_hooks_for(HookEvent::SessionStart) {
            let ctx = HookContext {
                event: HookEvent::SessionStart,
                messages: Some(prior_messages.clone()),
                model: Some(provider.model.clone()),
                session_id: Some(session_id.to_string()),
                ..Default::default()
            };
            hook_manager_loaded.execute(HookEvent::SessionStart, ctx, &disabled_hooks);
        }
    }

    let (ask_tx, ask_rx) = std::sync::mpsc::channel::<AskRequest>();
    let background_manager = Arc::new(BackgroundManager::new());
    let task_manager = Arc::new(TaskManager::new_with_session(session_id));
    let todo_manager = Arc::new(TodoManager::new());
    let hook_manager_for_registry = hook_manager_loaded.clone();
    let invoked_skills = new_invoked_skills_map();

    let tool_registry = Arc::new(ToolRegistry::new(
        vec![],
        ask_tx,
        Arc::clone(&background_manager),
        Arc::clone(&task_manager),
        Arc::new(Mutex::new(hook_manager_for_registry)),
        invoked_skills.clone(),
        crate::command::chat::storage::SessionPaths::new(session_id).todos_file(),
    ));

    // ── Ask 请求处理线程（┌──┐ 直角边框样式）──
    std::thread::spawn(move || {
        use colored::Colorize;
        use crossterm::event::{self, Event, KeyCode};
        use crossterm::{cursor, execute, terminal};

        while let Ok(req) = ask_rx.recv() {
            let mut answers = serde_json::Map::new();
            for q in &req.questions {
                let bw = box_width();
                let theme = Theme::terminal();
                let border_color_str = ratatui_color_to_colored(theme.tool_confirm_border);

                // ── 标题行 ──
                let title = if !q.header.is_empty() {
                    q.header.clone()
                } else {
                    "选择".to_string()
                };

                // 计算布局行数
                let question_lines = if q.question.is_empty() {
                    0
                } else {
                    let max_chars = bw.saturating_sub(6);
                    (q.question.chars().count() + max_chars - 1) / max_chars.max(1)
                };
                // 顶边框 1 + 问题区 question_lines + 空行 1 + 每选项 2行(label+desc) + 间隔 0 + 空行 1 + 提示 1 + 底边框 1
                let options_height = q.options.len() * 2;
                let inner_height = 1 + question_lines + 1 + options_height + 1 + 1;
                let total_lines = (inner_height + 2) as u16;

                if q.multi_select {
                    // ── 多选 ──
                    let mut selected = vec![false; q.options.len()];
                    let mut cursor_pos: usize = 0;

                    let draw_multi = |stdout: &mut io::Stdout,
                                      cursor_pos: usize,
                                      selected: &[bool],
                                      first: bool,
                                      total_lines: u16|
                     -> io::Result<()> {
                        if !first {
                            let _ = execute!(stdout, cursor::MoveUp(total_lines));
                        }
                        let _ =
                            execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));

                        // 顶边框
                        let title_text = format!(" {} ", title);
                        let title_display_len = title.chars().count() + 2;
                        let inner_w = bw.saturating_sub(2);
                        let dash_fill = inner_w.saturating_sub(title_display_len + 1);
                        let top_line = format!(
                            "  ┌{}{}{}┐",
                            "─".color(border_color_str.clone()),
                            title_text.color(border_color_str.clone()).bold(),
                            format!("{}─", "─".repeat(dash_fill)).color(border_color_str.clone()),
                        );
                        writeln!(stdout, "{}\r", top_line)?;

                        // 问题区域
                        if !q.question.is_empty() {
                            let max_chars = bw.saturating_sub(6);
                            let chars: Vec<char> = q.question.chars().collect();
                            let mut start = 0;
                            while start < chars.len() {
                                let end = (start + max_chars).min(chars.len());
                                let line: String = chars[start..end].iter().collect();
                                writeln!(
                                    stdout,
                                    "{}\r",
                                    format!("  │  {}", line).color(border_color_str.clone())
                                )?;
                                start = end;
                            }
                        }

                        // 空行
                        writeln!(
                            stdout,
                            "{}\r",
                            format!("  │{}", " ".repeat(bw.saturating_sub(4)))
                                .color(border_color_str.clone())
                        )?;

                        // 选项列表
                        for (i, opt) in q.options.iter().enumerate() {
                            let pointer = if cursor_pos == i { "❯" } else { " " };
                            let check = if selected[i] { "◉" } else { "○" };
                            let label_line = format!("  │  {} {} {}", pointer, check, opt.label);
                            let desc_line = format!("  │    {}", opt.description);

                            if cursor_pos == i {
                                writeln!(stdout, "{}\r", label_line.cyan().bold())?;
                                writeln!(stdout, "{}\r", desc_line.dimmed())?;
                            } else {
                                writeln!(stdout, "{}\r", label_line.dimmed())?;
                                writeln!(stdout, "{}\r", desc_line.dimmed())?;
                            }
                        }

                        // 空行
                        writeln!(
                            stdout,
                            "{}\r",
                            format!("  │{}", " ".repeat(bw.saturating_sub(4)))
                                .color(border_color_str.clone())
                        )?;

                        // 操作提示
                        writeln!(
                            stdout,
                            "{}\r",
                            format!("  │  {} ↑↓ 移动  {} 切换  {} 确认", "•", "Space", "Enter")
                                .dimmed()
                        )?;

                        // 底边框
                        writeln!(
                            stdout,
                            "{}\r",
                            format!("  └{}┘", "─".repeat(bw.saturating_sub(3)))
                                .color(border_color_str.clone())
                        )?;

                        stdout.flush()?;
                        Ok(())
                    };

                    let _ = terminal::enable_raw_mode();
                    let mut stdout = io::stdout();
                    let _ = draw_multi(&mut stdout, cursor_pos, &selected, true, total_lines);

                    loop {
                        if let Ok(Event::Key(key)) = event::read() {
                            match key.code {
                                KeyCode::Up | KeyCode::Char('k') => {
                                    cursor_pos = cursor_pos.saturating_sub(1);
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if cursor_pos + 1 < q.options.len() {
                                        cursor_pos += 1;
                                    }
                                }
                                KeyCode::Char(' ') => {
                                    selected[cursor_pos] = !selected[cursor_pos];
                                }
                                KeyCode::Enter => break,
                                KeyCode::Esc => break,
                                _ => continue,
                            }
                            let _ =
                                draw_multi(&mut stdout, cursor_pos, &selected, false, total_lines);
                        }
                    }
                    let _ = terminal::disable_raw_mode();
                    let _ = execute!(stdout, cursor::MoveUp(total_lines));
                    let _ = execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));

                    let result: Vec<String> = q
                        .options
                        .iter()
                        .zip(selected.iter())
                        .filter(|(_, s)| **s)
                        .map(|(o, _)| o.label.clone())
                        .collect();
                    let answer = if result.is_empty() {
                        "(无选择)".to_string()
                    } else {
                        result.join(", ")
                    };
                    println!("  → {}", answer.green());
                    answers.insert(q.header.clone(), serde_json::Value::String(answer));
                } else {
                    // ── 单选 ──
                    let mut cursor_pos: usize = 0;

                    let draw_single = |stdout: &mut io::Stdout,
                                       cursor_pos: usize,
                                       first: bool,
                                       total_lines: u16|
                     -> io::Result<()> {
                        if !first {
                            let _ = execute!(stdout, cursor::MoveUp(total_lines));
                        }
                        let _ =
                            execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));

                        // 顶边框
                        let title_text = format!(" {} ", title);
                        let title_display_len = title.chars().count() + 2;
                        let inner_w = bw.saturating_sub(2);
                        let dash_fill = inner_w.saturating_sub(title_display_len + 1);
                        let top_line = format!(
                            "  ┌{}{}{}┐",
                            "─".color(border_color_str.clone()),
                            title_text.color(border_color_str.clone()).bold(),
                            format!("{}─", "─".repeat(dash_fill)).color(border_color_str.clone()),
                        );
                        writeln!(stdout, "{}\r", top_line)?;

                        // 问题区域
                        if !q.question.is_empty() {
                            let max_chars = bw.saturating_sub(6);
                            let chars: Vec<char> = q.question.chars().collect();
                            let mut start = 0;
                            while start < chars.len() {
                                let end = (start + max_chars).min(chars.len());
                                let line: String = chars[start..end].iter().collect();
                                writeln!(
                                    stdout,
                                    "{}\r",
                                    format!("  │  {}", line).color(border_color_str.clone())
                                )?;
                                start = end;
                            }
                        }

                        // 空行
                        writeln!(
                            stdout,
                            "{}\r",
                            format!("  │{}", " ".repeat(bw.saturating_sub(4)))
                                .color(border_color_str.clone())
                        )?;

                        // 选项列表
                        for (i, opt) in q.options.iter().enumerate() {
                            let pointer = if cursor_pos == i { "❯" } else { " " };
                            let label_line = format!("  │  {} {}", pointer, opt.label);
                            let desc_line = format!("  │    {}", opt.description);

                            if cursor_pos == i {
                                writeln!(stdout, "{}\r", label_line.cyan().bold())?;
                                writeln!(stdout, "{}\r", desc_line.dimmed())?;
                            } else {
                                writeln!(stdout, "{}\r", label_line.dimmed())?;
                                writeln!(stdout, "{}\r", desc_line.dimmed())?;
                            }
                        }

                        // 空行
                        writeln!(
                            stdout,
                            "{}\r",
                            format!("  │{}", " ".repeat(bw.saturating_sub(4)))
                                .color(border_color_str.clone())
                        )?;

                        // 操作提示
                        writeln!(
                            stdout,
                            "{}\r",
                            format!("  │  {} ↑↓ 移动  {} 确认", "•", "Enter").dimmed()
                        )?;

                        // 底边框
                        writeln!(
                            stdout,
                            "{}\r",
                            format!("  └{}┘", "─".repeat(bw.saturating_sub(3)))
                                .color(border_color_str.clone())
                        )?;

                        stdout.flush()?;
                        Ok(())
                    };

                    let _ = terminal::enable_raw_mode();
                    let mut stdout = io::stdout();
                    let _ = draw_single(&mut stdout, cursor_pos, true, total_lines);

                    loop {
                        if let Ok(Event::Key(key)) = event::read() {
                            match key.code {
                                KeyCode::Up | KeyCode::Char('k') => {
                                    cursor_pos = cursor_pos.saturating_sub(1);
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if cursor_pos + 1 < q.options.len() {
                                        cursor_pos += 1;
                                    }
                                }
                                KeyCode::Enter => break,
                                KeyCode::Esc => break,
                                _ => continue,
                            }
                            let _ = draw_single(&mut stdout, cursor_pos, false, total_lines);
                        }
                    }
                    let _ = terminal::disable_raw_mode();
                    let _ = execute!(stdout, cursor::MoveUp(total_lines));
                    let _ = execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));

                    let answer = q
                        .options
                        .get(cursor_pos)
                        .map(|o| o.label.clone())
                        .unwrap_or_default();
                    println!("  → {}", answer.green());
                    answers.insert(q.header.clone(), serde_json::Value::String(answer));
                }
            }

            let response = serde_json::to_string(&serde_json::json!({ "answers": answers }))
                .unwrap_or_default();
            let _ = req.response_tx.send(response);
        }
    });

    // 构建消息
    let user_msg = ChatMessage::text(MessageRole::User, &message);
    let prior_len = prior_messages.len();
    let mut messages = prior_messages;
    messages.push(user_msg);

    let tools = tool_registry.to_llm_tools_filtered(&agent_config.disabled_tools);
    let loaded_skills = skill::load_all_skills();
    let system_prompt_fn = build_system_prompt_fn(
        loaded_skills,
        agent_config.disabled_skills.clone(),
        agent_config.disabled_tools.clone(),
        Arc::clone(&tool_registry),
    );

    let api_messages = select_messages(
        &messages,
        agent_config.max_history_messages,
        agent_config.max_context_tokens,
        agent_config.compact.keep_recent,
        &agent_config.compact.micro_compact_exempt_tools,
    );

    // 构造 AgentLoopConfig + AgentLoopSharedState
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let streaming_content: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let streaming_reasoning_content: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let pending_user_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
    let display_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
    let context_messages: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(vec![]));
    let estimated_context_tokens: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let derived_system_prompt: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    let agent_config_struct = AgentLoopConfig {
        provider: provider.clone(),
        max_llm_rounds: agent_config.max_tool_rounds,
        compact_config: agent_config.compact.clone(),
        hook_manager: hook_manager_loaded,
        disabled_hooks: agent_config.disabled_hooks.clone(),
        cancel_token: cancel_token.clone(),
    };
    let agent_shared = AgentLoopSharedState {
        streaming_content: Arc::clone(&streaming_content),
        streaming_reasoning_content: Arc::clone(&streaming_reasoning_content),
        pending_user_messages,
        background_manager,
        todo_manager,
        display_messages: Arc::clone(&display_messages),
        context_messages: Arc::clone(&context_messages),
        estimated_context_tokens,
        invoked_skills,
        session_id: session_id.to_string(),
        derived_system_prompt,
    };

    // Ctrl+C → cancel
    let cancel_for_ctrlc = cancel_token.clone();
    let _ = ctrlc::set_handler(move || {
        cancel_for_ctrlc.cancel();
    });

    // spawn agent loop
    let (handle, tool_result_tx) = MainAgentHandle::spawn(
        agent_config_struct,
        agent_shared,
        api_messages,
        tools,
        system_prompt_fn,
    );
    let tool_result_tx: std::sync::mpsc::SyncSender<ToolResultMsg> = tool_result_tx;

    // 启动思考动画
    let anim_stop = start_thinking_animation(thinking_style);
    let mut anim_running = true;

    // 消费循环
    let mut last_streaming_len: usize = 0;
    let mut raw_lines: usize = 0;
    let mut cur_col: usize = 0;
    let tw = term_width();
    let jcli_config = JcliConfig::load();
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut round: usize = 0;
    let mut first_content = true;

    loop {
        let msgs = handle.poll();
        if msgs.is_empty() {
            std::thread::sleep(Duration::from_millis(30));
            continue;
        }
        for msg in msgs {
            match msg {
                StreamMsg::Chunk => {
                    let content = streaming_content.lock().unwrap();
                    if content.len() > last_streaming_len {
                        // 停止思考动画（首次文本到来时）
                        if anim_running {
                            stop_thinking_animation(&anim_stop);
                            anim_running = false;
                        }
                        // 打印 AI 标签（首次文本到来时）
                        if first_content {
                            let theme = Theme::terminal();
                            let label_color = ratatui_color_to_colored(theme.label_ai);
                            eprintln!("  {}", "Sprite".color(label_color).bold());
                            first_content = false;
                        }

                        let delta = &content[last_streaming_len..];
                        print!("{}", delta);
                        let _ = io::stdout().flush();
                        for ch in delta.chars() {
                            if ch == '\n' {
                                raw_lines += 1;
                                cur_col = 0;
                            } else {
                                cur_col += 1;
                                if cur_col >= tw {
                                    raw_lines += 1;
                                    cur_col = 0;
                                }
                            }
                        }
                        last_streaming_len = content.len();
                    }
                }
                StreamMsg::ToolCallRequest(items) => {
                    // 停止思考动画
                    if anim_running {
                        stop_thinking_animation(&anim_stop);
                        anim_running = false;
                    }
                    // 先重绘已输出的流式文本
                    if last_streaming_len > 0 {
                        redraw_streaming_as_markdown(
                            &streaming_content,
                            &mut raw_lines,
                            &mut cur_col,
                        );
                        last_streaming_len = streaming_content.lock().unwrap().len();
                    }

                    round += 1;

                    // 轮次标题
                    eprintln!();
                    eprintln!("  {} R{} · {} 工具", "⚙".dimmed(), round, items.len());

                    // 逐个确认 + 执行 + 发送结果
                    for item in items.iter() {
                        let tool_result = handle_tool_call(
                            item,
                            tool_registry.as_ref(),
                            &jcli_config,
                            &cancelled,
                            bypass,
                        );
                        let _ = tool_result_tx.send(tool_result);
                    }

                    // 下一轮工具调用结束后，重置 first_content 标记
                    first_content = true;
                }
                StreamMsg::Done => {
                    if anim_running {
                        stop_thinking_animation(&anim_stop);
                    }
                    if last_streaming_len > 0 {
                        redraw_streaming_as_markdown(
                            &streaming_content,
                            &mut raw_lines,
                            &mut cur_col,
                        );
                    }
                    let ctx_msgs = context_messages.lock().unwrap();
                    let persist_from = if prior_len < ctx_msgs.len() {
                        prior_len
                    } else {
                        0
                    };
                    persist_messages(session_id, &ctx_msgs, persist_from);
                    if round > 0 {
                        eprintln!();
                    }
                    eprintln!("{} {}", "会话 ID:".dimmed(), session_id.dimmed());
                    fire_session_end(
                        &hook_manager_for_end,
                        &disabled_hooks,
                        &ctx_msgs,
                        session_id,
                        &provider.model,
                    );
                    return;
                }
                StreamMsg::Error(e) => {
                    if anim_running {
                        stop_thinking_animation(&anim_stop);
                    }
                    error!("\n{}", e.display_message());
                    let ctx_msgs = context_messages.lock().unwrap();
                    let persist_from = if prior_len < ctx_msgs.len() {
                        prior_len
                    } else {
                        0
                    };
                    persist_messages(session_id, &ctx_msgs, persist_from);
                    fire_session_end(
                        &hook_manager_for_end,
                        &disabled_hooks,
                        &ctx_msgs,
                        session_id,
                        &provider.model,
                    );
                    return;
                }
                StreamMsg::Cancelled => {
                    if anim_running {
                        stop_thinking_animation(&anim_stop);
                    }
                    println!();
                    let ctx_msgs = context_messages.lock().unwrap();
                    let persist_from = if prior_len < ctx_msgs.len() {
                        prior_len
                    } else {
                        0
                    };
                    persist_messages(session_id, &ctx_msgs, persist_from);
                    eprintln!("\n  {}", "⏹ 已中断".dimmed());
                    eprintln!("  {} {}", "会话 ID:".dimmed(), session_id.dimmed());
                    fire_session_end(
                        &hook_manager_for_end,
                        &disabled_hooks,
                        &ctx_msgs,
                        session_id,
                        &provider.model,
                    );
                    return;
                }
                StreamMsg::Retrying {
                    attempt,
                    max_attempts,
                    delay_ms,
                    error,
                } => {
                    if anim_running {
                        stop_thinking_animation(&anim_stop);
                        anim_running = false;
                    }
                    eprintln!(
                        "  {} 重试中 ({}/{}, {}ms) — {}",
                        "⟳".yellow(),
                        attempt,
                        max_attempts,
                        delay_ms,
                        error.dimmed()
                    );
                }
                StreamMsg::Compacting => {
                    eprintln!("  {} 压缩上下文中...", "📦".dimmed());
                }
                StreamMsg::Compacted { messages_before } => {
                    eprintln!("  {} 已压缩 {} 条消息", "📦".dimmed(), messages_before);
                }
            }
        }
    }
}

/// 处理单个工具调用
fn handle_tool_call(
    item: &ToolCallItem,
    tool_registry: &ToolRegistry,
    jcli_config: &JcliConfig,
    cancelled: &Arc<AtomicBool>,
    bypass: bool,
) -> ToolResultMsg {
    use colored::Colorize;

    // .jcli deny 检查
    if jcli_config.is_denied(&item.name, &item.arguments) {
        eprintln!(
            "  {} {} — {}",
            "✗".red(),
            item.name.red().bold(),
            "被权限规则拒绝".red()
        );
        return ToolResultMsg {
            tool_call_id: item.id.clone(),
            result: "工具调用被拒绝（deny 规则匹配）".to_string(),
            is_error: true,
            images: vec![],
            plan_decision: PlanDecision::None,
        };
    }

    let needs_confirm = tool_registry
        .get(&item.name)
        .map(|t| t.requires_confirmation())
        .unwrap_or(false)
        && !jcli_config.is_allowed(&item.name, &item.arguments);

    if needs_confirm && !bypass {
        // 需要确认：先显示工具调用行
        print_tool_call_line(&item.name, &item.arguments);

        let allow_rule = generate_allow_rule(&item.name, &item.arguments);
        let options = ["允许执行", "拒绝", &format!("始终允许 ({})", allow_rule)];
        let choice = interactive_confirm(&item.name, &item.arguments, &options, 0);
        match choice {
            Some(0) => {}
            Some(2) => {
                // 始终允许
            }
            _ => {
                eprintln!(
                    "  {} {} — {}",
                    "⏭".dimmed(),
                    item.name.dimmed(),
                    "已跳过".dimmed()
                );
                return ToolResultMsg {
                    tool_call_id: item.id.clone(),
                    result: "用户拒绝执行该工具".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        }
    } else {
        // 无需确认：直接显示工具调用行
        print_tool_call_line(&item.name, &item.arguments);
    }

    let start = std::time::Instant::now();
    let result = tool_registry.execute(&item.name, &item.arguments, cancelled);
    let elapsed = start.elapsed();
    let elapsed_str = format_duration(elapsed);

    let summary = get_result_summary_for_tool(
        &result.output,
        result.is_error,
        &item.name,
        Some(&item.arguments),
    );

    print_tool_result_line(&item.name, result.is_error, &summary, &elapsed_str);

    ToolResultMsg {
        tool_call_id: item.id.clone(),
        result: result.output,
        is_error: result.is_error,
        images: vec![],
        plan_decision: PlanDecision::None,
    }
}

fn format_duration(d: std::time::Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}

/// 触发 SessionEnd hook
fn fire_session_end(
    hook_manager: &HookManager,
    disabled_hooks: &[String],
    messages: &[ChatMessage],
    session_id: &str,
    model: &str,
) {
    if hook_manager.has_hooks_for(HookEvent::SessionEnd) {
        let ctx = HookContext {
            event: HookEvent::SessionEnd,
            messages: Some(messages.to_vec()),
            model: Some(model.to_string()),
            session_id: Some(session_id.to_string()),
            ..Default::default()
        };
        hook_manager.execute(HookEvent::SessionEnd, ctx, disabled_hooks);
    }
}
