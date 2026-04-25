use crate::command::chat::app::{Action, ChatApp, CursorDirection};
use crossterm::event::{KeyCode, KeyEvent};

/// 归档确认模式按键处理
pub fn handle_archive_confirm_mode(app: &mut ChatApp, key: KeyEvent) {
    if app.ui.archive_editing_name {
        // 正在编辑自定义名称
        let action = match key.code {
            KeyCode::Esc => {
                // 取消编辑（不退出模式，只退出编辑状态）
                app.ui.archive_editing_name = false;
                app.ui.archive_custom_name.clear();
                app.ui.archive_edit_cursor = 0;
                return;
            }
            KeyCode::Enter => {
                // 使用自定义（或默认）名称归档
                let name = if app.ui.archive_custom_name.is_empty() {
                    app.ui.archive_default_name.clone()
                } else {
                    app.ui.archive_custom_name.clone()
                };
                // 验证名称
                if let Err(e) = super::super::archive::validate_archive_name(&name) {
                    app.update(Action::ShowToast(e, true));
                    return;
                }
                // 检查是否重名
                if super::super::archive::archive_exists(&name) {
                    let _ = super::super::archive::delete_archive(&name);
                }
                app.do_archive(&name);
                return;
            }
            KeyCode::Backspace => Action::ArchiveConfirmDeleteChar,
            KeyCode::Left => Action::ArchiveConfirmMoveCursor(CursorDirection::Up),
            KeyCode::Right => Action::ArchiveConfirmMoveCursor(CursorDirection::Down),
            KeyCode::Char(c) => Action::ArchiveConfirmInputChar(c),
            _ => return,
        };
        app.update(action);
    } else {
        // 非编辑状态
        match key.code {
            KeyCode::Esc => {
                app.update(Action::ExitToChat);
            }
            KeyCode::Enter => {
                // 使用默认名称归档
                let name = app.ui.archive_default_name.clone();
                if super::super::archive::archive_exists(&name) {
                    let _ = super::super::archive::delete_archive(&name);
                }
                app.do_archive(&name);
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                // 进入编辑自定义名称模式
                app.ui.archive_editing_name = true;
                app.ui.archive_custom_name.clear();
                app.ui.archive_edit_cursor = 0;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // 仅清空对话，不归档
                app.update(Action::ClearSession);
                app.update(Action::ExitToChat);
            }
            _ => {}
        }
    }
}

/// 归档列表模式按键处理
pub fn handle_archive_list_mode(app: &mut ChatApp, key: KeyEvent) {
    let count = app.ui.archives.len();

    // 如果需要确认还原
    if app.ui.restore_confirm_needed {
        match key.code {
            KeyCode::Esc => {
                app.ui.restore_confirm_needed = false;
            }
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                app.update(Action::RestoreArchive);
            }
            _ => {}
        }
        return;
    }

    let action = match key.code {
        KeyCode::Esc => Action::ExitToChat,
        KeyCode::Up | KeyCode::Char('k') => Action::ArchiveListNavigate(CursorDirection::Up),
        KeyCode::Down | KeyCode::Char('j') => Action::ArchiveListNavigate(CursorDirection::Down),
        KeyCode::Enter => {
            if count > 0 {
                // 如果当前会话有消息，需要确认
                if !app.state.session.messages.is_empty() {
                    app.ui.restore_confirm_needed = true;
                } else {
                    app.update(Action::RestoreArchive);
                }
            }
            return;
        }
        KeyCode::Char('d') | KeyCode::Char('D') if count > 0 => Action::DeleteArchive,
        _ => return,
    };
    app.update(action);
}
