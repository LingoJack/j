use crate::command::chat::app::ChatApp;
use crate::tui::components::{ItemList, ToggleListItemCtx, toggle_list_item};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Commands tab 固定头部（已启用计数）
pub(super) fn draw_tab_commands_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;
    let total = app.state.loaded_commands.len();
    let enabled_count = total
        - app
            .state
            .agent_config
            .disabled_commands
            .iter()
            .filter(|d| {
                app.state
                    .loaded_commands
                    .iter()
                    .any(|c| &c.frontmatter.name == *d)
            })
            .count();

    lines.push(Line::from(vec![Span::styled(
        format!("  \u{5df2}\u{542f}\u{7528}: {}/{}", enabled_count, total),
        Style::default()
            .fg(t.config_toggle_on)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    if total == 0 {
        lines.push(Line::from(Span::styled(
            "  (\u{6ca1}\u{6709}\u{81ea}\u{5b9a}\u{4e49}\u{547d}\u{4ee4}\u{ff0c}\u{5728} ~/.jdata/agent/commands/ \u{6216} .jcli/commands/ \u{4e0b}\u{521b}\u{5efa})",
            Style::default().fg(t.config_dim),
        )));
    }
}

/// Commands tab 可滚动列表
pub(super) fn draw_tab_commands_list<'a>(app: &ChatApp) -> ItemList<'a> {
    let t = &app.ui.theme;
    let mut list = ItemList::new(t.bg_primary);

    for (i, cmd) in app.state.loaded_commands.iter().enumerate() {
        let is_selected = i == app.ui.config_field_idx;
        let name = &cmd.frontmatter.name;
        let is_enabled = !app
            .state
            .agent_config
            .disabled_commands
            .iter()
            .any(|d| d == name);
        list.push(toggle_list_item(&ToggleListItemCtx {
            name: name.to_string(),
            enabled: is_enabled,
            selected: is_selected,
            desc: Some(cmd.frontmatter.description.clone()),
            tag: Some(cmd.source.label().to_string()),
            theme: t,
        }));
    }
    list
}
