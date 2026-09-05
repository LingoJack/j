mod dump;
mod popups;

use crate::command::chat::app::{Action, ChatApp, ChatMode, ConfigTab, CursorDirection};
use crate::command::chat::infra::command;
use crate::command::chat::input::autocomplete::{SlashCommand, update_at_filter};
use crate::command::chat::render::theme::ThemeName;
use crate::command::chat::storage::{ChatMessage, ImageData, MessageRole};
use crate::util::safe_lock;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use popups::PopupResult;

/// 处理 Chat 模式下的键盘事件，包括输入、快捷键、弹窗交互等；返回 true 表示退出
pub fn handle_chat_mode(app: &mut ChatApp, key: KeyEvent) -> bool {
    // Ctrl+C 强制退出
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }

    // 弹窗拦截：依次检查 5 个弹窗，已激活的弹窗优先消费按键
    if app.ui.slash_popup_active
        && let PopupResult::Handled = popups::handle_slash_popup(app, key)
    {
        return false;
    }
    if app.ui.at_popup_active
        && let PopupResult::Handled = popups::handle_at_popup(app, key)
    {
        return false;
    }
    if app.ui.file_popup_active
        && let PopupResult::Handled = popups::handle_file_popup(app, key)
    {
        return false;
    }
    if app.ui.skill_popup_active
        && let PopupResult::Handled = popups::handle_skill_popup(app, key)
    {
        return false;
    }
    if app.ui.command_popup_active
        && let PopupResult::Handled = popups::handle_command_popup(app, key)
    {
        return false;
    }

    // Ctrl 快捷键
    if handle_ctrl_shortcut(app, key) {
        return false;
    }

    handle_main_key(app, key)
}

/// 处理 Ctrl+X 系列快捷键，返回 true 表示已消费
fn handle_ctrl_shortcut(app: &mut ChatApp, key: KeyEvent) -> bool {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return false;
    }
    let KeyCode::Char(c) = key.code else {
        return false;
    };
    match c {
        'y' => {
            app.update(Action::CopyLastAiReply);
            true
        }
        'b' => {
            let filtered = app.browse_filtered_indices();
            if let Some(&last_idx) = filtered.last() {
                app.ui.browse_msg_index = last_idx;
                app.ui.browse_scroll_offset = 0;
                app.ui.msg_lines_cache = None;
                app.update(Action::EnterMode(ChatMode::Browse));
            } else {
                app.update(Action::ShowToast("暂无消息可浏览".to_string(), true));
            }
            true
        }
        'g' => {
            app.update(Action::OpenLogWindows);
            true
        }
        'o' => {
            app.update(Action::ToggleExpandTools);
            true
        }
        'e' => {
            enter_config_mode(app);
            true
        }
        'v' => {
            // Ctrl+V：粘贴剪贴板图片（无图片时回退为文本粘贴）
            app.paste_from_clipboard();
            true
        }
        _ => false,
    }
}

/// 配置模式入口（Ctrl+E 或 /config）
fn enter_config_mode(app: &mut ChatApp) {
    app.ui.config_provider_idx = app
        .state
        .agent_config
        .active_index
        .min(app.state.agent_config.providers.len().saturating_sub(1));
    app.ui.config_field_idx = 0;
    app.ui.config_editing = false;
    app.ui.config_edit_buf.clear();
    app.ui.config_scroll_offset = 0;
    app.update(Action::EnterMode(ChatMode::Config));
}

