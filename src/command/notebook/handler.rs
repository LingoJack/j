use super::app::{
    AppMode, FlatEntryKind, Focus, NotebookApp, edit_note_with_editor, handle_command_popup_mode,
    handle_confirm_delete, handle_input_mode, handle_ratio_input_mode, load_notes, note_file_path,
    notebook_dir,
};
use super::ui::draw_ui;
use crate::command::chat::storage::load_agent_config;
use crate::constants::{notebook_action, shell};
use crate::theme::Theme;
use crate::util::fuzzy;

/// Notebook 事件轮询间隔（约 60fps）。
const NOTEBOOK_POLL_MS: u64 = 16;
use crate::{error, info};
use colored::Colorize;
use crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::layout::Rect;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::process::Command;

/// 处理 notebook/md 命令入口
///
/// `from_notebook_cmd=true` 表示由 `j notebook` 调用，无参数时进入 TUI 列表；
/// `from_notebook_cmd=false` 表示由 `j md` 调用，无参数时编辑默认临时笔记。
pub fn handle_notebook(args: &[String], from_notebook_cmd: bool) {
    // 优先检测 stdin 管道输入：非终端时读取并渲染 Markdown 到 stdout
    if !std::io::stdin().is_terminal() {
        handle_stdin_render();
        return;
    }

    if args.is_empty() {
        if from_notebook_cmd {
            run_notebook_tui();
        } else {
            handle_edit_default_temp_note();
        }
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

/// 从 stdin 读取 Markdown 文本，渲染为 ANSI 彩色输出到 stdout
fn handle_stdin_render() {
    use std::io::Read;

    let mut input = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("读取 stdin 失败: {e}");
        std::process::exit(1);
    }
    if input.trim().is_empty() {
        return;
    }
    crate::util::md_render::render_md(&input);
}

/// 默认临时笔记名前缀
const TEMP_NOTE_PREFIX: &str = "temp_note_";

/// 无参数时编辑默认临时笔记：自动选取 temp_note_{N}.md 中第一个不存在的编号
fn handle_edit_default_temp_note() {
    let dir = notebook_dir();
    let _ = fs::create_dir_all(&dir);

    let index = find_next_temp_note_index(&dir);
    let note_name = format!("{}{}", TEMP_NOTE_PREFIX, index);
    edit_note_with_editor(&note_name);
}

/// 找到下一个可用的临时笔记编号（从 0 开始递增，找到第一个不存在的）
fn find_next_temp_note_index(dir: &std::path::Path) -> u32 {
    let mut index = 0;
    loop {
        let file_name = format!("{}{}.md", TEMP_NOTE_PREFIX, index);
        if !dir.join(&file_name).exists() {
            return index;
        }
        index += 1;
    }
}

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

    let theme = Theme::from_name(&load_agent_config().theme);

    let title = if is_new_file {
        format!("{} (新文件)", path.display())
    } else {
        path.display().to_string()
    };

    match crate::tui::editor_markdown::open_markdown_editor(&title, &content, &theme) {
        Ok((Some(new_content), _)) => {
            if is_new_file || new_content != content {
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
        Ok((None, _)) => info!("已取消编辑"),
        Err(e) => error!("编辑器启动失败: {}", e),
    }
}

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

fn handle_list() {
    let notes = load_notes();
    if notes.is_empty() {
        info!("📓 notebook 为空");
        return;
    }

    println!("{}", format!("📓 共 {} 篇笔记：", notes.len()).bold());
    for note in &notes {
        println!(
            "  {}  {}",
            note.path,
            super::app::format_time(note.mtime).dimmed()
        );
    }
}

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
                super::app::cleanup_empty_dirs();
                info!("已删除笔记: {}", title);
            }
            Err(e) => error!("删除失败: {}", e),
        }
    } else {
        info!("已取消删除");
    }
}

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
            super::app::cleanup_empty_dirs();
            info!("已重命名: {} → {}", old_name, new_name);
        }
        Err(e) => error!("重命名失败: {}", e),
    }
}

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
            super::app::cleanup_empty_dirs();
            info!("已移动: {} → {}", source, target);
        }
        Err(e) => error!("移动失败: {}", e),
    }
}

