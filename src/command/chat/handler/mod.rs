mod archive;
mod browse;
mod chat;
mod config;
mod tool_confirm;

// Re-export all handler functions
pub use archive::{handle_archive_confirm_mode, handle_archive_list_mode};
pub use browse::handle_browse_mode;
pub use chat::handle_chat_mode;
pub use config::{
    handle_config_mode, handle_select_model, handle_skill_toggle_mode, handle_tool_toggle_mode,
};
pub use tool_confirm::handle_tool_confirm_mode;

// Re-export config_field_* from super::config (for ui/config.rs compatibility)
pub use super::config::{config_field_label, config_field_value};

// Re-export autocomplete functions (for ui/chat.rs compatibility)
pub use super::autocomplete::{get_filtered_files, get_filtered_skills};

use super::model::{
    load_style, load_system_prompt, save_chat_session, save_style, save_system_prompt,
};
use super::ui::draw_chat_ui;
use crate::command::chat::app::{ChatApp, ChatMode};
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
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
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
