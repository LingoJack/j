use super::super::autocomplete::{
    AtPopupItem, SlashCommand, complete_at_direct, complete_command_mention, complete_file_mention,
    complete_skill_mention, get_filtered_all_items, get_filtered_command_names, get_filtered_files,
    get_filtered_skill_names, get_filtered_slash_commands, update_at_filter, update_command_filter,
    update_file_filter, update_skill_filter,
};
use super::super::storage::{ChatMessage, MessageRole};
use super::super::theme::ThemeName;
use crate::command::chat::app::{Action, ChatApp, ChatMode, ConfigTab, CursorDirection};
use crate::command::chat::infra::command;
use crate::command::chat::infra::hook::{HookContext, HookEvent};
use crate::command::chat::storage::agent_data_dir;
use crate::util::safe_lock;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// 处理 Chat 模式下的键盘事件，包括输入、快捷键、弹窗交互等；返回 true 表示退出
pub fn handle_chat_mode(app: &mut ChatApp, key: KeyEvent) -> bool {
    // Ctrl+C 强制退出
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }

    // ===== / 斜杠命令弹窗拦截 =====
    if app.ui.slash_popup_active {
        let filtered = get_filtered_slash_commands(&app.ui.slash_popup_filter);
        match key.code {
            KeyCode::Up => {
                if !filtered.is_empty() {
                    if app.ui.slash_popup_selected > 0 {
                        app.ui.slash_popup_selected -= 1;
                    } else {
                        app.ui.slash_popup_selected = filtered.len() - 1;
                    }
                }
                return false;
            }
            KeyCode::Down => {
                if !filtered.is_empty() {
                    if app.ui.slash_popup_selected < filtered.len() - 1 {
                        app.ui.slash_popup_selected += 1;
                    } else {
                        app.ui.slash_popup_selected = 0;
                    }
                }
                return false;
            }
            KeyCode::Tab | KeyCode::Enter => {
                if !filtered.is_empty() {
                    let sel = app.ui.slash_popup_selected.min(filtered.len() - 1);
                    let cmd = filtered[sel].clone();
                    execute_slash_command(app, &cmd);
                }
                app.ui.slash_popup_active = false;
                return false;
            }
            KeyCode::Esc => {
                app.ui.slash_popup_active = false;
                return false;
            }
            KeyCode::Backspace => {
                if !app.ui.slash_popup_filter.is_empty() {
                    app.ui.slash_popup_filter.pop();
                    // 同步更新 input
                    app.ui.set_input_text(
                        &format!("/{}", app.ui.slash_popup_filter),
                        app.ui.slash_popup_filter.len() + 1,
                    );
                    app.ui.slash_popup_selected = 0;
                } else {
                    // filter 为空时关闭弹窗并删除 /
                    app.ui.slash_popup_active = false;
                    app.ui.clear_input();
                }
                return false;
            }
            KeyCode::Char(c) => {
                // 空格关闭斜杠弹窗
                if c == ' ' {
                    app.ui.slash_popup_active = false;
                    // fall through to normal char handling
                } else {
                    app.ui.slash_popup_filter.push(c);
                    // 同步更新 input，使输入框显示 "/filter"
                    app.ui.set_input_text(
                        &format!("/{}", app.ui.slash_popup_filter),
                        app.ui.slash_popup_filter.len() + 1,
                    );
                    app.ui.slash_popup_selected = 0;
                    return false;
                }
            }
            _ => {}
        }
    }

    // ===== @ 补全弹窗拦截 =====
    if app.ui.at_popup_active {
        let filtered = get_filtered_all_items(app);
        match key.code {
            KeyCode::Up => {
                if !filtered.is_empty() {
                    if app.ui.at_popup_selected > 0 {
                        app.ui.at_popup_selected -= 1;
                    } else {
                        app.ui.at_popup_selected = filtered.len() - 1;
                    }
                }
                return false;
            }
            KeyCode::Down => {
                if !filtered.is_empty() {
                    if app.ui.at_popup_selected < filtered.len() - 1 {
                        app.ui.at_popup_selected += 1;
                    } else {
                        app.ui.at_popup_selected = 0;
                    }
                }
                return false;
            }
            KeyCode::Tab | KeyCode::Enter => {
                if !filtered.is_empty() {
                    let sel = app.ui.at_popup_selected.min(filtered.len() - 1);
                    let item = filtered[sel].clone();
                    match item {
                        AtPopupItem::Category(ref name) if name == "skill:" => {
                            let text = app.ui.input_text();
                            let chars: Vec<char> = text.chars().collect();
                            let cursor_pos = app.ui.cursor_char_idx();
                            let before: String =
                                chars[..app.ui.at_popup_start_pos].iter().collect();
                            let after: String = if cursor_pos < chars.len() {
                                chars[cursor_pos..].iter().collect()
                            } else {
                                String::new()
                            };
                            let replacement = "@skill:";
                            let new_cursor = before.chars().count() + replacement.chars().count();
                            app.ui.set_input_text(
                                &format!("{}{}{}", before, replacement, after),
                                new_cursor,
                            );
                            app.ui.at_popup_active = false;
                            app.ui.skill_popup_active = true;
                            app.ui.skill_popup_start_pos = app.ui.at_popup_start_pos;
                            app.ui.skill_popup_filter.clear();
                            app.ui.skill_popup_selected = 0;
                        }
                        AtPopupItem::Category(ref name) if name == "command:" => {
                            let text = app.ui.input_text();
                            let chars: Vec<char> = text.chars().collect();
                            let cursor_pos = app.ui.cursor_char_idx();
                            let before: String =
                                chars[..app.ui.at_popup_start_pos].iter().collect();
                            let after: String = if cursor_pos < chars.len() {
                                chars[cursor_pos..].iter().collect()
                            } else {
                                String::new()
                            };
                            let replacement = "@command:";
                            let new_cursor = before.chars().count() + replacement.chars().count();
                            app.ui.set_input_text(
                                &format!("{}{}{}", before, replacement, after),
                                new_cursor,
                            );
                            app.ui.at_popup_active = false;
                            app.ui.command_popup_active = true;
                            app.ui.command_popup_start_pos = app.ui.at_popup_start_pos;
                            app.ui.command_popup_filter.clear();
                            app.ui.command_popup_selected = 0;
                        }
                        AtPopupItem::Category(ref name) if name == "file:" => {
                            let text = app.ui.input_text();
                            let chars: Vec<char> = text.chars().collect();
                            let cursor_pos = app.ui.cursor_char_idx();
                            let before: String =
                                chars[..app.ui.at_popup_start_pos].iter().collect();
                            let after: String = if cursor_pos < chars.len() {
                                chars[cursor_pos..].iter().collect()
                            } else {
                                String::new()
                            };
                            let replacement = "@file:";
                            let new_cursor = before.chars().count() + replacement.chars().count();
                            app.ui.set_input_text(
                                &format!("{}{}{}", before, replacement, after),
                                new_cursor,
                            );
                            app.ui.at_popup_active = false;
                            app.ui.file_popup_active = true;
                            app.ui.file_popup_start_pos = app.ui.at_popup_start_pos;
                            app.ui.file_popup_filter.clear();
                            app.ui.file_popup_selected = 0;
                        }
                        AtPopupItem::Skill(_) | AtPopupItem::Command(_) | AtPopupItem::File(_) => {
                            complete_at_direct(app, &item);
                            app.ui.at_popup_active = false;
                        }
                        _ => {
                            app.ui.at_popup_active = false;
                        }
                    }
                } else {
                    app.ui.at_popup_active = false;
                }
                return false;
            }
            KeyCode::Esc => {
                app.ui.at_popup_active = false;
                return false;
            }
            KeyCode::Char(' ') => {
                // 空格关闭弹窗，正常处理字符
                app.ui.at_popup_active = false;
                // fall through to normal char handling below
            }
            KeyCode::Backspace => {
                // 先执行删除，然后检查弹窗状态
                app.ui.input_buffer.backspace();
                let cursor_pos = app.ui.cursor_char_idx();
                // 如果光标退回到 @ 之前，关闭弹窗
                if cursor_pos <= app.ui.at_popup_start_pos {
                    app.ui.at_popup_active = false;
                } else {
                    update_at_filter(app);
                }
                return false;
            }
            _ => {
                // 其他按键不拦截，落入正常处理
            }
        }
    }

    // ===== 文件补全弹窗拦截 =====
    if app.ui.file_popup_active {
        let filtered = get_filtered_files(app);
        match key.code {
            KeyCode::Up => {
                if !filtered.is_empty() {
                    if app.ui.file_popup_selected > 0 {
                        app.ui.file_popup_selected -= 1;
                    } else {
                        app.ui.file_popup_selected = filtered.len() - 1;
                    }
                }
                return false;
            }
            KeyCode::Down => {
                if !filtered.is_empty() {
                    if app.ui.file_popup_selected < filtered.len() - 1 {
                        app.ui.file_popup_selected += 1;
                    } else {
                        app.ui.file_popup_selected = 0;
                    }
                }
                return false;
            }
            KeyCode::Tab | KeyCode::Enter => {
                if !filtered.is_empty() {
                    let sel = app.ui.file_popup_selected.min(filtered.len() - 1);
                    let entry = filtered[sel].clone();
                    if entry.ends_with('/') {
                        // 目录：直接用 entry 作为新 filter（已包含完整路径）
                        app.ui.file_popup_filter = entry.clone();
                        // 更新 input 中的文本
                        let text = app.ui.input_text();
                        let chars: Vec<char> = text.chars().collect();
                        let cursor_pos = app.ui.cursor_char_idx();
                        let before: String = chars[..app.ui.file_popup_start_pos].iter().collect();
                        let after: String = if cursor_pos < chars.len() {
                            chars[cursor_pos..].iter().collect()
                        } else {
                            String::new()
                        };
                        let replacement = format!("@file:{}", app.ui.file_popup_filter);
                        let new_cursor = before.chars().count() + replacement.chars().count();
                        app.ui.set_input_text(
                            &format!("{}{}{}", before, replacement, after),
                            new_cursor,
                        );
                        app.ui.file_popup_selected = 0;
                    } else {
                        // 文件：entry 已包含完整相对路径，直接补全
                        complete_file_mention(app, &entry);
                        app.ui.file_popup_active = false;
                    }
                    return false;
                }
                // filtered 为空时，关闭弹窗，让 Enter 继续处理（发送消息）
                app.ui.file_popup_active = false;
                // fall through to normal Enter handling
            }
            KeyCode::Esc => {
                app.ui.file_popup_active = false;
                return false;
            }
            KeyCode::Backspace => {
                app.ui.input_buffer.backspace();
                let cursor_pos = app.ui.cursor_char_idx();
                // @file: 占 6 个字符，起始位置 + 6 = 冒号之后
                let prefix_end = app.ui.file_popup_start_pos + 6;
                if cursor_pos < prefix_end {
                    app.ui.file_popup_active = false;
                } else {
                    update_file_filter(app);
                }
                return false;
            }
            KeyCode::Char(c) => {
                // 空格关闭文件弹窗，让后续输入正常处理
                if c == ' ' {
                    app.ui.file_popup_active = false;
                    // fall through to normal char handling
                } else {
                    app.ui.input_buffer.insert_char(c);
                    update_file_filter(app);
                    return false;
                }
            }
            _ => {}
        }
    }

    // ===== 技能补全弹窗拦截 =====
    if app.ui.skill_popup_active {
        let filtered = get_filtered_skill_names(app);
        match key.code {
            KeyCode::Up => {
                if !filtered.is_empty() {
                    if app.ui.skill_popup_selected > 0 {
                        app.ui.skill_popup_selected -= 1;
                    } else {
                        app.ui.skill_popup_selected = filtered.len() - 1;
                    }
                }
                return false;
            }
            KeyCode::Down => {
                if !filtered.is_empty() {
                    if app.ui.skill_popup_selected < filtered.len() - 1 {
                        app.ui.skill_popup_selected += 1;
                    } else {
                        app.ui.skill_popup_selected = 0;
                    }
                }
                return false;
            }
            KeyCode::Tab | KeyCode::Enter => {
                if !filtered.is_empty() {
                    let sel = app.ui.skill_popup_selected.min(filtered.len() - 1);
                    let entry = filtered[sel].clone();
                    complete_skill_mention(app, &entry);
                    app.ui.skill_popup_active = false;
                    return false;
                }
                // filtered 为空时，关闭弹窗，让 Enter 继续处理（发送消息）
                app.ui.skill_popup_active = false;
                // fall through to normal Enter handling
            }
            KeyCode::Esc => {
                app.ui.skill_popup_active = false;
                return false;
            }
            KeyCode::Backspace => {
                app.ui.input_buffer.backspace();
                let cursor_pos = app.ui.cursor_char_idx();
                // @skill: 占 7 个字符，起始位置 + 7 = 冒号之后
                let prefix_end = app.ui.skill_popup_start_pos + 7;
                if cursor_pos < prefix_end {
                    app.ui.skill_popup_active = false;
                } else {
                    update_skill_filter(app);
                }
                return false;
            }
            KeyCode::Char(c) => {
                // 空格关闭技能弹窗，让后续输入正常处理
                if c == ' ' {
                    app.ui.skill_popup_active = false;
                    // fall through to normal char handling
                } else {
                    app.ui.input_buffer.insert_char(c);
                    update_skill_filter(app);
                    return false;
                }
            }
            _ => {}
        }
    }

    // ===== 命令补全弹窗拦截 =====
    if app.ui.command_popup_active {
        let filtered = get_filtered_command_names(app);
        match key.code {
            KeyCode::Up => {
                if !filtered.is_empty() {
                    if app.ui.command_popup_selected > 0 {
                        app.ui.command_popup_selected -= 1;
                    } else {
                        app.ui.command_popup_selected = filtered.len() - 1;
                    }
                }
                return false;
            }
            KeyCode::Down => {
                if !filtered.is_empty() {
                    if app.ui.command_popup_selected < filtered.len() - 1 {
                        app.ui.command_popup_selected += 1;
                    } else {
                        app.ui.command_popup_selected = 0;
                    }
                }
                return false;
            }
            KeyCode::Tab | KeyCode::Enter => {
                if !filtered.is_empty() {
                    let sel = app.ui.command_popup_selected.min(filtered.len() - 1);
                    let entry = filtered[sel].clone();
                    complete_command_mention(app, &entry);
                    app.ui.command_popup_active = false;
                    return false;
                }
                // filtered 为空时，关闭弹窗，让 Enter 继续处理（发送消息）
                app.ui.command_popup_active = false;
                // fall through to normal Enter handling
            }
            KeyCode::Esc => {
                app.ui.command_popup_active = false;
                return false;
            }
            KeyCode::Backspace => {
                app.ui.input_buffer.backspace();
                let cursor_pos = app.ui.cursor_char_idx();
                // @command: 占 9 个字符，起始位置 + 9 = 冒号之后
                let prefix_end = app.ui.command_popup_start_pos + 9;
                if cursor_pos < prefix_end {
                    app.ui.command_popup_active = false;
                } else {
                    update_command_filter(app);
                }
                return false;
            }
            KeyCode::Char(c) => {
                // 空格关闭命令弹窗，让后续输入正常处理
                if c == ' ' {
                    app.ui.command_popup_active = false;
                    // fall through to normal char handling
                } else {
                    app.ui.input_buffer.insert_char(c);
                    update_command_filter(app);
                    return false;
                }
            }
            _ => {}
        }
    }

    // ===== Ctrl 快捷键 → Actions =====

    // Ctrl+Y 复制最后一条 AI 回复
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('y') {
        app.update(Action::CopyLastAiReply);
        return false;
    }

    // Ctrl+B 进入消息浏览模式
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
        if !app.state.session.messages.is_empty() {
            app.ui.browse_msg_index = app.state.session.messages.len() - 1;
            app.ui.browse_scroll_offset = 0;
            app.ui.msg_lines_cache = None;
            app.update(Action::EnterMode(ChatMode::Browse));
        } else {
            app.update(Action::ShowToast("暂无消息可浏览".to_string(), true));
        }
        return false;
    }

    // Ctrl+G 打开日志窗口
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('g') {
        app.update(Action::OpenLogWindows);
        return false;
    }

    // Ctrl+O 切换工具详情展开/折叠
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('o') {
        app.update(Action::ToggleExpandTools);
        return false;
    }

    // Ctrl+E 打开配置界面
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e') {
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
        return false;
    }

    match key.code {
        KeyCode::Esc => {
            if app.state.is_loading {
                app.update(Action::CancelStream);
            } else {
                return true;
            }
        }

        KeyCode::Enter => {
            // Shift+Enter 或 Alt+Enter：插入换行符
            // Shift+Enter 需要 kitty keyboard protocol（kitty/WezTerm 等）
            // Alt+Enter 在所有终端均可用，作为通用备选
            if key.modifiers.contains(KeyModifiers::SHIFT)
                || key.modifiers.contains(KeyModifiers::ALT)
            {
                app.ui.input_buffer.insert_newline();
            } else if app.state.is_loading {
                // agent loop 期间：将用户消息追加到待处理队列
                let text = app.ui.input_text().trim().to_string();
                if !text.is_empty() {
                    // 展开 @command:name 引用
                    let text = command::expand_command_mentions(
                        &text,
                        &app.state.loaded_commands,
                        &app.state.agent_config.disabled_commands,
                    );
                    app.state
                        .session
                        .messages
                        .push(ChatMessage::text(MessageRole::User, &text));
                    {
                        let mut pending = safe_lock(
                            &app.state.pending_user_messages,
                            "handler_chat::pending_user_messages",
                        );
                        pending.push(ChatMessage::text(MessageRole::User, &text));
                    }
                    app.ui.clear_input();
                    app.ui.msg_lines_cache = None;
                    app.ui.auto_scroll = true;
                    app.ui.scroll_offset = u16::MAX;
                }
            } else {
                app.update(Action::SendMessage);
            }
        }

        // Up/Down: 多行输入时移动光标，单行时滚动消息
        KeyCode::Up => {
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
                app.ui
                    .input_buffer
                    .move_cursor_visual_up(app.ui.input_wrap_width);
            } else {
                app.update(Action::Scroll(CursorDirection::Up));
            }
        }
        KeyCode::Down => {
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
                app.ui
                    .input_buffer
                    .move_cursor_visual_down(app.ui.input_wrap_width);
            } else {
                app.update(Action::Scroll(CursorDirection::Down));
            }
        }
        KeyCode::PageUp => app.update(Action::PageScroll(CursorDirection::Up)),
        KeyCode::PageDown => app.update(Action::PageScroll(CursorDirection::Down)),

        // 光标移动
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

        // 删除
        KeyCode::Backspace => {
            app.ui.input_buffer.backspace();
        }
        KeyCode::Delete => {
            app.ui.input_buffer.delete_char();
        }

        // F1 帮助
        KeyCode::F(1) => {
            app.update(Action::ShowHelp);
        }
        // 输入框为空时，? 也可唤起帮助
        KeyCode::Char('?') if app.ui.is_input_empty() => {
            app.update(Action::ShowHelp);
        }
        KeyCode::Char(c) => {
            app.ui.input_buffer.insert_char(c);

            let cursor_pos = app.ui.cursor_char_idx();
            let input_text = app.ui.input_text();

            // / 斜杠命令弹窗触发逻辑（仅输入框为空时）
            if c == '/' && input_text == "/" {
                app.ui.slash_popup_active = true;
                app.ui.slash_popup_filter.clear();
                app.ui.slash_popup_selected = 0;
            }
            // @ 补全弹窗触发逻辑
            else if c == '@' {
                let valid = cursor_pos <= 1 || {
                    let chars: Vec<char> = input_text.chars().collect();
                    cursor_pos >= 2 && chars[cursor_pos - 2].is_whitespace()
                };
                if valid {
                    app.ui.at_popup_active = true;
                    app.ui.at_popup_start_pos = cursor_pos - 1;
                    app.ui.at_popup_filter.clear();
                    app.ui.at_popup_selected = 0;
                }
            } else if app.ui.at_popup_active {
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
                } else if app.ui.at_popup_filter == "command:" {
                    app.ui.at_popup_active = false;
                    app.ui.command_popup_active = true;
                    app.ui.command_popup_start_pos = app.ui.at_popup_start_pos;
                    app.ui.command_popup_filter.clear();
                    app.ui.command_popup_selected = 0;
                }
            }
        }

        // Tab：无弹窗时切换 bypass 模式
        KeyCode::Tab => {
            app.update(Action::ToggleAutoApprove);
        }

        _ => {}
    }

    false
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
            // 检查 @ 是否在有效位置（行首或前面是空白）
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
        // 光标不在任何 @mention 范围内，关闭所有弹窗
        close_all_popups(app);
        return;
    };

    // 检查光标是否在该 @mention 的有效范围内
    // 有效范围：从 @ 到第一个空白字符之前（不含空白字符本身）
    // 如果光标停在 mention 末尾（后面是空白），应该激活弹窗
    // 如果光标停在空白字符后面（前面是空白），不应该激活弹窗

    // 检查光标前面的字符：如果是空白，说明光标已离开 mention
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

    // 判断是否匹配特定类型的 mention
    if let Some(stripped) = after_at.strip_prefix("skill:") {
        // 激活技能弹窗
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
        // 激活文件弹窗
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
        // 激活命令弹窗
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
fn execute_slash_command(app: &mut ChatApp, cmd: &SlashCommand) {
    // 清空输入框
    app.ui.clear_input();

    match cmd {
        SlashCommand::Copy => {
            app.update(Action::CopyLastAiReply);
        }
        SlashCommand::Log => {
            app.update(Action::OpenLogWindows);
        }
        SlashCommand::Browse => {
            if !app.state.session.messages.is_empty() {
                app.ui.browse_msg_index = app.state.session.messages.len() - 1;
                app.ui.browse_scroll_offset = 0;
                app.ui.msg_lines_cache = None;
                app.update(Action::EnterMode(ChatMode::Browse));
            } else {
                app.update(Action::ShowToast("暂无消息可浏览".to_string(), true));
            }
        }
        SlashCommand::Config => {
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
        SlashCommand::Model => {
            if !app.state.agent_config.providers.is_empty() {
                app.ui
                    .model_list_state
                    .select(Some(app.state.agent_config.active_index));
                app.update(Action::EnterMode(ChatMode::SelectModel));
            }
        }
        SlashCommand::Archive => {
            if app.state.session.messages.is_empty() {
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
            dump_current_request(app, false);
        }
        SlashCommand::DumpProcessed => {
            dump_current_request(app, true);
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

/// 将当前 session 的 system prompt + messages 导出到
/// ~/.jdata/agent/dumps/<subdir>_<ts>/ 目录，分文件存放。
/// 同时导出所有当前注册的 Teammate 和运行中子 Agent 的数据。
///
/// `processed=false`（`/dump`）：导出原始 session messages，不经过任何处理管线。
/// `processed=true`（`/dump-processed`）：对 messages 应用与 agent loop 一致的处理管线：
/// rolling window → micro_compact → PreLlmRequest hooks → sanitize，
/// 产出与最终发给 LLM 一致的数据。
fn dump_current_request(app: &mut ChatApp, processed: bool) {
    let mut system_prompt = app.build_current_system_prompt();
    // 未处理模式：导出全部原生 message（session.messages 原样导出）
    // 已处理模式：经过完整管线（window → micro_compact → hooks → sanitize）
    let mut messages = if processed {
        app.build_api_messages()
    } else {
        app.state.session.messages.clone()
    };

    if processed {
        // Layer 1: micro_compact（替换旧 tool results）
        {
            use crate::command::chat::context::compact;
            let compact_config = &app.state.agent_config.compact;
            if compact_config.enabled {
                compact::micro_compact(
                    &mut messages,
                    compact_config.keep_recent,
                    &compact_config.micro_compact_exempt_tools,
                );
            }
        }
        // Layer 2: PreLlmRequest hook 链（内置→用户→项目→session）
        {
            let hook_manager = app.hook_manager.lock();
            if let Ok(mgr) = hook_manager
                && mgr.has_hooks_for(HookEvent::PreLlmRequest)
            {
                let model_name = app.active_model_name();
                let ctx = HookContext {
                    event: HookEvent::PreLlmRequest,
                    messages: Some(messages.clone()),
                    system_prompt: system_prompt.clone(),
                    model: Some(model_name),
                    session_id: Some(app.session_id.clone()),
                    cwd: std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| ".".to_string()),
                    ..Default::default()
                };
                if let Some(result) = mgr.execute(
                    HookEvent::PreLlmRequest,
                    ctx,
                    &app.state.agent_config.disabled_hooks,
                ) {
                    if let Some(new_msgs) = result.messages {
                        messages = new_msgs;
                    }
                    if let Some(new_prompt) = result.system_prompt {
                        system_prompt = Some(new_prompt);
                    }
                    if let Some(inject) = result.inject_messages {
                        messages.extend(inject);
                    }
                }
            }
        }
        // Layer 3: sanitize（移除孤立 tool_call / tool result）
        {
            use crate::command::chat::agent::api::sanitize_messages;
            messages = sanitize_messages(&messages);
        }
    }

    let timestamp = chrono::Local::now().format("%Y_%m_%d_%H_%M_%S").to_string();
    let subdir = if processed { "processed" } else { "dump" };
    let dump_dir = agent_data_dir()
        .join("dumps")
        .join(format!("{}_{}", subdir, timestamp));
    if let Err(e) = std::fs::create_dir_all(&dump_dir) {
        app.update(Action::ShowToast(
            format!("创建 dump 目录失败: {}", e),
            true,
        ));
        return;
    }

    // 写入处理管线说明
    if processed {
        let pipeline_info = "处理管线: window.select_messages (三阶段: 时间保底 → 豁免保底 → 比例配额+溢出) → micro_compact → PreLlmRequest hooks → sanitize_messages\n\
             - window: 最近 keep_recent*2 个 unit 无条件保留；含豁免工具 (LoadSkill/Task/Todo/Ask/...) 的 ToolGroup 优先保留；剩余预算按 User:AsstText:ToolGroup = 35:25:40 分配，tier 间有溢出。\n\
             - micro_compact: 将旧 tool result (>800 chars) 替换为 [Previous: used X]，最近 keep_recent 个保留原样，豁免工具不压缩。\n\
             - PreLlmRequest hooks 已执行（含内置 hooks: tasks_status, background_status, session_state, teammates_status, todo_nag）。\n\
             - sanitize_messages: 移除孤立的 tool_call / tool result。\n\
             注意: auto_compact (LLM 摘要) 需要 API 调用，未在此处执行。\n";
        let _ = std::fs::write(dump_dir.join("pipeline.txt"), pipeline_info);
    }

    if let Err(e) = write_agent_dump(&dump_dir, system_prompt.as_deref(), &messages) {
        app.update(Action::ShowToast(e, true));
        return;
    }

    // 导出 teammates
    let teammate_count = dump_teammates(app, &dump_dir);
    // 导出运行中的子 Agent
    let sub_agent_count = dump_sub_agents(app, &dump_dir);

    let mut toast = if processed {
        format!("已导出 processed 数据到 {}", dump_dir.display())
    } else {
        format!("已导出到 {}", dump_dir.display())
    };
    if teammate_count > 0 || sub_agent_count > 0 {
        toast.push_str(&format!(
            "（含 {} 个 teammate, {} 个 sub-agent）",
            teammate_count, sub_agent_count
        ));
    }
    app.update(Action::ShowToast(toast, false));
}

/// 写入单个 agent 的 system_prompt.md + messages.json 到指定目录
fn write_agent_dump(
    dir: &std::path::Path,
    system_prompt: Option<&str>,
    messages: &[ChatMessage],
) -> Result<(), String> {
    let sp_content = system_prompt
        .map(|s| s.to_string())
        .unwrap_or_else(|| "（未设置 system prompt）".to_string());
    std::fs::write(dir.join("system_prompt.md"), sp_content)
        .map_err(|e| format!("写入 {}/system_prompt.md 失败: {}", dir.display(), e))?;

    let msgs_content = serde_json::to_string_pretty(messages)
        .map_err(|e| format!("序列化 messages 失败: {}", e))?;
    std::fs::write(dir.join("messages.json"), msgs_content)
        .map_err(|e| format!("写入 {}/messages.json 失败: {}", dir.display(), e))?;
    Ok(())
}

/// 将所有 teammate 写入 dump_dir/teammates/<name>/，返回成功导出的数量
fn dump_teammates(app: &ChatApp, dump_dir: &std::path::Path) -> usize {
    let manager = match app.teammate_manager.lock() {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if manager.teammates.is_empty() {
        return 0;
    }
    let teammates_root = dump_dir.join("teammates");
    if std::fs::create_dir_all(&teammates_root).is_err() {
        return 0;
    }

    let mut count = 0;
    for (name, handle) in &manager.teammates {
        let safe_name = sanitize_dir_name(name);
        let tm_dir = teammates_root.join(&safe_name);
        if std::fs::create_dir_all(&tm_dir).is_err() {
            continue;
        }
        let sp_snapshot = handle
            .system_prompt_snapshot
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();
        let msgs_snapshot = handle
            .messages_snapshot
            .lock()
            .map(|m| m.clone())
            .unwrap_or_default();
        if write_agent_dump(&tm_dir, Some(&sp_snapshot), &msgs_snapshot).is_ok() {
            count += 1;
        }
    }
    count
}

/// 将正在运行的子 Agent 写入 dump_dir/subagents/<id>/，返回成功导出的数量
fn dump_sub_agents(app: &ChatApp, dump_dir: &std::path::Path) -> usize {
    let snapshots = app.sub_agent_tracker.snapshot_running();
    if snapshots.is_empty() {
        return 0;
    }
    let sub_root = dump_dir.join("subagents");
    if std::fs::create_dir_all(&sub_root).is_err() {
        return 0;
    }
    let mut count = 0;
    for (id, description, mode, sp, msgs) in snapshots {
        let safe_id = sanitize_dir_name(&id);
        let agent_dir = sub_root.join(&safe_id);
        if std::fs::create_dir_all(&agent_dir).is_err() {
            continue;
        }
        let meta = format!("id: {}\nmode: {}\ndescription: {}\n", id, mode, description);
        if std::fs::write(agent_dir.join("meta.txt"), meta).is_err() {
            continue;
        }
        if write_agent_dump(&agent_dir, Some(&sp), &msgs).is_ok() {
            count += 1;
        }
    }
    count
}

/// 将不适合做目录名的字符替换为 `_`
fn sanitize_dir_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
