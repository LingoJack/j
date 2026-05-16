use super::{
    handle_agent_perm_confirm_mode, handle_archive_confirm_mode, handle_archive_list_mode,
    handle_browse_mode, handle_chat_mode, handle_config_mode, handle_plan_approval_confirm_mode,
    handle_select_model, handle_select_theme, handle_tool_confirm_mode,
};
use crate::command::chat::agent_md;
use crate::command::chat::app::types::PlanDecision;
use crate::command::chat::app::{Action, ChatApp, ChatMode, ConfigTab, CursorDirection};
use crate::command::chat::app::{ContextMenu, MouseSelection};
use crate::command::chat::constants::{TUI_IDLE_POLL_MS, TUI_LOADING_POLL_MS};
use crate::command::chat::infra::hook::{HookContext, HookEvent, HookManager};
use crate::command::chat::input_thread::InputThread;
use crate::command::chat::remote;
use crate::command::chat::remote::bridge::WsBridge;
use crate::command::chat::remote::protocol::{WsInbound, WsOutbound};
use crate::command::chat::render::cache::copy_to_clipboard;
use crate::command::chat::storage::{
    load_style, load_system_prompt, save_style, save_system_prompt,
};
use crate::command::chat::ui::chat::{
    copy_selection_to_clipboard, extract_selection_text, screen_to_text_pos,
};
use crate::command::chat::ui::context_menu::is_point_in_menu;
use crate::command::chat::ui::draw_chat_ui;
use crate::command::chat::ui::help::{help_extract_selection_text, help_screen_to_text_pos};
use crate::error;
use crate::util::safe_lock;
use crossterm::{
    event::{
        self, Event, KeyCode, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags, MouseButton,
        MouseEventKind, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect};
use std::io;

/// RAII guard：确保 TUI 退出时（含 panic / `?` 传播）恢复终端到正常状态。
///
/// 正常退出路径调用 [`TerminalGuard::disarm`] 后，`Drop` 不再重复恢复。
/// 异常路径（panic、loop 内 `?` 提前返回）由 `Drop` 兜底执行完整恢复序列。
struct TerminalGuard {
    /// keyboard enhancement 协议是否已 push
    keyboard_enhancement_active: bool,
    /// 是否已手动恢复（disarm），避免 Drop 重复恢复
    disarmed: bool,
}

impl TerminalGuard {
    fn new() -> Self {
        Self {
            keyboard_enhancement_active: false,
            disarmed: false,
        }
    }

    /// 标记 `PushKeyboardEnhancementFlags` 已执行成功
    fn set_keyboard_active(&mut self) {
        self.keyboard_enhancement_active = true;
    }

    /// 正常退出路径：手动完成恢复后调用，阻止 `Drop` 再次恢复。
    fn disarm(&mut self) {
        self.disarmed = true;
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }
        let _ = terminal::disable_raw_mode();
        // 使用 io::stdout() 而非 terminal.backend_mut()，
        // 因为 Drop 发生时 terminal 可能已经被 move 或 drop。
        let mut stdout = io::stdout();
        let _ = restore_terminal_state(&mut stdout, self.keyboard_enhancement_active);
    }
}

/// 尝试启用 keyboard enhancement。
///
/// 部分终端会直接忽略该协议，但 legacy WindowsAPI 会显式返回错误。
/// 这里将其视为可选能力：失败时继续运行，只是少了更细粒度的按键区分。
fn try_enable_keyboard_enhancement<W: io::Write>(writer: &mut W) -> bool {
    execute!(
        writer,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )
    .is_ok()
}

/// 恢复终端状态。
///
/// keyboard enhancement 的 `Pop` 必须单独处理，避免其失败时短路后续恢复步骤。
fn restore_terminal_state<W: io::Write>(
    writer: &mut W,
    keyboard_enhancement_active: bool,
) -> io::Result<()> {
    if keyboard_enhancement_active {
        let _ = execute!(writer, PopKeyboardEnhancementFlags);
    }

    execute!(
        writer,
        event::DisableMouseCapture,
        event::DisableBracketedPaste,
        LeaveAlternateScreen
    )
}

/// 根据当前 ChatMode (及 ConfigTab) 将鼠标滚轮事件路由到对应的导航 Action。
fn mouse_scroll_action(app: &ChatApp, dir: CursorDirection) -> Action {
    match app.ui.mode {
        ChatMode::Config if !app.ui.config_editing => {
            // 不同 ConfigTab 使用各自的导航索引和 Action
            match app.ui.config_tab {
                ConfigTab::Session => Action::SessionListNavigate(dir),
                ConfigTab::Archive => Action::ArchiveListNavigate(dir),
                ConfigTab::Teammates => Action::TeammatesNavigate(dir),
                ConfigTab::Tools | ConfigTab::Skills | ConfigTab::Hooks | ConfigTab::Commands => {
                    Action::ToggleMenuNavigate(dir)
                }
                _ => Action::ConfigNavigate(dir),
            }
        }
        ChatMode::SelectModel => Action::ModelSelectNavigate(dir),
        ChatMode::SelectTheme => Action::ThemeSelectNavigate(dir),
        ChatMode::ArchiveList => Action::ArchiveListNavigate(dir),
        _ => Action::Scroll(dir),
    }
}

