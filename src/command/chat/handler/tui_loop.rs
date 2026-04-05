use super::super::input_thread::InputThread;
use super::super::remote;
use super::super::remote::bridge::WsBridge;
use super::super::remote::protocol::{WsInbound, WsOutbound};
use super::super::storage::{
    ChatSession, legacy_chat_history_path, load_style, load_system_prompt, save_style,
    save_system_prompt,
};
use super::super::ui::draw_chat_ui;
use super::{
    handle_archive_confirm_mode, handle_archive_list_mode, handle_browse_mode, handle_chat_mode,
    handle_config_mode, handle_select_model, handle_tool_confirm_mode,
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
        event::DisableBracketedPaste,
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
            }
            false
        }
        Event::Paste(text) => {
            // 粘贴事件：逐字符插入到输入框（仅 Chat 模式且非 loading）
            if matches!(app.ui.mode, ChatMode::Chat) {
                for c in text.chars() {
                    if c == '\n' || c == '\r' {
                        continue; // 忽略换行，输入框为单行
                    }
                    let byte_idx = app
                        .ui
                        .input
                        .char_indices()
                        .nth(app.ui.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(app.ui.input.len());
                    app.ui.input.insert(byte_idx, c);
                    app.ui.cursor_pos += 1;
                }
                *needs_redraw = true;
            }
            false
        }
        Event::Resize(_, _) => {
            *needs_redraw = true;
            false
        }
        Event::Mouse(mouse) => match mouse.kind {
            MouseEventKind::ScrollUp => {
                app.update(Action::Scroll(CursorDirection::Up));
                *needs_redraw = true;
                false
            }
            MouseEventKind::ScrollDown => {
                app.update(Action::Scroll(CursorDirection::Down));
                *needs_redraw = true;
                false
            }
            _ => false,
        },
        _ => false,
    }
}

pub fn run_chat_tui(remote_mode: bool, port: u16) {
    // 设置 panic hook，确保 panic 时也能恢复终端状态
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        original_hook(info);
    }));

    // 远程模式：先启动 WS 服务器，显示二维码，等待连接
    let ws_bridge = if remote_mode {
        match remote::start_remote_and_wait(port) {
            Ok((bridge, _url)) => Some(bridge),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::Interrupted {
                    // Ctrl+C 取消，直接返回不进入 TUI
                    return;
                }
                crate::error!("远程服务启动失败: {}", e);
                None
            }
        }
    } else {
        None
    };

    let result = run_chat_tui_internal(ws_bridge);

    // 恢复默认 panic hook
    let _ = std::panic::take_hook();

    if let Err(e) = result {
        restore_terminal();
        error!("✖️ Chat TUI 启动失败: {}", e);
    }
}