fn run_notebook_tui() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        let _ = execute!(io::stdout(), crossterm::event::DisableMouseCapture);
        let _ = execute!(io::stdout(), crossterm::event::DisableBracketedPaste);
        default_hook(info);
    }));

    let result = run_notebook_tui_internal();

    let _ = std::panic::take_hook();

    if let Err(e) = result {
        error!("TUI 启动失败: {}", e);
    }
}

/// 切换笔记时自动保存（如果内容有修改）
fn auto_save_if_dirty(app: &mut NotebookApp) {
    if app.editor_dirty {
        app.save_editor_content();
    }
}

/// 目录树焦点下的按键处理
fn handle_tree_focus_key(app: &mut NotebookApp, key: crossterm::event::KeyEvent) {
    match key.code {
        // / 打开目录树命令面板
        KeyCode::Char('/') => {
            app.mode = AppMode::CommandPopup;
            app.cmd_popup_filter.clear();
            app.cmd_popup_selected = 0;
            app.message = None;
        }
        // Esc / q 退出 notebook
        KeyCode::Esc | KeyCode::Char('q') => {
            auto_save_if_dirty(app);
            app.should_exit = true;
        }
        // 上移
        KeyCode::Up | KeyCode::Char('k') => {
            auto_save_if_dirty(app);
            app.move_up();
        }
        // 下移
        KeyCode::Down | KeyCode::Char('j') => {
            auto_save_if_dirty(app);
            app.move_down();
        }
        // a 新建笔记
        KeyCode::Char('a') => {
            app.mode = AppMode::Adding;
            app.input.clear();
            app.cursor_pos = 0;
            app.message = None;
        }
        // r 重命名选中文件
        KeyCode::Char('r') => {
            if let Some(idx) = app.selected_real_index() {
                app.input = app.notes[idx].path.clone();
                app.cursor_pos = app.input.chars().count();
                app.rename_index = Some(idx);
                app.mode = AppMode::Renaming;
                app.message = None;
            } else {
                app.message = Some("没有选中的笔记".to_string());
            }
        }
        // d 删除选中文件（需 y 确认）
        KeyCode::Char('d') => {
            if app.selected_real_index().is_some() {
                app.mode = AppMode::ConfirmDelete;
            } else {
                app.message = Some("没有选中的笔记".to_string());
            }
        }
        // o 在文件管理器中打开 notebook 目录
        KeyCode::Char('o') => {
            super::app::io::open_in_finder();
            app.message = Some("已打开目录".to_string());
        }
        // s 刷新笔记列表
        KeyCode::Char('s') => {
            auto_save_if_dirty(app);
            app.reload();
        }
        // [ / ] 调整左侧面板比例
        KeyCode::Char('[') => {
            app.panel_ratio = app.panel_ratio.saturating_sub(5).max(15);
            super::app::io::save_panel_ratio(app.panel_ratio);
            app.message = Some(format!(
                "面板比例: {}:{}",
                app.panel_ratio,
                100 - app.panel_ratio
            ));
        }
        KeyCode::Char(']') => {
            app.panel_ratio = app.panel_ratio.saturating_add(5).min(60);
            super::app::io::save_panel_ratio(app.panel_ratio);
            app.message = Some(format!(
                "面板比例: {}:{}",
                app.panel_ratio,
                100 - app.panel_ratio
            ));
        }
        // Enter: 目录→展开/折叠, 文件→焦点到编辑器
        KeyCode::Enter => {
            if let Some(entry) = app.selected_entry().cloned() {
                match &entry.kind {
                    FlatEntryKind::Dir { dir_path, .. } => {
                        app.expanded_dirs.toggle(dir_path);
                        super::app::io::save_expanded_dirs(&app.expanded_dirs);
                        app.build_flat_entries();
                    }
                    FlatEntryKind::File { .. } => {
                        app.focus = Focus::Editor;
                    }
                }
            }
        }
        _ => {}
    }
}