/// 主键位处理（弹窗与 Ctrl 快捷键之外的所有按键）；返回 true 表示退出
fn handle_main_key(app: &mut ChatApp, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => {
            if app.state.is_loading {
                app.update(Action::CancelStream);
            } else {
                return true;
            }
        }

        KeyCode::Enter => handle_enter_key(app, key),

        KeyCode::Up => handle_arrow_vertical(app, CursorDirection::Up),
        KeyCode::Down => handle_arrow_vertical(app, CursorDirection::Down),
        KeyCode::PageUp => app.update(Action::PageScroll(CursorDirection::Up)),
        KeyCode::PageDown => app.update(Action::PageScroll(CursorDirection::Down)),

        KeyCode::Left => {
            app.ui.input_buffer.move_cursor_back();
            check_and_activate_mention_popup(app);
        }
        KeyCode::Right => {
            app.ui.input_buffer.move_cursor_forward();
            check_and_activate_mention_popup(app);
        }
        KeyCode::Home => {
            app.ui.input_buffer.move_cursor_head();
            close_all_popups(app);
        }
        KeyCode::End => {
            app.ui.input_buffer.move_cursor_end();
            close_all_popups(app);
        }

        KeyCode::Backspace => {
            // 输入框为空且有图片附件时：移除最后一张图片
            if app.ui.is_input_empty() && !app.ui.pending_images.is_empty() {
                if app.ui.pending_images.pop().is_some() {
                    app.show_toast("已移除最后一张图片附件", false);
                }
            } else {
                app.ui.input_buffer.backspace();
                check_and_activate_mention_popup(app);
            }
        }
        KeyCode::Delete => {
            app.ui.input_buffer.delete_char();
            check_and_activate_mention_popup(app);
        }

        KeyCode::F(1) => {
            app.update(Action::ShowHelp);
        }
        KeyCode::Char('?') if app.ui.is_input_empty() => {
            app.update(Action::ShowHelp);
        }
        KeyCode::Char(c) => handle_char_input(app, c),

        // Tab：无弹窗时切换 bypass 模式
        KeyCode::Tab => {
            app.update(Action::ToggleAutoApprove);
        }

        _ => {}
    }

    false
}

/// Enter 键处理：Shift/Alt+Enter 插入换行、loading 中入队、否则发送
fn handle_enter_key(app: &mut ChatApp, key: KeyEvent) {
    // Shift+Enter 需要 kitty keyboard protocol（kitty/WezTerm 等），
    // Alt+Enter 在所有终端均可用，作为通用备选
    if key.modifiers.contains(KeyModifiers::SHIFT) || key.modifiers.contains(KeyModifiers::ALT) {
        app.ui.input_buffer.insert_newline();
        return;
    }
    if !app.state.is_loading {
        app.update(Action::SendMessage);
        return;
    }

    // agent loop 期间：将用户消息追加到待处理队列
    let text = app.ui.input_text().trim().to_string();
    let images: Vec<ImageData> = std::mem::take(&mut app.ui.pending_images)
        .into_iter()
        .map(|p| p.data)
        .collect();
    if text.is_empty() && images.is_empty() {
        return;
    }
    let text = if text.is_empty() {
        "[图片]".to_string()
    } else {
        command::expand_command_mentions(
            &text,
            &app.state.loaded_commands,
            &app.state.agent_config.disabled_commands,
        )
    };
    let mut user_msg = ChatMessage::text(MessageRole::User, &text);
    if !images.is_empty() {
        user_msg.images = Some(images);
    }
    // 双通道写入：display_messages 渲染 + context_messages 持久化
    app.push_both_channels(user_msg.clone());
    {
        let mut pending = safe_lock(
            &app.state.pending_user_messages,
            "handler_chat::pending_user_messages",
        );
        pending.push(user_msg);
    }
    app.ui.clear_input();
    app.ui.msg_lines_cache = None;
    app.ui.auto_scroll = true;
    app.ui.scroll_offset = usize::MAX;
}

/// 上下箭头：多行输入或视觉换行时移动光标，否则滚动消息列表
fn handle_arrow_vertical(app: &mut ChatApp, dir: CursorDirection) {
    let line_count = app.ui.input_buffer.line_count();
    let has_visual_wrap = if line_count == 1 && app.ui.input_wrap_width > 0 {
        let (row, _) = app.ui.input_buffer.cursor();
        app.ui
            .input_buffer
            .visual_line_count(row, app.ui.input_wrap_width)
            > 1
    } else {
        false
    };
    if line_count > 1 || has_visual_wrap {
        match dir {
            CursorDirection::Up => app
                .ui
                .input_buffer
                .move_cursor_visual_up(app.ui.input_wrap_width),
            CursorDirection::Down => app
                .ui
                .input_buffer
                .move_cursor_visual_down(app.ui.input_wrap_width),
        }
    } else {
        app.update(Action::Scroll(dir));
    }
}

