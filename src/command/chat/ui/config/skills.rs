use crate::command::chat::app::ChatApp;
use crate::tui::components::{ItemList, toggle_list_item};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Skills tab 固定头部（已启用计数）
pub(super) fn draw_tab_skills_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;
    let total = app.state.loaded_skills.len();
    let enabled_count = total
        - app
            .state
            .agent_config
            .disabled_skills
            .iter()
            .filter(|d| {
                app.state
                    .loaded_skills
                    .iter()
                    .any(|s| &s.frontmatter.name == *d)
            })
            .count();

    lines.push(Line::from(vec![Span::styled(
        format!("  \u{5df2}\u{542f}\u{7528}: {}/{}", enabled_count, total),
        Style::default()
            .fg(t.config_toggle_on)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));
}

/// Skills tab 可滚动列表
pub(super) fn draw_tab_skills_list<'a>(app: &ChatApp) -> ItemList<'a> {
    let t = &app.ui.theme;
    let mut list = ItemList::new(t.bg_primary);

    for (i, skill) in app.state.loaded_skills.iter().enumerate() {
        let is_selected = i == app.ui.config_field_idx;
        let name = &skill.frontmatter.name;
        let is_enabled = !app
            .state
            .agent_config
            .disabled_skills
            .iter()
            .any(|d| d == name);
        list.push(toggle_list_item(
            name,
            is_enabled,
            is_selected,
            Some(&skill.frontmatter.description),
            Some(skill.source.label()),
            t,
        ));
    }
    list
}
