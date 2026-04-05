use super::app::ChatApp;
use super::storage::{load_style, load_system_prompt, save_style, save_system_prompt};
use super::theme::ThemeName;
use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS_TAB};

// ========== Model tab helpers ==========

pub fn config_field_label_model(idx: usize) -> &'static str {
    if idx >= CONFIG_FIELDS.len() {
        return "";
    }
    match CONFIG_FIELDS[idx] {
        "name" => "显示名称",
        "api_base" => "API Base",
        "api_key" => "API Key",
        "model" => "模型名称",
        "supports_vision" => "支持视觉",
        _ => CONFIG_FIELDS[idx],
    }
}

pub fn config_field_value_model(app: &ChatApp, idx: usize) -> String {
    if idx >= CONFIG_FIELDS.len() {
        return String::new();
    }
    let Some(p) = app
        .state
        .agent_config
        .providers
        .get(app.ui.config_provider_idx)
    else {
        return String::new();
    };
    match CONFIG_FIELDS[idx] {
        "name" => p.name.clone(),
        "api_base" => p.api_base.clone(),
        "api_key" => {
            let chars: Vec<char> = p.api_key.chars().collect();
            if chars.len() > 8 {
                let prefix: String = chars[..4].iter().collect();
                let suffix: String = chars[chars.len() - 4..].iter().collect();
                format!("{}****{}", prefix, suffix)
            } else {
                p.api_key.clone()
            }
        }
        "model" => p.model.clone(),
        "supports_vision" => {
            if p.supports_vision {
                "开启".into()
            } else {
                "关闭".into()
            }
        }
        _ => String::new(),
    }
}

pub fn config_field_raw_value_model(app: &ChatApp, idx: usize) -> String {
    if idx >= CONFIG_FIELDS.len() {
        return String::new();
    }
    let Some(p) = app
        .state
        .agent_config
        .providers
        .get(app.ui.config_provider_idx)
    else {
        return String::new();
    };
    match CONFIG_FIELDS[idx] {
        "name" => p.name.clone(),
        "api_base" => p.api_base.clone(),
        "api_key" => p.api_key.clone(),
        "model" => p.model.clone(),
        "supports_vision" => p.supports_vision.to_string(),
        _ => String::new(),
    }
}

pub fn config_field_set_model(app: &mut ChatApp, idx: usize, value: &str) {
    if idx >= CONFIG_FIELDS.len() {
        return;
    }
    let Some(p) = app
        .state
        .agent_config
        .providers
        .get_mut(app.ui.config_provider_idx)
    else {
        return;
    };
    match CONFIG_FIELDS[idx] {
        "name" => p.name = value.to_string(),
        "api_base" => p.api_base = value.to_string(),
        "api_key" => p.api_key = value.to_string(),
        "model" => p.model = value.to_string(),
        "supports_vision" => {
            p.supports_vision = matches!(
                value.trim().to_lowercase().as_str(),
                "true" | "1" | "开启" | "on" | "yes"
            );
            app.ui.msg_lines_cache = None;
        }
        _ => {}
    }
}

// ========== Global tab helpers ==========

pub fn config_field_label_global(idx: usize) -> &'static str {
    let Some(field_name) = CONFIG_GLOBAL_FIELDS_TAB.get(idx) else {
        return "";
    };
    match *field_name {
        "system_prompt" => "系统提示词",
        "style" => "回复风格",
        "max_history_messages" => "历史消息数",
        "theme" => "主题风格",
        "max_tool_rounds" => "工具轮数上限",
        "tool_confirm_timeout" => "确认超时(秒)",
        "auto_restore_session" => "自动恢复会话",
        _ => field_name,
    }
}

pub fn config_field_value_global(app: &ChatApp, idx: usize) -> String {
    let Some(field_name) = CONFIG_GLOBAL_FIELDS_TAB.get(idx) else {
        return String::new();
    };
    match *field_name {
        "system_prompt" => load_system_prompt().unwrap_or_default(),
        "style" => load_style().unwrap_or_default(),
        "max_history_messages" => app.state.agent_config.max_history_messages.to_string(),
        "theme" => app.state.agent_config.theme.display_name().to_string(),
        "max_tool_rounds" => app.state.agent_config.max_tool_rounds.to_string(),
        "tool_confirm_timeout" => {
            if app.state.agent_config.tool_confirm_timeout == 0 {
                "关闭".into()
            } else {
                format!("{}秒", app.state.agent_config.tool_confirm_timeout)
            }
        }
        "auto_restore_session" => {
            if app.state.agent_config.auto_restore_session {
                "开启".into()
            } else {
                "关闭".into()
            }
        }
        _ => String::new(),
    }
}

pub fn config_field_raw_value_global(app: &ChatApp, idx: usize) -> String {
    let Some(field_name) = CONFIG_GLOBAL_FIELDS_TAB.get(idx) else {
        return String::new();
    };
    match *field_name {
        "system_prompt" => load_system_prompt().unwrap_or_default(),
        "style" => load_style().unwrap_or_default(),
        "theme" => app.state.agent_config.theme.to_str().to_string(),
        "max_tool_rounds" => app.state.agent_config.max_tool_rounds.to_string(),
        "tool_confirm_timeout" => app.state.agent_config.tool_confirm_timeout.to_string(),
        "auto_restore_session" => {
            if app.state.agent_config.auto_restore_session {
                "true".into()
            } else {
                "false".into()
            }
        }
        _ => String::new(),
    }
}

pub fn config_field_set_global(app: &mut ChatApp, idx: usize, value: &str) {
    let Some(field_name) = CONFIG_GLOBAL_FIELDS_TAB.get(idx) else {
        return;
    };
    match *field_name {
        "system_prompt" => {
            save_system_prompt(value);
        }
        "style" => {
            save_style(value);
        }
        "max_history_messages" => {
            if let Ok(num) = value.trim().parse::<usize>() {
                app.state.agent_config.max_history_messages = num;
            }
        }
        "theme" => {
            app.state.agent_config.theme = ThemeName::from_str(value.trim());
            app.ui.theme = super::theme::Theme::from_name(&app.state.agent_config.theme);
            app.ui.msg_lines_cache = None;
        }
        "max_tool_rounds" => {
            if let Ok(num) = value.trim().parse::<usize>() {
                app.state.agent_config.max_tool_rounds = num;
            }
        }
        "tool_confirm_timeout" => {
            if let Ok(num) = value.trim().parse::<u64>() {
                app.state.agent_config.tool_confirm_timeout = num;
            }
        }
        "auto_restore_session" => {
            app.state.agent_config.auto_restore_session = matches!(
                value.trim().to_lowercase().as_str(),
                "true" | "1" | "开启" | "on" | "yes"
            );
        }
        _ => {}
    }
}
