pub mod app;
pub mod ui;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::theme::ThemeName;
use app::{AppMode, HelpApp};
use ui::draw_ui;

/// 帮助页事件轮询间隔（毫秒）。
const HELP_POLL_MS: u64 = 100;

/// RAII guard：确保终端模式恢复（即使 panic 也会执行清理）
struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    fn activate() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        Ok(Self { active: true })
    }

    fn deactivate(&mut self) {
        if self.active {
            let _ = execute!(
                io::stdout(),
                crossterm::event::DisableMouseCapture,
                LeaveAlternateScreen
            );
            let _ = terminal::disable_raw_mode();
            self.active = false;
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        self.deactivate();
    }
}

/// 处理 help 命令：启动 TUI 帮助界面
pub fn handle_help() {
    match run_help_tui() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("TUI 启动失败: {}", e);
        }
    }
}

fn run_help_tui() -> io::Result<()> {
    let mut guard = TerminalGuard::activate()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = HelpApp::new();

    let result = {
        let mut app_ref = AssertUnwindSafe(&mut app);
        let mut terminal_ref = AssertUnwindSafe(&mut terminal);
        catch_unwind(move || {
            let app = &mut *app_ref;
            let terminal = &mut *terminal_ref;

            loop {
                terminal.draw(|f| draw_ui(f, app))?;

                if event::poll(std::time::Duration::from_millis(HELP_POLL_MS))? {
                    match event::read()? {
                        Event::Key(key) => {
                            // Ctrl+C：有选区时复制，否则退出
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                                && key.code == KeyCode::Char('c')
                            {
                                if app.mouse_selection.is_some() {
                                    app.copy_selection();
                                } else {
                                    break;
                                }
                            }
                            match app.mode {
                                AppMode::Normal => {
                                    if handle_normal_key(app, key, terminal.get_frame().area()) {
                                        break;
                                    }
                                }
                                AppMode::CommandPopup => handle_command_popup_key(app, key),
                                AppMode::ThemeSelect => handle_theme_select_key(app, key),
                            }
                        }
                        Event::Mouse(mouse) if app.mode == AppMode::Normal => {
                            let frame_area = terminal.get_frame().area();
                            handle_mouse_event(app, mouse, frame_area);
                        }
                        Event::Resize(_, _) => {
                            app.invalidate_cache();
                        }
                        _ => {}
                    }
                }
            }
            Ok::<(), io::Error>(())
        })
    };

    // 先手动清理，guard.drop() 会再次检查确保清理
    guard.deactivate();

    match result {
        Ok(inner_result) => inner_result,
        Err(panic_info) => {
            if let Some(s) = panic_info.downcast_ref::<&str>() {
                eprintln!("Help TUI panic: {}", s);
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                eprintln!("Help TUI panic: {}", s);
            } else {
                eprintln!("Help TUI panic: unknown error");
            }
            Err(io::Error::other("panic occurred"))
        }
    }
}

// ========== 鼠标事件处理 ==========

/// 处理鼠标事件
fn handle_mouse_event(app: &mut HelpApp, mouse: MouseEvent, frame_area: ratatui::layout::Rect) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            handle_left_click(app, mouse.column, mouse.row, frame_area);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if app.is_dragging_panel {
                handle_panel_drag(app, mouse.column, frame_area);
            } else if app.mouse_selection.is_some() {
                handle_selection_drag(app, mouse.column, mouse.row);
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.is_dragging_panel = false;
            // 选区保持，等待 Ctrl+C 复制或下次点击清除
        }
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            handle_scroll(app, mouse.column, mouse.row, frame_area, mouse.kind);
        }
        _ => {}
    }
}

/// 主区域 Y 起始位置（标题栏 3 行）
const MAIN_AREA_Y_OFFSET: u16 = 3;

/// 处理左键点击
fn handle_left_click(app: &mut HelpApp, col: u16, row: u16, frame_area: ratatui::layout::Rect) {
    let main_y = frame_area.y + MAIN_AREA_Y_OFFSET;
    let main_height = frame_area.height.saturating_sub(MAIN_AREA_Y_OFFSET + 1); // +1 hint bar

    // 检测是否点击分割线区域
    let left_width = app.compute_left_panel_width(frame_area.width as usize) as u16;
    let divider_x = frame_area.x + left_width;
    if col >= divider_x.saturating_sub(2)
        && col <= divider_x + 2
        && row >= main_y
        && row < main_y + main_height
    {
        app.is_dragging_panel = true;
        app.mouse_selection = None;
        return;
    }

    // 点击左侧列表区
    if col >= frame_area.x
        && col < frame_area.x + left_width
        && row >= main_y
        && row < main_y + main_height
    {
        let inner_y = row.saturating_sub(main_y).saturating_sub(1) as usize; // -1 边框
        let max_visible = main_height.saturating_sub(2) as usize;
        if inner_y < max_visible && inner_y < app.entries().len() {
            app.selected = inner_y;
            app.content_scroll = 0;
        }
        app.mouse_selection = None;
        return;
    }

    // 点击右侧内容区 → 开始选区
    if col >= frame_area.x + left_width && row >= main_y && row < main_y + main_height {
        if let Some(pos) = app.screen_to_content_pos(col, row) {
            app.mouse_selection = Some(app::MouseSelection {
                anchor: pos,
                current: pos,
            });
        } else {
            app.mouse_selection = None;
        }
    }
}