/// 输入字符：插入到 buffer，并根据 / 或 @ 触发对应弹窗
fn handle_char_input(app: &mut ChatApp, c: char) {
    app.ui.input_buffer.insert_char(c);

    let cursor_pos = app.ui.cursor_char_idx();
    let input_text = app.ui.input_text();

    // / 斜杠命令弹窗触发逻辑（仅输入框为空时）
    if c == '/' && input_text == "/" {
        app.ui.slash_popup_active = true;
        app.ui.slash_popup_filter.clear();
        app.ui.slash_popup_selected = 0;
        return;
    }

    // @ 补全弹窗触发逻辑
    if c == '@' {
        let valid = cursor_pos <= 1 || {
            let chars: Vec<char> = input_text.chars().collect();
            cursor_pos >= 2 && chars[cursor_pos - 2].is_whitespace()
        };
        if valid {
            app.ui.at_popup_active = true;
            app.ui.at_popup_start_pos = cursor_pos - 1;
            app.ui.at_popup_filter.clear();
            app.ui.at_popup_selected = 0;
            // 打开弹窗时触发文件索引刷新，确保最新
            app.file_index.refresh();
        }
        return;
    }

    // 已激活的 @ 弹窗：根据用户输入进一步切换为子弹窗（skill:/file:/command:）
    if app.ui.at_popup_active {
        update_at_filter(app);
        if app.ui.at_popup_filter == "skill:" {
            app.ui.at_popup_active = false;
            app.ui.skill_popup_active = true;
            app.ui.skill_popup_start_pos = app.ui.at_popup_start_pos;
            app.ui.skill_popup_filter.clear();
            app.ui.skill_popup_selected = 0;
        } else if app.ui.at_popup_filter == "file:" {
            app.ui.at_popup_active = false;
            app.ui.file_popup_active = true;
            app.ui.file_popup_start_pos = app.ui.at_popup_start_pos;
            app.ui.file_popup_filter.clear();
            app.ui.file_popup_selected = 0;
            // file: 弹窗激活时刷新文件索引
            app.file_index.refresh();
        } else if app.ui.at_popup_filter == "command:" {
            app.ui.at_popup_active = false;
            app.ui.command_popup_active = true;
            app.ui.command_popup_start_pos = app.ui.at_popup_start_pos;
            app.ui.command_popup_filter.clear();
            app.ui.command_popup_selected = 0;
        }
    }
}

/// 检测光标是否在某个 mention 范围内，若是则激活对应的补全弹窗
fn check_and_activate_mention_popup(app: &mut ChatApp) {
    let text = app.ui.input_text();
    let chars: Vec<char> = text.chars().collect();
    let pos = app.ui.cursor_char_idx();

    // 从光标位置向前搜索 @ 符号
    let mut at_pos: Option<usize> = None;
    for i in (0..pos).rev() {
        if chars.get(i) == Some(&'@') {
            // @ 在行首或前面是空白时为有效位置
            if i == 0 || chars.get(i - 1).map(|c| c.is_whitespace()).unwrap_or(true) {
                at_pos = Some(i);
                break;
            }
        }
        // 遇到空白字符停止搜索（@mention 不能跨空格）
        if chars.get(i).map(|c| c.is_whitespace()).unwrap_or(false) {
            break;
        }
    }

    let Some(at_idx) = at_pos else {
        close_all_popups(app);
        return;
    };

    // 光标前面是空白说明已离开 mention
    if pos > 0
        && chars
            .get(pos - 1)
            .map(|c| c.is_whitespace())
            .unwrap_or(false)
    {
        close_all_popups(app);
        return;
    }

    // 提取 @ 之后的内容用于判断类型
    let after_at: String = chars[at_idx + 1..pos.min(chars.len())].iter().collect();

    if let Some(stripped) = after_at.strip_prefix("skill:") {
        app.ui.at_popup_active = false;
        app.ui.file_popup_active = false;
        app.ui.command_popup_active = false;
        app.ui.skill_popup_active = true;
        app.ui.skill_popup_start_pos = at_idx;
        app.ui.skill_popup_filter = if after_at.len() > 6 {
            stripped.to_string()
        } else {
            String::new()
        };
        app.ui.skill_popup_selected = 0;
    } else if let Some(stripped) = after_at.strip_prefix("file:") {
        app.ui.at_popup_active = false;
        app.ui.skill_popup_active = false;
        app.ui.command_popup_active = false;
        app.ui.file_popup_active = true;
        app.ui.file_popup_start_pos = at_idx;
        app.ui.file_popup_filter = if after_at.len() > 5 {
            stripped.to_string()
        } else {
            String::new()
        };
        app.ui.file_popup_selected = 0;
    } else if let Some(stripped) = after_at.strip_prefix("command:") {
        app.ui.at_popup_active = false;
        app.ui.skill_popup_active = false;
        app.ui.file_popup_active = false;
        app.ui.command_popup_active = true;
        app.ui.command_popup_start_pos = at_idx;
        app.ui.command_popup_filter = if after_at.len() > 8 {
            stripped.to_string()
        } else {
            String::new()
        };
        app.ui.command_popup_selected = 0;
    } else {
        // 普通的 @ 弹窗
        app.ui.skill_popup_active = false;
        app.ui.file_popup_active = false;
        app.ui.command_popup_active = false;
        app.ui.at_popup_active = true;
        app.ui.at_popup_start_pos = at_idx;
        app.ui.at_popup_filter = after_at;
        app.ui.at_popup_selected = 0;
    }
}

