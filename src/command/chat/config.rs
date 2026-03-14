use super::app::ChatApp;
use super::model::{load_style, load_system_prompt, save_style, save_system_prompt};
use super::theme::ThemeName;
use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS};

pub fn config_field_label(idx: usize) -> &'static str {
    let total_provider = CONFIG_FIELDS.len();
    if idx < total_provider {
        match CONFIG_FIELDS[idx] {
            "name" => "显示名称",
            "api_base" => "API Base",
            "api_key" => "API Key",
            "model" => "模型名称",
            _ => CONFIG_FIELDS[idx],
        }
    } else {
        let gi = idx - total_provider;
        match CONFIG_GLOBAL_FIELDS[gi] {
            "system_prompt" => "系统提示词",
            "style" => "回复风格",
            "stream_mode" => "流式输出",
            "max_history_messages" => "历史消息数",
            "theme" => "主题风格",
            "tools_enabled" => "工具调用",
            "max_tool_rounds" => "工具轮数上限",
            "tool_confirm_timeout" => "确认超时(秒)",
            "skills_enabled" => "Skills",
            _ => CONFIG_GLOBAL_FIELDS[gi],
        }
    }
}

/// 获取配置界面中当前字段的值
pub fn config_field_value(app: &ChatApp, field_idx: usize) -> String {
    let total_provider = CONFIG_FIELDS.len();
    if field_idx < total_provider {
        if app.agent_config.providers.is_empty() {
            return String::new();
        }
        let p = &app.agent_config.providers[app.config_provider_idx];
        match CONFIG_FIELDS[field_idx] {
            "name" => p.name.clone(),
            "api_base" => p.api_base.clone(),
            "api_key" => {
                // 显示时隐藏 API Key 中间部分
                if p.api_key.len() > 8 {
                    format!(
                        "{}****{}",
                        &p.api_key[..4],
                        &p.api_key[p.api_key.len() - 4..]
                    )
                } else {
                    p.api_key.clone()
                }
            }
            "model" => p.model.clone(),
            _ => String::new(),
        }
    } else {
        let gi = field_idx - total_provider;
        match CONFIG_GLOBAL_FIELDS[gi] {
            "system_prompt" => load_system_prompt().unwrap_or_default(),
            "style" => load_style().unwrap_or_default(),
            "stream_mode" => {
                if app.agent_config.stream_mode {
                    "开启".into()
                } else {
                    "关闭".into()
                }
            }
            "max_history_messages" => app.agent_config.max_history_messages.to_string(),
            "theme" => app.agent_config.theme.display_name().to_string(),
            "tools_enabled" => {
                if app.agent_config.tools_enabled {
                    "开启".into()
                } else {
                    "关闭".into()
                }
            }
            "max_tool_rounds" => app.agent_config.max_tool_rounds.to_string(),
            "tool_confirm_timeout" => {
                if app.agent_config.tool_confirm_timeout == 0 {
                    "关闭".into()
                } else {
                    format!("{}秒", app.agent_config.tool_confirm_timeout)
                }
            }
            "skills_enabled" => {
                let total = app.loaded_skills.len();
                let enabled = total
                    - app
                        .agent_config
                        .disabled_skills
                        .iter()
                        .filter(|d| app.loaded_skills.iter().any(|s| &s.frontmatter.name == *d))
                        .count();
                format!("{}/{} 已启用", enabled, total)
            }
            _ => String::new(),
        }
    }
}

/// 获取配置字段的原始值（用于编辑时填入输入框）
pub fn config_field_raw_value(app: &ChatApp, field_idx: usize) -> String {
    let total_provider = CONFIG_FIELDS.len();
    if field_idx < total_provider {
        if app.agent_config.providers.is_empty() {
            return String::new();
        }
        let p = &app.agent_config.providers[app.config_provider_idx];
        match CONFIG_FIELDS[field_idx] {
            "name" => p.name.clone(),
            "api_base" => p.api_base.clone(),
            "api_key" => p.api_key.clone(),
            "model" => p.model.clone(),
            _ => String::new(),
        }
    } else {
        let gi = field_idx - total_provider;
        match CONFIG_GLOBAL_FIELDS[gi] {
            "system_prompt" => load_system_prompt().unwrap_or_default(),
            "style" => load_style().unwrap_or_default(),
            "stream_mode" => {
                if app.agent_config.stream_mode {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            "theme" => app.agent_config.theme.to_str().to_string(),
            "tools_enabled" => {
                if app.agent_config.tools_enabled {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            "max_tool_rounds" => app.agent_config.max_tool_rounds.to_string(),
            "tool_confirm_timeout" => app.agent_config.tool_confirm_timeout.to_string(),
            "skills_enabled" => String::new(), // 不可直接编辑，进入子菜单
            _ => String::new(),
        }
    }
}

/// 将编辑结果写回配置
pub fn config_field_set(app: &mut ChatApp, field_idx: usize, value: &str) {
    let total_provider = CONFIG_FIELDS.len();
    if field_idx < total_provider {
        if app.agent_config.providers.is_empty() {
            return;
        }
        let p = &mut app.agent_config.providers[app.config_provider_idx];
        match CONFIG_FIELDS[field_idx] {
            "name" => p.name = value.to_string(),
            "api_base" => p.api_base = value.to_string(),
            "api_key" => p.api_key = value.to_string(),
            "model" => p.model = value.to_string(),
            _ => {}
        }
    } else {
        let gi = field_idx - total_provider;
        match CONFIG_GLOBAL_FIELDS[gi] {
            "system_prompt" => {
                save_system_prompt(value);
            }
            "style" => {
                save_style(value);
            }
            "stream_mode" => {
                app.agent_config.stream_mode = matches!(
                    value.trim().to_lowercase().as_str(),
                    "true" | "1" | "开启" | "on" | "yes"
                );
            }
            "max_history_messages" => {
                if let Ok(num) = value.trim().parse::<usize>() {
                    app.agent_config.max_history_messages = num;
                }
            }
            "theme" => {
                app.agent_config.theme = ThemeName::from_str(value.trim());
                app.theme = super::theme::Theme::from_name(&app.agent_config.theme);
                app.msg_lines_cache = None;
            }
            "tools_enabled" => {
                app.agent_config.tools_enabled = matches!(
                    value.trim().to_lowercase().as_str(),
                    "true" | "1" | "开启" | "on" | "yes"
                );
            }
            "max_tool_rounds" => {
                if let Ok(num) = value.trim().parse::<usize>() {
                    app.agent_config.max_tool_rounds = num;
                }
            }
            "tool_confirm_timeout" => {
                if let Ok(num) = value.trim().parse::<u64>() {
                    app.agent_config.tool_confirm_timeout = num;
                }
            }
            _ => {}
        }
    }
}
