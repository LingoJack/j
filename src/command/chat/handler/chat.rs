use super::super::autocomplete::{
    complete_at_mention, complete_file_mention, filter_dir_part, get_filtered_files,
    get_filtered_skills, update_at_filter, update_file_filter,
};
use super::super::model::save_agent_config;
use super::super::render::copy_to_clipboard;
use crate::command::chat::app::{ChatApp, ChatMode};
use crate::constants::{AGENT_DIR, AGENT_LOG_DIR, AGENT_LOG_ERROR, AGENT_LOG_INFO};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_chat_mode(app: &mut ChatApp, key: KeyEvent) -> bool {
    // Ctrl+C 强制退出
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }

    // ===== @ 补全弹窗拦截 =====
    if app.at_popup_active {
        let filtered = get_filtered_skills(app);
        match key.code {
            KeyCode::Up => {
                if !filtered.is_empty() && app.at_popup_selected > 0 {
                    app.at_popup_selected -= 1;
                }
                return false;
            }
            KeyCode::Down => {
                if !filtered.is_empty() && app.at_popup_selected < filtered.len().saturating_sub(1)
                {
                    app.at_popup_selected += 1;
                }
                return false;
            }
            KeyCode::Tab | KeyCode::Enter => {
                if !filtered.is_empty() {
                    let sel = app.at_popup_selected.min(filtered.len() - 1);
                    let name = filtered[sel].clone();
                    if name == "file:" {
                        // 选中 file: 选项，补全 @file: 到输入框，然后切换到文件补全模式
                        let chars: Vec<char> = app.input.chars().collect();
                        let before: String = chars[..app.at_popup_start_pos].iter().collect();
                        let after: String = if app.cursor_pos < chars.len() {
                            chars[app.cursor_pos..].iter().collect()
                        } else {
                            String::new()
                        };
                        let replacement = "@file:";
                        let new_cursor = before.chars().count() + replacement.chars().count();
                        app.input = format!("{}{}{}", before, replacement, after);
                        app.cursor_pos = new_cursor;
                        app.at_popup_active = false;
                        app.file_popup_active = true;
                        app.file_popup_start_pos = app.at_popup_start_pos;
                        app.file_popup_filter.clear();
                        app.file_popup_selected = 0;
                    } else {
                        complete_at_mention(app, &name);
                        app.at_popup_active = false;
                    }
                } else {
                    app.at_popup_active = false;
                }
                return false;
            }
            KeyCode::Esc => {
                app.at_popup_active = false;
                return false;
            }
            KeyCode::Char(' ') => {
                // 空格关闭弹窗，正常处理字符
                app.at_popup_active = false;
                // fall through to normal char handling below
            }
            KeyCode::Backspace => {
                // 先执行删除，然后检查弹窗状态
                if app.cursor_pos > 0 {
                    let start = app
                        .input
                        .char_indices()
                        .nth(app.cursor_pos - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end = app
                        .input
                        .char_indices()
                        .nth(app.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(app.input.len());
                    app.input.drain(start..end);
                    app.cursor_pos -= 1;
                }
                // 如果光标退回到 @ 之前，关闭弹窗
                if app.cursor_pos <= app.at_popup_start_pos {
                    app.at_popup_active = false;
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
    if app.file_popup_active {
        let filtered = get_filtered_files(app);
        match key.code {
            KeyCode::Up => {
                if !filtered.is_empty() && app.file_popup_selected > 0 {
                    app.file_popup_selected -= 1;
                }
                return false;
            }
            KeyCode::Down => {
                if !filtered.is_empty()
                    && app.file_popup_selected < filtered.len().saturating_sub(1)
                {
                    app.file_popup_selected += 1;
                }
                return false;
            }
            KeyCode::Tab | KeyCode::Enter => {
                if !filtered.is_empty() {
                    let sel = app.file_popup_selected.min(filtered.len() - 1);
                    let entry = filtered[sel].clone();
                    if entry.ends_with('/') {
                        // 目录：用 dir_part + entry 更新 filter，继续补全
                        let dir = filter_dir_part(&app.file_popup_filter);
                        app.file_popup_filter = format!("{}{}", dir, entry);
                        // 更新 input 中的文本
                        let chars: Vec<char> = app.input.chars().collect();
                        let before: String = chars[..app.file_popup_start_pos].iter().collect();
                        let after: String = if app.cursor_pos < chars.len() {
                            chars[app.cursor_pos..].iter().collect()
                        } else {
                            String::new()
                        };
                        let replacement = format!("@file:{}", app.file_popup_filter);
                        let new_cursor = before.chars().count() + replacement.chars().count();
                        app.input = format!("{}{}{}", before, replacement, after);
                        app.cursor_pos = new_cursor;
                        app.file_popup_selected = 0;
                    } else {
                        // 文件：用 dir_part + entry 拼接完整路径，补全并关闭弹窗
                        let dir = filter_dir_part(&app.file_popup_filter);
                        let full_path = format!("{}{}", dir, entry);
                        complete_file_mention(app, &full_path);
                        app.file_popup_active = false;
                    }
                    return false;
                }
                // filtered 为空时，关闭弹窗，让 Enter 继续处理（发送消息）
                app.file_popup_active = false;
                // fall through to normal Enter handling
            }
            KeyCode::Esc => {
                app.file_popup_active = false;
                return false;
            }
            KeyCode::Backspace => {
                if app.cursor_pos > 0 {
                    let start = app
                        .input
                        .char_indices()
                        .nth(app.cursor_pos - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end = app
                        .input
                        .char_indices()
                        .nth(app.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(app.input.len());
                    app.input.drain(start..end);
                    app.cursor_pos -= 1;
                }
                // @file: 占 6 个字符，起始位置 + 6 = 冒号之后
                let prefix_end = app.file_popup_start_pos + 6;
                if app.cursor_pos < prefix_end {
                    app.file_popup_active = false;
                } else {
                    update_file_filter(app);
                }
                return false;
            }
            KeyCode::Char(c) => {
                // 空格关闭文件弹窗，让后续输入正常处理
                if c == ' ' {
                    app.file_popup_active = false;
                    // fall through to normal char handling
                } else {
                    let byte_idx = app
                        .input
                        .char_indices()
                        .nth(app.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(app.input.len());
                    app.input.insert(byte_idx, c);
                    app.cursor_pos += 1;
                    update_file_filter(app);
                    return false;
                }
            }
            _ => {}
        }
    }

    // Ctrl+T 切换模型（替代 Ctrl+M，因为 Ctrl+M 在终端中等于 Enter）
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('t') {
        if !app.agent_config.providers.is_empty() {
            app.mode = ChatMode::SelectModel;
            app.model_list_state
                .select(Some(app.agent_config.active_index));
        }
        return false;
    }

    // Ctrl+L 归档对话
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('l') {
        if app.session.messages.is_empty() {
            app.show_toast("当前对话为空，无法归档", true);
        } else {
            app.start_archive_confirm();
        }
        return false;
    }

    // Ctrl+R 还原归档
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r') {
        app.start_archive_list();
        return false;
    }

    // Ctrl+Y 复制最后一条 AI 回复
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('y') {
        if let Some(last_ai) = app
            .session
            .messages
            .iter()
            .rev()
            .find(|m| m.role == "assistant")
        {
            if copy_to_clipboard(&last_ai.content) {
                app.show_toast("已复制最后一条 AI 回复", false);
            } else {
                app.show_toast("复制到剪切板失败", true);
            }
        } else {
            app.show_toast("暂无 AI 回复可复制", true);
        }
        return false;
    }

    // Ctrl+B 进入消息浏览模式（可选中历史消息并复制）
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
        if !app.session.messages.is_empty() {
            // 默认选中最后一条消息
            app.browse_msg_index = app.session.messages.len() - 1;
            app.browse_scroll_offset = 0; // 重置消息内偏移
            app.mode = ChatMode::Browse;
            app.msg_lines_cache = None; // 清除缓存以触发高亮重绘
        } else {
            app.show_toast("暂无消息可浏览", true);
        }
        return false;
    }

    // Ctrl+G 在新终端窗口中 tail -f 实时查看日志（info.log + error.log 各一个窗口）
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('g') {
        let log_dir = crate::config::YamlConfig::data_dir()
            .join(AGENT_DIR)
            .join(AGENT_LOG_DIR);
        let info_log = log_dir.join(AGENT_LOG_INFO);
        let error_log = log_dir.join(AGENT_LOG_ERROR);
        let info_cmd = format!("tail -f '{}'; exit", info_log.to_string_lossy())
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        let error_cmd = format!("tail -f '{}'; exit", error_log.to_string_lossy())
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        let apple_script = format!(
            "tell application \"Terminal\"\n\
                do script \"{}\"\n\
                do script \"{}\"\n\
                activate\n\
            end tell",
            info_cmd, error_cmd
        );
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(&apple_script)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        return false;
    }

    // Ctrl+E 打开配置界面
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e') {
        // 初始化配置界面状态
        app.config_provider_idx = app
            .agent_config
            .active_index
            .min(app.agent_config.providers.len().saturating_sub(1));
        app.config_field_idx = 0;
        app.config_editing = false;
        app.config_edit_buf.clear();
        app.mode = ChatMode::Config;
        return false;
    }

    // Ctrl+S 切换流式/非流式输出
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        app.agent_config.stream_mode = !app.agent_config.stream_mode;
        let _ = save_agent_config(&app.agent_config);
        let mode_str = if app.agent_config.stream_mode {
            "流式输出"
        } else {
            "整体输出"
        };
        app.show_toast(format!("已切换为: {}", mode_str), false);
        return false;
    }

    let char_count = app.input.chars().count();

    match key.code {
        KeyCode::Esc => {
            if app.is_loading {
                if app.tools_executing_count > 0
                    && !app
                        .tool_cancelled
                        .load(std::sync::atomic::Ordering::Relaxed)
                {
                    // 首次按 Esc：只取消工具，不终止 agent loop
                    app.cancel_tools_only();
                } else {
                    // 二次按 Esc（工具已在取消中）或无工具执行：取消整个请求
                    app.cancel_stream();
                }
            } else {
                // 非加载中：原有行为（退出 TUI）
                return true;
            }
        }

        KeyCode::Enter => {
            if app.is_loading {
                // agent loop 期间：将用户消息追加到待处理队列（不启动新 loop）
                let text = app.input.trim().to_string();
                if !text.is_empty() {
                    app.session
                        .messages
                        .push(super::super::model::ChatMessage::text("user", &text));
                    {
                        let mut pending = app.pending_user_messages.lock().unwrap();
                        pending.push(super::super::model::ChatMessage::text("user", &text));
                    }
                    app.input.clear();
                    app.cursor_pos = 0;
                    app.msg_lines_cache = None;
                    app.auto_scroll = true;
                    app.scroll_offset = u16::MAX;
                }
            } else {
                app.send_message();
            }
        }

        // 滚动消息
        KeyCode::Up => app.scroll_up(),
        KeyCode::Down => app.scroll_down(),
        KeyCode::PageUp => {
            for _ in 0..10 {
                app.scroll_up();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                app.scroll_down();
            }
        }

        // 光标移动
        KeyCode::Left => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
            }
        }
        KeyCode::Right => {
            if app.cursor_pos < char_count {
                app.cursor_pos += 1;
            }
        }
        KeyCode::Home => app.cursor_pos = 0,
        KeyCode::End => app.cursor_pos = char_count,

        // 删除
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                let start = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let end = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
                app.input.drain(start..end);
                app.cursor_pos -= 1;
            }
        }
        KeyCode::Delete => {
            if app.cursor_pos < char_count {
                let start = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
                let end = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos + 1)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
                app.input.drain(start..end);
            }
        }

        // F1 任何时候都能唤起帮助
        KeyCode::F(1) => {
            app.mode = ChatMode::Help;
        }
        // 输入框为空时，? 也可唤起帮助
        KeyCode::Char('?') if app.input.is_empty() => {
            app.mode = ChatMode::Help;
        }
        KeyCode::Char(c) => {
            let byte_idx = app
                .input
                .char_indices()
                .nth(app.cursor_pos)
                .map(|(i, _)| i)
                .unwrap_or(app.input.len());
            app.input.insert(byte_idx, c);
            app.cursor_pos += 1;

            // @ 补全弹窗触发逻辑
            if c == '@' {
                // @ 在行首或前一个字符是空白
                let valid = app.cursor_pos <= 1 || {
                    let chars: Vec<char> = app.input.chars().collect();
                    app.cursor_pos >= 2 && chars[app.cursor_pos - 2].is_whitespace()
                };
                if valid {
                    app.at_popup_active = true;
                    app.at_popup_start_pos = app.cursor_pos - 1;
                    app.at_popup_filter.clear();
                    app.at_popup_selected = 0;
                }
            } else if app.at_popup_active {
                update_at_filter(app);
                // 检测是否输入了 @file: ，切换到文件补全模式
                if app.at_popup_filter == "file:" {
                    app.at_popup_active = false;
                    app.file_popup_active = true;
                    app.file_popup_start_pos = app.at_popup_start_pos;
                    app.file_popup_filter.clear();
                    app.file_popup_selected = 0;
                }
            }
        }

        _ => {}
    }

    false
}
