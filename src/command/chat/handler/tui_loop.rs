use super::super::input_thread::InputThread;
use super::super::storage::{
    ChatSession, legacy_chat_history_path, load_style, load_system_prompt, save_style,
    save_system_prompt,
};
use super::super::ui::draw_chat_ui;
use super::{
    handle_archive_confirm_mode, handle_archive_list_mode, handle_browse_mode, handle_chat_mode,
    handle_config_mode, handle_select_model, handle_skill_toggle_mode, handle_tool_confirm_mode,
    handle_tool_toggle_mode,
};
use crate::command::chat::app::{Action, ChatApp, ChatMode, CursorDirection};
use crate::error;
use crate::util::safe_lock;
use crossterm::{
    event::{self, Event, KeyEventKind, MouseEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

/// 恢复终端状态：关闭鼠标追踪、离开备用屏幕、关闭 raw mode
fn restore_terminal() {
    let _ = terminal::disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        event::DisableMouseCapture,
        LeaveAlternateScreen
    );
}

/// 将单个 crossterm Event 分发到对应的 handler / Action。
/// 返回 true 表示应退出主循环。
fn dispatch_event(app: &mut ChatApp, evt: Event, needs_redraw: &mut bool) -> bool {
    match evt {
        Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
            *needs_redraw = true;
            match app.ui.mode {
                ChatMode::Chat => {
                    if handle_chat_mode(app, key) {
                        return true; // quit
                    }
                }
                ChatMode::SelectModel => handle_select_model(app, key),
                ChatMode::Browse => handle_browse_mode(app, key),
                ChatMode::Help => {
                    app.update(Action::ExitToChat);
                }
                ChatMode::Config => handle_config_mode(app, key),
                ChatMode::ArchiveConfirm => handle_archive_confirm_mode(app, key),
                ChatMode::ArchiveList => handle_archive_list_mode(app, key),
                ChatMode::ToolConfirm => handle_tool_confirm_mode(app, key),
                ChatMode::ToolToggle => handle_tool_toggle_mode(app, key),
                ChatMode::SkillToggle => handle_skill_toggle_mode(app, key),
            }
        }
        Event::Resize(_, _) => {
            *needs_redraw = true;
        }
        Event::Mouse(mouse) => match mouse.kind {
            MouseEventKind::ScrollUp => {
                app.update(Action::Scroll(CursorDirection::Up));
                *needs_redraw = true;
            }
            MouseEventKind::ScrollDown => {
                app.update(Action::Scroll(CursorDirection::Down));
                *needs_redraw = true;
            }
            _ => {}
        },
        _ => {}
    }
    false
}

pub fn run_chat_tui() {
    // 设置 panic hook，确保 panic 时也能恢复终端状态
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        original_hook(info);
    }));

    let result = run_chat_tui_internal();

    // 恢复默认 panic hook
    let _ = std::panic::take_hook();

    if let Err(e) = result {
        restore_terminal();
        error!("✖️ Chat TUI 启动失败: {}", e);
    }
}

/// 生成本次会话 ID（时间戳微秒 + 进程 ID，无需外部依赖）
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    let pid = std::process::id();
    format!("{:x}-{:x}", ts, pid)
}

/// 一次性迁移旧 chat_history.json → 归档，保留历史对话
fn migrate_legacy_session_if_needed() {
    let old_path = legacy_chat_history_path();
    if !old_path.exists() {
        return;
    }
    let migrated = (|| {
        let content = std::fs::read_to_string(&old_path).ok()?;
        let session: ChatSession = serde_json::from_str(&content).ok()?;
        if session.messages.is_empty() {
            return None;
        }
        let name = format!("migrated-{}", chrono::Local::now().format("%Y-%m-%d"));
        super::super::archive::create_archive(&name, session.messages).ok()?;
        Some(name)
    })();
    // 无论迁移是否成功，删除旧文件避免重复迁移
    let _ = std::fs::remove_file(&old_path);
    if let Some(name) = migrated {
        crate::util::log::write_info_log(
            "migrate_legacy_session",
            &format!("旧对话历史已迁移为归档: {}", name),
        );
    }
}

