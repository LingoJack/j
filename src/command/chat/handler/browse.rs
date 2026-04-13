use crate::command::chat::app::{Action, ChatApp, CursorDirection};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// 消息浏览模式按键处理
///
/// 按键说明：
/// - ↑↓        在匹配的消息之间跳转（循环）
/// - j/k       同 ↑↓
/// - PageUp/Dn 当前消息内微调滚动
/// - y/Enter   复制选中消息
/// - Tab       切换角色过滤（全部→AI→用户）
/// - Esc       有过滤时清除过滤，无过滤时退出浏览
/// - 其他字符  输入关键词过滤
/// - Backspace 删除过滤字符
pub fn handle_browse_mode(app: &mut ChatApp, key: KeyEvent) {
    let msg_count = app.state.session.messages.len();
    if msg_count == 0 {
        app.update(Action::ExitToChat);
        app.ui.msg_lines_cache = None;
        return;
    }

    let action = match key.code {
        KeyCode::Esc => {
            if !app.ui.browse_filter.is_empty() || app.ui.browse_role_filter.is_some() {
                Action::BrowseClearFilter
            } else {
                Action::ExitToChat
            }
        }
        KeyCode::Up => Action::BrowseNavigate(CursorDirection::Up),
        KeyCode::Down => Action::BrowseNavigate(CursorDirection::Down),
        KeyCode::PageUp => Action::BrowseFineScroll(CursorDirection::Up),
        KeyCode::PageDown => Action::BrowseFineScroll(CursorDirection::Down),
        KeyCode::Enter | KeyCode::Char('y') => Action::BrowseCopyMessage,
        KeyCode::Tab => Action::BrowseToggleRole,
        KeyCode::Backspace => Action::BrowseDeleteChar,
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return;
            }
            match c {
                'j' => Action::BrowseNavigate(CursorDirection::Down),
                'k' => Action::BrowseNavigate(CursorDirection::Up),
                _ => Action::BrowseInputChar(c),
            }
        }
        _ => return,
    };

    app.update(action);

    // ExitToChat 时清除高亮缓存
    if matches!(key.code, KeyCode::Esc)
        && app.ui.browse_filter.is_empty()
        && app.ui.browse_role_filter.is_none()
    {
        app.ui.msg_lines_cache = None;
    }
}
