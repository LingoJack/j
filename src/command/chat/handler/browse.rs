use crate::command::chat::app::{Action, ChatApp, CursorDirection};
use crossterm::event::{KeyCode, KeyEvent};

/// 消息浏览模式按键处理：↑↓ 选择消息，y/Enter 复制选中消息，Esc 退出
pub fn handle_browse_mode(app: &mut ChatApp, key: KeyEvent) {
    let msg_count = app.state.session.messages.len();
    if msg_count == 0 {
        app.update(Action::ExitToChat);
        app.ui.msg_lines_cache = None;
        return;
    }

    let action = match key.code {
        KeyCode::Esc => Action::ExitToChat,
        KeyCode::Up | KeyCode::Char('k') => Action::BrowseNavigate(CursorDirection::Up),
        KeyCode::Down | KeyCode::Char('j') => Action::BrowseNavigate(CursorDirection::Down),
        KeyCode::Char('a') | KeyCode::Char('A') => Action::BrowseFineScroll(CursorDirection::Up),
        KeyCode::Char('d') | KeyCode::Char('D') => Action::BrowseFineScroll(CursorDirection::Down),
        KeyCode::Enter | KeyCode::Char('y') => Action::BrowseCopyMessage,
        _ => return,
    };

    app.update(action);

    // ExitToChat 时清除高亮缓存
    if matches!(key.code, KeyCode::Esc) {
        app.ui.msg_lines_cache = None;
    }
}