pub fn run_chat_tui_internal() -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, event::EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 一次性迁移旧格式
    migrate_legacy_session_if_needed();

    let session_id = generate_session_id();
    let mut app = ChatApp::new(session_id);

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
                compact: Default::default(),
            };
            let _ = save_agent_config(&example);
            app.state.agent_config = example;
        }
        // 直接进入配置界面
        app.ui.mode = ChatMode::Config;
        app.show_toast("尚未配置模型，请先完成配置 (Esc 保存退出)", true);
    }

    let mut needs_redraw = true; // 首次必须绘制

    // 启动独立输入线程：持续从 crossterm 读事件放入 channel，
    // 主循环只从 channel 取，无论渲染多慢输入永远不丢。
    let input_thread = InputThread::spawn();

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

        // 流式加载中的节流策略（只锁一次获取长度，避免多次 safe_lock）
        let streaming_snapshot_len: usize = if app.state.is_loading {
            let len = safe_lock(&app.state.streaming_content, "tui_loop::streaming_throttle").len();
            let bytes_delta = len.saturating_sub(app.ui.last_rendered_streaming_len);
            let time_elapsed = app.ui.last_stream_render_time.elapsed();
            if bytes_delta >= 200
                || time_elapsed >= std::time::Duration::from_millis(150)
                || len == 0
            {
                needs_redraw = true;
            }
            len
        } else {
            if was_loading {
                needs_redraw = true;
            }
            0
        };

        // ToolConfirm 模式下：仅在有倒计时时才周期性重绘（用于更新秒数显示）
        if app.ui.mode == ChatMode::ToolConfirm && app.state.agent_config.tool_confirm_timeout > 0 {
            needs_redraw = true;
        }

        // ================================================================
        // Phase 3: Render — 只在状态变化时重绘
        // ================================================================
        if needs_redraw {
            terminal.draw(|f| draw_chat_ui(f, &mut app))?;
            needs_redraw = false;
            // 更新流式节流状态（复用 Phase 2 已获取的长度，不再重新加锁）
            if app.state.is_loading {
                app.ui.last_rendered_streaming_len = streaming_snapshot_len;
                app.ui.last_stream_render_time = std::time::Instant::now();
            }
        }

        // ================================================================
        // Phase 4: Collect Input — 从 channel 读事件（输入线程持续收集，不受渲染阻塞影响）
        // ================================================================
        let poll_timeout = if app.state.is_loading {
            std::time::Duration::from_millis(300)
        } else if app.ui.mode == ChatMode::ToolConfirm {
            std::time::Duration::from_millis(1000)
        } else {
            std::time::Duration::from_millis(2000)
        };

        // 阻塞等待第一个事件（受 poll_timeout 限制）
        let first = input_thread.rx.recv_timeout(poll_timeout);
        if let Ok(evt) = first {
            let mut should_quit = dispatch_event(&mut app, evt, &mut needs_redraw);
            // 批量消费所有已缓冲的后续事件（非阻塞）
            if !should_quit {
                while let Ok(evt) = input_thread.rx.try_recv() {
                    if dispatch_event(&mut app, evt, &mut needs_redraw) {
                        should_quit = true;
                        break;
                    }
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
                // 暂停输入线程，编辑器需要独占 stdin
                input_thread.pause();
                input_thread.drain();
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
                // 恢复输入线程，清空编辑器期间可能产生的残留事件
                input_thread.drain();
                input_thread.resume();
                needs_redraw = true;
            }

            if app.ui.pending_style_edit {
                app.ui.pending_style_edit = false;
                // 暂停输入线程，编辑器需要独占 stdin
                input_thread.pause();
                input_thread.drain();
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
                // 恢复输入线程，清空编辑器期间可能产生的残留事件
                input_thread.drain();
                input_thread.resume();
                needs_redraw = true;
            }
        }
    }

    // 停止输入线程
    input_thread.shutdown();

    // ★ 先恢复终端，再跑 SessionEnd hook（避免 hook 阻塞时终端卡在 raw mode）
    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        event::DisableMouseCapture,
        LeaveAlternateScreen
    )?;

    // ★ SessionEnd hook（fire-and-forget，终端已恢复）
    {
        use crate::command::chat::hook::{HookContext, HookEvent, HookManager};
        let has_hooks = app
            .hook_manager
            .lock()
            .map(|m| m.has_hooks_for(HookEvent::SessionEnd))
            .unwrap_or(false);
        if has_hooks {
            let ctx = HookContext {
                event: HookEvent::SessionEnd,
                messages: Some(app.state.session.messages.clone()),
                cwd: std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
                ..Default::default()
            };
            HookManager::execute_fire_and_forget(
                std::sync::Arc::clone(&app.hook_manager),
                HookEvent::SessionEnd,
                ctx,
            );
        }
    }

    Ok(())
}
