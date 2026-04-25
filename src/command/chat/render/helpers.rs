use super::theme::ThemeName;
use crate::command::chat::agent_md;
use crate::command::chat::app::ChatApp;
use crate::command::chat::context::compact::BUILTIN_EXEMPT_TOOLS;
use crate::command::chat::storage::{
    load_style, load_system_prompt, save_style, save_system_prompt,
};
use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS_TAB};

// ========== Model tab helpers ==========

/// 返回 Model 配置页中指定索引字段的显示标签。
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

/// 返回 Model 配置页中指定索引字段的显示值（敏感信息脱敏）。
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

/// 返回 Model 配置页中指定索引字段的原始值（不脱敏，用于编辑回填）。
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

/// 将用户输入的值写入 Model 配置页指定索引字段。
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

/// 返回 Global 配置页中指定索引字段的显示标签。
pub fn config_field_label_global(idx: usize) -> &'static str {
    let Some(field_name) = CONFIG_GLOBAL_FIELDS_TAB.get(idx) else {
        return "";
    };
    match *field_name {
        "system_prompt" => "系统提示词",
        "agent_md" => "AGENT.md",
        "style" => "回复风格",
        "max_history_messages" => "历史消息数",
        "max_context_tokens" => "窗口Token",
        "theme" => "主题风格",
        "max_tool_rounds" => "工具轮数上限",
        "tool_confirm_timeout" => "确认超时",
        "auto_restore_session" => "自动恢复会话",
        "thinking_style" => "思考动画",
        "compact_enabled" => "上下文压缩",
        "compact_token_threshold" => "压缩阈值(K)",
        "compact_keep_recent" => "保留最近轮数",
        "compact_exempt_tools" => "豁免压缩工具",
        _ => field_name,
    }
}

/// 全局配置字段的简要说明（显示在字段值下方）
pub fn config_field_desc_global(idx: usize) -> &'static str {
    let Some(field_name) = CONFIG_GLOBAL_FIELDS_TAB.get(idx) else {
        return "";
    };
    match *field_name {
        "system_prompt" => "AI 的基础角色与行为指令",
        "agent_md" => "项目级上下文文件，自动注入到提示词",
        "style" => "AI 回复的语言风格与格式偏好",
        "max_history_messages" => "发送给 AI 的最大消息条数，0=无限",
        "max_context_tokens" => "消息窗口的 Token 上限(K)，0=无限",
        "theme" => "终端配色方案",
        "max_tool_rounds" => "单次对话中工具调用最大轮数",
        "tool_confirm_timeout" => "工具确认等待秒数，0=关闭自动确认",
        "auto_restore_session" => "启动时自动恢复上次会话",
        "thinking_style" => "AI 思考时的加载动画风格",
        "compact_enabled" => "开启后自动压缩过长的上下文",
        "compact_token_threshold" => "上下文 Token 数超过此值时触发压缩(K)，0=使用默认值",
        "compact_keep_recent" => "micro_compact 保留最近几个工具结果不替换",
        "compact_exempt_tools" => "不压缩的工具名称(逗号分隔，如 Read,Grep,Glob)",
        _ => "",
    }
}

/// 返回 Global 配置页中指定索引字段的显示值（敏感信息脱敏）。
pub fn config_field_value_global(app: &ChatApp, idx: usize) -> String {
    let Some(field_name) = CONFIG_GLOBAL_FIELDS_TAB.get(idx) else {
        return String::new();
    };
    match *field_name {
        "system_prompt" => load_system_prompt().unwrap_or_default(),
        "agent_md" => std::fs::read_to_string(agent_md::agent_md_path()).unwrap_or_default(),
        "style" => load_style().unwrap_or_default(),
        "max_history_messages" => {
            let v = app.state.agent_config.max_history_messages;
            if v == 0 {
                "无限制".into()
            } else {
                v.to_string()
            }
        }
        "max_context_tokens" => {
            let v = app.state.agent_config.max_context_tokens;
            if v == 0 {
                "无限制".into()
            } else {
                format!("{}K", v)
            }
        }
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
        "thinking_style" => app
            .state
            .agent_config
            .thinking_style
            .display_name()
            .to_string(),
        "compact_enabled" => {
            if app.state.agent_config.compact.enabled {
                "开启".into()
            } else {
                "关闭".into()
            }
        }
        "compact_token_threshold" => {
            let v = app.state.agent_config.compact.token_threshold;
            if v == 0 {
                "默认".into()
            } else {
                format!("{}K", v / 1000)
            }
        }
        "compact_keep_recent" => app.state.agent_config.compact.keep_recent.to_string(),
        "compact_exempt_tools" => {
            // (已豁免数/工具总数) 格式展示
            let builtin = BUILTIN_EXEMPT_TOOLS.len();
            let user_extra = &app.state.agent_config.compact.micro_compact_exempt_tools;
            let exempt_count = builtin + user_extra.len();
            let total = app.tool_registry.tool_names().len();
            format!("({}/{})", exempt_count, total)
        }
        _ => String::new(),
    }
}

