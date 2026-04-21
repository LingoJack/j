use crate::command::chat::app::ChatApp;
use crate::tui::components::{ItemList, TOGGLE_OFF, TOGGLE_ON, toggle_list_item};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Hooks tab 固定头部（统计信息）
pub(super) fn draw_tab_hooks_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;
    let hooks: Vec<_> = if let Ok(manager) = app.hook_manager.lock() {
        manager.list_hooks()
    } else {
        Vec::new()
    };

    if hooks.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  (暂无 hooks)",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  用户级: ~/.jdata/agent/hooks/",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(Span::styled(
            "  项目级: .jcli/hooks/",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(Span::styled(
            "  运行时: 通过 RegisterHook 工具注册",
            Style::default().fg(t.config_dim),
        )));
        return;
    }

    let total = hooks.len();
    let enabled_count = hooks
        .iter()
        .filter(|h| !app.state.agent_config.disabled_hooks.contains(&h.unique_id))
        .count();

    lines.push(Line::from(vec![
        Span::styled(
            format!("  🪝 已注册 Hooks ({}/{})", enabled_count, total),
            Style::default()
                .fg(t.config_label)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  (Space 切换, e 全部启用, d 全部禁用)",
            Style::default().fg(t.config_dim),
        ),
    ]));
    lines.push(Line::from(""));
}

/// Hooks tab 可滚动列表
pub(super) fn draw_tab_hooks_list<'a>(app: &ChatApp) -> ItemList<'a> {
    let t = &app.ui.theme;
    let hooks: Vec<_> = if let Ok(manager) = app.hook_manager.lock() {
        manager.list_hooks()
    } else {
        Vec::new()
    };

    let mut list = ItemList::new(t.bg_primary);

    for (i, entry) in hooks.iter().enumerate() {
        let is_selected = i == app.ui.config_field_idx;
        let is_enabled = !app
            .state
            .agent_config
            .disabled_hooks
            .contains(&entry.unique_id);

        let _source_style = match entry.source {
            "builtin" => Style::default()
                .fg(ratatui::style::Color::Magenta)
                .add_modifier(Modifier::BOLD),
            "user" => Style::default()
                .fg(ratatui::style::Color::Green)
                .add_modifier(Modifier::BOLD),
            "project" => Style::default()
                .fg(ratatui::style::Color::Blue)
                .add_modifier(Modifier::BOLD),
            _ => Style::default()
                .fg(ratatui::style::Color::Yellow)
                .add_modifier(Modifier::BOLD),
        };

        let label_display: String = if entry.label.chars().count() > 30 {
            let truncated: String = entry.label.chars().take(30).collect();
            format!("{truncated}...")
        } else {
            entry.label.clone()
        };

        let display_name = format!(
            "[{:<7}] {:<20} {}",
            entry.source,
            entry.event.as_str(),
            label_display
        );

        let _toggle_str = if is_enabled { TOGGLE_ON } else { TOGGLE_OFF };
        let _toggle_style = if is_enabled {
            Style::default().fg(t.config_toggle_on)
        } else {
            Style::default().fg(t.config_toggle_off)
        };

        list.push(toggle_list_item(
            &display_name,
            is_enabled,
            is_selected,
            None,
            None,
            t,
        ));
    }

    list
}
