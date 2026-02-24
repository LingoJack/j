use super::app::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS, ChatApp, ChatMode, config_total_fields};
use super::model::{ModelProvider, save_agent_config, save_chat_session};
use super::render::copy_to_clipboard;
use super::ui::draw_chat_ui;
use crate::{error, info};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseEventKind,
    },
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
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = ChatApp::new();

    if app.agent_config.providers.is_empty() {
        terminal::disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        info!("⚠️  尚未配置 LLM 模型提供方，请先运行 j chat 查看配置说明。");
        return Ok(());
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

        // 等待事件：加载中用短间隔以刷新流式内容，空闲时用长间隔节省 CPU
        let poll_timeout = if app.is_loading {
            std::time::Duration::from_millis(150)
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
                        }
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
                    Event::Resize(_, _) => {
                        needs_redraw = true;
                    }
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
        }
    }

    // 保存对话历史
    let _ = save_chat_session(&app.session);

    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}

/// 绘制 TUI 界面

pub fn handle_chat_mode(app: &mut ChatApp, key: KeyEvent) -> bool {
    // Ctrl+C 强制退出
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
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

    // Ctrl+L 清空对话
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('l') {
        app.clear_session();
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
            app.mode = ChatMode::Browse;
            app.msg_lines_cache = None; // 清除缓存以触发高亮重绘
        } else {
            app.show_toast("暂无消息可浏览", true);
        }
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
        app.show_toast(&format!("已切换为: {}", mode_str), false);
        return false;
    }

    let char_count = app.input.chars().count();

    match key.code {
        KeyCode::Esc => return true,

        KeyCode::Enter => {
            if !app.is_loading {
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
            app.input.insert_str(byte_idx, &c.to_string());
            app.cursor_pos += 1;
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
                app.msg_lines_cache = None; // 选中变化时清缓存
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.browse_msg_index < msg_count - 1 {
                app.browse_msg_index += 1;
                app.msg_lines_cache = None; // 选中变化时清缓存
            }
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
                        &format!("已复制第 {} 条{}消息", app.browse_msg_index + 1, role_label),
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
pub fn config_field_label(idx: usize) -> &'static str {
    let total_provider = CONFIG_FIELDS.len();
    if idx < total_provider {
        match CONFIG_FIELDS[idx] {
            "name" => "显示名称",
            "api_base" => "API Base",
            "api_key" => "API Key",
            "model" => "模型名称",
            _ => CONFIG_FIELDS[idx],
        }
    } else {
        let gi = idx - total_provider;
        match CONFIG_GLOBAL_FIELDS[gi] {
            "system_prompt" => "系统提示词",
            "stream_mode" => "流式输出",
            "max_history_messages" => "历史消息数",
            _ => CONFIG_GLOBAL_FIELDS[gi],
        }
    }
}

/// 获取配置界面中当前字段的值
pub fn config_field_value(app: &ChatApp, field_idx: usize) -> String {
    let total_provider = CONFIG_FIELDS.len();
    if field_idx < total_provider {
        if app.agent_config.providers.is_empty() {
            return String::new();
        }
        let p = &app.agent_config.providers[app.config_provider_idx];
        match CONFIG_FIELDS[field_idx] {
            "name" => p.name.clone(),
            "api_base" => p.api_base.clone(),
            "api_key" => {
                // 显示时隐藏 API Key 中间部分
                if p.api_key.len() > 8 {
                    format!(
                        "{}****{}",
                        &p.api_key[..4],
                        &p.api_key[p.api_key.len() - 4..]
                    )
                } else {
                    p.api_key.clone()
                }
            }
            "model" => p.model.clone(),
            _ => String::new(),
        }
    } else {
        let gi = field_idx - total_provider;
        match CONFIG_GLOBAL_FIELDS[gi] {
            "system_prompt" => app.agent_config.system_prompt.clone().unwrap_or_default(),
            "stream_mode" => {
                if app.agent_config.stream_mode {
                    "开启".into()
                } else {
                    "关闭".into()
                }
            }
            "max_history_messages" => app.agent_config.max_history_messages.to_string(),
            _ => String::new(),
        }
    }
}

/// 获取配置字段的原始值（用于编辑时填入输入框）
pub fn config_field_raw_value(app: &ChatApp, field_idx: usize) -> String {
    let total_provider = CONFIG_FIELDS.len();
    if field_idx < total_provider {
        if app.agent_config.providers.is_empty() {
            return String::new();
        }
        let p = &app.agent_config.providers[app.config_provider_idx];
        match CONFIG_FIELDS[field_idx] {
            "name" => p.name.clone(),
            "api_base" => p.api_base.clone(),
            "api_key" => p.api_key.clone(),
            "model" => p.model.clone(),
            _ => String::new(),
        }
    } else {
        let gi = field_idx - total_provider;
        match CONFIG_GLOBAL_FIELDS[gi] {
            "system_prompt" => app.agent_config.system_prompt.clone().unwrap_or_default(),
            "stream_mode" => {
                if app.agent_config.stream_mode {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            _ => String::new(),
        }
    }
}

/// 将编辑结果写回配置
pub fn config_field_set(app: &mut ChatApp, field_idx: usize, value: &str) {
    let total_provider = CONFIG_FIELDS.len();
    if field_idx < total_provider {
        if app.agent_config.providers.is_empty() {
            return;
        }
        let p = &mut app.agent_config.providers[app.config_provider_idx];
        match CONFIG_FIELDS[field_idx] {
            "name" => p.name = value.to_string(),
            "api_base" => p.api_base = value.to_string(),
            "api_key" => p.api_key = value.to_string(),
            "model" => p.model = value.to_string(),
            _ => {}
        }
    } else {
        let gi = field_idx - total_provider;
        match CONFIG_GLOBAL_FIELDS[gi] {
            "system_prompt" => {
                if value.is_empty() {
                    app.agent_config.system_prompt = None;
                } else {
                    app.agent_config.system_prompt = Some(value.to_string());
                }
            }
            "stream_mode" => {
                app.agent_config.stream_mode = matches!(
                    value.trim().to_lowercase().as_str(),
                    "true" | "1" | "开启" | "on" | "yes"
                );
            }
            "max_history_messages" => {
                if let Ok(num) = value.trim().parse::<usize>() {
                    app.agent_config.max_history_messages = num;
                }
            }
            _ => {}
        }
    }
}

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
            // 保存并返回
            let _ = save_agent_config(&app.agent_config);
            app.show_toast("配置已保存 ✅", false);
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