/// 处理面板拖拽（调整面板宽度）
fn handle_panel_drag(app: &mut HelpApp, col: u16, frame_area: ratatui::layout::Rect) {
    let main_x = frame_area.x;
    let main_width = frame_area.width;
    app.set_panel_width_from_drag(col, main_x, main_width);
}

/// 处理内容区选区拖拽
fn handle_selection_drag(app: &mut HelpApp, col: u16, row: u16) {
    let pos = app.screen_to_content_pos(col, row);
    if let (Some(sel), Some(p)) = (&mut app.mouse_selection, pos) {
        sel.current = p;
    }
}

/// 处理滚轮事件
fn handle_scroll(
    app: &mut HelpApp,
    col: u16,
    row: u16,
    frame_area: ratatui::layout::Rect,
    kind: MouseEventKind,
) {
    let main_y = frame_area.y + MAIN_AREA_Y_OFFSET;
    let main_height = frame_area.height.saturating_sub(MAIN_AREA_Y_OFFSET + 1);
    let left_width = app.compute_left_panel_width(frame_area.width as usize) as u16;

    // 在左侧列表区滚轮
    if col >= frame_area.x
        && col < frame_area.x + left_width
        && row >= main_y
        && row < main_y + main_height
    {
        match kind {
            MouseEventKind::ScrollUp => app.move_up(),
            MouseEventKind::ScrollDown => app.move_down(),
            _ => {}
        }
        return;
    }

    // 在右侧内容区滚轮
    if col >= frame_area.x + left_width && row >= main_y && row < main_y + main_height {
        match kind {
            MouseEventKind::ScrollUp => app.scroll_up(3),
            MouseEventKind::ScrollDown => app.scroll_down(3),
            _ => {}
        }
    }
}

// ========== 键盘事件处理 ==========

/// 正常模式按键处理，返回 true 表示退出
fn handle_normal_key(
    app: &mut HelpApp,
    key: crossterm::event::KeyEvent,
    frame_area: ratatui::layout::Rect,
) -> bool {
    let frame_width = frame_area.width as usize;
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return true,

        // 列表上下移动
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),

        // 展开/折叠目录
        KeyCode::Enter | KeyCode::Char(' ') => app.toggle_expand(),

        // 调整左右面板宽度
        KeyCode::Char('[') => app.shrink_left(frame_width),
        KeyCode::Char(']') => app.widen_left(frame_width),

        // 内容滚动
        KeyCode::PageDown => app.scroll_down(10),
        KeyCode::PageUp => app.scroll_up(10),
        KeyCode::Home => app.scroll_to_top(),
        KeyCode::End => app.scroll_to_bottom(),

        // 命令面板
        KeyCode::Char('/') => app.open_command_popup(),

        _ => {}
    }
    false
}

/// 命令面板按键处理
fn handle_command_popup_key(app: &mut HelpApp, key: crossterm::event::KeyEvent) {
    let items = app.filtered_cmd_items();
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Up | KeyCode::Char('k') if !items.is_empty() => {
            if app.cmd_popup_selected > 0 {
                app.cmd_popup_selected -= 1;
            } else {
                app.cmd_popup_selected = items.len() - 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') if !items.is_empty() => {
            if app.cmd_popup_selected < items.len() - 1 {
                app.cmd_popup_selected += 1;
            } else {
                app.cmd_popup_selected = 0;
            }
        }
        KeyCode::Backspace => {
            if app.cmd_popup_filter.pop().is_none() {
                app.mode = AppMode::Normal;
            } else {
                app.cmd_popup_selected = 0;
            }
        }
        KeyCode::Enter => {
            let selected = app.cmd_popup_selected.min(items.len().saturating_sub(1));
            if let Some((_, key, _)) = items.get(selected) {
                match *key {
                    "theme" => {
                        app.open_theme_select();
                        return;
                    }
                    "quit" => {
                        app.mode = AppMode::Normal;
                        app.message = Some("按 q 退出".to_string());
                        return;
                    }
                    _ => {}
                }
            }
            app.mode = AppMode::Normal;
        }
        KeyCode::Char(c) => {
            app.cmd_popup_filter.push(c);
            app.cmd_popup_selected = 0;
        }
        _ => {}
    }
}

/// 主题选择按键处理
fn handle_theme_select_key(app: &mut HelpApp, key: crossterm::event::KeyEvent) {
    let count = ThemeName::all().len();
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Up | KeyCode::Char('k') if count > 0 => {
            if app.theme_popup_selected > 0 {
                app.theme_popup_selected -= 1;
            } else {
                app.theme_popup_selected = count - 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') if count > 0 => {
            if app.theme_popup_selected < count - 1 {
                app.theme_popup_selected += 1;
            } else {
                app.theme_popup_selected = 0;
            }
        }
        KeyCode::Enter => {
            app.apply_selected_theme();
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}
