use super::super::storage::save_agent_config;
use crate::command::chat::app::{Action, ChatApp, ChatMode, CursorDirection};
use crossterm::event::{KeyCode, KeyEvent};

/// 配置模式按键处理
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

    let action = match key.code {
        KeyCode::Esc => Action::SaveConfig,
        KeyCode::Up | KeyCode::Char('k') => Action::ConfigNavigate(CursorDirection::Up),
        KeyCode::Down | KeyCode::Char('j') => Action::ConfigNavigate(CursorDirection::Down),
        KeyCode::Tab | KeyCode::Right => Action::ConfigSwitchProvider(CursorDirection::Down),
        KeyCode::BackTab | KeyCode::Left => Action::ConfigSwitchProvider(CursorDirection::Up),
        KeyCode::Enter => Action::ConfigEnter,
        KeyCode::Char('a') => Action::ConfigAddProvider,
        KeyCode::Char('d') => Action::ConfigDeleteProvider,
        KeyCode::Char('s') => Action::ConfigSetActiveProvider,
        _ => return,
    };
    app.update(action);
}

/// 工具开关子菜单按键处理
pub fn handle_tool_toggle_mode(app: &mut ChatApp, key: KeyEvent) {
    let tool_names = app.tool_registry.tool_names();
    let total = tool_names.len();
    if total == 0 {
        app.ui.mode = ChatMode::Config;
        return;
    }

    let action = match key.code {
        KeyCode::Esc => {
            save_agent_config(&app.state.agent_config);
            Action::EnterMode(ChatMode::Config)
        }
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
    };
    app.update(action);
}

pub fn handle_skill_toggle_mode(app: &mut ChatApp, key: KeyEvent) {
    let total = app.state.loaded_skills.len();
    if total == 0 {
        app.ui.mode = ChatMode::Config;
        return;
    }

    let action = match key.code {
        KeyCode::Esc => {
            save_agent_config(&app.state.agent_config);
            Action::EnterMode(ChatMode::Config)
        }
        KeyCode::Up | KeyCode::Char('k') => Action::ToggleMenuNavigate(CursorDirection::Up),
        KeyCode::Down | KeyCode::Char('j') => Action::ToggleMenuNavigate(CursorDirection::Down),
        KeyCode::Enter | KeyCode::Char(' ') => Action::ToggleMenuToggle,
        KeyCode::Char('a') => Action::ToggleMenuEnableAll,
        KeyCode::Char('d') => Action::ToggleMenuDisableAll,
        _ => return,
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
