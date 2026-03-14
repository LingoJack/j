use super::model::{
    ModelProvider, load_style, load_system_prompt, save_agent_config, save_chat_session,
    save_style, save_system_prompt,
};
use super::render::copy_to_clipboard;
use super::ui::draw_chat_ui;
use crate::command::chat::app::{AskAnswer, ChatApp, ChatMode, config_total_fields};
use crate::constants::{
    AGENT_DIR, AGENT_LOG_DIR, AGENT_LOG_ERROR, AGENT_LOG_INFO, CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS,
};
use crate::error;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

pub fn run_chat_tui() {
    match run_chat_tui_internal() {
        Ok(_) => {}
        Err(e) => {
            error!("❌ Chat TUI 启动失败: {}", e);
        }
    }
}

pub fn run_chat_tui_internal() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = ChatApp::new();

    // 首次运行（尚未配置 provider）时，自动进入配置界面引导用户完成配置
    if app.agent_config.providers.is_empty() {
        use super::model::{AgentConfig, ModelProvider, agent_config_path, save_agent_config};
        use super::theme::ThemeName;
        // 自动创建示例配置文件（如果不存在）
        if !agent_config_path().exists() {
            let example = AgentConfig {
                providers: vec![ModelProvider {
                    name: "OpenAI".to_string(),
                    api_base: "https://api.openai.com/v1".to_string(),
                    api_key: "sk-your-api-key".to_string(),
                    model: "gpt-4o".to_string(),
                }],
                active_index: 0,
                system_prompt: None,
                stream_mode: true,
                max_history_messages: 20,
                theme: ThemeName::default(),
                tools_enabled: false,
                max_tool_rounds: 10,
                style: None,
                tool_confirm_timeout: 0,
                disabled_tools: Vec::new(),
                disabled_skills: Vec::new(),
            };
            let _ = save_agent_config(&example);
            app.agent_config = example;
        }
        // 直接进入配置界面
        app.mode = ChatMode::Config;
        app.show_toast("尚未配置模型，请先完成配置 (Esc 保存退出)", true);
    }

    let mut needs_redraw = true; // 首次必须绘制

    loop {
        // 清理过期 toast（如果有 toast 被清理，需要重绘）
        let had_toast = app.toast.is_some();
        app.tick_toast();
        if had_toast && app.toast.is_none() {
            needs_redraw = true;
        }

        // 非阻塞地处理后台流式消息
        let was_loading = app.is_loading;
        app.poll_stream();
        // 有待执行的工具时强制重绘，让工具执行状态先渲染出来
        if app.pending_tool_execution {
            needs_redraw = true;
        }
        // 流式加载中使用节流策略：只在内容增长超过阈值或超时才重绘
        if app.is_loading {
            let current_len = app.streaming_content.lock().unwrap().len();
            let bytes_delta = current_len.saturating_sub(app.last_rendered_streaming_len);
            let time_elapsed = app.last_stream_render_time.elapsed();
            // 每增加 200 字节或距离上次渲染超过 200ms 才重绘
            if bytes_delta >= 200
                || time_elapsed >= std::time::Duration::from_millis(200)
                || current_len == 0
            {
                needs_redraw = true;
            }
        } else if was_loading {
            // 加载刚结束时必须重绘一次
            needs_redraw = true;
        }
        // ToolConfirm 模式下强制重绘，确保确认弹窗可见
        if app.mode == ChatMode::ToolConfirm {
            needs_redraw = true;
        }

        // 只在状态发生变化时才重绘，大幅降低 CPU 占用
        if needs_redraw {
            terminal.draw(|f| draw_chat_ui(f, &mut app))?;
            needs_redraw = false;
            // 更新流式节流状态
            if app.is_loading {
                app.last_rendered_streaming_len = app.streaming_content.lock().unwrap().len();
                app.last_stream_render_time = std::time::Instant::now();
            }
        }

        // ToolConfirm 超时自动执行检查
        if app.mode == ChatMode::ToolConfirm && app.agent_config.tool_confirm_timeout > 0 {
            let elapsed = app.tool_confirm_entered_at.elapsed();
            let timeout = std::time::Duration::from_secs(app.agent_config.tool_confirm_timeout);
            if elapsed >= timeout {
                app.execute_pending_tool();
                needs_redraw = true;
            } else {
                // 倒计时变化需要重绘
                needs_redraw = true;
            }
        }

        // 等待事件：加载中用短间隔以刷新流式内容，空闲时用长间隔节省 CPU
        let poll_timeout = if app.is_loading {
            std::time::Duration::from_millis(150)
        } else if app.mode == ChatMode::ToolConfirm {
            std::time::Duration::from_millis(500) // 确认模式需要更频繁刷新
        } else {
            std::time::Duration::from_millis(1000)
        };

        if event::poll(poll_timeout)? {
            // 批量消费所有待处理事件，避免快速滚动/打字时事件堆积
            let mut should_break = false;
            loop {
                let evt = event::read()?;
                match evt {
                    Event::Key(key) => {
                        needs_redraw = true;
                        match app.mode {
                            ChatMode::Chat => {
                                if handle_chat_mode(&mut app, key) {
                                    should_break = true;
                                    break;
                                }
                            }
                            ChatMode::SelectModel => handle_select_model(&mut app, key),
                            ChatMode::Browse => handle_browse_mode(&mut app, key),
                            ChatMode::Help => {
                                app.mode = ChatMode::Chat;
                            }
                            ChatMode::Config => handle_config_mode(&mut app, key),
                            ChatMode::ArchiveConfirm => handle_archive_confirm_mode(&mut app, key),
                            ChatMode::ArchiveList => handle_archive_list_mode(&mut app, key),
                            ChatMode::ToolConfirm => handle_tool_confirm_mode(&mut app, key),
                            ChatMode::ToolToggle => handle_tool_toggle_mode(&mut app, key),
                            ChatMode::SkillToggle => handle_skill_toggle_mode(&mut app, key),
                        }
                    }
                    Event::Resize(_, _) => {
                        needs_redraw = true;
                    }
                    Event::Mouse(mouse) => match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            app.scroll_up();
                            needs_redraw = true;
                        }
                        MouseEventKind::ScrollDown => {
                            app.scroll_down();
                            needs_redraw = true;
                        }
                        _ => {}
                    },
                    _ => {}
                }
                // 继续消费剩余事件（非阻塞，Duration::ZERO）
                if !event::poll(std::time::Duration::ZERO)? {
                    break;
                }
            }
            if should_break {
                break;
            }

            // 检查 system_prompt 全屏编辑器标志
            if app.pending_system_prompt_edit {
                app.pending_system_prompt_edit = false;
                let current_prompt = load_system_prompt().unwrap_or_default();
                match crate::tui::editor::open_editor_on_terminal(
                    &mut terminal,
                    "编辑系统提示词 (System Prompt)",
                    &current_prompt,
                ) {
                    Ok(Some(new_text)) => {
                        if save_system_prompt(&new_text) {
                            app.show_toast("系统提示词已更新", false);
                        } else {
                            app.show_toast("系统提示词保存失败", true);
                        }
                    }
                    Ok(None) => {
                        // 用户取消编辑
                    }
                    Err(e) => {
                        app.show_toast(format!("编辑器错误: {}", e), true);
                    }
                }
                needs_redraw = true;
            }

            // 检查 style 全屏编辑器标志
            if app.pending_style_edit {
                app.pending_style_edit = false;
                let current_style = load_style().unwrap_or_default();
                match crate::tui::editor::open_editor_on_terminal(
                    &mut terminal,
                    "编辑回复风格 (Style)",
                    &current_style,
                ) {
                    Ok(Some(new_text)) => {
                        if save_style(&new_text) {
                            app.show_toast("回复风格已更新", false);
                        } else {
                            app.show_toast("回复风格保存失败", true);
                        }
                    }
                    Ok(None) => {
                        // 用户取消编辑
                    }
                    Err(e) => {
                        app.show_toast(format!("编辑器错误: {}", e), true);
                    }
                }
                needs_redraw = true;
            }
        }
    }

    // 保存对话历史
    let _ = save_chat_session(&app.session);

    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        crossterm::event::DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    Ok(())
}

