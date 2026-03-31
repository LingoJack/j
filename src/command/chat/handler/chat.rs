use super::super::autocomplete::{
    complete_at_mention, complete_command_mention, complete_file_mention, complete_skill_mention,
    get_filtered_command_names, get_filtered_files, get_filtered_skill_names, get_filtered_skills,
    update_at_filter, update_command_filter, update_file_filter, update_skill_filter,
};
use crate::command::chat::app::{Action, ChatApp, ChatMode, CursorDirection};
use crate::util::safe_lock;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_chat_mode(app: &mut ChatApp, key: KeyEvent) -> bool {
    // Ctrl+C 强制退出
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }

    // ===== @ 补全弹窗拦截 =====
    if app.ui.at_popup_active {
        let filtered = get_filtered_skills(app);
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
                    let name = filtered[sel].clone();
                    if name == "skill:" {
                        // 选中 skill: 选项，补全 @skill: 到输入框，然后切换到技能补全模式
                        let chars: Vec<char> = app.ui.input.chars().collect();
                        let before: String = chars[..app.ui.at_popup_start_pos].iter().collect();
                        let after: String = if app.ui.cursor_pos < chars.len() {
                            chars[app.ui.cursor_pos..].iter().collect()
                        } else {
                            String::new()
                        };
                        let replacement = "@skill:";
                        let new_cursor = before.chars().count() + replacement.chars().count();
                        app.ui.input = format!("{}{}{}", before, replacement, after);
                        app.ui.cursor_pos = new_cursor;
                        app.ui.at_popup_active = false;
                        app.ui.skill_popup_active = true;
                        app.ui.skill_popup_start_pos = app.ui.at_popup_start_pos;
                        app.ui.skill_popup_filter.clear();
                        app.ui.skill_popup_selected = 0;
                    } else if name == "command:" {
                        // 选中 command: 选项，补全 @command: 到输入框，然后切换到命令补全模式
                        let chars: Vec<char> = app.ui.input.chars().collect();
                        let before: String = chars[..app.ui.at_popup_start_pos].iter().collect();
                        let after: String = if app.ui.cursor_pos < chars.len() {
                            chars[app.ui.cursor_pos..].iter().collect()
                        } else {
                            String::new()
                        };
                        let replacement = "@command:";
                        let new_cursor = before.chars().count() + replacement.chars().count();
                        app.ui.input = format!("{}{}{}", before, replacement, after);
                        app.ui.cursor_pos = new_cursor;
                        app.ui.at_popup_active = false;
                        app.ui.command_popup_active = true;
                        app.ui.command_popup_start_pos = app.ui.at_popup_start_pos;
                        app.ui.command_popup_filter.clear();
                        app.ui.command_popup_selected = 0;
                    } else if name == "file:" {
                        // 选中 file: 选项，补全 @file: 到输入框，然后切换到文件补全模式
                        let chars: Vec<char> = app.ui.input.chars().collect();
                        let before: String = chars[..app.ui.at_popup_start_pos].iter().collect();
                        let after: String = if app.ui.cursor_pos < chars.len() {
                            chars[app.ui.cursor_pos..].iter().collect()
                        } else {
                            String::new()
                        };
                        let replacement = "@file:";
                        let new_cursor = before.chars().count() + replacement.chars().count();
                        app.ui.input = format!("{}{}{}", before, replacement, after);
                        app.ui.cursor_pos = new_cursor;
                        app.ui.at_popup_active = false;
                        app.ui.file_popup_active = true;
                        app.ui.file_popup_start_pos = app.ui.at_popup_start_pos;
                        app.ui.file_popup_filter.clear();
                        app.ui.file_popup_selected = 0;
                    } else {
                        complete_at_mention(app, &name);
                        app.ui.at_popup_active = false;
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
                if app.ui.cursor_pos > 0 {
                    let start = app
                        .ui
                        .input
                        .char_indices()
                        .nth(app.ui.cursor_pos - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end = app
                        .ui
                        .input
                        .char_indices()
                        .nth(app.ui.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(app.ui.input.len());
                    app.ui.input.drain(start..end);
                    app.ui.cursor_pos -= 1;
                }
                // 如果光标退回到 @ 之前，关闭弹窗
                if app.ui.cursor_pos <= app.ui.at_popup_start_pos {
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
                        let chars: Vec<char> = app.ui.input.chars().collect();
                        let before: String = chars[..app.ui.file_popup_start_pos].iter().collect();
                        let after: String = if app.ui.cursor_pos < chars.len() {
                            chars[app.ui.cursor_pos..].iter().collect()
                        } else {
                            String::new()
                        };
                        let replacement = format!("@file:{}", app.ui.file_popup_filter);
                        let new_cursor = before.chars().count() + replacement.chars().count();
                        app.ui.input = format!("{}{}{}", before, replacement, after);
                        app.ui.cursor_pos = new_cursor;
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
                if app.ui.cursor_pos > 0 {
                    let start = app
                        .ui
                        .input
                        .char_indices()
                        .nth(app.ui.cursor_pos - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end = app
                        .ui
                        .input
                        .char_indices()
                        .nth(app.ui.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(app.ui.input.len());
                    app.ui.input.drain(start..end);
                    app.ui.cursor_pos -= 1;
                }
                // @file: 占 6 个字符，起始位置 + 6 = 冒号之后
                let prefix_end = app.ui.file_popup_start_pos + 6;
                if app.ui.cursor_pos < prefix_end {
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
                    let byte_idx = app
                        .ui
                        .input
                        .char_indices()
                        .nth(app.ui.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(app.ui.input.len());
                    app.ui.input.insert(byte_idx, c);
                    app.ui.cursor_pos += 1;
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
                if app.ui.cursor_pos > 0 {
                    let start = app
                        .ui
                        .input
                        .char_indices()
                        .nth(app.ui.cursor_pos - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end = app
                        .ui
                        .input
                        .char_indices()
                        .nth(app.ui.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(app.ui.input.len());
                    app.ui.input.drain(start..end);
                    app.ui.cursor_pos -= 1;
                }
                // @skill: 占 7 个字符，起始位置 + 7 = 冒号之后
                let prefix_end = app.ui.skill_popup_start_pos + 7;
                if app.ui.cursor_pos < prefix_end {
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
                    let byte_idx = app
                        .ui
                        .input
                        .char_indices()
                        .nth(app.ui.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(app.ui.input.len());
                    app.ui.input.insert(byte_idx, c);
                    app.ui.cursor_pos += 1;
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
                if app.ui.cursor_pos > 0 {
                    let start = app
                        .ui
                        .input
                        .char_indices()
                        .nth(app.ui.cursor_pos - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end = app
                        .ui
                        .input
                        .char_indices()
                        .nth(app.ui.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(app.ui.input.len());
                    app.ui.input.drain(start..end);
                    app.ui.cursor_pos -= 1;
                }
                // @command: 占 9 个字符，起始位置 + 9 = 冒号之后
                let prefix_end = app.ui.command_popup_start_pos + 9;
                if app.ui.cursor_pos < prefix_end {
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
                    let byte_idx = app
                        .ui
                        .input
                        .char_indices()
                        .nth(app.ui.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(app.ui.input.len());
                    app.ui.input.insert(byte_idx, c);
                    app.ui.cursor_pos += 1;
                    update_command_filter(app);
                    return false;
                }
            }
            _ => {}
        }
    }

    // ===== Ctrl 快捷键 → Actions =====

    // Ctrl+T 切换模型
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('t') {
        if !app.state.agent_config.providers.is_empty() {
            app.ui
                .model_list_state
                .select(Some(app.state.agent_config.active_index));
            app.update(Action::EnterMode(ChatMode::SelectModel));
        }
        return false;
    }

    // Ctrl+L 归档对话
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('l') {
        if app.state.session.messages.is_empty() {
            app.update(Action::ShowToast(
                "当前对话为空，无法归档".to_string(),
                true,
            ));
        } else {
            app.update(Action::StartArchiveConfirm);
        }
        return false;
    }

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

    let char_count = app.ui.input.chars().count();

    match key.code {
        KeyCode::Esc => {
            if app.state.is_loading {
                if app.tool_executor.tools_executing_count > 0
                    && !app
                        .tool_executor
                        .tool_cancelled
                        .load(std::sync::atomic::Ordering::Relaxed)
                {
                    app.update(Action::CancelToolsOnly);
                } else {
                    app.update(Action::CancelStream);
                }
            } else {
                return true;
            }
        }

        KeyCode::Enter => {
            if app.state.is_loading {
                // agent loop 期间：将用户消息追加到待处理队列
                let text = app.ui.input.trim().to_string();
                if !text.is_empty() {
                    app.state
                        .session
                        .messages
                        .push(super::super::storage::ChatMessage::text("user", &text));
                    {
                        let mut pending = safe_lock(
                            &app.state.pending_user_messages,
                            "handler_chat::pending_user_messages",
                        );
                        pending.push(super::super::storage::ChatMessage::text("user", &text));
                    }
                    app.ui.input.clear();
                    app.ui.cursor_pos = 0;
                    app.ui.msg_lines_cache = None;
                    app.ui.auto_scroll = true;
                    app.ui.scroll_offset = u16::MAX;
                }
            } else {
                app.update(Action::SendMessage);
            }
        }

        // 滚动消息
        KeyCode::Up => app.update(Action::Scroll(CursorDirection::Up)),
        KeyCode::Down => app.update(Action::Scroll(CursorDirection::Down)),
        KeyCode::PageUp => app.update(Action::PageScroll(CursorDirection::Up)),
        KeyCode::PageDown => app.update(Action::PageScroll(CursorDirection::Down)),

        // 光标移动
        KeyCode::Left => {
            if app.ui.cursor_pos > 0 {
                app.ui.cursor_pos -= 1;
            }
        }
        KeyCode::Right => {
            if app.ui.cursor_pos < char_count {
                app.ui.cursor_pos += 1;
            }
        }
        KeyCode::Home => app.ui.cursor_pos = 0,
        KeyCode::End => app.ui.cursor_pos = char_count,

        // 删除
        KeyCode::Backspace => {
            if app.ui.cursor_pos > 0 {
                let start = app
                    .ui
                    .input
                    .char_indices()
                    .nth(app.ui.cursor_pos - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let end = app
                    .ui
                    .input
                    .char_indices()
                    .nth(app.ui.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(app.ui.input.len());
                app.ui.input.drain(start..end);
                app.ui.cursor_pos -= 1;
            }
        }
        KeyCode::Delete => {
            if app.ui.cursor_pos < char_count {
                let start = app
                    .ui
                    .input
                    .char_indices()
                    .nth(app.ui.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(app.ui.input.len());
                let end = app
                    .ui
                    .input
                    .char_indices()
                    .nth(app.ui.cursor_pos + 1)
                    .map(|(i, _)| i)
                    .unwrap_or(app.ui.input.len());
                app.ui.input.drain(start..end);
            }
        }

        // F1 帮助
        KeyCode::F(1) => {
            app.update(Action::ShowHelp);
        }
        // 输入框为空时，? 也可唤起帮助
        KeyCode::Char('?') if app.ui.input.is_empty() => {
            app.update(Action::ShowHelp);
        }
        KeyCode::Char(c) => {
            let byte_idx = app
                .ui
                .input
                .char_indices()
                .nth(app.ui.cursor_pos)
                .map(|(i, _)| i)
                .unwrap_or(app.ui.input.len());
            app.ui.input.insert(byte_idx, c);
            app.ui.cursor_pos += 1;

            // @ 补全弹窗触发逻辑
            if c == '@' {
                let valid = app.ui.cursor_pos <= 1 || {
                    let chars: Vec<char> = app.ui.input.chars().collect();
                    app.ui.cursor_pos >= 2 && chars[app.ui.cursor_pos - 2].is_whitespace()
                };
                if valid {
                    app.ui.at_popup_active = true;
                    app.ui.at_popup_start_pos = app.ui.cursor_pos - 1;
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
                }
            }
        }

        _ => {}
    }

    false
}