/// 处理配置界面的鼠标左键点击事件。
///
/// 返回 `Some(action)` 表示需要执行的 Action，返回 `None` 表示点击未命中任何可交互区域。
fn config_mouse_click(app: &mut ChatApp, col: u16, row: u16) -> Option<Action> {
    // 不可编辑状态下才处理点击
    if app.ui.config_editing {
        return None;
    }

    // ── 1. 检测 Tab 栏点击 ──
    if let Some(tab_bar_y) = app.ui.config_tab_bar_y
        && row == tab_bar_y
    {
        for hitbox in &app.ui.config_tab_hitboxes {
            if col >= hitbox.start_col && col < hitbox.end_col {
                if app.ui.config_tab != hitbox.tab {
                    return Some(Action::ConfigSwitchTabTo(hitbox.tab));
                }
                return None; // 点击当前已激活的 Tab，不做任何操作
            }
        }
        return None; // 点击了 Tab 栏但不在任何 Tab 上
    }

    // ── 2. 检测列表项点击 ──
    let list_area = app.ui.config_list_area?;
    if !is_point_in_rect(col, row, list_area) {
        return None;
    }

    // 计算列表内的内容行号
    // list_area 无顶部 border（header_block 只有 TOP|LEFT|RIGHT，list_block 只有 BOTTOM|LEFT|RIGHT）
    // 所以列表内容从 list_area.y 开始
    let inner_y = (row - list_area.y) as usize;
    let content_y = inner_y + app.ui.config_scroll_offset as usize;

    // 在 field_line_indices 中查找：每个可交互项占据连续的若干行，
    // field_line_indices[i] 是第 i 个项的起始行号
    let field_lines = &app.ui.config_field_lines;
    if field_lines.is_empty() {
        return None;
    }

    // 二分查找：找到最后一个起始行 <= content_y 的项
    let clicked_idx = match field_lines.binary_search(&content_y) {
        Ok(idx) => idx,        // 精确命中某项的起始行
        Err(0) => return None, // content_y 小于第一项，点在空白区
        Err(idx) => {
            // idx 是第一个 > content_y 的位置，所以 idx-1 是最后一个 <= content_y 的
            let candidate = idx - 1;
            // 检查 content_y 是否在 candidate 的范围内
            // 如果有下一项，范围是 [candidate_start, next_start)；否则到列表末尾
            let candidate_start = field_lines[candidate];
            if content_y < candidate_start {
                return None;
            }
            candidate
        }
    };

    // 获取当前选中索引，判断是否为"再次点击已选中项"
    let current_idx = match app.ui.config_tab {
        ConfigTab::Session => app.ui.session_list_index,
        ConfigTab::Archive => app.ui.archive_list_index,
        ConfigTab::Teammates => app.ui.teammate_list_index,
        ConfigTab::Global if app.ui.compact_exempt_sublist => app.ui.compact_exempt_idx,
        ConfigTab::Model
        | ConfigTab::Tools
        | ConfigTab::Skills
        | ConfigTab::Hooks
        | ConfigTab::Commands
        | ConfigTab::Global => app.ui.config_field_idx,
    };

    if clicked_idx == current_idx {
        // 再次点击已选中项：触发 Enter 操作
        return Some(config_enter_action(app));
    }

    // 单击新项：选中它
    Some(config_select_action(app, clicked_idx))
}

/// 根据当前 ConfigTab 返回"选中指定索引"的 Action。
fn config_select_action(app: &ChatApp, idx: usize) -> Action {
    match app.ui.config_tab {
        ConfigTab::Session => Action::SessionListSelect(idx),
        ConfigTab::Archive => Action::ArchiveListSelect(idx),
        ConfigTab::Teammates => Action::TeammatesSelect(idx),
        ConfigTab::Global if app.ui.compact_exempt_sublist => Action::CompactExemptSelect(idx),
        _ => Action::ConfigFieldSelect(idx),
    }
}

/// 根据当前 ConfigTab 返回"确认/进入"的 Action。
fn config_enter_action(app: &ChatApp) -> Action {
    match app.ui.config_tab {
        ConfigTab::Session | ConfigTab::Archive | ConfigTab::Teammates => Action::ConfigEnter,
        ConfigTab::Tools | ConfigTab::Skills | ConfigTab::Hooks | ConfigTab::Commands => {
            Action::ToggleMenuToggle
        }
        _ => Action::ConfigEnter,
    }
}

/// 判断点 (col, row) 是否在 Rect 内部（不含 border）。
fn is_point_in_rect(col: u16, row: u16, rect: Rect) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