/// 绘制 TUI 界面
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
                        .push(super::model::ChatMessage::text("user", &text));
                    {
                        let mut pending = app.pending_user_messages.lock().unwrap();
                        pending.push(super::model::ChatMessage::text("user", &text));
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

/// 获取配置界面中当前字段的标签
// config_field_* 函数已移至 super::config 模块
pub use super::config::{
    config_field_label, config_field_raw_value, config_field_set, config_field_value,
};

/// 配置模式按键处理
pub fn handle_config_mode(app: &mut ChatApp, key: KeyEvent) {
    let total_fields = config_total_fields();

    if app.config_editing {
        // 正在编辑某个字段
        match key.code {
            KeyCode::Esc => {
                // 取消编辑
                app.config_editing = false;
            }
            KeyCode::Enter => {
                // 确认编辑
                let val = app.config_edit_buf.clone();
                config_field_set(app, app.config_field_idx, &val);
                app.config_editing = false;
            }
            KeyCode::Backspace => {
                if app.config_edit_cursor > 0 {
                    let idx = app
                        .config_edit_buf
                        .char_indices()
                        .nth(app.config_edit_cursor - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end_idx = app
                        .config_edit_buf
                        .char_indices()
                        .nth(app.config_edit_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(app.config_edit_buf.len());
                    app.config_edit_buf = format!(
                        "{}{}",
                        &app.config_edit_buf[..idx],
                        &app.config_edit_buf[end_idx..]
                    );
                    app.config_edit_cursor -= 1;
                }
            }
            KeyCode::Left => {
                app.config_edit_cursor = app.config_edit_cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                let char_count = app.config_edit_buf.chars().count();
                if app.config_edit_cursor < char_count {
                    app.config_edit_cursor += 1;
                }
            }
            KeyCode::Char(c) => {
                let byte_idx = app
                    .config_edit_buf
                    .char_indices()
                    .nth(app.config_edit_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.config_edit_buf.len());
                app.config_edit_buf.insert(byte_idx, c);
                app.config_edit_cursor += 1;
            }
            _ => {}
        }
        return;
    }

    // 非编辑状态
    match key.code {
        KeyCode::Esc => {
            // 保存并返回（system_prompt 和 style 已在 config_field_set 中即时写入文件）
            let config_saved = save_agent_config(&app.agent_config);
            if config_saved {
                app.show_toast("配置已保存 ✅", false);
            } else {
                app.show_toast("配置保存失败", true);
            }
            app.mode = ChatMode::Chat;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if total_fields > 0 {
                if app.config_field_idx == 0 {
                    app.config_field_idx = total_fields - 1;
                } else {
                    app.config_field_idx -= 1;
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if total_fields > 0 {
                app.config_field_idx = (app.config_field_idx + 1) % total_fields;
            }
        }
        KeyCode::Tab | KeyCode::Right => {
            // 切换 provider
            let count = app.agent_config.providers.len();
            if count > 1 {
                app.config_provider_idx = (app.config_provider_idx + 1) % count;
                // 切换后如果在 provider 字段区域，保持字段位置不变
            }
        }
        KeyCode::BackTab | KeyCode::Left => {
            // 反向切换 provider
            let count = app.agent_config.providers.len();
            if count > 1 {
                if app.config_provider_idx == 0 {
                    app.config_provider_idx = count - 1;
                } else {
                    app.config_provider_idx -= 1;
                }
            }
        }
        KeyCode::Enter => {
            // 进入编辑模式
            let total_provider = CONFIG_FIELDS.len();
            if app.config_field_idx < total_provider && app.agent_config.providers.is_empty() {
                app.show_toast("还没有 Provider，按 a 新增", true);
                return;
            }
            // stream_mode 字段直接切换，不进入编辑模式
            let gi = app.config_field_idx.checked_sub(total_provider);
            if let Some(gi) = gi {
                if CONFIG_GLOBAL_FIELDS[gi] == "stream_mode" {
                    app.agent_config.stream_mode = !app.agent_config.stream_mode;
                    return;
                }
                // tools_enabled 字段：Enter 进入工具开关子菜单
                if CONFIG_GLOBAL_FIELDS[gi] == "tools_enabled" {
                    app.tool_toggle_index = 0;
                    app.mode = ChatMode::ToolToggle;
                    return;
                }
                // skills_enabled 字段：Enter 进入 Skill 开关子菜单
                if CONFIG_GLOBAL_FIELDS[gi] == "skills_enabled" {
                    app.skill_toggle_index = 0;
                    app.mode = ChatMode::SkillToggle;
                    return;
                }
                // theme 字段直接循环切换，不进入编辑模式
                if CONFIG_GLOBAL_FIELDS[gi] == "theme" {
                    app.switch_theme();
                    return;
                }
                // system_prompt 字段使用全屏编辑器
                if CONFIG_GLOBAL_FIELDS[gi] == "system_prompt" {
                    app.pending_system_prompt_edit = true;
                    return;
                }
                // style 字段使用全屏编辑器
                if CONFIG_GLOBAL_FIELDS[gi] == "style" {
                    app.pending_style_edit = true;
                    return;
                }
            }
            app.config_edit_buf = config_field_raw_value(app, app.config_field_idx);
            app.config_edit_cursor = app.config_edit_buf.chars().count();
            app.config_editing = true;
        }
        KeyCode::Char('a') => {
            // 新增 Provider
            let new_provider = ModelProvider {
                name: format!("Provider-{}", app.agent_config.providers.len() + 1),
                api_base: "https://api.openai.com/v1".to_string(),
                api_key: String::new(),
                model: String::new(),
            };
            app.agent_config.providers.push(new_provider);
            app.config_provider_idx = app.agent_config.providers.len() - 1;
            app.config_field_idx = 0; // 跳到 name 字段
            app.show_toast("已新增 Provider，请填写配置", false);
        }
        KeyCode::Char('d') => {
            // 删除当前 Provider
            let count = app.agent_config.providers.len();
            if count == 0 {
                app.show_toast("没有可删除的 Provider", true);
            } else {
                let removed_name = app.agent_config.providers[app.config_provider_idx]
                    .name
                    .clone();
                app.agent_config.providers.remove(app.config_provider_idx);
                // 调整索引
                if app.config_provider_idx >= app.agent_config.providers.len()
                    && app.config_provider_idx > 0
                {
                    app.config_provider_idx -= 1;
                }
                // 调整 active_index
                if app.agent_config.active_index >= app.agent_config.providers.len()
                    && app.agent_config.active_index > 0
                {
                    app.agent_config.active_index -= 1;
                }
                app.show_toast(format!("已删除 Provider: {}", removed_name), false);
            }
        }
        KeyCode::Char('s') => {
            // 将当前 provider 设为活跃
            if !app.agent_config.providers.is_empty() {
                app.agent_config.active_index = app.config_provider_idx;
                let name = app.agent_config.providers[app.config_provider_idx]
                    .name
                    .clone();
                app.show_toast(format!("已设为活跃模型: {}", name), false);
            }
        }
        _ => {}
    }
}

/// 工具开关子菜单按键处理
pub fn handle_tool_toggle_mode(app: &mut ChatApp, key: KeyEvent) {
    let tool_names = app.tool_registry.tool_names();
    let total = tool_names.len();
    if total == 0 {
        app.mode = ChatMode::Config;
        return;
    }

    match key.code {
        KeyCode::Esc => {
            // 返回配置模式并保存
            save_agent_config(&app.agent_config);
            app.mode = ChatMode::Config;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.tool_toggle_index == 0 {
                app.tool_toggle_index = total - 1;
            } else {
                app.tool_toggle_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.tool_toggle_index = (app.tool_toggle_index + 1) % total;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            // 切换当前工具的启用/禁用状态
            let name = tool_names[app.tool_toggle_index].to_string();
            if let Some(pos) = app
                .agent_config
                .disabled_tools
                .iter()
                .position(|d| d == &name)
            {
                app.agent_config.disabled_tools.remove(pos);
            } else {
                app.agent_config.disabled_tools.push(name);
            }
        }
        KeyCode::Char('a') => {
            // 全部启用
            app.agent_config.disabled_tools.clear();
            app.show_toast("已启用全部工具", false);
        }
        KeyCode::Char('d') => {
            // 全部禁用
            app.agent_config.disabled_tools = tool_names.iter().map(|n| n.to_string()).collect();
            app.show_toast("已禁用全部工具", false);
        }
        KeyCode::Char('t') => {
            // 切换总开关
            app.agent_config.tools_enabled = !app.agent_config.tools_enabled;
            let status = if app.agent_config.tools_enabled {
                "开启"
            } else {
                "关闭"
            };
            app.show_toast(format!("工具调用已{}", status), false);
        }
        _ => {}
    }
}

pub fn handle_skill_toggle_mode(app: &mut ChatApp, key: KeyEvent) {
    let total = app.loaded_skills.len();
    if total == 0 {
        app.mode = ChatMode::Config;
        return;
    }

    match key.code {
        KeyCode::Esc => {
            save_agent_config(&app.agent_config);
            app.mode = ChatMode::Config;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.skill_toggle_index == 0 {
                app.skill_toggle_index = total - 1;
            } else {
                app.skill_toggle_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.skill_toggle_index = (app.skill_toggle_index + 1) % total;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let name = app.loaded_skills[app.skill_toggle_index]
                .frontmatter
                .name
                .clone();
            if let Some(pos) = app
                .agent_config
                .disabled_skills
                .iter()
                .position(|d| d == &name)
            {
                app.agent_config.disabled_skills.remove(pos);
            } else {
                app.agent_config.disabled_skills.push(name);
            }
        }
        KeyCode::Char('a') => {
            app.agent_config.disabled_skills.clear();
            app.show_toast("已启用全部 Skills", false);
        }
        KeyCode::Char('d') => {
            app.agent_config.disabled_skills = app
                .loaded_skills
                .iter()
                .map(|s| s.frontmatter.name.clone())
                .collect();
            app.show_toast("已禁用全部 Skills", false);
        }
        _ => {}
    }
}

/// 绘制配置编辑界面
pub fn handle_select_model(app: &mut ChatApp, key: KeyEvent) {
    let count = app.agent_config.providers.len();
    match key.code {
        KeyCode::Esc => {
            app.mode = ChatMode::Chat;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if count > 0 {
                let i = app
                    .model_list_state
                    .selected()
                    .map(|i| if i == 0 { count - 1 } else { i - 1 })
                    .unwrap_or(0);
                app.model_list_state.select(Some(i));
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if count > 0 {
                let i = app
                    .model_list_state
                    .selected()
                    .map(|i| if i >= count - 1 { 0 } else { i + 1 })
                    .unwrap_or(0);
                app.model_list_state.select(Some(i));
            }
        }
        KeyCode::Enter => {
            app.switch_model();
        }
        _ => {}
    }
}

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
                if let Err(e) = super::archive::validate_archive_name(&name) {
                    app.show_toast(e, true);
                    return;
                }
                // 检查是否重名
                if super::archive::archive_exists(&name) {
                    // 直接覆盖
                    let _ = super::archive::delete_archive(&name);
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
                if super::archive::archive_exists(&name) {
                    let _ = super::archive::delete_archive(&name);
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

/// 统一交互区域按键处理：选项式（↑↓ 选择，Enter 确认，Esc 拒绝/退出）
pub fn handle_tool_confirm_mode(app: &mut ChatApp, key: KeyEvent) {
    let is_ask = app.tool_ask_mode;

    // ask 模式使用新的结构化问答处理
    if is_ask {
        handle_ask_mode(app, key);
        app.msg_lines_cache = None;
        return;
    }

    if app.tool_interact_typing {
        // 输入模式（工具确认）
        match key.code {
            KeyCode::Esc => {
                app.tool_interact_typing = false;
            }
            KeyCode::Enter => {
                let input_text = app.tool_interact_input.trim().to_string();
                app.reject_pending_tool(&input_text);
                app.tool_interact_input.clear();
                app.tool_interact_cursor = 0;
                app.tool_interact_typing = false;
            }
            KeyCode::Backspace => {
                if app.tool_interact_cursor > 0 {
                    let start = app
                        .tool_interact_input
                        .char_indices()
                        .nth(app.tool_interact_cursor - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end = app
                        .tool_interact_input
                        .char_indices()
                        .nth(app.tool_interact_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(app.tool_interact_input.len());
                    app.tool_interact_input.drain(start..end);
                    app.tool_interact_cursor -= 1;
                }
            }
            KeyCode::Left => {
                if app.tool_interact_cursor > 0 {
                    app.tool_interact_cursor -= 1;
                }
            }
            KeyCode::Right => {
                let char_count = app.tool_interact_input.chars().count();
                if app.tool_interact_cursor < char_count {
                    app.tool_interact_cursor += 1;
                }
            }
            KeyCode::Char(c) => {
                let byte_idx = app
                    .tool_interact_input
                    .char_indices()
                    .nth(app.tool_interact_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.tool_interact_input.len());
                app.tool_interact_input.insert(byte_idx, c);
                app.tool_interact_cursor += 1;
            }
            _ => {}
        }
        app.msg_lines_cache = None;
        return;
    }

    // 工具确认选项模式
    match key.code {
        KeyCode::Up => {
            if app.tool_interact_selected > 0 {
                app.tool_interact_selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.tool_interact_selected < 2 {
                app.tool_interact_selected += 1;
            }
        }
        KeyCode::Enter => match app.tool_interact_selected {
            0 => app.execute_pending_tool(),
            1 => app.reject_pending_tool(""),
            2 => {
                app.tool_interact_typing = true;
                app.tool_interact_input.clear();
                app.tool_interact_cursor = 0;
            }
            _ => {}
        },
        KeyCode::Esc => {
            app.reject_pending_tool("");
        }
        _ => {}
    }
    app.msg_lines_cache = None;
}

/// Ask 模式的结构化问答交互处理
fn handle_ask_mode(app: &mut ChatApp, key: KeyEvent) {
    let total_questions = app.tool_ask_questions.len();
    if total_questions == 0 {
        return;
    }

    // 自由输入模式
    if app.tool_interact_typing {
        match key.code {
            KeyCode::Esc => {
                // 退出输入模式，回到选项
                app.tool_interact_typing = false;
            }
            KeyCode::Enter => {
                // 提交自由输入作为当前题答案
                let input_text = app.tool_interact_input.trim().to_string();
                let answer = if input_text.is_empty() {
                    AskAnswer::FreeText("（空）".to_string())
                } else {
                    AskAnswer::FreeText(input_text)
                };
                ask_submit_answer(app, answer);
                app.tool_interact_input.clear();
                app.tool_interact_cursor = 0;
                app.tool_interact_typing = false;
            }
            KeyCode::Backspace => {
                if app.tool_interact_cursor > 0 {
                    let start = app
                        .tool_interact_input
                        .char_indices()
                        .nth(app.tool_interact_cursor - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end = app
                        .tool_interact_input
                        .char_indices()
                        .nth(app.tool_interact_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(app.tool_interact_input.len());
                    app.tool_interact_input.drain(start..end);
                    app.tool_interact_cursor -= 1;
                }
            }
            KeyCode::Left => {
                if app.tool_interact_cursor > 0 {
                    app.tool_interact_cursor -= 1;
                }
            }
            KeyCode::Right => {
                let char_count = app.tool_interact_input.chars().count();
                if app.tool_interact_cursor < char_count {
                    app.tool_interact_cursor += 1;
                }
            }
            KeyCode::Char(c) => {
                let byte_idx = app
                    .tool_interact_input
                    .char_indices()
                    .nth(app.tool_interact_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.tool_interact_input.len());
                app.tool_interact_input.insert(byte_idx, c);
                app.tool_interact_cursor += 1;
            }
            _ => {}
        }
        return;
    }

    let cur_q = &app.tool_ask_questions[app.tool_ask_current_idx];
    let option_count = cur_q.options.len() + 1; // +1 for free input
    let is_multi = cur_q.multi_select;

    match key.code {
        KeyCode::Up => {
            if app.tool_ask_cursor > 0 {
                app.tool_ask_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.tool_ask_cursor < option_count - 1 {
                app.tool_ask_cursor += 1;
            }
        }
        KeyCode::Char(' ') if is_multi => {
            // 多选 toggle（不对"自由输入"选项 toggle）
            if app.tool_ask_cursor < cur_q.options.len() {
                let idx = app.tool_ask_cursor;
                if idx < app.tool_ask_selections.len() {
                    app.tool_ask_selections[idx] = !app.tool_ask_selections[idx];
                }
            }
        }
        KeyCode::Enter => {
            let cursor = app.tool_ask_cursor;
            if cursor == cur_q.options.len() {
                // "自由输入"选项：进入输入模式
                app.tool_interact_typing = true;
                app.tool_interact_input.clear();
                app.tool_interact_cursor = 0;
            } else if is_multi {
                // 多选：收集所有选中的选项
                let selected: Vec<usize> = app
                    .tool_ask_selections
                    .iter()
                    .enumerate()
                    .filter(|(i, sel)| **sel && *i < cur_q.options.len())
                    .map(|(i, _)| i)
                    .collect();
                if selected.is_empty() {
                    // 没有勾选任何项，就以当前光标所在项为选择
                    ask_submit_answer(app, AskAnswer::Selected(vec![cursor]));
                } else {
                    ask_submit_answer(app, AskAnswer::Selected(selected));
                }
            } else {
                // 单选：直接选中当前项
                ask_submit_answer(app, AskAnswer::Selected(vec![cursor]));
            }
        }
        // 回退到上一题
        KeyCode::Left | KeyCode::BackTab => {
            if app.tool_ask_current_idx > 0 {
                app.tool_ask_current_idx -= 1;
                // 恢复上一题的状态
                if app.tool_ask_answers.len() > app.tool_ask_current_idx {
                    app.tool_ask_answers.truncate(app.tool_ask_current_idx);
                }
                app.init_ask_question_state();
            }
        }
        // 前进（仅当已回答过时才能快速前进）
        KeyCode::Right | KeyCode::Tab => {
            if app.tool_ask_current_idx < total_questions - 1
                && app.tool_ask_current_idx < app.tool_ask_answers.len()
            {
                app.tool_ask_current_idx += 1;
                app.init_ask_question_state();
            }
        }
        KeyCode::Esc => {
            // 取消整个问答
            if let Some(tx) = app.ask_response_tx.take() {
                let _ = tx.send("用户取消了问答".to_string());
            }
            app.tool_ask_mode = false;
            app.tool_ask_questions.clear();
            app.tool_ask_current_idx = 0;
            app.tool_ask_answers.clear();
            app.tool_ask_selections.clear();
            app.tool_ask_cursor = 0;
            app.mode = ChatMode::Chat;
        }
        // PageUp/PageDown 滚动消息区（查看长问题内容）
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
        _ => {}
    }
}

/// 提交当前问题的答案，前进到下一题或完成全部
fn ask_submit_answer(app: &mut ChatApp, answer: AskAnswer) {
    let total = app.tool_ask_questions.len();

    // 存储答案
    if app.tool_ask_current_idx < app.tool_ask_answers.len() {
        app.tool_ask_answers[app.tool_ask_current_idx] = answer;
    } else {
        app.tool_ask_answers.push(answer);
    }

    if app.tool_ask_current_idx + 1 < total {
        // 下一题
        app.tool_ask_current_idx += 1;
        app.init_ask_question_state();
    } else {
        // 全部完成，构建 JSON 响应
        let mut answers_map = serde_json::Map::new();
        for (i, q) in app.tool_ask_questions.iter().enumerate() {
            if let Some(ans) = app.tool_ask_answers.get(i) {
                let val = match ans {
                    AskAnswer::Selected(indices) => {
                        let labels: Vec<&str> = indices
                            .iter()
                            .filter_map(|&idx| q.options.get(idx).map(|o| o.label.as_str()))
                            .collect();
                        labels.join(", ")
                    }
                    AskAnswer::FreeText(text) => text.clone(),
                };
                answers_map.insert(q.question.clone(), serde_json::Value::String(val));
            }
        }

        let response = serde_json::json!({ "answers": answers_map }).to_string();
        if let Some(tx) = app.ask_response_tx.take() {
            let _ = tx.send(response);
        }

        // 清理状态
        app.tool_ask_mode = false;
        app.tool_ask_questions.clear();
        app.tool_ask_current_idx = 0;
        app.tool_ask_answers.clear();
        app.tool_ask_selections.clear();
        app.tool_ask_cursor = 0;
        app.mode = ChatMode::Chat;
    }
}

// ========== @ 补全辅助函数 ==========

use super::autocomplete::{
    complete_at_mention, complete_file_mention, filter_dir_part, update_at_filter,
    update_file_filter,
};
/// 从 input 中提取 @ 之后的过滤文本
// autocomplete 函数已移至 super::autocomplete 模块
pub use super::autocomplete::{get_filtered_files, get_filtered_skills};
