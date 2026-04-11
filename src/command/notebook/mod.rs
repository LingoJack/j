pub mod app;
pub mod ui;

use crate::config::YamlConfig;
use crate::constants::{notebook_action, shell};
use crate::util::fuzzy;
use crate::{error, info};
use app::{
    AppMode, NotebookApp, edit_note_with_editor, handle_confirm_delete, handle_help_mode,
    handle_input_mode, handle_normal_mode, handle_preview_mode, load_notes, note_file_path,
    notebook_dir,
};
use colored::Colorize;
use crossterm::event::KeyCode;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::fs;
use std::io::{self, Write};
use std::process::Command;
use ui::draw_ui;

/// notebook 命令入口
pub fn handle_notebook(args: &[String], _config: &YamlConfig) {
    if args.is_empty() {
        run_notebook_tui();
        return;
    }

    let first = args[0].as_str();
    match first {
        f if f == notebook_action::LIST => handle_list(),
        f if f == notebook_action::SEARCH => {
            if let Some(keyword) = args.get(1) {
                handle_search(keyword);
            } else {
                error!("用法: notebook search <关键词>");
            }
        }
        f if f == notebook_action::DELETE => {
            if let Some(title) = args.get(1) {
                handle_delete(title);
            } else {
                error!("用法: notebook delete <笔记名>");
            }
        }
        f if f == notebook_action::OPEN => handle_open(),
        f if f == notebook_action::RENAME => {
            if args.len() >= 3 {
                handle_rename(&args[1], &args[2]);
            } else {
                error!("用法: notebook rename <旧名称> <新名称>");
            }
        }
        _ => {
            // 其余参数视为笔记标题，直接打开编辑器
            let title = args.join(" ");
            edit_note_with_editor(&title);
        }
    }
}

// ========== CLI 子命令（非 TUI） ==========

/// 列出所有笔记
fn handle_list() {
    let notes = load_notes();
    if notes.is_empty() {
        info!("📓 notebook 为空");
        return;
    }

    println!("{}", format!("📓 共 {} 篇笔记：", notes.len()).bold());
    for note in &notes {
        println!("  {}  {}", note.name, app::format_time(note.mtime).dimmed());
    }
}

/// 搜索笔记内容
fn handle_search(keyword: &str) {
    let notes = load_notes();
    if notes.is_empty() {
        info!("📓 notebook 为空");
        return;
    }

    let mut found = false;
    for note in &notes {
        let file_path = note_file_path(&note.name);
        if let Ok(content) = fs::read_to_string(&file_path)
            && (fuzzy::fuzzy_match(&content, keyword) || fuzzy::fuzzy_match(&note.name, keyword))
        {
            if !found {
                println!("{}", format!("🔍 搜索 \"{}\" 的结果：", keyword).bold());
                found = true;
            }
            println!("\n  {}", note.name.cyan().bold());
            for (line_num, line) in content.lines().enumerate() {
                if fuzzy::fuzzy_match(line, keyword) {
                    println!(
                        "    {}: {}",
                        format!("L{}", line_num + 1).dimmed(),
                        line.trim()
                    );
                }
            }
        }
    }

    if !found {
        info!("未找到包含 \"{}\" 的笔记", keyword);
    }
}

/// 删除笔记
fn handle_delete(title: &str) {
    let file_path = note_file_path(title);
    if !file_path.exists() {
        let notes = load_notes();
        let matched: Vec<&str> = notes
            .iter()
            .map(|n| n.name.as_str())
            .filter(|name| fuzzy::fuzzy_match(name, title))
            .collect();

        if matched.is_empty() {
            error!("未找到笔记: {}", title);
        } else {
            println!("未找到精确匹配，你是否要删除以下笔记？");
            for name in &matched {
                println!("  - {}", name);
            }
            info!("请使用精确名称: notebook delete <名称>");
        }
        return;
    }

    print!("确认删除笔记 \"{}\"？(y/N): ", title);
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return;
    }
    if input.trim().to_lowercase() == "y" {
        match fs::remove_file(&file_path) {
            Ok(()) => info!("已删除笔记: {}", title),
            Err(e) => error!("删除失败: {}", e),
        }
    } else {
        info!("已取消删除");
    }
}

/// 打开 notebook 目录
fn handle_open() {
    let dir = notebook_dir();
    let path = dir.to_string_lossy().to_string();
    let os = std::env::consts::OS;
    let result = if os == shell::MACOS_OS {
        Command::new("open").arg(&path).status()
    } else if os == shell::WINDOWS_OS {
        Command::new(shell::WINDOWS_CMD)
            .args([shell::WINDOWS_CMD_FLAG, "start", "", &path])
            .status()
    } else {
        Command::new("xdg-open").arg(&path).status()
    };

    if let Err(e) = result {
        error!("打开目录失败: {}", e);
    }
}

/// 重命名笔记
fn handle_rename(old_name: &str, new_name: &str) {
    let old_path = note_file_path(old_name);
    let new_path = note_file_path(new_name);

    if !old_path.exists() {
        error!("未找到笔记: {}", old_name);
        return;
    }
    if new_path.exists() {
        error!("目标笔记已存在: {}", new_name);
        return;
    }

    match fs::rename(&old_path, &new_path) {
        Ok(()) => info!("已重命名: {} → {}", old_name, new_name),
        Err(e) => error!("重命名失败: {}", e),
    }
}

// ========== TUI 启动 ==========

/// 启动 TUI 笔记管理界面
fn run_notebook_tui() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        default_hook(info);
    }));

    let result = run_notebook_tui_internal();

    let _ = std::panic::take_hook();

    if let Err(e) = result {
        error!("TUI 启动失败: {}", e);
    }
}

fn run_notebook_tui_internal() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = NotebookApp::new();

    loop {
        terminal.draw(|f| draw_ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            // 用于记录编辑操作请求（需要在 TUI loop 中暂停终端处理）
            let mut edit_requested: Option<String> = None;

            match app.mode {
                AppMode::Normal => {
                    if handle_normal_mode(&mut app, key) {
                        break;
                    }
                    // Enter/e 触发编辑
                    if (key.code == KeyCode::Enter || key.code == KeyCode::Char('e'))
                        && app.mode == AppMode::Normal
                        && let Some(name) = app.selected_name()
                    {
                        edit_requested = Some(name);
                    }
                }
                AppMode::Preview => handle_preview_mode(&mut app, key),
                AppMode::Adding => {
                    handle_input_mode(&mut app, key);
                    // Adding+Enter 后 mode 变为 Normal 且 pending_edit_title 有值
                    if let Some(title) = app.pending_edit_title.take() {
                        edit_requested = Some(title);
                    }
                }
                AppMode::Renaming | AppMode::Search => handle_input_mode(&mut app, key),
                AppMode::ConfirmDelete => handle_confirm_delete(&mut app, key),
                AppMode::Help => handle_help_mode(&mut app, key),
            }

            // 处理编辑请求（需暂停/恢复终端）
            if let Some(title) = edit_requested {
                let needs_reload = suspend_and_edit(&title);
                if needs_reload {
                    app.reload();
                } else {
                    app.update_preview();
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}

/// 暂停 TUI 并打开编辑器，返回是否有内容变化
fn suspend_and_edit(title: &str) -> bool {
    let _ = terminal::disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);

    let changed = edit_note_with_editor(title);

    let _ = terminal::enable_raw_mode();
    let _ = execute!(io::stdout(), EnterAlternateScreen);

    // 清除编辑器残留的按键事件（如 :wq 中的 q），避免误触发 TUI 退出
    while event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
        let _ = event::read();
    }

    changed
}