/// 返回 Global 配置页中指定索引字段的原始值（不脱敏，用于编辑回填）。
pub fn config_field_raw_value_global(app: &ChatApp, idx: usize) -> String {
    let Some(field_name) = CONFIG_GLOBAL_FIELDS_TAB.get(idx) else {
        return String::new();
    };
    match *field_name {
        "system_prompt" => load_system_prompt().unwrap_or_default(),
        "agent_md" => std::fs::read_to_string(agent_md::agent_md_path()).unwrap_or_default(),
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
        "thinking_style" => app.state.agent_config.thinking_style.as_str().to_string(),
        "compact_enabled" => {
            if app.state.agent_config.compact.enabled {
                "true".into()
            } else {
                "false".into()
            }
        }
        "compact_token_threshold" => {
            let v = app.state.agent_config.compact.token_threshold;
            if v == 0 {
                "0".into()
            } else {
                format!("{}", v / 1000)
            }
        }
        "compact_keep_recent" => app.state.agent_config.compact.keep_recent.to_string(),
        "compact_exempt_tools" => {
            // 合并内置 + 用户自定义，编辑时展示完整列表
            let mut all: Vec<&str> = BUILTIN_EXEMPT_TOOLS.to_vec();
            let user_extra = &app.state.agent_config.compact.micro_compact_exempt_tools;
            for t in user_extra {
                if !all.contains(&t.as_str()) {
                    all.push(t);
                }
            }
            all.join(",")
        }
        "max_history_messages" => app.state.agent_config.max_history_messages.to_string(),
        "max_context_tokens" => app.state.agent_config.max_context_tokens.to_string(),
        _ => String::new(),
    }
}

/// 将用户输入的值写入 Global 配置页指定索引字段。
pub fn config_field_set_global(app: &mut ChatApp, idx: usize, value: &str) {
    let Some(field_name) = CONFIG_GLOBAL_FIELDS_TAB.get(idx) else {
        return;
    };
    match *field_name {
        "system_prompt" => {
            save_system_prompt(value);
        }
        "agent_md" => {
            if let Some(parent) = agent_md::agent_md_path().parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(agent_md::agent_md_path(), value);
        }
        "style" => {
            save_style(value);
        }
        "max_history_messages" => {
            if let Ok(num) = value.trim().parse::<usize>() {
                app.state.agent_config.max_history_messages = num;
            }
        }
        "max_context_tokens" => {
            if let Ok(num) = value.trim().parse::<usize>() {
                app.state.agent_config.max_context_tokens = num;
            }
        }
        "theme" => {
            app.state.agent_config.theme = ThemeName::parse(value.trim());
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
        "thinking_style" => {
            app.state.agent_config.thinking_style =
                crate::command::chat::storage::config::ThinkingStyle::parse(value.trim());
        }
        "compact_enabled" => {
            app.state.agent_config.compact.enabled = matches!(
                value.trim().to_lowercase().as_str(),
                "true" | "1" | "开启" | "on" | "yes"
            );
        }
        "compact_token_threshold" => {
            if let Ok(num) = value.trim().parse::<usize>() {
                // UI 使用 K 为单位，存储时转换为绝对值
                app.state.agent_config.compact.token_threshold = num * 1000;
            }
        }
        "compact_keep_recent" => {
            if let Ok(num) = value.trim().parse::<usize>() {
                app.state.agent_config.compact.keep_recent = num;
            }
        }
        "compact_exempt_tools" => {
            // 保存时剥离内置工具，只保留用户额外新增的部分
            let builtin: Vec<&str> = BUILTIN_EXEMPT_TOOLS.to_vec();
            app.state.agent_config.compact.micro_compact_exempt_tools = value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && !builtin.contains(&s.as_str()))
                .collect();
        }
        _ => {}
    }
}