/// 生成本次会话 ID（委托给 storage 模块）
fn generate_session_id() -> String {
    super::super::storage::generate_session_id()
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

pub fn run_chat_tui_internal(ws_bridge: Option<WsBridge>) -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        event::EnableMouseCapture,
        event::EnableBracketedPaste
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 一次性迁移旧格式
    migrate_legacy_session_if_needed();

    let session_id = generate_session_id();
    let mut app = ChatApp::new(session_id);
    app.ws_bridge = ws_bridge;
    app.remote_connected = app
        .ws_bridge
        .as_ref()
        .map(|ws| ws.has_client())
        .unwrap_or(false);

    // 自动恢复最近的 session（如果开启了 auto_restore_session）
    if app.state.agent_config.auto_restore_session
        && let Some(latest_id) = super::super::storage::find_latest_session_id()
    {
        let session = super::super::storage::load_session(&latest_id);
        if !session.messages.is_empty() {
            app.session_id = latest_id;
            app.last_persisted_len = session.messages.len();
            app.state.session = session;
            app.ui.scroll_offset = u16::MAX; // 滚动到底部
            app.ui.msg_lines_cache = None;
        }
    }

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
                    supports_vision: false,
                }],
                active_index: 0,
                system_prompt: None,
                max_history_messages: 20,
                theme: ThemeName::default(),
                tools_enabled: false,
                max_tool_rounds: 10,
                style: None,
                tool_confirm_timeout: 0,
                disabled_tools: Vec::new(),
                disabled_skills: Vec::new(),
                disabled_commands: Vec::new(),
                compact: Default::default(),
                auto_restore_session: false,
            };
            let _ = save_agent_config(&example);
            app.state.agent_config = example;
        }
        // 直接进入配置界面
        app.ui.mode = ChatMode::Config;
        app.show_toast("尚未配置模型，请先完成配置 (Esc 保存退出)", true);
    }

    let mut needs_redraw = true; // 首次必须绘制
    let mut last_render_time = std::time::Instant::now();
    const RENDER_INTERVAL: std::time::Duration = std::time::Duration::from_millis(33); // ~30fps

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

        // Phase 2b: 收集 WebSocket 远程消息
        if app.ws_bridge.is_some() {
            // 取出 ws_bridge 来避免借用冲突
            let mut ws = app.ws_bridge.take().unwrap();
            let mut ws_actions: Vec<(WsInbound,)> = Vec::new();
            while let Some(msg) = ws.try_recv() {
                ws_actions.push((msg,));
            }
            app.remote_connected = ws.has_client();
            app.ws_bridge = Some(ws);

            for (msg,) in ws_actions {
                needs_redraw = true;
                match msg {
                    WsInbound::SendMessage { content } => {
                        app.inject_remote_message(content);
                    }
                    WsInbound::ToolConfirm { action, reason } => match action.as_str() {
                        "allow" => app.update(Action::ExecutePendingTool),
                        "allow_always" => app.update(Action::AllowAndExecutePendingTool),
                        "reject_with_reason" => {
                            let r = reason.unwrap_or_default();
                            app.update(Action::RejectPendingToolWithReason(r));
                        }
                        _ => app.update(Action::RejectPendingTool),
                    },
                    WsInbound::AskResponse { answers } => {
                        if app.ui.tool_ask_mode {
                            // 将远程回答直接构建为 JSON 响应发送给 Ask 工具
                            let response = serde_json::json!({ "answers": answers }).to_string();
                            if let Some(tx) = app.ask_response_tx.take() {
                                let _ = tx.send(response);
                            }
                            // 清理 ask 状态
                            app.ui.tool_ask_mode = false;
                            app.ui.tool_ask_questions.clear();
                            app.ui.tool_ask_current_idx = 0;
                            app.ui.tool_ask_answers.clear();
                            app.ui.tool_ask_selections.clear();
                            app.ui.tool_ask_cursor = 0;
                            if !app.tool_executor.has_pending_confirm() {
                                app.ui.mode = ChatMode::Chat;
                            }
                            app.broadcast_ws(WsOutbound::Status {
                                state: "loading".to_string(),
                            });
                        }
                    }
                    WsInbound::Cancel => {
                        app.update(Action::CancelStream);
                    }
                    WsInbound::Sync => {
                        let sync = app.build_sync_outbound();
                        app.broadcast_ws(sync);
                    }
                    WsInbound::Ping => {
                        app.broadcast_ws(WsOutbound::Pong);
                    }
                    WsInbound::ListSessions => {
                        app.update(Action::ListSessions);
                    }
                    WsInbound::SwitchSession { session_id } => {
                        app.update(Action::SwitchSession { session_id });
                    }
                    WsInbound::NewSession => {
                        app.update(Action::NewSession);
                    }
                    // KeyExchange 在 server.rs 层处理，不会到达 TUI 层
                    WsInbound::KeyExchange { .. } => {}
                }
            }
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
        // Phase 3: Render — 只在状态变化时重绘，带 30fps 节流
        // ================================================================
        if needs_redraw {
            // 节流：间隔至少 33ms（~30fps），快速连续事件合并为一帧
            if last_render_time.elapsed() >= RENDER_INTERVAL {
                terminal.draw(|f| draw_chat_ui(f, &mut app))?;
                needs_redraw = false;
                last_render_time = std::time::Instant::now();
                // 更新流式节流状态（复用 Phase 2 已获取的长度，不再重新加锁）
                if app.state.is_loading {
                    app.ui.last_rendered_streaming_len = streaming_snapshot_len;
                    app.ui.last_stream_render_time = std::time::Instant::now();
                }
            }
            // 如果被节流跳过，needs_redraw 保持 true，下一轮循环会补上
        }

        // ================================================================
        // Phase 4: Collect Input — 从 channel 读事件（输入线程持续收集，不受渲染阻塞影响）
        // ================================================================
        #[allow(clippy::if_same_then_else)]
        let poll_timeout = if app.state.is_loading {
            std::time::Duration::from_millis(300)
        } else if app.ui.mode == ChatMode::ToolConfirm {
            std::time::Duration::from_millis(500)
        } else {
            std::time::Duration::from_millis(500)
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

    // ★ 空会话不保存：删除无消息的 session 文件
    if app.state.session.messages.is_empty() {
        super::super::storage::delete_session(&app.session_id);
    }

    // ★ 先恢复终端，再跑 SessionEnd hook（避免 hook 阻塞时终端卡在 raw mode）
    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        event::DisableBracketedPaste,
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