/// 执行右键菜单的复制操作。
///
/// 优先级：有选区时复制选区内容，无选区时复制整条消息。
fn execute_context_menu_copy(app: &mut ChatApp) {
    // 优先复制选区
    if app.ui.mouse_selection.is_some() {
        copy_selection_to_clipboard(app);
        app.ui.mouse_selection = None;
        return;
    }

    // 无选区：通过 global_line 定位消息并复制整条消息内容
    let global_line = match &app.ui.context_menu {
        Some(m) => m.global_line,
        None => return,
    };

    let cached = match app.ui.msg_lines_cache.as_ref() {
        Some(c) => c,
        None => return,
    };

    // 二分查找 global_line 所属消息索引
    let msg_index = {
        let mut lo = 0usize;
        let mut hi = cached.msg_start_lines.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let (_, start) = cached.msg_start_lines[mid];
            if start <= global_line {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        // lo 是第一个 start > global_line 的位置，所属消息是 lo - 1
        lo.saturating_sub(1)
    };

    // 从 display_messages 获取消息内容
    let content = {
        let display = crate::util::safe_lock(&app.display_messages, "ContextMenuCopy");
        display.get(msg_index).map(|msg| msg.content.clone())
    };

    if let Some(content) = content
        && !content.is_empty()
    {
        use crate::command::chat::render::cache::copy_to_clipboard;
        if copy_to_clipboard(&content) {
            app.show_toast("已复制到剪贴板", false);
        } else {
            app.show_toast("复制到剪贴板失败", true);
        }
    }
}

/// 将单个 crossterm Event 分发到对应的 handler / Action。
/// 返回 true 表示应退出主循环。
fn dispatch_event(
    app: &mut ChatApp,
    evt: Event,
    needs_redraw: &mut bool,
    mouse_capture_enabled: &mut bool,
) -> bool {
    match evt {
        Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
            // Ctrl+M: 切换鼠标捕获（滚动模式/选择模式）
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('m') {
                *mouse_capture_enabled = !*mouse_capture_enabled;
                if *mouse_capture_enabled {
                    let _ = execute!(io::stdout(), event::EnableMouseCapture);
                    app.show_toast("鼠标: 滚轮滚动 (Shift+拖拽可选中)", false);
                } else {
                    let _ = execute!(io::stdout(), event::DisableMouseCapture);
                    app.show_toast("鼠标: 自由选中 (Ctrl+M 切回滚轮)", false);
                }
                *needs_redraw = true;
                return false;
            }
            *needs_redraw = true;

            // 右键菜单快捷键拦截（最高优先级）
            if app.ui.context_menu.is_some() {
                match key.code {
                    KeyCode::Enter => {
                        execute_context_menu_copy(app);
                        app.ui.context_menu = None;
                        return false;
                    }
                    KeyCode::Esc => {
                        app.ui.context_menu = None;
                        return false;
                    }
                    _ => {}
                }
            }

            // 选区快捷键：c 复制、Esc 取消（优先级高于模式分发）
            if app.ui.mouse_selection.is_some() {
                match key.code {
                    KeyCode::Char('c') => {
                        // 根据当前模式选择对应的缓存
                        let text = if matches!(app.ui.mode, ChatMode::Help) {
                            app.ui
                                .help_lines_cache
                                .as_ref()
                                .zip(app.ui.mouse_selection.as_ref())
                                .map(|(cached, sel)| {
                                    help_extract_selection_text(cached, sel.anchor, sel.current)
                                })
                                .unwrap_or_default()
                        } else {
                            app.ui
                                .msg_lines_cache
                                .as_ref()
                                .zip(app.ui.mouse_selection.as_ref())
                                .map(|(cached, sel)| {
                                    extract_selection_text(cached, sel.anchor, sel.current)
                                })
                                .unwrap_or_default()
                        };
                        if !text.is_empty() {
                            if copy_to_clipboard(&text) {
                                app.show_toast("已复制到剪贴板", false);
                            } else {
                                app.show_toast("复制到剪贴板失败", true);
                            }
                        }
                        app.ui.mouse_selection = None;
                        return false;
                    }
                    KeyCode::Esc => {
                        app.ui.mouse_selection = None;
                        return false;
                    }
                    _ => {}
                }
            }

            match app.ui.mode {
                ChatMode::Chat => {
                    if handle_chat_mode(app, key) {
                        return true; // quit
                    }
                }
                ChatMode::SelectModel => handle_select_model(app, key),
                ChatMode::SelectTheme => handle_select_theme(app, key),
                ChatMode::Browse => handle_browse_mode(app, key),
                ChatMode::Help => {
                    // Help 模式下支持滚动（复制已由全局选区处理）
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.ui.help_scroll_offset = app.ui.help_scroll_offset.saturating_sub(1);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.ui.help_scroll_offset = app.ui.help_scroll_offset.saturating_add(1);
                        }
                        KeyCode::PageUp => {
                            app.ui.help_scroll_offset =
                                app.ui.help_scroll_offset.saturating_sub(10);
                        }
                        KeyCode::PageDown => {
                            app.ui.help_scroll_offset =
                                app.ui.help_scroll_offset.saturating_add(10);
                        }
                        _ => {
                            // 任意其他键退出帮助
                            app.ui.mouse_selection = None;
                            app.update(Action::ExitToChat);
                        }
                    }
                }
                ChatMode::Config => handle_config_mode(app, key),
                ChatMode::ArchiveConfirm => handle_archive_confirm_mode(app, key),
                ChatMode::ArchiveList => handle_archive_list_mode(app, key),
                ChatMode::ToolConfirm => handle_tool_confirm_mode(app, key),
                ChatMode::AgentPermConfirm => handle_agent_perm_confirm_mode(app, key),
                ChatMode::PlanApprovalConfirm => handle_plan_approval_confirm_mode(app, key),
            }
            false
        }
        Event::Paste(text) => {
            // 粘贴事件：逐字符插入到输入框（保留换行）
            if matches!(app.ui.mode, ChatMode::Chat) {
                for c in text.chars() {
                    if c == '\r' {
                        continue; // 忽略 \r，统一用 \n 换行
                    }
                    if c == '\n' {
                        app.ui.input_buffer.insert_newline();
                    } else {
                        app.ui.input_buffer.insert_char(c);
                    }
                }
                *needs_redraw = true;
            } else if matches!(app.ui.mode, ChatMode::Config) && app.ui.config_editing {
                for c in text.chars() {
                    if c == '\n' || c == '\r' {
                        continue; // 忽略换行，配置字段为单行
                    }
                    app.update(Action::ConfigEditChar(c));
                }
                *needs_redraw = true;
            } else if matches!(app.ui.mode, ChatMode::ToolConfirm) {
                // Ask 选项模式下直接粘贴：与按 Char 键行为一致，自动切到自由输入行
                if app.ui.tool_ask_mode
                    && !app.ui.tool_interact_typing
                    && let Some(cur_q) = app.ui.tool_ask_questions.get(app.ui.tool_ask_current_idx)
                {
                    app.ui.tool_ask_cursor = cur_q.options.len();
                    app.ui.tool_interact_typing = true;
                    app.ui.tool_interact_input.clear();
                    app.ui.tool_interact_cursor = 0;
                }
                // Ask 自由输入 / 工具拒绝原因输入 的粘贴支持
                if app.ui.tool_interact_typing {
                    for c in text.chars() {
                        if c == '\r' {
                            continue;
                        }
                        if c == '\n' && !app.ui.tool_ask_mode {
                            continue; // 工具拒绝原因：单行，忽略换行
                        }
                        if app.ui.tool_ask_mode {
                            app.update(Action::AskInputChar(c));
                        } else {
                            app.update(Action::ToolInteractInputChar(c));
                        }
                    }
                    *needs_redraw = true;
                }
            }
            false
        }
        Event::Resize(_, _) => {
            *needs_redraw = true;
            false
        }
        Event::Mouse(mouse) if *mouse_capture_enabled => match mouse.kind {
            MouseEventKind::ScrollUp => {
                app.update(mouse_scroll_action(app, CursorDirection::Up));
                *needs_redraw = true;
                false
            }
            MouseEventKind::ScrollDown => {
                app.update(mouse_scroll_action(app, CursorDirection::Down));
                *needs_redraw = true;
                false
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // 右键菜单激活时：点击菜单内执行复制，点击菜单外关闭菜单
                if app.ui.context_menu.is_some() {
                    if is_point_in_menu(app, mouse.column, mouse.row) {
                        execute_context_menu_copy(app);
                    }
                    app.ui.context_menu = None;
                    *needs_redraw = true;
                    return false;
                }

                // ── Config 模式：Tab 切换 + 列表项选中 ──
                if matches!(app.ui.mode, ChatMode::Config) {
                    if let Some(action) = config_mouse_click(app, mouse.column, mouse.row) {
                        app.update(action);
                        *needs_redraw = true;
                    }
                    return false;
                }

                // ── Help 模式：选区 ──
                if matches!(app.ui.mode, ChatMode::Help) {
                    if let Some(inner) = app.ui.help_area_inner
                        && let Some(ref cached) = app.ui.help_lines_cache
                        && let Some((gline, coff)) = help_screen_to_text_pos(
                            mouse.column,
                            mouse.row,
                            inner,
                            app.ui.help_scroll_offset,
                            cached,
                        )
                    {
                        app.ui.mouse_selection = Some(MouseSelection {
                            anchor: (gline, coff),
                            current: (gline, coff),
                        });
                        *needs_redraw = true;
                    }
                    return false;
                }

                // 点击消息区域：开始选择
                // 点击空白区域：清除选区
                if let Some(inner) = app.ui.msg_area_inner
                    && let Some(ref cached) = app.ui.msg_lines_cache
                    && let Some((gline, coff)) = screen_to_text_pos(
                        mouse.column,
                        mouse.row,
                        inner,
                        app.ui.scroll_offset,
                        cached,
                    )
                {
                    app.ui.mouse_selection = Some(MouseSelection {
                        anchor: (gline, coff),
                        current: (gline, coff),
                    });
                    *needs_redraw = true;
                } else {
                    // 点击空白区域：清除选区
                    if app.ui.mouse_selection.is_some() {
                        app.ui.mouse_selection = None;
                        *needs_redraw = true;
                    }
                }
                false
            }
            MouseEventKind::Down(MouseButton::Right) => {
                // 右键点击消息区域：弹出复制菜单
                if let Some(inner) = app.ui.msg_area_inner
                    && let Some(ref cached) = app.ui.msg_lines_cache
                    && let Some((gline, _)) = screen_to_text_pos(
                        mouse.column,
                        mouse.row,
                        inner,
                        app.ui.scroll_offset,
                        cached,
                    )
                {
                    app.ui.context_menu = Some(ContextMenu {
                        global_line: gline,
                        screen_pos: (mouse.column, mouse.row),
                    });
                    *needs_redraw = true;
                } else {
                    // 右键点击空白区域：关闭已有菜单
                    if app.ui.context_menu.is_some() {
                        app.ui.context_menu = None;
                        *needs_redraw = true;
                    }
                }
                false
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                // Help 模式：拖拽更新选区
                if matches!(app.ui.mode, ChatMode::Help) {
                    if let Some(inner) = app.ui.help_area_inner
                        && let Some(ref cached) = app.ui.help_lines_cache
                        && let Some((gline, coff)) = help_screen_to_text_pos(
                            mouse.column,
                            mouse.row,
                            inner,
                            app.ui.help_scroll_offset,
                            cached,
                        )
                        && let Some(ref mut sel) = app.ui.mouse_selection
                    {
                        sel.current = (gline, coff);
                        *needs_redraw = true;
                    }
                    return false;
                }
                // 拖拽：更新选区终点
                if let Some(inner) = app.ui.msg_area_inner
                    && let Some(ref cached) = app.ui.msg_lines_cache
                    && let Some((gline, coff)) = screen_to_text_pos(
                        mouse.column,
                        mouse.row,
                        inner,
                        app.ui.scroll_offset,
                        cached,
                    )
                    && let Some(ref mut sel) = app.ui.mouse_selection
                {
                    sel.current = (gline, coff);
                    *needs_redraw = true;
                }
                false
            }
            MouseEventKind::Up(MouseButton::Left) => {
                // 松手：如果选区范围为空（无拖拽），清除选区
                // 避免空选区残留导致 Esc 被消费（需要按两次才能退出）
                if let Some(ref sel) = app.ui.mouse_selection
                    && sel.anchor == sel.current
                {
                    app.ui.mouse_selection = None;
                }
                false
            }
            _ => false,
        },
        _ => false,
    }
}

