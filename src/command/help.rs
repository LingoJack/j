pub mod app;
pub mod ui;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::theme::ThemeName;
use app::{AppMode, HelpApp};
use ui::draw_help_ui;

/// 帮助页事件轮询间隔（毫秒）。
const HELP_POLL_MS: u64 = 100;

/// RAII guard：确保终端模式恢复（即使 panic 也会执行清理）
struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    fn activate() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self { active: true })
    }

    fn deactivate(&mut self) {
        if self.active {
            let _ = terminal::disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
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
                terminal.draw(|f| draw_help_ui(f, app))?;

                if event::poll(std::time::Duration::from_millis(HELP_POLL_MS))? {
                    match event::read()? {
                        Event::Key(key) => {
                            if key.modifiers.contains(KeyModifiers::CONTROL)
                                && key.code == KeyCode::Char('c')
                            {
                                break;
                            }
                            match app.mode {
                                AppMode::Normal => {
                                    if handle_normal_key(app, key) {
                                        break;
                                    }
                                }
                                AppMode::CommandPopup => handle_command_popup_key(app, key),
                                AppMode::ThemeSelect => handle_theme_select_key(app, key),
                            }
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
            // 打印 panic 信息（已恢复终端）
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

/// 正常模式按键处理，返回 true 表示退出
fn handle_normal_key(app: &mut HelpApp, key: crossterm::event::KeyEvent) -> bool {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return true,

        // Tab 切换
        KeyCode::Right | KeyCode::Char('l') => app.next_tab(),
        KeyCode::Left | KeyCode::Char('h') => app.prev_tab(),
        KeyCode::Char('[') => app.prev_tab(),
        KeyCode::Char(']') => app.next_tab(),
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.prev_tab();
            } else {
                app.next_tab();
            }
        }
        KeyCode::BackTab => app.prev_tab(),

        // 数字键跳转
        KeyCode::Char(c @ '1'..='9') => {
            let idx = (c as usize) - ('1' as usize);
            app.goto_tab(idx);
        }
        KeyCode::Char('0') => app.goto_tab(9),

        // 滚动
        KeyCode::Down | KeyCode::Char('j') => app.scroll_down(1),
        KeyCode::Up | KeyCode::Char('k') => app.scroll_up(1),
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
                    "help" => {
                        app.goto_tab(0);
                        app.mode = AppMode::Normal;
                        return;
                    }
                    "quit" => {
                        // quit 命令：设置为 Normal，主循环下一轮用 q 退出
                        // 这里直接发出退出信号不方便，回到 Normal 让用户再按 q
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
