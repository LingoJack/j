use crate::command::chat::app::ChatApp;
use crate::tui::components::{
    ItemList, TOGGLE_OFF, TOGGLE_ON, ToggleListItemCtx, toggle_list_item,
};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Tools tab 固定头部（总开关）
pub(super) fn draw_tab_tools_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;
    let tool_names = app.tool_registry.tool_names();
    let total = tool_names.len();
    let enabled_count = total
        - app
            .state
            .agent_config
            .disabled_tools
            .iter()
            .filter(|d| tool_names.contains(&d.as_str()))
            .count();

    let master_style = if app.state.agent_config.tools_enabled {
        Style::default()
            .fg(t.config_toggle_on)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.config_toggle_off)
    };
    let master_text = if app.state.agent_config.tools_enabled {
        format!(
            "  \u{603b}\u{5f00}\u{5173}: {} \u{5f00}\u{542f} ({}/{})",
            TOGGLE_ON, enabled_count, total
        )
    } else {
        format!(
            "  \u{603b}\u{5f00}\u{5173}: {} \u{5173}\u{95ed}",
            TOGGLE_OFF
        )
    };
    lines.push(Line::from(vec![
        Span::styled(master_text, master_style),
        Span::styled("  (t \u{5207}\u{6362})", Style::default().fg(t.config_dim)),
    ]));
    lines.push(Line::from(""));
}

/// Tools tab 可滚动列表（层级导航模式）
///
/// 列表竖直排列所有工具名，当前选中的工具下方展开两个选项（启用/defer）。
/// Tab 键在工具列表层级和选项层级之间切换。
pub(super) fn draw_tab_tools_list<'a>(app: &ChatApp) -> ItemList<'a> {
    let t = &app.ui.theme;
    let tool_names = app.tool_registry.tool_names();
    let mut list = ItemList::new(t.bg_primary);

    let deferred_tools = match app.deferred_tools.lock() {
        Ok(guard) => guard,
        Err(e) => e.into_inner(),
    };

    for (i, name) in tool_names.iter().enumerate() {
        let is_selected = i == app.ui.config_field_idx;
        let is_enabled = !app
            .state
            .agent_config
            .disabled_tools
            .iter()
            .any(|d| d == *name);
        let is_deferred = deferred_tools.iter().any(|d| d == name);

        if is_selected {
            // 选中工具：显示工具名 + 展开选项
            let name_style = Style::default()
                .fg(t.config_section)
                .add_modifier(Modifier::BOLD);
            let marker = "▸";
            list.push(Line::from(vec![
                Span::styled(format!("  {marker} "), name_style),
                Span::styled(name.to_string(), name_style),
            ]));

            // 展开选项（启用 / defer）
            let opt_on_style = |focused: bool| {
                if focused {
                    Style::default()
                        .fg(t.config_toggle_on)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.config_toggle_on)
                }
            };
            let opt_off_style = |focused: bool| {
                if focused {
                    Style::default()
                        .fg(t.config_toggle_off)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.config_toggle_off)
                }
            };
            let dim_style = Style::default().fg(t.config_dim);

            // 选项1：启用
            let enable_focused = app.ui.tools_in_options && app.ui.tools_option_idx == 0;
            let enable_marker = if enable_focused { "›" } else { " " };
            let enable_toggle = if is_enabled {
                Span::styled(TOGGLE_ON.to_string(), opt_on_style(enable_focused))
            } else {
                Span::styled(TOGGLE_OFF.to_string(), opt_off_style(enable_focused))
            };
            list.push(Line::from(vec![
                Span::styled(format!("    {enable_marker} "), dim_style),
                Span::styled(
                    "启用 ",
                    if enable_focused {
                        Style::default()
                            .fg(t.config_section)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        dim_style
                    },
                ),
                enable_toggle,
            ]));

            // 选项2：defer
            let defer_focused = app.ui.tools_in_options && app.ui.tools_option_idx == 1;
            let defer_marker = if defer_focused { "›" } else { " " };
            // 禁用的工具 defer 选项置灰
            let defer_effective = is_enabled;
            let defer_toggle = if is_deferred && defer_effective {
                Span::styled(TOGGLE_ON.to_string(), opt_on_style(defer_focused))
            } else if defer_effective {
                Span::styled(TOGGLE_OFF.to_string(), opt_off_style(defer_focused))
            } else {
                // 禁用状态下 defer 无意义，置灰
                Span::styled(TOGGLE_OFF.to_string(), Style::default().fg(t.config_dim))
            };
            list.push(Line::from(vec![
                Span::styled(format!("    {defer_marker} "), dim_style),
                Span::styled(
                    "defer ",
                    if defer_focused {
                        Style::default()
                            .fg(t.config_section)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        dim_style
                    },
                ),
                defer_toggle,
            ]));
        } else {
            // 非选中工具：只显示工具名
            list.push(toggle_list_item(&ToggleListItemCtx {
                name: name.to_string(),
                enabled: is_enabled,
                selected: false,
                desc: None,
                tag: if is_deferred && is_enabled {
                    Some("defer".to_string())
                } else {
                    None
                },
                theme: t,
            }));
        }
    }
    list
}
