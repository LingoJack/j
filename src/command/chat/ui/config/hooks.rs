use crate::command::chat::app::ChatApp;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Hooks tab（展示已注册的 hooks）
pub(super) fn draw_tab_hooks_lines<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;
    let hooks: Vec<_> = if let Ok(manager) = app.hook_manager.lock() {
        manager.list_hooks()
    } else {
        Vec::new()
    };

    if hooks.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  (\u{6682}\u{65e0} hooks)",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  \u{7528}\u{6237}\u{7ea7}: ~/.jdata/agent/hooks/",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(Span::styled(
            "  \u{9879}\u{76ee}\u{7ea7}: .jcli/hooks/",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(Span::styled(
            "  \u{8fd0}\u{884c}\u{65f6}: \u{901a}\u{8fc7} RegisterHook \u{5de5}\u{5177}\u{6ce8}\u{518c}",
            Style::default().fg(t.config_dim),
        )));
        return;
    }

    lines.push(Line::from(Span::styled(
        format!(
            "  \u{1fa9d} \u{5df2}\u{6ce8}\u{518c} Hooks ({})",
            hooks.len()
        ),
        Style::default()
            .fg(t.config_label)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for entry in &hooks {
        let source_style = match entry.source {
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

        let label_display: String = if entry.label.chars().count() > 40 {
            let truncated: String = entry.label.chars().take(40).collect();
            format!("{truncated}...")
        } else {
            entry.label.clone()
        };

        let timeout_str = entry
            .timeout
            .map(|t| format!("  {}s", t))
            .unwrap_or_default();

        lines.push(Line::from(vec![
            Span::styled(format!("    [{:<7}]  ", entry.source), source_style),
            Span::styled(
                format!("{:<22}  ", entry.event.as_str()),
                Style::default().fg(t.config_label),
            ),
            Span::styled(label_display, Style::default().fg(t.config_value)),
            Span::styled(timeout_str, Style::default().fg(t.config_dim)),
        ]));
    }
}
