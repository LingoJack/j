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

/// Tools tab 可滚动列表
pub(super) fn draw_tab_tools_list<'a>(app: &ChatApp) -> ItemList<'a> {
    let t = &app.ui.theme;
    let tool_names = app.tool_registry.tool_names();
    let mut list = ItemList::new(t.bg_primary);

    for (i, name) in tool_names.iter().enumerate() {
        let is_selected = i == app.ui.config_field_idx;
        let is_enabled = !app
            .state
            .agent_config
            .disabled_tools
            .iter()
            .any(|d| d == *name);
        list.push(toggle_list_item(&ToggleListItemCtx {
            name: name.to_string(),
            enabled: is_enabled,
            selected: is_selected,
            desc: None,
            tag: None,
            theme: t,
        }));
    }
    list
}
