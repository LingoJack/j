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

use app::HelpApp;
use ui::draw_help_ui;

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

                if event::poll(std::time::Duration::from_millis(100))? {
                    match event::read()? {
                        Event::Key(key) => {
                            match key.code {
                                // 退出
                                KeyCode::Char('q') | KeyCode::Esc => break,
                                KeyCode::Char('c')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    break;
                                }

                                // Tab 切换
                                KeyCode::Right | KeyCode::Char('l') => app.next_tab(),
                                KeyCode::Left | KeyCode::Char('h') => app.prev_tab(),
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

                                _ => {}
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