/// 关闭所有补全弹窗
fn close_all_popups(app: &mut ChatApp) {
    app.ui.at_popup_active = false;
    app.ui.skill_popup_active = false;
    app.ui.file_popup_active = false;
    app.ui.command_popup_active = false;
    app.ui.slash_popup_active = false;
}

/// 执行斜杠命令
pub(super) fn execute_slash_command(app: &mut ChatApp, cmd: &SlashCommand) {
    app.ui.clear_input();

    match cmd {
        SlashCommand::Copy => {
            app.update(Action::CopyLastAiReply);
        }
        SlashCommand::Log => {
            app.update(Action::OpenLogWindows);
        }
        SlashCommand::Browse => {
            let filtered = app.browse_filtered_indices();
            if let Some(&last_idx) = filtered.last() {
                app.ui.browse_msg_index = last_idx;
                app.ui.browse_scroll_offset = 0;
                app.ui.msg_lines_cache = None;
                app.update(Action::EnterMode(ChatMode::Browse));
            } else {
                app.update(Action::ShowToast("暂无消息可浏览".to_string(), true));
            }
        }
        SlashCommand::Config => enter_config_mode(app),
        SlashCommand::Model => {
            if !app.state.agent_config.providers.is_empty() {
                app.ui
                    .model_list_state
                    .select(Some(app.state.agent_config.active_index));
                app.update(Action::EnterMode(ChatMode::SelectModel));
            }
        }
        SlashCommand::Archive => {
            if safe_lock(&app.display_messages, "slash_archive::empty").is_empty() {
                app.update(Action::ShowToast(
                    "当前对话为空，无法归档".to_string(),
                    true,
                ));
            } else {
                app.update(Action::StartArchiveConfirm);
            }
        }
        SlashCommand::Clear => {
            app.update(Action::ClearSession);
        }
        SlashCommand::Theme => {
            let all = ThemeName::all();
            let current_idx = all
                .iter()
                .position(|t| *t == app.state.agent_config.theme)
                .unwrap_or(0);
            app.ui.theme_list_state.select(Some(current_idx));
            app.update(Action::EnterMode(ChatMode::SelectTheme));
        }
        SlashCommand::Resume => {
            app.update(Action::LoadSessionList);
            app.ui.config_tab = ConfigTab::Session;
            app.ui.config_scroll_offset = 0;
            app.update(Action::EnterMode(ChatMode::Config));
        }
        SlashCommand::Dump => {
            dump::dump_current_request(app, false);
        }
        SlashCommand::DumpProcessed => {
            dump::dump_current_request(app, true);
        }
        SlashCommand::Teammate => {
            app.ui.config_tab = ConfigTab::Teammates;
            app.ui.teammate_list_index = 0;
            app.ui.config_field_idx = 0;
            app.ui.config_scroll_offset = 0;
            app.update(Action::EnterMode(ChatMode::Config));
        }
    }
}
