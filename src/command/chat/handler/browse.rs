use super::super::render::copy_to_clipboard;
use crate::command::chat::app::{ChatApp, ChatMode};
use crossterm::event::{KeyCode, KeyEvent};

/// 消息浏览模式按键处理：↑↓ 选择消息，y/Enter 复制选中消息，Esc 退出
pub fn handle_browse_mode(app: &mut ChatApp, key: KeyEvent) {
    let msg_count = app.session.messages.len();
    if msg_count == 0 {
        app.mode = ChatMode::Chat;
        app.msg_lines_cache = None;
        return;
    }

    match key.code {
        KeyCode::Esc => {
            app.mode = ChatMode::Chat;
            app.msg_lines_cache = None; // 退出浏览模式时清除缓存，去掉高亮
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.browse_msg_index > 0 {
                app.browse_msg_index -= 1;
                app.browse_scroll_offset = 0; // 切换消息时从头显示
                app.msg_lines_cache = None; // 选中变化时清缓存
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.browse_msg_index < msg_count - 1 {
                app.browse_msg_index += 1;
                app.browse_scroll_offset = 0; // 切换消息时从头显示
                app.msg_lines_cache = None; // 选中变化时清缓存
            }
        }
        // A/D 细粒度滚动当前消息内容（每次 3 行）
        KeyCode::Char('a') | KeyCode::Char('A') => {
            app.browse_scroll_offset = app.browse_scroll_offset.saturating_sub(3);
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            app.browse_scroll_offset = app.browse_scroll_offset.saturating_add(3);
        }
        KeyCode::Enter | KeyCode::Char('y') => {
            // 复制选中消息的原始内容到剪切板
            if let Some(msg) = app.session.messages.get(app.browse_msg_index) {
                let content = msg.content.clone();
                let role_label = if msg.role == "assistant" {
                    "AI"
                } else if msg.role == "user" {
                    "用户"
                } else {
                    "系统"
                };
                if copy_to_clipboard(&content) {
                    app.show_toast(
                        format!("已复制第 {} 条{}消息", app.browse_msg_index + 1, role_label),
                        false,
                    );
                } else {
                    app.show_toast("复制到剪切板失败", true);
                }
            }
        }
        _ => {}
    }
}
