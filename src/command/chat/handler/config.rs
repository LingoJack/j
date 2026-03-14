use super::super::model::{ModelProvider, save_agent_config};
use crate::command::chat::app::{ChatApp, ChatMode, config_total_fields};
use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS};
use crossterm::event::{KeyCode, KeyEvent};

// config_field_* 函数已移至 super::super::config 模块
use super::super::config::{config_field_raw_value, config_field_set};

/// 配置模式按键处理
pub fn handle_config_mode(app: &mut ChatApp, key: KeyEvent) {
    let total_fields = config_total_fields();

    if app.config_editing {
        // 正在编辑某个字段
        match key.code {
            KeyCode::Esc => {
                // 取消编辑
                app.config_editing = false;
            }
            KeyCode::Enter => {
                // 确认编辑
                let val = app.config_edit_buf.clone();
                config_field_set(app, app.config_field_idx, &val);
                app.config_editing = false;
            }
            KeyCode::Backspace => {
                if app.config_edit_cursor > 0 {
                    let idx = app
                        .config_edit_buf
                        .char_indices()
                        .nth(app.config_edit_cursor - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end_idx = app
                        .config_edit_buf
                        .char_indices()
                        .nth(app.config_edit_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(app.config_edit_buf.len());
                    app.config_edit_buf = format!(
                        "{}{}",
                        &app.config_edit_buf[..idx],
                        &app.config_edit_buf[end_idx..]
                    );
                    app.config_edit_cursor -= 1;
                }
            }
            KeyCode::Left => {
                app.config_edit_cursor = app.config_edit_cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                let char_count = app.config_edit_buf.chars().count();
                if app.config_edit_cursor < char_count {
                    app.config_edit_cursor += 1;
                }
            }
            KeyCode::Char(c) => {
                let byte_idx = app
                    .config_edit_buf
                    .char_indices()
                    .nth(app.config_edit_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.config_edit_buf.len());
                app.config_edit_buf.insert(byte_idx, c);
                app.config_edit_cursor += 1;
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            // 保存配置并退出
            save_agent_config(&app.agent_config);
            app.mode = ChatMode::Chat;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.config_field_idx > 0 {
                app.config_field_idx -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.config_field_idx < total_fields - 1 {
                app.config_field_idx += 1;
            }
        }
        KeyCode::Tab | KeyCode::Right => {
            // 切换到下一个 provider
            let count = app.agent_config.providers.len();
            if count > 1 {
                app.config_provider_idx = (app.config_provider_idx + 1) % count;
                // 切换后如果在 provider 字段区域，保持字段位置不变
            }
        }
        KeyCode::BackTab | KeyCode::Left => {
            // 反向切换 provider
            let count = app.agent_config.providers.len();
            if count > 1 {
                if app.config_provider_idx == 0 {
                    app.config_provider_idx = count - 1;
                } else {
                    app.config_provider_idx -= 1;
                }
            }
        }
        KeyCode::Enter => {
            // 进入编辑模式
            let total_provider = CONFIG_FIELDS.len();
            if app.config_field_idx < total_provider && app.agent_config.providers.is_empty() {
                app.show_toast("还没有 Provider，按 a 新增", true);
                return;
            }
            // stream_mode 字段直接切换，不进入编辑模式
            let gi = app.config_field_idx.checked_sub(total_provider);
            if let Some(gi) = gi {
                if CONFIG_GLOBAL_FIELDS[gi] == "stream_mode" {
                    app.agent_config.stream_mode = !app.agent_config.stream_mode;
                    return;
                }
                // tools_enabled 字段：Enter 进入工具开关子菜单
                if CONFIG_GLOBAL_FIELDS[gi] == "tools_enabled" {
                    app.tool_toggle_index = 0;
                    app.mode = ChatMode::ToolToggle;
                    return;
                }
                // skills_enabled 字段：Enter 进入 Skill 开关子菜单
                if CONFIG_GLOBAL_FIELDS[gi] == "skills_enabled" {
                    app.skill_toggle_index = 0;
                    app.mode = ChatMode::SkillToggle;
                    return;
                }
                // theme 字段直接循环切换，不进入编辑模式
                if CONFIG_GLOBAL_FIELDS[gi] == "theme" {
                    app.switch_theme();
                    return;
                }
                // system_prompt 字段使用全屏编辑器
                if CONFIG_GLOBAL_FIELDS[gi] == "system_prompt" {
                    app.pending_system_prompt_edit = true;
                    return;
                }
                // style 字段使用全屏编辑器
                if CONFIG_GLOBAL_FIELDS[gi] == "style" {
                    app.pending_style_edit = true;
                    return;
                }
            }
            app.config_edit_buf = config_field_raw_value(app, app.config_field_idx);
            app.config_edit_cursor = app.config_edit_buf.chars().count();
            app.config_editing = true;
        }
        KeyCode::Char('a') => {
            // 新增 Provider
            let new_provider = ModelProvider {
                name: format!("Provider-{}", app.agent_config.providers.len() + 1),
                api_base: "https://api.openai.com/v1".to_string(),
                api_key: String::new(),
                model: String::new(),
            };
            app.agent_config.providers.push(new_provider);
            app.config_provider_idx = app.agent_config.providers.len() - 1;
            app.config_field_idx = 0; // 跳到 name 字段
            app.show_toast("已新增 Provider，请填写配置", false);
        }
        KeyCode::Char('d') => {
            // 删除当前 Provider
            let count = app.agent_config.providers.len();
            if count == 0 {
                app.show_toast("没有可删除的 Provider", true);
            } else {
                let removed_name = app.agent_config.providers[app.config_provider_idx]
                    .name
                    .clone();
                app.agent_config.providers.remove(app.config_provider_idx);
                // 调整索引
                if app.config_provider_idx >= app.agent_config.providers.len()
                    && app.config_provider_idx > 0
                {
                    app.config_provider_idx -= 1;
                }
                // 调整 active_index
                if app.agent_config.active_index >= app.agent_config.providers.len()
                    && app.agent_config.active_index > 0
                {
                    app.agent_config.active_index -= 1;
                }
                app.show_toast(format!("已删除 Provider: {}", removed_name), false);
            }
        }
        KeyCode::Char('s') => {
            // 将当前 provider 设为活跃
            if !app.agent_config.providers.is_empty() {
                app.agent_config.active_index = app.config_provider_idx;
                let name = app.agent_config.providers[app.config_provider_idx]
                    .name
                    .clone();
                app.show_toast(format!("已设为活跃模型: {}", name), false);
            }
        }
        _ => {}
    }
}

/// 工具开关子菜单按键处理
pub fn handle_tool_toggle_mode(app: &mut ChatApp, key: KeyEvent) {
    let tool_names = app.tool_registry.tool_names();
    let total = tool_names.len();
    if total == 0 {
        app.mode = ChatMode::Config;
        return;
    }

    match key.code {
        KeyCode::Esc => {
            // 返回配置模式并保存
            save_agent_config(&app.agent_config);
            app.mode = ChatMode::Config;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.tool_toggle_index == 0 {
                app.tool_toggle_index = total - 1;
            } else {
                app.tool_toggle_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.tool_toggle_index = (app.tool_toggle_index + 1) % total;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            // 切换当前工具的启用/禁用状态
            let name = tool_names[app.tool_toggle_index].to_string();
            if let Some(pos) = app
                .agent_config
                .disabled_tools
                .iter()
                .position(|d| d == &name)
            {
                app.agent_config.disabled_tools.remove(pos);
            } else {
                app.agent_config.disabled_tools.push(name);
            }
        }
        KeyCode::Char('a') => {
            // 全部启用
            app.agent_config.disabled_tools.clear();
            app.show_toast("已启用全部工具", false);
        }
        KeyCode::Char('d') => {
            // 全部禁用
            app.agent_config.disabled_tools = tool_names.iter().map(|n| n.to_string()).collect();
            app.show_toast("已禁用全部工具", false);
        }
        KeyCode::Char('t') => {
            // 切换总开关
            app.agent_config.tools_enabled = !app.agent_config.tools_enabled;
            let status = if app.agent_config.tools_enabled {
                "开启"
            } else {
                "关闭"
            };
            app.show_toast(format!("工具调用已{}", status), false);
        }
        _ => {}
    }
}

pub fn handle_skill_toggle_mode(app: &mut ChatApp, key: KeyEvent) {
    let total = app.loaded_skills.len();
    if total == 0 {
        app.mode = ChatMode::Config;
        return;
    }

    match key.code {
        KeyCode::Esc => {
            save_agent_config(&app.agent_config);
            app.mode = ChatMode::Config;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.skill_toggle_index == 0 {
                app.skill_toggle_index = total - 1;
            } else {
                app.skill_toggle_index -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.skill_toggle_index = (app.skill_toggle_index + 1) % total;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            let name = app.loaded_skills[app.skill_toggle_index]
                .frontmatter
                .name
                .clone();
            if let Some(pos) = app
                .agent_config
                .disabled_skills
                .iter()
                .position(|d| d == &name)
            {
                app.agent_config.disabled_skills.remove(pos);
            } else {
                app.agent_config.disabled_skills.push(name);
            }
        }
        KeyCode::Char('a') => {
            app.agent_config.disabled_skills.clear();
            app.show_toast("已启用全部 Skills", false);
        }
        KeyCode::Char('d') => {
            app.agent_config.disabled_skills = app
                .loaded_skills
                .iter()
                .map(|s| s.frontmatter.name.clone())
                .collect();
            app.show_toast("已禁用全部 Skills", false);
        }
        _ => {}
    }
}

/// 模型选择列表按键处理
pub fn handle_select_model(app: &mut ChatApp, key: KeyEvent) {
    let count = app.agent_config.providers.len();
    match key.code {
        KeyCode::Esc => {
            app.mode = ChatMode::Chat;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if count > 0 {
                let i = app
                    .model_list_state
                    .selected()
                    .map(|i| if i == 0 { count - 1 } else { i - 1 })
                    .unwrap_or(0);
                app.model_list_state.select(Some(i));
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if count > 0 {
                let i = app
                    .model_list_state
                    .selected()
                    .map(|i| if i >= count - 1 { 0 } else { i + 1 })
                    .unwrap_or(0);
                app.model_list_state.select(Some(i));
            }
        }
        KeyCode::Enter => {
            app.switch_model();
        }
        _ => {}
    }
}
