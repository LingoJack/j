use crate::command::chat::app::{ChatApp, ChatMode};
use crossterm::event::{KeyCode, KeyEvent};

/// 归档确认模式按键处理
pub fn handle_archive_confirm_mode(app: &mut ChatApp, key: KeyEvent) {
    if app.archive_editing_name {
        // 正在编辑自定义名称
        match key.code {
            KeyCode::Esc => {
                app.archive_editing_name = false;
                app.archive_custom_name.clear();
                app.archive_edit_cursor = 0;
            }
            KeyCode::Enter => {
                let name = if app.archive_custom_name.is_empty() {
                    app.archive_default_name.clone()
                } else {
                    app.archive_custom_name.clone()
                };
                // 验证名称
                if let Err(e) = super::super::archive::validate_archive_name(&name) {
                    app.show_toast(e, true);
                    return;
                }
                // 检查是否重名
                if super::super::archive::archive_exists(&name) {
                    // 直接覆盖
                    let _ = super::super::archive::delete_archive(&name);
                }
                app.do_archive(&name);
            }
            KeyCode::Backspace => {
                if app.archive_edit_cursor > 0 {
                    let chars: Vec<char> = app.archive_custom_name.chars().collect();
                    app.archive_custom_name = chars[..app.archive_edit_cursor - 1]
                        .iter()
                        .chain(chars[app.archive_edit_cursor..].iter())
                        .collect();
                    app.archive_edit_cursor -= 1;
                }
            }
            KeyCode::Left => {
                app.archive_edit_cursor = app.archive_edit_cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                let char_count = app.archive_custom_name.chars().count();
                if app.archive_edit_cursor < char_count {
                    app.archive_edit_cursor += 1;
                }
            }
            KeyCode::Char(c) => {
                let chars: Vec<char> = app.archive_custom_name.chars().collect();
                app.archive_custom_name = chars[..app.archive_edit_cursor]
                    .iter()
                    .chain(std::iter::once(&c))
                    .chain(chars[app.archive_edit_cursor..].iter())
                    .collect();
                app.archive_edit_cursor += 1;
            }
            _ => {}
        }
    } else {
        // 非编辑状态
        match key.code {
            KeyCode::Esc => {
                app.mode = ChatMode::Chat;
            }
            KeyCode::Enter => {
                // 使用默认名称归档
                let name = app.archive_default_name.clone();
                // 检查是否重名（generate_default_archive_name 应该已经处理了重名，但这里可能用户一直在同一个界面）
                if super::super::archive::archive_exists(&name) {
                    let _ = super::super::archive::delete_archive(&name);
                }
                app.do_archive(&name);
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                // 进入编辑自定义名称模式
                app.archive_editing_name = true;
                app.archive_custom_name.clear();
                app.archive_edit_cursor = 0;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // 仅清空对话，不归档
                app.clear_session();
                app.mode = ChatMode::Chat;
            }
            _ => {}
        }
    }
}

/// 归档列表模式按键处理
pub fn handle_archive_list_mode(app: &mut ChatApp, key: KeyEvent) {
    let count = app.archives.len();

    // 如果需要确认还原
    if app.restore_confirm_needed {
        match key.code {
            KeyCode::Esc => {
                app.restore_confirm_needed = false;
            }
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                app.do_restore();
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.mode = ChatMode::Chat;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if count > 0 {
                app.archive_list_index = if app.archive_list_index == 0 {
                    count - 1
                } else {
                    app.archive_list_index - 1
                };
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if count > 0 {
                app.archive_list_index = if app.archive_list_index >= count - 1 {
                    0
                } else {
                    app.archive_list_index + 1
                };
            }
        }
        KeyCode::Enter => {
            if count > 0 {
                // 如果当前会话有消息，需要确认
                if !app.session.messages.is_empty() {
                    app.restore_confirm_needed = true;
                } else {
                    app.do_restore();
                }
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            // 删除选中的归档
            if count > 0 {
                app.do_delete_archive();
            }
        }
        _ => {}
    }
}
