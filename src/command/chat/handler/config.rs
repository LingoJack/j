use super::super::storage::save_agent_config;
use crate::command::chat::app::{Action, ChatApp, ConfigTab, CursorDirection};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// 配置模式按键处理（Tab 感知）
pub fn handle_config_mode(app: &mut ChatApp, key: KeyEvent) {
    if app.ui.config_editing {
        // 正在编辑某个字段
        let action = match key.code {
            KeyCode::Esc => {
                app.ui.config_editing = false;
                return;
            }
            KeyCode::Enter => Action::ConfigEditSubmit,
            KeyCode::Backspace => Action::ConfigEditDelete,
            KeyCode::Delete => Action::ConfigEditDeleteForward,
            KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    Action::ConfigEditMoveHome
                } else {
                    Action::ConfigEditMoveCursor(CursorDirection::Up)
                }
            }
            KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    Action::ConfigEditMoveEnd
                } else {
                    Action::ConfigEditMoveCursor(CursorDirection::Down)
                }
            }
            KeyCode::Home => Action::ConfigEditMoveHome,
            KeyCode::End => Action::ConfigEditMoveEnd,
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Action::ConfigEditMoveHome
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Action::ConfigEditMoveEnd
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Action::ConfigEditClearLine
            }
            KeyCode::Char(c) => Action::ConfigEditChar(c),
            _ => return,
        };
        app.update(action);
        return;
    }

    // 统一拦截 [ / ] 切换 tab（非编辑模式下）
    match key.code {
        KeyCode::Char('[') => {
            app.update(Action::ConfigSwitchTab(CursorDirection::Up));
            return;
        }
        KeyCode::Char(']') => {
            app.update(Action::ConfigSwitchTab(CursorDirection::Down));
            return;
        }
        _ => {}
    }

    let action = match app.ui.config_tab {
        ConfigTab::Model => match key.code {
            KeyCode::Esc => Action::SaveConfig,
            KeyCode::Left => Action::ConfigSwitchTab(CursorDirection::Up),
            KeyCode::Right => Action::ConfigSwitchTab(CursorDirection::Down),
            KeyCode::Up | KeyCode::Char('k') => Action::ConfigNavigate(CursorDirection::Up),
            KeyCode::Down | KeyCode::Char('j') => Action::ConfigNavigate(CursorDirection::Down),
            KeyCode::Tab => Action::ConfigSwitchProvider(CursorDirection::Down),
            KeyCode::BackTab => Action::ConfigSwitchProvider(CursorDirection::Up),
            KeyCode::Enter => Action::ConfigEnter,
            KeyCode::Char('a') => Action::ConfigAddProvider,
            KeyCode::Char('d') => Action::ConfigDeleteProvider,
            KeyCode::Char('s') => Action::ConfigSetActiveProvider,
            _ => return,
        },
        ConfigTab::Global => {
            // compact_exempt_tools 子列表模式
            if app.ui.compact_exempt_sublist {
                match key.code {
                    KeyCode::Esc => {
                        app.ui.compact_exempt_sublist = false;
                        app.ui.config_scroll_offset = 0;
                        save_agent_config(&app.state.agent_config);
                        return;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.ui.compact_exempt_idx > 0 {
                            app.ui.compact_exempt_idx -= 1;
                        }
                        return;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let total = app.tool_registry.tool_names().len();
                        if total > 0 && app.ui.compact_exempt_idx < total - 1 {
                            app.ui.compact_exempt_idx += 1;
                        }
                        return;
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        app.update(Action::CompactExemptToggle);
                        return;
                    }
                    _ => return,
                }
            }
            match key.code {
                KeyCode::Esc => Action::SaveConfig,
                KeyCode::Left => Action::ConfigSwitchTab(CursorDirection::Up),
                KeyCode::Right => Action::ConfigSwitchTab(CursorDirection::Down),
                KeyCode::Up | KeyCode::Char('k') => Action::ConfigNavigate(CursorDirection::Up),
                KeyCode::Down | KeyCode::Char('j') => Action::ConfigNavigate(CursorDirection::Down),
                KeyCode::Enter => Action::ConfigEnter,
                _ => return,
            }
        }
        ConfigTab::Tools => match key.code {
            KeyCode::Esc => {
                save_agent_config(&app.state.agent_config);
                Action::SaveConfig
            }
            KeyCode::Left => Action::ConfigSwitchTab(CursorDirection::Up),
            KeyCode::Right => Action::ConfigSwitchTab(CursorDirection::Down),
            KeyCode::Up | KeyCode::Char('k') => Action::ToggleMenuNavigate(CursorDirection::Up),
            KeyCode::Down | KeyCode::Char('j') => Action::ToggleMenuNavigate(CursorDirection::Down),
            KeyCode::Enter | KeyCode::Char(' ') => Action::ToggleMenuToggle,
            KeyCode::Char('a') => Action::ToggleMenuEnableAll,
            KeyCode::Char('d') => Action::ToggleMenuDisableAll,
            KeyCode::Char('t') => {
                // 切换总开关
                app.state.agent_config.tools_enabled = !app.state.agent_config.tools_enabled;
                let status = if app.state.agent_config.tools_enabled {
                    "开启"
                } else {
                    "关闭"
                };
                app.show_toast(format!("工具调用已{}", status), false);
                return;
            }
            _ => return,
        },
        ConfigTab::Skills => match key.code {
            KeyCode::Esc => {
                save_agent_config(&app.state.agent_config);
                Action::SaveConfig
            }
            KeyCode::Left => Action::ConfigSwitchTab(CursorDirection::Up),
            KeyCode::Right => Action::ConfigSwitchTab(CursorDirection::Down),
            KeyCode::Up | KeyCode::Char('k') => Action::ToggleMenuNavigate(CursorDirection::Up),
            KeyCode::Down | KeyCode::Char('j') => Action::ToggleMenuNavigate(CursorDirection::Down),
            KeyCode::Enter | KeyCode::Char(' ') => Action::ToggleMenuToggle,
            KeyCode::Char('a') => Action::ToggleMenuEnableAll,
            KeyCode::Char('d') => Action::ToggleMenuDisableAll,
            _ => return,
        },
        ConfigTab::Hooks => match key.code {
            KeyCode::Esc => {
                save_agent_config(&app.state.agent_config);
                Action::SaveConfig
            }
            KeyCode::Left => Action::ConfigSwitchTab(CursorDirection::Up),
            KeyCode::Right => Action::ConfigSwitchTab(CursorDirection::Down),
            KeyCode::Up | KeyCode::Char('k') => Action::ToggleMenuNavigate(CursorDirection::Up),
            KeyCode::Down | KeyCode::Char('j') => Action::ToggleMenuNavigate(CursorDirection::Down),
            KeyCode::Enter | KeyCode::Char(' ') => Action::ToggleMenuToggle,
            KeyCode::Char('e') => Action::ToggleMenuEnableAll,
            KeyCode::Char('d') => Action::ToggleMenuDisableAll,
            _ => return,
        },
        ConfigTab::Commands => match key.code {
            KeyCode::Esc => {
                save_agent_config(&app.state.agent_config);
                Action::SaveConfig
            }
            KeyCode::Left => Action::ConfigSwitchTab(CursorDirection::Up),
            KeyCode::Right => Action::ConfigSwitchTab(CursorDirection::Down),
            KeyCode::Up | KeyCode::Char('k') => Action::ToggleMenuNavigate(CursorDirection::Up),
            KeyCode::Down | KeyCode::Char('j') => Action::ToggleMenuNavigate(CursorDirection::Down),
            KeyCode::Enter | KeyCode::Char(' ') => Action::ToggleMenuToggle,
            KeyCode::Char('a') => Action::ToggleMenuEnableAll,
            KeyCode::Char('d') => Action::ToggleMenuDisableAll,
            _ => return,
        },
        ConfigTab::Teammates => match key.code {
            KeyCode::Esc => Action::SaveConfig,
            KeyCode::Left => Action::ConfigSwitchTab(CursorDirection::Up),
            KeyCode::Right => Action::ConfigSwitchTab(CursorDirection::Down),
            KeyCode::Up | KeyCode::Char('k') => {
                if app.ui.teammate_list_index > 0 {
                    app.ui.teammate_list_index -= 1;
                }
                return;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let count = app
                    .teammate_manager
                    .lock()
                    .map(|m| m.teammates.len())
                    .unwrap_or(0);
                if count > 0 && app.ui.teammate_list_index < count - 1 {
                    app.ui.teammate_list_index += 1;
                }
                return;
            }
            KeyCode::Char('s') => {
                // 停止选中的 teammate
                let snapshots = app
                    .teammate_manager
                    .lock()
                    .map(|m| m.teammate_snapshots())
                    .unwrap_or_default();
                if let Some(snap) = snapshots.get(app.ui.teammate_list_index) {
                    if !snap.status.is_terminal() {
                        if let Ok(mut manager) = app.teammate_manager.lock() {
                            manager.stop_teammate(&snap.name);
                        }
                        Action::ShowToast(format!("已停止 Teammate: {}", snap.name), false)
                    } else {
                        Action::ShowToast(
                            format!("{} 已处于终态 ({})", snap.name, snap.status.label()),
                            true,
                        )
                    }
                } else {
                    return;
                }
            }
            KeyCode::Enter => {
                // 显示详情 Toast
                let snapshots = app
                    .teammate_manager
                    .lock()
                    .map(|m| m.teammate_snapshots())
                    .unwrap_or_default();
                if let Some(snap) = snapshots.get(app.ui.teammate_list_index) {
                    let tool_info = snap
                        .current_tool
                        .as_deref()
                        .map(|t| format!(", 当前工具: {}", t))
                        .unwrap_or_default();
                    Action::ShowToast(
                        format!(
                            "{} ({}) — {} {}{}",
                            snap.name,
                            snap.role,
                            snap.status.icon(),
                            snap.status.label(),
                            tool_info,
                        ),
                        false,
                    )
                } else {
                    return;
                }
            }
            _ => return,
        },
        ConfigTab::Archive => {
            if app.ui.restore_confirm_needed {
                // 确认还原模式
                match key.code {
                    KeyCode::Char('y') | KeyCode::Enter => {
                        app.ui.restore_confirm_needed = false;
                        Action::RestoreArchive
                    }
                    KeyCode::Esc | KeyCode::Char('n') => {
                        app.ui.restore_confirm_needed = false;
                        return;
                    }
                    _ => return,
                }
            } else {
                match key.code {
                    KeyCode::Esc => Action::SaveConfig,
                    KeyCode::Left => Action::ConfigSwitchTab(CursorDirection::Up),
                    KeyCode::Right => Action::ConfigSwitchTab(CursorDirection::Down),
                    KeyCode::Up | KeyCode::Char('k') => {
                        Action::ArchiveListNavigate(CursorDirection::Up)
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        Action::ArchiveListNavigate(CursorDirection::Down)
                    }
                    KeyCode::Enter => {
                        if app.ui.archives.is_empty() {
                            return;
                        }
                        if !app.state.session.messages.is_empty() {
                            app.ui.restore_confirm_needed = true;
                            return;
                        }
                        Action::RestoreArchive
                    }
                    KeyCode::Char('d') => Action::DeleteArchive,
                    _ => return,
                }
            }
        }
        ConfigTab::Session => {
            if app.ui.session_restore_confirm {
                // 确认恢复模式
                match key.code {
                    KeyCode::Char('y') | KeyCode::Enter => {
                        app.ui.session_restore_confirm = false;
                        Action::RestoreSession
                    }
                    KeyCode::Esc | KeyCode::Char('n') => {
                        app.ui.session_restore_confirm = false;
                        return;
                    }
                    _ => return,
                }
            } else {
                match key.code {
                    KeyCode::Esc => Action::SaveConfig,
                    KeyCode::Left => Action::ConfigSwitchTab(CursorDirection::Up),
                    KeyCode::Right => Action::ConfigSwitchTab(CursorDirection::Down),
                    KeyCode::Up | KeyCode::Char('k') => {
                        Action::SessionListNavigate(CursorDirection::Up)
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        Action::SessionListNavigate(CursorDirection::Down)
                    }
                    KeyCode::Enter => {
                        if app.ui.session_list.is_empty() {
                            return;
                        }
                        if !app.state.session.messages.is_empty() {
                            app.ui.session_restore_confirm = true;
                            return;
                        }
                        Action::RestoreSession
                    }
                    KeyCode::Char('d') => Action::DeleteSession,
                    KeyCode::Char('n') => Action::NewSessionFromList,
                    _ => return,
                }
            }
        }
    };
    app.update(action);
}

/// 模型选择列表按键处理
pub fn handle_select_model(app: &mut ChatApp, key: KeyEvent) {
    let action = match key.code {
        KeyCode::Esc => Action::ExitToChat,
        KeyCode::Up | KeyCode::Char('k') => Action::ModelSelectNavigate(CursorDirection::Up),
        KeyCode::Down | KeyCode::Char('j') => Action::ModelSelectNavigate(CursorDirection::Down),
        KeyCode::Enter => Action::ModelSelectConfirm,
        _ => return,
    };
    app.update(action);
}

/// 主题选择模式按键处理
pub fn handle_select_theme(app: &mut ChatApp, key: KeyEvent) {
    let action = match key.code {
        KeyCode::Esc => Action::ExitToChat,
        KeyCode::Up | KeyCode::Char('k') => Action::ThemeSelectNavigate(CursorDirection::Up),
        KeyCode::Down | KeyCode::Char('j') => Action::ThemeSelectNavigate(CursorDirection::Down),
        KeyCode::Enter => Action::ThemeSelectConfirm,
        _ => return,
    };
    app.update(action);
}
