pub mod app;
pub mod ui;

use crate::constants::{notebook_action, shell};
use crate::util::fuzzy;
use crate::{error, info};
use app::{
    AppMode, NotebookApp, edit_note_on_terminal, edit_note_with_editor, handle_command_popup_mode,
    handle_confirm_delete, handle_help_mode, handle_input_mode, handle_normal_mode,
    handle_preview_mode, handle_ratio_input_mode, load_notes, note_file_path, notebook_dir,
};
use colored::Colorize;
use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};
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

/// notebook/md 命令入口
pub fn handle_notebook(args: &[String]) {
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
                error!("用法: md search <关键词>");
            }
        }
        f if f == notebook_action::DELETE => {
            if let Some(title) = args.get(1) {
                handle_delete(title);
            } else {
                error!("用法: md delete <笔记路径>");
            }
        }
        f if f == notebook_action::OPEN => handle_open(),
        f if f == notebook_action::RENAME => {
            if args.len() >= 3 {
                handle_rename(&args[1], &args[2]);
            } else {
                error!("用法: md rename <旧路径> <新路径>");
            }
        }
        f if f == notebook_action::MKDIR => {
            if let Some(name) = args.get(1) {
                handle_mkdir(name);
            } else {
                error!("用法: md mkdir <目录名>");
            }
        }
        f if f == notebook_action::MV => {
            if args.len() >= 3 {
                handle_mv(&args[1], &args[2]);
            } else {
                error!("用法: md mv <源路径> <目标路径>");
            }
        }
        _ => {
            let joined = args.join(" ");
            if is_file_path(&joined) {
                edit_file_with_editor(&joined);
            } else {
                edit_note_with_editor(&joined);
            }
        }
    }
}

/// 判断字符串是否为文件路径（而非 notebook 笔记标题）
fn is_file_path(s: &str) -> bool {
    if s.starts_with('~') || s.contains('.') {
        return true;
    }
    if s.contains('/') {
        let potential_note = note_file_path(s);
        if potential_note.starts_with(notebook_dir()) {
            return false;
        }
        return true;
    }
    false
}

/// 用 Markdown 编辑器编辑外部文件（原 md 命令逻辑）
fn edit_file_with_editor(file_str: &str) {
    let expanded = expand_tilde(file_str);
    let path = std::path::PathBuf::from(&expanded);

    let (content, is_new_file) = if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(c) => (c, false),
            Err(e) => {
                error!("读取文件失败: {} - {}", path.display(), e);
                return;
            }
        }
    } else {
        (String::new(), true)
    };

    let theme = crate::command::chat::theme::Theme::from_name(
        &crate::command::chat::theme::ThemeName::default(),
    );

    let title = if is_new_file {
        format!("{} (新文件)", path.display())
    } else {
        path.display().to_string()
    };

    match crate::tui::editor_markdown::open_markdown_editor(&title, &content, &theme) {
        Ok(Some(new_content)) => {
            if new_content != content {
                if let Some(parent) = path.parent()
                    && !parent.exists()
                    && let Err(e) = std::fs::create_dir_all(parent)
                {
                    error!("创建目录失败: {} - {}", parent.display(), e);
                    return;
                }

                match std::fs::write(&path, &new_content) {
                    Ok(()) => info!("文件已保存: {}", path.display()),
                    Err(e) => error!("保存文件失败: {} - {}", path.display(), e),
                }
            } else {
                info!("内容未变化，跳过保存");
            }
        }
        Ok(None) => info!("已取消编辑"),
        Err(e) => error!("编辑器启动失败: {}", e),
    }
}