/// 编辑器焦点下的按键处理（完整 vim 编辑器，和独立编辑器一致）
fn handle_editor_focus_key(app: &mut NotebookApp, key: crossterm::event::KeyEvent) {
    if let Some(ref mut editor) = app.editor {
        // Esc 在编辑器空闲 Normal 模式时：焦点回目录树
        if key.code == KeyCode::Esc && editor.is_idle_normal_mode() {
            app.focus = Focus::Tree;
            return;
        }

        // 其他按键正常传递给编辑器
        let input = crate::tui::editor_core::vim::Input::from_keycode(key.code, key.modifiers);
        let action = editor.handle_input(&input);
        match action {
            crate::tui::editor_core::EditorAction::Save(_) => {
                app.save_editor_content();
            }
            crate::tui::editor_core::EditorAction::Submit(_) => {
                app.save_editor_content();
                app.focus = Focus::Tree;
            }
            crate::tui::editor_core::EditorAction::Cancel => {
                app.focus = Focus::Tree;
            }
            crate::tui::editor_core::EditorAction::Continue => {
                app.editor_dirty = true;
            }
        }
    }
}

/// 处理 bracketed paste：根据当前模式把整段文本一次性灌入对应输入位置。
fn handle_paste_event(app: &mut NotebookApp, text: String) {
    match app.mode {
        AppMode::Normal => {
            // 仅当焦点在编辑器时才注入到 markdown 缓冲区，避免在目录树焦点
            // 下意外修改笔记内容。
            if app.focus == Focus::Editor
                && let Some(ref mut editor) = app.editor
            {
                editor.insert_text(&text);
                app.editor_dirty = true;
            }
        }
        AppMode::Adding | AppMode::Renaming | AppMode::Search | AppMode::Mkdir | AppMode::Mv => {
            insert_text_into_input(app, &text, /*digits_only=*/ false);
        }
        AppMode::RatioInput => {
            insert_text_into_input(app, &text, /*digits_only=*/ true);
        }
        // CommandPopup / ConfirmDelete 模式下忽略粘贴。
        _ => {}
    }
}

/// 把字符串追加到 `app.input` 当前光标处（忽略换行）。
/// `digits_only=true` 时仅保留数字与冒号（用于 RatioInput 模式）。
fn insert_text_into_input(app: &mut NotebookApp, text: &str, digits_only: bool) {
    for c in text.chars() {
        if c == '\r' || c == '\n' {
            continue;
        }
        if digits_only && !(c.is_ascii_digit() || c == ':') {
            continue;
        }
        let byte_idx = app
            .input
            .char_indices()
            .nth(app.cursor_pos)
            .map(|(i, _)| i)
            .unwrap_or(app.input.len());
        app.input.insert(byte_idx, c);
        app.cursor_pos += 1;
    }
}

fn run_notebook_tui_internal() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, crossterm::event::EnableMouseCapture)?;
    execute!(stdout, crossterm::event::EnableBracketedPaste)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = NotebookApp::new();

    loop {
        terminal.draw(|f| draw_ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(NOTEBOOK_POLL_MS))? {
            match event::read()? {
                Event::Key(key) => match app.mode {
                    AppMode::Normal => match app.focus {
                        Focus::Tree => {
                            handle_tree_focus_key(&mut app, key);
                            if app.should_exit {
                                break;
                            }
                        }
                        Focus::Editor => {
                            handle_editor_focus_key(&mut app, key);
                        }
                    },
                    AppMode::Adding => {
                        handle_input_mode(&mut app, key);
                        if let Some(title) = app.pending_edit_title.take() {
                            let file_path = super::app::io::note_file_path(&title);
                            if let Some(parent) = file_path.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            let _ = std::fs::write(&file_path, "");
                            app.reload();
                            if let Some(pos) = app.flat_entries.iter().position(|e| {
                                    matches!(&e.kind, FlatEntryKind::File { note_index } if app.notes[*note_index].path == title)
                                }) {
                                    app.state.select(Some(pos));
                                    app.load_editor_for_selected();
                                }
                        }
                    }
                    AppMode::Renaming | AppMode::Search | AppMode::Mkdir | AppMode::Mv => {
                        handle_input_mode(&mut app, key);
                    }
                    AppMode::ConfirmDelete => handle_confirm_delete(&mut app, key),
                    AppMode::CommandPopup => handle_command_popup_mode(&mut app, key),
                    AppMode::RatioInput => handle_ratio_input_mode(&mut app, key),
                },
                Event::Mouse(mouse) => {
                    let frame_area = terminal.get_frame().area();
                    let layout = compute_mouse_layout(frame_area, &app);
                    let editor_area = layout.preview_area.unwrap_or_default();
                    handle_mouse_event(&mut app, mouse, &layout, editor_area);

                    // 消费后续鼠标事件
                    while event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
                        if let Ok(Event::Mouse(m)) = event::read() {
                            handle_mouse_event(&mut app, m, &layout, editor_area);
                        }
                    }
                }
                Event::Paste(text) => {
                    handle_paste_event(&mut app, text);
                }
                _ => {}
            }
        }
    }

    execute!(
        terminal.backend_mut(),
        crossterm::event::DisableMouseCapture
    )?;
    execute!(
        terminal.backend_mut(),
        crossterm::event::DisableBracketedPaste
    )?;
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}

