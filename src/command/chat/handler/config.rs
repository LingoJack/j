use super::super::storage::save_agent_config;
use crate::command::chat::app::{Action, ChatApp, ConfigTab, CursorDirection};
use crossterm::event::{KeyCode, KeyEvent};

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
            KeyCode::Left => Action::ConfigEditMoveCursor(CursorDirection::Up),
            KeyCode::Right => Action::ConfigEditMoveCursor(CursorDirection::Down),
            KeyCode::Char(c) => Action::ConfigEditChar(c),
            _ => return,
        };
        app.update(action);
        return;
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
        ConfigTab::Global => match key.code {
            KeyCode::Esc => Action::SaveConfig,
            KeyCode::Left => Action::ConfigSwitchTab(CursorDirection::Up),
            KeyCode::Right => Action::ConfigSwitchTab(CursorDirection::Down),
            KeyCode::Up | KeyCode::Char('k') => Action::ConfigNavigate(CursorDirection::Up),
            KeyCode::Down | KeyCode::Char('j') => Action::ConfigNavigate(CursorDirection::Down),
            KeyCode::Enter => Action::ConfigEnter,
            _ => return,
        },
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
        ConfigTab::Hooks | ConfigTab::Commands => match key.code {
            KeyCode::Esc => Action::SaveConfig,
            KeyCode::Left => Action::ConfigSwitchTab(CursorDirection::Up),
            KeyCode::Right => Action::ConfigSwitchTab(CursorDirection::Down),
            _ => return,
        },
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
