use super::super::storage::{
    load_style, load_system_prompt, save_chat_session, save_style, save_system_prompt,
};
use super::super::ui::draw_chat_ui;
use super::{
    handle_archive_confirm_mode, handle_archive_list_mode, handle_browse_mode, handle_chat_mode,
    handle_config_mode, handle_select_model, handle_skill_toggle_mode, handle_tool_confirm_mode,
    handle_tool_toggle_mode,
};
use crate::command::chat::app::{Action, ChatApp, ChatMode, CursorDirection};
use crate::error;
use crossterm::{
    event::{self, Event, KeyEventKind, MouseEventKind},
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
    execute!(stdout, EnterAlternateScreen, event::EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = ChatApp::new();

    // 首次运行（尚未配置 provider）时，自动进入配置界面引导用户完成配置
    if app.state.agent_config.providers.is_empty() {
        use super::super::storage::{
            AgentConfig, ModelProvider, agent_config_path, save_agent_config,
        };
        use super::super::theme::ThemeName;
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
            app.state.agent_config = example;
        }
        // 直接进入配置界面
        app.ui.mode = ChatMode::Config;
        app.show_toast("尚未配置模型，请先完成配置 (Esc 保存退出)", true);
    }

    let mut needs_redraw = true; // 首次必须绘制

    loop {
        // ================================================================
        // Phase 1: Tick — 定时器和周期性状态更新
        // ================================================================
        let had_toast = app.ui.toast.is_some();
        app.update(Action::TickToast);
        if had_toast && app.ui.toast.is_none() {
            needs_redraw = true;
        }

        // ================================================================
        // Phase 2: Poll Backend — 收集后台事件 → Actions → dispatch
        // ================================================================
        let was_loading = app.state.is_loading;
        let stream_actions = app.poll_stream_actions();
        if !stream_actions.is_empty() {
            needs_redraw = true;
        }
        for action in stream_actions {
            app.update(action);
        }

        // 有待执行的工具时强制重绘
        if app.tool_executor.pending_tool_execution {
            needs_redraw = true;
        }

        // ToolConfirm 超时自动执行 → Action
        if app.ui.mode == ChatMode::ToolConfirm && app.state.agent_config.tool_confirm_timeout > 0 {
            let elapsed = app.tool_executor.tool_confirm_entered_at.elapsed();
            let timeout =
                std::time::Duration::from_secs(app.state.agent_config.tool_confirm_timeout);
            if elapsed >= timeout {
                app.update(Action::ExecutePendingTool);
                needs_redraw = true;
            } else {
                needs_redraw = true; // 倒计时变化需要重绘
            }
        }

        // 流式加载中的节流策略
        if app.state.is_loading {
            let current_len = app.state.streaming_content.lock().unwrap().len();
            let bytes_delta = current_len.saturating_sub(app.ui.last_rendered_streaming_len);
            let time_elapsed = app.ui.last_stream_render_time.elapsed();
            if bytes_delta >= 200
                || time_elapsed >= std::time::Duration::from_millis(150)
                || current_len == 0
            {
                needs_redraw = true;
            }
        } else if was_loading {
            needs_redraw = true;
        }

        // ToolConfirm 模式下强制重绘
        if app.ui.mode == ChatMode::ToolConfirm {
            needs_redraw = true;
        }

        // ================================================================
        // Phase 3: Render — 只在状态变化时重绘
        // ================================================================
        if needs_redraw {
            terminal.draw(|f| draw_chat_ui(f, &mut app))?;
            needs_redraw = false;
            // 更新流式节流状态
            if app.state.is_loading {
                app.ui.last_rendered_streaming_len =
                    app.state.streaming_content.lock().unwrap().len();
                app.ui.last_stream_render_time = std::time::Instant::now();
            }
        }

        // ================================================================
        // Phase 4: Collect Input — 等待事件 → Actions → dispatch
        // ================================================================
        let poll_timeout = if app.state.is_loading {
            std::time::Duration::from_millis(150)
        } else if app.ui.mode == ChatMode::ToolConfirm {
            std::time::Duration::from_millis(500)
        } else {
            std::time::Duration::from_millis(1000)
        };

        if event::poll(poll_timeout)? {
            // 批量消费所有待处理事件
            let mut should_quit = false;
            loop {
                let evt = event::read()?;
                match evt {
                    Event::Key(key)
                        if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                    {
                        needs_redraw = true;
                        match app.ui.mode {
                            ChatMode::Chat => {
                                if handle_chat_mode(&mut app, key) {
                                    should_quit = true;
                                    break;
                                }
                            }
                            ChatMode::SelectModel => handle_select_model(&mut app, key),
                            ChatMode::Browse => handle_browse_mode(&mut app, key),
                            ChatMode::Help => {
                                app.update(Action::ExitToChat);
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
                            app.update(Action::Scroll(CursorDirection::Up));
                            needs_redraw = true;
                        }
                        MouseEventKind::ScrollDown => {
                            app.update(Action::Scroll(CursorDirection::Down));
                            needs_redraw = true;
                        }
                        _ => {}
                    },
                    _ => {}
                }
                // 继续消费剩余事件（非阻塞）
                if !event::poll(std::time::Duration::ZERO)? {
                    break;
                }
            }
            if should_quit {
                break;
            }

            // ================================================================
            // Phase 5: Side-effects — 全屏编辑器等需要临时离开 TUI 的操作
            // ================================================================
            if app.ui.pending_system_prompt_edit {
                app.ui.pending_system_prompt_edit = false;
                let current_prompt = load_system_prompt().unwrap_or_default();
                match crate::tui::editor::open_editor_on_terminal(
                    &mut terminal,
                    "编辑系统提示词 (System Prompt)",
                    &current_prompt,
                ) {
                    Ok(Some(new_text)) => {
                        if save_system_prompt(&new_text) {
                            app.update(Action::ShowToast("系统提示词已更新".to_string(), false));
                        } else {
                            app.update(Action::ShowToast("系统提示词保存失败".to_string(), true));
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        app.update(Action::ShowToast(format!("编辑器错误: {}", e), true));
                    }
                }
                needs_redraw = true;
            }

            if app.ui.pending_style_edit {
                app.ui.pending_style_edit = false;
                let current_style = load_style().unwrap_or_default();
                match crate::tui::editor::open_editor_on_terminal(
                    &mut terminal,
                    "编辑回复风格 (Style)",
                    &current_style,
                ) {
                    Ok(Some(new_text)) => {
                        if save_style(&new_text) {
                            app.update(Action::ShowToast("回复风格已更新".to_string(), false));
                        } else {
                            app.update(Action::ShowToast("回复风格保存失败".to_string(), true));
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        app.update(Action::ShowToast(format!("编辑器错误: {}", e), true));
                    }
                }
                needs_redraw = true;
            }
        }
    }

    // 保存对话历史
    let _ = save_chat_session(&app.state.session);

    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        event::DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    Ok(())
}
