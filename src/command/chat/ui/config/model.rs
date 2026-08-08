use crate::command::chat::app::ChatApp;
use crate::command::chat::render::helpers::{config_field_label_model, config_field_value_model};
use crate::command::chat::ui::components::{RowContext, secret_field_row};
use crate::constants::CONFIG_FIELDS;
use crate::tui::components::{
    ItemList, TOGGLE_OFF, TOGGLE_ON, TextFieldRowCtx, ToggleRowCtx, pointer_span, text_field_row,
    toggle_row,
};
use crate::util::text::display_width;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Model tab：左侧 Provider 列表
pub(super) fn draw_tab_model_providers<'a>(app: &ChatApp, area_width: u16) -> ItemList<'a> {
    let t = &app.ui.theme;
    let item_bg = t.bg_primary;
    let mut list = ItemList::new(item_bg);
    let provider_count = app.state.agent_config.providers.len();

    if provider_count == 0 {
        list.push(Line::from(Span::styled(
            "  (无 Provider，按 a 新增)",
            Style::default().fg(t.config_toggle_off),
        )));
        return list;
    }

    for (i, p) in app.state.agent_config.providers.iter().enumerate() {
        let is_current = i == app.ui.config_provider_idx;
        let is_active = i == app.state.agent_config.active_index;
        let focused = !app.ui.model_in_fields && is_current;

        // 箭头和圆点不参与高亮，使用统一风格
        let pointer = pointer_span(focused, t);
        let marker_style = if is_active {
            Style::default().fg(t.config_toggle_on)
        } else {
            Style::default().fg(t.config_toggle_off)
        };
        let marker = if is_active { TOGGLE_ON } else { TOGGLE_OFF };
        let marker_span = Span::styled(format!("{marker} "), marker_style);

        let name_style = if focused {
            Style::default()
                .fg(t.config_tab_active_fg)
                .bg(t.config_tab_active_bg)
                .add_modifier(Modifier::BOLD)
        } else if is_current {
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.config_tab_inactive)
        };
        let name_span = Span::styled(p.name.clone(), name_style);

        if focused {
            // name 及填充空白带高亮背景，延伸到行尾
            // pointer 占 4 字符宽（"  ❯ "），marker 占 2 字符宽（"● "）
            let used: usize = 4 + 2 + display_width(p.name.as_str());
            let pad = (area_width as usize).saturating_sub(used);
            list.push(Line::from(vec![
                pointer,
                marker_span,
                name_span,
                Span::styled(" ".repeat(pad), Style::default().bg(t.config_tab_active_bg)),
            ]));
        } else {
            list.push(Line::from(vec![pointer, marker_span, name_span]));
        }
    }

    list
}

/// Model tab：右侧配置字段详情
pub(super) fn draw_tab_model_detail<'a>(app: &ChatApp) -> ItemList<'a> {
    let t = &app.ui.theme;
    let mut list = ItemList::new(t.bg_primary);
    let provider_count = app.state.agent_config.providers.len();

    if provider_count == 0 {
        return list;
    }

    // 顶部空行
    list.push(Line::from(""));

    for (i, provider_field) in CONFIG_FIELDS.iter().enumerate() {
        let is_selected = app.ui.model_field_idx == i && app.ui.model_in_fields;
        let label = config_field_label_model(i);
        let value = if app.ui.config_editing && is_selected {
            app.ui.config_edit_buf.clone()
        } else {
            config_field_value_model(app, i)
        };

        let line = if *provider_field == "api_key" {
            let ctx = RowContext {
                selected: is_selected,
                theme: t,
            };
            secret_field_row(
                label,
                &value,
                app.ui.config_editing,
                app.ui.config_edit_cursor,
                &ctx,
            )
        } else if *provider_field == "supports_vision" {
            let toggle_on = if let Some(p) = app
                .state
                .agent_config
                .providers
                .get(app.ui.config_provider_idx)
            {
                p.supports_vision
            } else {
                false
            };
            toggle_row(&ToggleRowCtx {
                label: label.to_string(),
                is_on: toggle_on,
                selected: is_selected,
                hint: "Enter 切换".to_string(),
                theme: t,
            })
        } else if *provider_field == "tool_call_mode" {
            let mode = app
                .state
                .agent_config
                .providers
                .get(app.ui.config_provider_idx)
                .map(|p| p.tool_call_mode.display_name())
                .unwrap_or("原生工具调用");
            toggle_row(&ToggleRowCtx {
                label: label.to_string(),
                is_on: app
                    .state
                    .agent_config
                    .providers
                    .get(app.ui.config_provider_idx)
                    .map(|p| p.tool_call_mode.allows_tools())
                    .unwrap_or(true),
                selected: is_selected,
                hint: format!("Enter 切换 ({mode})"),
                theme: t,
            })
        } else {
            let hint = if *provider_field == "thinking_effort" {
                Some("low/high/max/xhigh".to_string())
            } else {
                None
            };
            text_field_row(&TextFieldRowCtx {
                label: label.to_string(),
                value,
                selected: is_selected,
                editing: app.ui.config_editing,
                cursor: app.ui.config_edit_cursor,
                theme: t,
                hint,
            })
        };
        list.push(line);
    }

    // 底部操作提示
    list.push(Line::from(""));
    list.push(Line::from(Span::styled(
        " Tab 进入编辑 | Enter 确认 | Esc 取消",
        Style::default().fg(t.config_dim),
    )));

    list
}

/// 计算 Model tab 左侧 Provider 列表所需的最小宽度
pub(super) fn model_providers_min_width(app: &ChatApp) -> u16 {
    // 指针(4) + marker(1) + 空格(1) + 名称 + 右侧 padding(3) + 左右边框(2)
    const BASE: usize = 11;

    let max_name_len = app
        .state
        .agent_config
        .providers
        .iter()
        .map(|p| display_width(&p.name))
        .max()
        .unwrap_or(10);

    (BASE + max_name_len).clamp(18, 34) as u16
}
