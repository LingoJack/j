use super::app::{
    AppMode, TodoApp, TodoItem, handle_command_popup_mode, handle_confirm_cancel_input,
    handle_confirm_delete, handle_confirm_report, handle_help_mode, handle_input_mode,
    handle_normal_mode, load_todo_list, save_todo_list,
};
use super::constant::{
    CMD_ADD, CMD_LIST, FILTER_DONE, FILTER_DONE_SHORT, FILTER_UNDONE, FILTER_UNDONE_SHORT,
};
use super::ui::draw_ui;
use crate::config::YamlConfig;

/// Todo TUI 事件轮询间隔（毫秒）。
const TODO_POLL_MS: u64 = 100;
use crate::{error, info, usage};
use chrono::Local;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

/// 处理 todo 命令入口：无参数启动 TUI，有参数按子命令分发
pub fn handle_todo(content: &[String], config: &mut YamlConfig) {
    if content.is_empty() {
        run_todo_tui(config);
        return;
    }

    match content[0].as_str() {
        CMD_LIST => {
            let filter = parse_list_filter(&content[1..]);
            handle_todo_list(filter);
        }
        CMD_ADD => {
            let text = content[1..].join(" ");
            let text = text.trim().trim_matches('"').to_string();
            if text.is_empty() {
                error!("⚠️ 内容为空，无法添加待办");
                return;
            }
            quick_add_todo(&text);
        }
        _ => {
            usage!("j todo | j todo list [--done/-d | --undone/-u] | j todo add <内容>");
        }
    }
}

fn parse_list_filter(args: &[String]) -> Option<bool> {
    for arg in args {
        match arg.as_str() {
            FILTER_DONE | FILTER_DONE_SHORT => return Some(true),
            FILTER_UNDONE | FILTER_UNDONE_SHORT => return Some(false),
            _ => {}
        }
    }
    None
}

fn quick_add_todo(text: &str) {
    let mut todo_list = load_todo_list();
    todo_list.items.push(TodoItem {
        content: text.to_string(),
        done: false,
        created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        done_at: None,
    });

    if save_todo_list(&todo_list) {
        info!("☑️ 已添加待办: {}", text);
        let undone = todo_list.items.iter().filter(|i| !i.done).count();
        info!("📋 当前未完成待办: {} 条", undone);
    }
}

fn handle_todo_list(filter: Option<bool>) {
    let todo_list = load_todo_list();

    if todo_list.items.is_empty() {
        info!("📋 暂无待办");
        return;
    }

    let total = todo_list.items.len();
    let done_count = todo_list.items.iter().filter(|i| i.done).count();
    let undone_count = total - done_count;

    let filtered_items: Vec<&TodoItem> = match filter {
        None => todo_list.items.iter().collect(),
        Some(true) => todo_list.items.iter().filter(|i| i.done).collect(),
        Some(false) => todo_list.items.iter().filter(|i| !i.done).collect(),
    };

    if filtered_items.is_empty() {
        let filter_label = match filter {
            Some(true) => "已完成",
            Some(false) => "未完成",
            None => "全部",
        };
        info!("📋 暂无{}待办", filter_label);
        return;
    }

    let mut md = match filter {
        None => format!(
            "## 待办备忘录 — 共 {} 条 | ☑️ {} | ⬜ {}\n\n",
            total, done_count, undone_count
        ),
        Some(true) => format!("## 待办备忘录 — 已完成 ({}/{})\n\n", done_count, total),
        Some(false) => format!("## 待办备忘录 — 未完成 ({}/{})\n\n", undone_count, total),
    };

    for item in &filtered_items {
        if item.done {
            md.push_str(&format!("- [x] {}\n", item.content));
        } else {
            md.push_str(&format!("- [ ] {}\n", item.content));
        }
    }

    crate::md!("{}", md);
}

fn run_todo_tui(config: &mut YamlConfig) {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    let result = run_todo_tui_internal(config);

    let _ = std::panic::take_hook();

    match result {
        Ok(_) => {}
        Err(e) => {
            error!("✖️ TUI 启动失败: {}", e);
        }
    }
}

fn run_todo_tui_internal(config: &mut YamlConfig) -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TodoApp::new();
    let mut prev_input_mode: Option<AppMode> = None;

    loop {
        terminal.draw(|f| draw_ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(TODO_POLL_MS))?
            && let Event::Key(key) = event::read()?
        {
            match app.mode {
                AppMode::Normal => {
                    if handle_normal_mode(&mut app, key) {
                        break;
                    }
                }
                AppMode::Adding => {
                    prev_input_mode = Some(AppMode::Adding);
                    handle_input_mode(&mut app, key);
                }
                AppMode::Editing => {
                    prev_input_mode = Some(AppMode::Editing);
                    handle_input_mode(&mut app, key);
                }
                AppMode::ConfirmDelete => handle_confirm_delete(&mut app, key),
                AppMode::ConfirmReport => handle_confirm_report(&mut app, key, config),
                AppMode::ConfirmCancelInput => {
                    let prev = prev_input_mode.clone().unwrap_or(AppMode::Adding);
                    handle_confirm_cancel_input(&mut app, key, prev);
                }
                AppMode::Help => handle_help_mode(&mut app, key),
                AppMode::CommandPopup => handle_command_popup_mode(&mut app, key),
            }
        }
    }

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