// ========== 鼠标事件处理 ==========

/// 鼠标事件处理时需要的布局信息
struct MouseLayoutInfo {
    /// 主区域
    main_area: Rect,
    /// 笔记列表区域（仅在 Normal 模式有效）
    list_area: Option<Rect>,
    /// 预览/编辑区域（仅在 Normal 模式有效）
    preview_area: Option<Rect>,
    /// 分割线 x 坐标（列表区和预览区的交界列）
    divider_x: Option<u16>,
}

/// 处理鼠标事件
fn handle_mouse_event(
    app: &mut NotebookApp,
    mouse: MouseEvent,
    layout: &MouseLayoutInfo,
    editor_area: Rect,
) {
    if app.mode != AppMode::Normal {
        return;
    }

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            handle_left_click(app, mouse.column, mouse.row, layout, editor_area);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            // 分割线拖拽（调整面板比例）优先，其次把 Drag 透给编辑器以驱动鼠标拖选。
            // 鼠标拖出 editor_area 是常态（向下/向上甩选），由编辑器自身 fallback
            // 到最近的有效渲染行（见 `clamped_render_pos_for_drag`），这里不再
            // 用 `rect_contains` 卡 area。
            if app.is_dragging_panel {
                handle_drag(app, mouse.column, layout);
            } else if app.focus == Focus::Editor
                && let Some(ref mut editor) = app.editor
            {
                editor.handle_mouse(mouse, editor_area);
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            handle_mouse_up(app);
            // 同时通知编辑器结束拖选（提交 mouse_selection）
            if app.focus == Focus::Editor
                && let Some(ref mut editor) = app.editor
            {
                editor.handle_mouse(mouse, editor_area);
            }
        }
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            handle_scroll(
                app,
                mouse.column,
                mouse.row,
                layout,
                mouse.kind,
                editor_area,
            );
        }
        _ => {}
    }
}

/// 检查点是否在矩形区域内（含边界）
fn rect_contains(area: Rect, col: u16, row: u16) -> bool {
    col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height
}

/// 处理左键点击
fn handle_left_click(
    app: &mut NotebookApp,
    col: u16,
    row: u16,
    layout: &MouseLayoutInfo,
    editor_area: Rect,
) {
    // 检测是否点击分割线（优先级最高）
    if let Some(divider_x) = layout.divider_x
        && col >= divider_x.saturating_sub(2)
        && col <= divider_x + 2
        && row >= layout.main_area.y
        && row < layout.main_area.y + layout.main_area.height
    {
        app.is_dragging_panel = true;
        return;
    }

    // 点击编辑区：传递点击事件给编辑器，并设置焦点
    if rect_contains(editor_area, col, row) {
        app.focus = Focus::Editor;
        if let Some(ref mut editor) = app.editor {
            let mouse_event = MouseEvent {
                column: col,
                row,
                kind: MouseEventKind::Down(MouseButton::Left),
                modifiers: crossterm::event::KeyModifiers::empty(),
            };
            editor.handle_mouse(mouse_event, editor_area);
        }
        return;
    }

    // 点击列表区：选择笔记，并设置焦点
    if let Some(list_area) = layout.list_area
        && rect_contains(list_area, col, row)
    {
        app.focus = Focus::Tree;
        let inner_y = row.saturating_sub(list_area.y).saturating_sub(1);
        let max_visible = list_area.height.saturating_sub(2) as usize;

        if (inner_y as usize) < max_visible {
            let index = app.state.offset() + inner_y as usize;
            if index < app.flat_entries.len() {
                let now = std::time::Instant::now();

                let is_double_click = app
                    .last_click_time
                    .map(|t| now.duration_since(t).as_millis() < 500)
                    .unwrap_or(false)
                    && app.last_click_index == Some(index);

                if is_double_click {
                    let entry = &app.flat_entries[index];
                    if let FlatEntryKind::Dir { dir_path, .. } = &entry.kind {
                        app.expanded_dirs.toggle(dir_path);
                        super::app::io::save_expanded_dirs(&app.expanded_dirs);
                        app.build_flat_entries();
                    }
                }

                // 切换笔记时自动保存
                auto_save_if_dirty(app);
                app.state.select(Some(index));
                app.load_editor_for_selected();

                app.last_click_time = Some(now);
                app.last_click_pos = Some((col, row));
                app.last_click_index = Some(index);
            }
        }
    }
}