/// 展开 ~ 为 home 目录
fn expand_tilde(path: &str) -> String {
    if (path == "~" || path.starts_with("~/"))
        && let Some(home) = dirs::home_dir()
    {
        if path == "~" {
            home.display().to_string()
        } else {
            format!("{}{}", home.display(), &path[1..])
        }
    } else {
        path.to_string()
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
        println!("  {}  {}", note.path, app::format_time(note.mtime).dimmed());
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
        let file_path = note_file_path(&note.path);
        if let Ok(content) = fs::read_to_string(&file_path)
            && (fuzzy::fuzzy_match(&content, keyword) || fuzzy::fuzzy_match(&note.path, keyword))
        {
            if !found {
                println!("{}", format!("🔍 搜索 \"{}\" 的结果：", keyword).bold());
                found = true;
            }
            println!("\n  {}", note.path.cyan().bold());
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
            .map(|n| n.path.as_str())
            .filter(|path| fuzzy::fuzzy_match(path, title))
            .collect();

        if matched.is_empty() {
            error!("未找到笔记: {}", title);
        } else {
            println!("未找到精确匹配，你是否要删除以下笔记？");
            for path in &matched {
                println!("  - {}", path);
            }
            info!("请使用精确路径: md delete <路径>");
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
            Ok(()) => {
                app::cleanup_empty_dirs();
                info!("已删除笔记: {}", title);
            }
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

    if let Some(parent) = new_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    match fs::rename(&old_path, &new_path) {
        Ok(()) => {
            app::cleanup_empty_dirs();
            info!("已重命名: {} → {}", old_name, new_name);
        }
        Err(e) => error!("重命名失败: {}", e),
    }
}

/// 创建目录
fn handle_mkdir(name: &str) {
    let dir_path = notebook_dir().join(name);
    if dir_path.exists() {
        error!("目录已存在: {}", name);
        return;
    }
    match fs::create_dir_all(&dir_path) {
        Ok(()) => info!("已创建目录: {}", name),
        Err(e) => error!("创建目录失败: {}", e),
    }
}

/// 移动笔记
fn handle_mv(source: &str, target: &str) {
    let old_path = note_file_path(source);
    let new_path = note_file_path(target);
    if !old_path.exists() {
        error!("源笔记不存在: {}", source);
        return;
    }
    if new_path.exists() {
        error!("目标笔记已存在: {}", target);
        return;
    }
    if let Some(parent) = new_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::rename(&old_path, &new_path) {
        Ok(()) => {
            app::cleanup_empty_dirs();
            info!("已移动: {} → {}", source, target);
        }
        Err(e) => error!("移动失败: {}", e),
    }
}

// ========== TUI 启动 ==========

/// 启动 TUI 笔记管理界面
fn run_notebook_tui() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = execute!(io::stdout(), crossterm::event::DisableMouseCapture);
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
    execute!(stdout, crossterm::event::EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = NotebookApp::new();

    loop {
        terminal.draw(|f| draw_ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    let mut edit_requested: Option<String> = None;

                    match app.mode {
                        AppMode::Normal => {
                            if handle_normal_mode(&mut app, key) {
                                break;
                            }
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
                            if let Some(title) = app.pending_edit_title.take() {
                                edit_requested = Some(title);
                            }
                        }
                        AppMode::Renaming | AppMode::Search | AppMode::Mkdir | AppMode::Mv => {
                            handle_input_mode(&mut app, key);
                        }
                        AppMode::ConfirmDelete => handle_confirm_delete(&mut app, key),
                        AppMode::Help => handle_help_mode(&mut app, key),
                        AppMode::CommandPopup => handle_command_popup_mode(&mut app, key),
                        AppMode::RatioInput => handle_ratio_input_mode(&mut app, key),
                    }

                    if let Some(title) = edit_requested {
                        let needs_reload = edit_note_on_terminal(&title, &mut terminal);
                        if needs_reload {
                            app.reload();
                        } else {
                            app.update_preview();
                        }
                        while event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
                            let _ = event::read();
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse_event(&mut app, mouse, terminal.get_frame().area());
                }
                _ => {}
            }
        }
    }

    execute!(
        terminal.backend_mut(),
        crossterm::event::DisableMouseCapture
    )?;
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}

/// 处理鼠标事件
fn handle_mouse_event(app: &mut NotebookApp, mouse: MouseEvent, frame_area: ratatui::layout::Rect) {
    // 只在 Normal/Preview 模式下处理滚轮
    if !matches!(app.mode, AppMode::Normal | AppMode::Preview) {
        return;
    }

    let scroll_delta = match mouse.kind {
        MouseEventKind::ScrollUp => -1i16,
        MouseEventKind::ScrollDown => 1i16,
        _ => return,
    };

    // 计算布局区域，判断鼠标在左侧还是右侧面板
    // 布局: 标题(3) | 主区域 | 状态栏(3) | 帮助栏(1)
    let main_y_start = frame_area.y + 3;
    let main_y_end = frame_area.y + frame_area.height.saturating_sub(4);

    // 鼠标不在主区域内，忽略
    if mouse.row < main_y_start || mouse.row >= main_y_end {
        return;
    }

    // 滚轮统一滚动右侧预览区（左侧目录树仅通过上下键控制）
    match scroll_delta {
        -1 => {
            app.preview_scroll = app.preview_scroll.saturating_sub(3);
        }
        1 => {
            app.preview_scroll = app.preview_scroll.saturating_add(3);
        }
        _ => {}
    }
}
