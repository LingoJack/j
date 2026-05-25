//! 事件分发逻辑

use crate::command::chat::app::{Action, ChatApp, ChatMode, CursorDirection};
use crate::command::chat::app::{ContextMenu, MouseSelection};
use crate::command::chat::render::cache::copy_to_clipboard;
use crate::command::chat::ui::chat::{extract_selection_text, screen_to_text_pos};
use crate::command::chat::ui::context_menu::is_point_in_menu;
use crate::command::chat::ui::help::{help_extract_selection_text, help_screen_to_text_pos};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind},
    execute,
};
use std::io;

use super::super::{
    handle_agent_perm_confirm_mode, handle_archive_confirm_mode, handle_archive_list_mode,
    handle_browse_mode, handle_chat_mode, handle_config_mode, handle_plan_approval_confirm_mode,
    handle_select_model, handle_select_theme, handle_tool_confirm_mode,
};
use super::mouse::{config_mouse_click, execute_context_menu_copy, mouse_scroll_action};

/// 将单个 crossterm Event 分发到对应的 handler / Action。
/// 返回 true 表示应退出主循环。
pub(super) fn dispatch_event(
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
                        } else if matches!(app.ui.mode, ChatMode::Config) {
                            app.ui
                                .config_lines_cache
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

                // ── Config 模式：Tab 切换 + 列表项选中 + 选区 ──
                if matches!(app.ui.mode, ChatMode::Config) {
                    // 先检查是否在内容区域内（用于选区）
                    if let Some(inner) = app.ui.config_content_inner
                        && let Some(ref cached) = app.ui.config_lines_cache
                        && let Some((gline, coff)) = help_screen_to_text_pos(
                            mouse.column,
                            mouse.row,
                            inner,
                            app.ui.config_content_scroll as usize,
                            cached,
                        )
                    {
                        app.ui.mouse_selection = Some(MouseSelection {
                            anchor: (gline, coff),
                            current: (gline, coff),
                        });
                        *needs_redraw = true;
                    } else if let Some(action) = config_mouse_click(app, mouse.column, mouse.row) {
                        // 不在内容区域：走原来的 Tab 切换 / 列表选中逻辑
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
                // Config 模式：拖拽更新选区
                if matches!(app.ui.mode, ChatMode::Config) {
                    if let Some(inner) = app.ui.config_content_inner
                        && let Some(ref cached) = app.ui.config_lines_cache
                        && let Some((gline, coff)) = help_screen_to_text_pos(
                            mouse.column,
                            mouse.row,
                            inner,
                            app.ui.config_content_scroll as usize,
                            cached,
                        )
                        && let Some(ref mut sel) = app.ui.mouse_selection
                    {
                        sel.current = (gline, coff);
                        *needs_redraw = true;
                    }
                    return false;
                }
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