/// 恢复终端状态（仅用于 panic hook）。
/// panic 发生时 `TerminalGuard` 也会 Drop 恢复，此处作为双重保险。
fn restore_terminal() {
    let _ = terminal::disable_raw_mode();
    let mut stdout = io::stdout();
    let _ = restore_terminal_state(&mut stdout, true);
}

/// Chat TUI 入口函数：初始化 panic hook，按需启动远程 WS 服务，然后进入主循环
pub fn run_chat_tui(remote_mode: bool, port: u16) {
    // 注入 Hook 帮助文档到 j-cli-core（供 RegisterHookTool 使用）
    if let Some(asset) = crate::assets::Assets::get("help/hook.md") {
        let content = String::from_utf8_lossy(&asset.data).into_owned();
        crate::command::chat::tools::hook::set_hook_help_content(content);
    }

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
        // TerminalGuard 的 Drop 已恢复终端状态，此处仅打印错误
        error!("✖️ Chat TUI 启动失败: {}", e);
    }
}

/// 生成本次会话 ID（委托给 storage 模块）
fn generate_session_id() -> String {
    crate::command::chat::storage::generate_session_id()
}

/// Chat TUI 主循环：初始化终端、会话状态，持续处理事件轮询、后台任务和渲染
pub fn run_chat_tui_internal(ws_bridge: Option<WsBridge>) -> io::Result<()> {
    // RAII guard：异常退出（panic / `?` 传播）时自动恢复终端状态
    let mut guard = TerminalGuard::new();

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        event::EnableMouseCapture,
        event::EnableBracketedPaste
    )?;
    // 启用 kitty keyboard protocol，使终端能区分 Shift+Enter / Ctrl+Enter 等组合键。
    // 不支持该协议时继续运行，仅降级为基础按键行为。
    if try_enable_keyboard_enhancement(&mut stdout) {
        guard.set_keyboard_active();
    }

    let mut mouse_capture_enabled = true;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

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
        && let Some(latest_id) = crate::command::chat::storage::find_latest_session_id()
    {
        let messages = crate::command::chat::storage::load_session(&latest_id);
        if !messages.is_empty() {
            app.session_id = latest_id;
            // 重建双通道（从加载的消息 → display + context）
            app.rebuild_channels_from_loaded(messages);
            // 恢复 session 状态（tasks/todos/skills/hooks/teammates 等）
            app.restore_session_state();
            app.ui.scroll_offset = usize::MAX; // 滚动到底部
            app.ui.msg_lines_cache = None;
        }
    }

    // 首次运行（尚未配置 provider）时，自动进入配置界面引导用户完成配置
    if app.state.agent_config.providers.is_empty() {
        use crate::command::chat::render::theme::ThemeName;
        use crate::command::chat::storage::{
            AgentConfig, ModelProvider, agent_config_path, save_agent_config,
        };
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
                max_context_tokens: 0,
                theme: ThemeName::default(),
                tools_enabled: false,
                max_tool_rounds: 10,
                style: None,
                tool_confirm_timeout: 0,
                disabled_tools: Vec::new(),
                deferred_tools: Vec::new(),
                disabled_skills: Vec::new(),
                disabled_commands: Vec::new(),
                disabled_hooks: Vec::new(),
                compact: Default::default(),
                auto_restore_session: false,
                flat_bubble: true,
                thinking_style: Default::default(),
                welcome_quote: true,
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

        // Phase 2b: 轮询子 agent 权限请求队列
        // 只在没有当前待决请求且处于 Chat 模式时弹出（不打断 ToolConfirm 等交互）
        if app.ui.pending_agent_perm.is_none()
            && matches!(app.ui.mode, ChatMode::Chat)
            && let Some(req) = app.permission_queue.pop_pending()
        {
            if app.ui.auto_approve {
                // bypass 模式：自动批准
                req.resolve(true);
            } else {
                app.ui.pending_agent_perm = Some(req);
                app.ui.mode = ChatMode::AgentPermConfirm;
                app.ui.msg_lines_cache = None;
                needs_redraw = true;
            }
        }

        // Phase 2b2: 轮询 Teammate Plan 审批请求队列
        if app.ui.pending_plan_approval.is_none()
            && matches!(app.ui.mode, ChatMode::Chat)
            && let Some(req) = app.plan_approval_queue.pop_pending()
        {
            if app.ui.auto_approve {
                // bypass 模式：自动批准
                req.resolve(PlanDecision::Approve);
            } else {
                app.ui.pending_plan_approval = Some(req);
                app.ui.mode = ChatMode::PlanApprovalConfirm;
                app.ui.msg_lines_cache = None;
                needs_redraw = true;
            }
        }

        // Phase 2c: main agent 空闲时，检测 teammate 唤醒信号并触发新 agent loop
        // teammate 通过 broadcast 向 main_agent_inbox 注入轻量唤醒信号，该 Arc 与 pending_user_messages 共享。
        // 广播内容已通过 push_both 写入双通道。
        // 如果 main agent 已结束 agent loop（is_loading=false），inbox 中的信号无人消费，
        // 需要在此唤醒 main agent 响应 teammate 消息。
        if !app.state.is_loading {
            let has_inbox =
                !safe_lock(&app.state.pending_user_messages, "tui_loop::inbox_check").is_empty();
            if has_inbox {
                app.wake_from_teammate_inbox();
                needs_redraw = true;
            }
        }

        // Phase 2d: 收集 WebSocket 远程消息
        if app.ws_bridge.is_some() {
            // 取出 ws_bridge 来避免借用冲突
            // is_some() 已在上方判断，take() 必返回 Some
            let mut ws = app
                .ws_bridge
                .take()
                .expect("ws_bridge checked is_some() above");
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
                        app.inject_remote_message(&content);
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
                    WsInbound::SelectModel { index } => {
                        app.ui.model_list_state.select(Some(index));
                        app.update(Action::ModelSelectConfirm);
                    }
                    WsInbound::SelectTheme { index } => {
                        app.ui.theme_list_state.select(Some(index));
                        app.update(Action::ThemeSelectConfirm);
                    }
                    WsInbound::RequestConfig { tab } => {
                        let config_tab = match tab.as_str() {
                            "session" => ConfigTab::Session,
                            "global" => ConfigTab::Global,
                            "tools" => ConfigTab::Tools,
                            "skills" => ConfigTab::Skills,
                            "hooks" => ConfigTab::Hooks,
                            "commands" => ConfigTab::Commands,
                            "teammates" => ConfigTab::Teammates,
                            "archive" => ConfigTab::Archive,
                            _ => ConfigTab::Model,
                        };
                        app.update(Action::ConfigSwitchTabTo(config_tab));
                        app.broadcast_config_state();
                    }
                    WsInbound::ConfigEditSubmit { value } => {
                        // 直接写入当前编辑字段并提交
                        app.ui.config_edit_buf = value.clone();
                        app.ui.config_edit_cursor = value.chars().count();
                        app.update(Action::ConfigEditSubmit);
                        app.broadcast_config_state();
                    }
                    WsInbound::ConfigToggle { index } => {
                        // 远程 toggle：根据当前 tab 直接操作配置
                        match app.ui.config_tab {
                            ConfigTab::Tools => {
                                let all_tools: Vec<String> = app
                                    .tool_registry
                                    .tool_names()
                                    .into_iter()
                                    .map(|s| s.to_string())
                                    .collect();
                                if let Some(name) = all_tools.get(index) {
                                    if app.state.agent_config.disabled_tools.contains(name) {
                                        app.state.agent_config.disabled_tools.retain(|n| n != name);
                                    } else {
                                        app.state.agent_config.disabled_tools.push(name.clone());
                                    }
                                    let _ = crate::command::chat::storage::save_agent_config(
                                        &app.state.agent_config,
                                    );
                                }
                                app.broadcast_config_state();
                            }
                            ConfigTab::Skills => {
                                let names: Vec<String> = app
                                    .state
                                    .loaded_skills
                                    .iter()
                                    .map(|s| s.frontmatter.name.clone())
                                    .collect();
                                if let Some(name) = names.get(index) {
                                    if app.state.agent_config.disabled_skills.contains(name) {
                                        app.state
                                            .agent_config
                                            .disabled_skills
                                            .retain(|n| n != name);
                                    } else {
                                        app.state.agent_config.disabled_skills.push(name.clone());
                                    }
                                    let _ = crate::command::chat::storage::save_agent_config(
                                        &app.state.agent_config,
                                    );
                                }
                                app.broadcast_config_state();
                            }
                            ConfigTab::Global => {
                                // 远程切换全局布尔设置
                                let fields =
                                    ["tools_enabled", "auto_restore_session", "flat_bubble"];
                                if index < fields.len() {
                                    match fields[index] {
                                        "tools_enabled" => {
                                            app.state.agent_config.tools_enabled =
                                                !app.state.agent_config.tools_enabled
                                        }
                                        "auto_restore_session" => {
                                            app.state.agent_config.auto_restore_session =
                                                !app.state.agent_config.auto_restore_session
                                        }
                                        "flat_bubble" => {
                                            app.state.agent_config.flat_bubble =
                                                !app.state.agent_config.flat_bubble
                                        }
                                        _ => {}
                                    }
                                    let _ = crate::command::chat::storage::save_agent_config(
                                        &app.state.agent_config,
                                    );
                                }
                                app.broadcast_config_state();
                            }
                            _ => {}
                        }
                    }
                    WsInbound::StartArchive => {
                        app.start_archive_confirm();
                        app.broadcast_archive_confirm_state();
                    }
                    WsInbound::ArchiveWithDefault => {
                        app.do_archive(&app.ui.archive_default_name.clone());
                        let sync = app.build_sync_outbound();
                        app.broadcast_ws(sync);
                    }
                    WsInbound::ArchiveWithCustom { name } => {
                        app.do_archive(&name);
                        let sync = app.build_sync_outbound();
                        app.broadcast_ws(sync);
                    }
                    WsInbound::ClearSession => {
                        app.clear_session();
                        let sync = app.build_sync_outbound();
                        app.broadcast_ws(sync);
                    }
                    WsInbound::StartArchiveList => {
                        app.start_archive_list();
                        app.broadcast_archive_list_state();
                    }
                    WsInbound::RestoreArchive { index } => {
                        app.ui.archive_list_index = index;
                        app.do_restore();
                        let sync = app.build_sync_outbound();
                        app.broadcast_ws(sync);
                    }
                    WsInbound::DeleteArchive { index } => {
                        app.ui.archive_list_index = index;
                        app.do_delete_archive();
                        app.broadcast_archive_list_state();
                    }
                    WsInbound::DeleteSession { index } => {
                        if index < app.ui.session_list.len() {
                            app.ui.session_list_index = index;
                            app.update(Action::DeleteSession);
                            app.broadcast_session_list_state();
                        }
                    }
                    WsInbound::AgentPermConfirm { approve } => {
                        if let Some(req) = app.ui.pending_agent_perm.take() {
                            req.resolve(approve);
                        }
                        app.ui.mode = ChatMode::Chat;
                        app.ui.msg_lines_cache = None;
                    }
                    WsInbound::PlanApproval { approve, content } => {
                        use crate::command::chat::app::types::PlanDecision;
                        if let Some(req) = app.ui.pending_plan_approval.take() {
                            let decision = if approve {
                                match content.as_deref() {
                                    Some("clear") => PlanDecision::ApproveAndClearContext,
                                    _ => PlanDecision::Approve,
                                }
                            } else {
                                PlanDecision::Reject
                            };
                            req.resolve(decision);
                        }
                        app.ui.mode = ChatMode::Chat;
                        app.ui.msg_lines_cache = None;
                    }
                    WsInbound::ToggleAutoApprove => {
                        app.update(Action::ToggleAutoApprove);
                    }
                    // ── 文件操作 ──
                    WsInbound::FileList { path } => {
                        let entries = ChatApp::handle_file_list(&path);
                        app.broadcast_ws(WsOutbound::FileListResult { path, entries });
                    }
                    WsInbound::FileRead { path } => {
                        let (content, error) = ChatApp::handle_file_read(&path);
                        app.broadcast_ws(WsOutbound::FileReadResult {
                            path,
                            content,
                            error,
                        });
                    }
                    WsInbound::FileWrite { path, content } => {
                        let (success, error) = ChatApp::handle_file_write(&path, &content);
                        app.broadcast_ws(WsOutbound::FileWriteResult {
                            path,
                            success,
                            error,
                        });
                    }
                    // ── 终端操作 ──
                    WsInbound::TerminalExec { command } => {
                        let (output, exit_code) = ChatApp::handle_terminal_exec(&command);
                        app.broadcast_ws(WsOutbound::TerminalOutput { output, exit_code });
                    }
                    WsInbound::TerminalInterrupt => {
                        // 终端中断暂不实现（需要进程管理）
                    }
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
            std::time::Duration::from_millis(TUI_LOADING_POLL_MS)
        } else if app.ui.mode == ChatMode::ToolConfirm {
            std::time::Duration::from_millis(TUI_IDLE_POLL_MS)
        } else {
            std::time::Duration::from_millis(TUI_IDLE_POLL_MS)
        };

        // 阻塞等待第一个事件（受 poll_timeout 限制）
        let first = input_thread.rx.recv_timeout(poll_timeout);
        if let Ok(evt) = first {
            let mut should_quit =
                dispatch_event(&mut app, evt, &mut needs_redraw, &mut mouse_capture_enabled);
            // 批量消费所有已缓冲的后续事件（非阻塞）
            if !should_quit {
                while let Ok(evt) = input_thread.rx.try_recv() {
                    if dispatch_event(&mut app, evt, &mut needs_redraw, &mut mouse_capture_enabled)
                    {
                        should_quit = true;
                        break;
                    }
                }
            }
            if should_quit {
                break;
            }

            // 事件处理后若状态改变，立即渲染（避免因 poll_timeout 阻塞导致粘贴等操作延迟显示）
            if needs_redraw {
                terminal.draw(|f| draw_chat_ui(f, &mut app))?;
                needs_redraw = false;
                last_render_time = std::time::Instant::now();
                // 更新流式节流状态
                if app.state.is_loading {
                    app.ui.last_rendered_streaming_len =
                        safe_lock(&app.state.streaming_content, "tui_loop::immediate_render").len();
                    app.ui.last_stream_render_time = std::time::Instant::now();
                }
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
                match crate::tui::editor_markdown::open_markdown_editor_on_terminal(
                    &mut terminal,
                    "编辑系统提示词 (System Prompt)",
                    &current_prompt,
                    &app.ui.theme,
                ) {
                    Ok((Some(new_text), _)) => {
                        if save_system_prompt(&new_text) {
                            app.update(Action::ShowToast("系统提示词已更新".to_string(), false));
                        } else {
                            app.update(Action::ShowToast("系统提示词保存失败".to_string(), true));
                        }
                    }
                    Ok((None, _)) => {}
                    Err(e) => {
                        app.update(Action::ShowToast(format!("编辑器错误: {}", e), true));
                    }
                }
                // 恢复输入线程，清空编辑器期间可能产生的残留事件
                input_thread.drain();
                input_thread.resume();
                needs_redraw = true;
            }

            if app.ui.pending_agent_md_edit {
                app.ui.pending_agent_md_edit = false;
                input_thread.pause();
                input_thread.drain();
                let current_agent_md =
                    std::fs::read_to_string(agent_md::agent_md_path()).unwrap_or_default();
                match crate::tui::editor_markdown::open_markdown_editor_on_terminal(
                    &mut terminal,
                    "编辑项目指令 (AGENTS.md)",
                    &current_agent_md,
                    &app.ui.theme,
                ) {
                    Ok((Some(new_text), _)) => {
                        let path = agent_md::agent_md_path();
                        if let Some(parent) = path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        match std::fs::write(&path, &new_text) {
                            Ok(_) => {
                                app.update(Action::ShowToast("项目指令已更新".to_string(), false));
                            }
                            Err(_) => {
                                app.update(Action::ShowToast("项目指令保存失败".to_string(), true));
                            }
                        }
                    }
                    Ok((None, _)) => {}
                    Err(e) => {
                        app.update(Action::ShowToast(format!("编辑器错误: {}", e), true));
                    }
                }
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
                match crate::tui::editor_markdown::open_markdown_editor_on_terminal(
                    &mut terminal,
                    "编辑回复风格 (Style)",
                    &current_style,
                    &app.ui.theme,
                ) {
                    Ok((Some(new_text), _)) => {
                        if save_style(&new_text) {
                            app.update(Action::ShowToast("回复风格已更新".to_string(), false));
                        } else {
                            app.update(Action::ShowToast("回复风格保存失败".to_string(), true));
                        }
                    }
                    Ok((None, _)) => {}
                    Err(e) => {
                        app.update(Action::ShowToast(format!("编辑器错误: {}", e), true));
                    }
                }
                // 恢复输入线程，清空编辑器期间可能产生的残留事件
                input_thread.drain();
                input_thread.resume();
                needs_redraw = true;
            }

            if app.ui.pending_command_create {
                app.ui.pending_command_create = false;
                input_thread.pause();
                input_thread.drain();

                use crate::command::chat::infra::command::CommandSource;
                let source = app.ui.command_create_source;
                let title = match source {
                    CommandSource::User => "创建命令 (用户级)",
                    CommandSource::Project => "创建命令 (项目级)",
                };
                let template = concat!(
                    "---\n",
                    "name: my-command\n",
                    "description: 命令描述\n",
                    "---\n",
                    "\n",
                    "# 命令内容\n",
                    "\n",
                    "在这里编写命令的提示词正文...\n",
                );

                match crate::tui::editor_markdown::open_markdown_editor_on_terminal(
                    &mut terminal,
                    title,
                    template,
                    &app.ui.theme,
                ) {
                    Ok((Some(new_text), _)) => {
                        match crate::command::chat::infra::command::save_new_command(
                            source, &new_text,
                        ) {
                            Ok((path, name)) => {
                                app.state.loaded_commands =
                                    crate::command::chat::infra::command::load_all_commands();
                                app.update(Action::ShowToast(
                                    format!("命令 '{}' 已创建: {}", name, path.display()),
                                    false,
                                ));
                            }
                            Err(e) => {
                                app.update(Action::ShowToast(format!("创建命令失败: {}", e), true));
                            }
                        }
                    }
                    Ok((None, _)) => {}
                    Err(e) => {
                        app.update(Action::ShowToast(format!("编辑器错误: {}", e), true));
                    }
                }

                input_thread.drain();
                input_thread.resume();
                needs_redraw = true;
            }
        }
    }

    // 停止输入线程
    input_thread.shutdown();

    // ★ 保存会话状态（非空会话才保存）
    let is_empty = safe_lock(&app.display_messages, "tui_exit::empty").is_empty();
    if !is_empty {
        app.save_session_state();
    }

    // ★ 空会话不保存：删除无消息的 session 文件
    if is_empty {
        crate::command::chat::storage::delete_session(&app.session_id);
    }

    // ★ 先恢复终端，再跑 SessionEnd hook（避免 hook 阻塞时终端卡在 raw mode）
    terminal::disable_raw_mode()?;
    restore_terminal_state(terminal.backend_mut(), guard.keyboard_enhancement_active)?;
    guard.disarm(); // 已手动恢复，阻止 Drop 重复执行

    // ★ SessionEnd hook（fire-and-forget，终端已恢复）
    {
        let has_hooks = app
            .hook_manager
            .lock()
            .map(|m| m.has_hooks_for(HookEvent::SessionEnd))
            .unwrap_or(false);
        if has_hooks {
            let ctx = HookContext {
                event: HookEvent::SessionEnd,
                messages: Some(safe_lock(&app.context_messages, "SessionEnd::ctx_msgs").clone()),
                session_id: Some(app.session_id.clone()),
                cwd: std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| ".".to_string()),
                ..Default::default()
            };
            HookManager::execute_fire_and_forget(
                std::sync::Arc::clone(&app.hook_manager),
                HookEvent::SessionEnd,
                ctx,
                app.state.agent_config.disabled_hooks.clone(),
            );
        }
    }

    Ok(())
}