/// 处理鼠标拖拽（调整面板比例）
fn handle_drag(app: &mut NotebookApp, col: u16, layout: &MouseLayoutInfo) {
    if !app.is_dragging_panel {
        return;
    }

    let frame_width = layout.main_area.width;
    if frame_width == 0 {
        return;
    }

    let relative_x = col.saturating_sub(layout.main_area.x);
    let new_ratio = (relative_x as u32 * 100 / frame_width as u32) as u16;
    app.panel_ratio = new_ratio.clamp(15, 60);
}

/// 处理鼠标释放
fn handle_mouse_up(app: &mut NotebookApp) {
    if app.is_dragging_panel {
        app.is_dragging_panel = false;
        super::app::io::save_panel_ratio(app.panel_ratio);
    }
}

/// 处理滚轮滚动（根据鼠标位置决定滚动列表还是编辑器）
fn handle_scroll(
    app: &mut NotebookApp,
    col: u16,
    row: u16,
    layout: &MouseLayoutInfo,
    kind: MouseEventKind,
    _editor_area: Rect,
) {
    // 编辑区滚轮：移动光标
    if let Some(preview_area) = layout.preview_area
        && rect_contains(preview_area, col, row)
        && let Some(ref mut editor) = app.editor
    {
        let scroll_lines = 3;
        match kind {
            MouseEventKind::ScrollUp => {
                for _ in 0..scroll_lines {
                    editor.move_cursor_visual_up();
                }
            }
            MouseEventKind::ScrollDown => {
                for _ in 0..scroll_lines {
                    editor.move_cursor_visual_down();
                }
            }
            _ => {}
        }
        return;
    }

    // 列表区滚轮：切换选择项
    if let Some(list_area) = layout.list_area
        && rect_contains(list_area, col, row)
    {
        auto_save_if_dirty(app);
        match kind {
            MouseEventKind::ScrollUp => app.move_up(),
            MouseEventKind::ScrollDown => app.move_down(),
            _ => {}
        }
    }
}

/// 计算鼠标事件处理所需的布局信息
fn compute_mouse_layout(frame_area: Rect, app: &NotebookApp) -> MouseLayoutInfo {
    // 主区域：标题栏之后、状态栏之前
    let main_area = Rect {
        x: frame_area.x,
        y: frame_area.y + 3,
        width: frame_area.width,
        height: frame_area.height.saturating_sub(7),
    };

    // Normal/CommandPopup 模式下计算列表/预览区域
    let (list_area, preview_area, divider_x) =
        if matches!(app.mode, AppMode::Normal | AppMode::CommandPopup) {
            let list_width = frame_area.width * app.panel_ratio / 100;
            let preview_width = frame_area.width.saturating_sub(list_width);
            (
                Some(Rect {
                    x: frame_area.x,
                    y: main_area.y,
                    width: list_width,
                    height: main_area.height,
                }),
                Some(Rect {
                    x: frame_area.x + list_width,
                    y: main_area.y,
                    width: preview_width,
                    height: main_area.height,
                }),
                Some(frame_area.x + list_width),
            )
        } else {
            (None, None, None)
        };

    MouseLayoutInfo {
        main_area,
        list_area,
        preview_area,
        divider_x,
    }
}
