use super::super::ui_helpers::{
    config_field_label_global, config_field_label_model, config_field_value_global,
    config_field_value_model,
};
use super::components::{
    preview_field_row, secret_field_row, selectable_row, separator_line, tab_bar, text_field_row,
    theme_field_row, toggle_list_item, toggle_row,
};
use crate::command::chat::app::{ChatApp, ConfigTab};
use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS_TAB};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// 绘制顶部 Tab 栏（支持窄屏水平滚动）
fn draw_tab_bar_line<'a>(app: &ChatApp) -> Line<'a> {
    let current = app.ui.config_tab;
    let all_tabs = [
        ConfigTab::Model,
        ConfigTab::Session,
        ConfigTab::Global,
        ConfigTab::Tools,
        ConfigTab::Skills,
        ConfigTab::Hooks,
        ConfigTab::Commands,
        ConfigTab::Archive,
    ];
    let tabs: Vec<(&str, bool)> = all_tabs
        .iter()
        .map(|tab| (tab.label(), *tab == current))
        .collect();
    tab_bar(
        &tabs,
        "\u{2190}\u{2192} \u{5207}\u{6362}\u{6807}\u{7b7e}",
        &app.ui.theme,
    )
}

/// 配置界面主入口（分发器）
pub fn draw_config_screen(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let t = &app.ui.theme;
    let bg = t.bg_title;

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        draw_tab_bar_line(app),
        Line::from(""),
        separator_line(area.width, t),
        Line::from(""),
    ];

    let mut field_line_indices: Vec<usize> = Vec::new();

    match app.ui.config_tab {
        ConfigTab::Model => draw_tab_model_lines(&mut lines, &mut field_line_indices, app),
        ConfigTab::Global => draw_tab_global_lines(&mut lines, &mut field_line_indices, app),
        ConfigTab::Tools => draw_tab_tools_lines(&mut lines, &mut field_line_indices, app),
        ConfigTab::Skills => draw_tab_skills_lines(&mut lines, &mut field_line_indices, app),
        ConfigTab::Hooks => draw_tab_hooks_lines(&mut lines, app),
        ConfigTab::Commands => draw_tab_commands_lines(&mut lines, &mut field_line_indices, app),
        ConfigTab::Session => draw_tab_session_lines(&mut lines, &mut field_line_indices, app),
        ConfigTab::Archive => draw_tab_archive_lines(&mut lines, &mut field_line_indices, app),
    }

    // 滚动：确保选中字段始终可见
    let inner_height = area.height.saturating_sub(2) as usize;
    let selected_idx = match app.ui.config_tab {
        ConfigTab::Session => app.ui.session_list_index,
        ConfigTab::Archive => app.ui.archive_list_index,
        _ => app.ui.config_field_idx,
    };
    if let Some(&selected_line) = field_line_indices.get(selected_idx) {
        let scroll = app.ui.config_scroll_offset as usize;
        let new_scroll = if selected_line < scroll {
            selected_line
        } else if selected_line >= scroll + inner_height {
            selected_line.saturating_sub(inner_height - 1)
        } else {
            scroll
        };
        app.ui.config_scroll_offset = new_scroll as u16;
    }

    let title = match app.ui.config_tab {
        ConfigTab::Model => " \u{2699}\u{fe0f} \u{6a21}\u{578b}\u{914d}\u{7f6e} ",
        ConfigTab::Global => " \u{1f310} \u{5168}\u{5c40}\u{914d}\u{7f6e} ",
        ConfigTab::Tools => " \u{1f527} \u{5de5}\u{5177}\u{5f00}\u{5173} ",
        ConfigTab::Skills => " \u{1f4e6} \u{6280}\u{80fd}\u{5f00}\u{5173} ",
        ConfigTab::Hooks => " \u{1fa9d} Hooks ",
        ConfigTab::Commands => " \u{1f4cb} \u{81ea}\u{5b9a}\u{4e49}\u{547d}\u{4ee4} ",
        ConfigTab::Session => " \u{1f4ac} \u{4f1a}\u{8bdd}\u{7ba1}\u{7406} ",
        ConfigTab::Archive => " \u{1f4e6} \u{5f52}\u{6863}\u{7ba1}\u{7406} ",
    };

    let content = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(t.border_config))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(t.config_label_selected)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(bg)),
        )
        .scroll((app.ui.config_scroll_offset, 0));
    f.render_widget(content, area);
}

/// Model tab 内容
fn draw_tab_model_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
    let t = &app.ui.theme;

    let provider_count = app.state.agent_config.providers.len();
    if provider_count > 0 {
        let tabs: Vec<(&str, bool)> = app
            .state
            .agent_config
            .providers
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let is_active = i == app.state.agent_config.active_index;
                let is_current = i == app.ui.config_provider_idx;
                // We need owned strings but tab_bar takes &str, so we build manually
                let _ = (is_active, is_current, p);
                // Fall back to manual rendering for provider sub-tabs due to marker prefix
                ("", false) // placeholder, not used
            })
            .collect();
        let _ = tabs;

        // Provider sub-tabs with ● / ○ markers need custom rendering
        let mut tab_spans: Vec<Span> = vec![Span::styled("  ", Style::default())];
        for (i, p) in app.state.agent_config.providers.iter().enumerate() {
            let is_current = i == app.ui.config_provider_idx;
            let is_active = i == app.state.agent_config.active_index;
            let marker = if is_active {
                super::components::TOGGLE_ON
            } else {
                super::components::TOGGLE_OFF
            };
            let label = format!(" {marker} {} ", p.name);
            if is_current {
                tab_spans.push(Span::styled(
                    label,
                    Style::default()
                        .fg(t.config_tab_active_fg)
                        .bg(t.config_tab_active_bg)
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                tab_spans.push(Span::styled(
                    label,
                    Style::default().fg(t.config_tab_inactive),
                ));
            }
            if i < provider_count - 1 {
                tab_spans.push(Span::styled(
                    format!(" {} ", super::components::SEPARATOR_V),
                    Style::default().fg(t.separator),
                ));
            }
        }
        tab_spans.push(Span::styled(
            format!(
                "    ({} = \u{6d3b}\u{8dc3}\u{6a21}\u{578b}, Tab \u{5207}\u{6362}, s \u{8bbe}\u{4e3a}\u{6d3b}\u{8dc3})",
                super::components::TOGGLE_ON
            ),
            Style::default().fg(t.config_dim),
        ));
        lines.push(Line::from(tab_spans));
    } else {
        lines.push(Line::from(Span::styled(
            "  (\u{65e0} Provider\u{ff0c}\u{6309} a \u{65b0}\u{589e})",
            Style::default().fg(t.config_toggle_off),
        )));
    }
    lines.push(Line::from(""));

    if provider_count > 0 {
        for (i, provider_field) in CONFIG_FIELDS.iter().enumerate() {
            field_line_indices.push(lines.len());
            let is_selected = app.ui.config_field_idx == i;
            let label = config_field_label_model(i);
            let value = if app.ui.config_editing && is_selected {
                app.ui.config_edit_buf.clone()
            } else {
                config_field_value_model(app, i)
            };

            let line = if *provider_field == "api_key" {
                secret_field_row(
                    label,
                    &value,
                    is_selected,
                    app.ui.config_editing,
                    app.ui.config_edit_cursor,
                    t,
                )
            } else {
                text_field_row(
                    label,
                    &value,
                    is_selected,
                    app.ui.config_editing,
                    app.ui.config_edit_cursor,
                    t,
                )
            };
            lines.push(line);
            lines.push(Line::from(""));
        }
    }
}

/// Global tab 内容
fn draw_tab_global_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
    let t = &app.ui.theme;

    for (i, field) in CONFIG_GLOBAL_FIELDS_TAB.iter().enumerate() {
        field_line_indices.push(lines.len());
        let is_selected = app.ui.config_field_idx == i;
        let label = config_field_label_global(i);
        let value = if app.ui.config_editing && is_selected {
            app.ui.config_edit_buf.clone()
        } else {
            config_field_value_global(app, i)
        };

        let line = if *field == "stream_mode" || *field == "auto_restore_session" {
            let toggle_on = match *field {
                "stream_mode" => app.state.agent_config.stream_mode,
                "auto_restore_session" => app.state.agent_config.auto_restore_session,
                _ => false,
            };
            toggle_row(label, toggle_on, is_selected, "Enter \u{5207}\u{6362}", t)
        } else if *field == "theme" {
            let theme_name = app.state.agent_config.theme.display_name();
            theme_field_row(label, theme_name, is_selected, "Enter \u{5207}\u{6362}", t)
        } else if *field == "system_prompt" || *field == "style" {
            preview_field_row(label, &value, is_selected, "Enter \u{7f16}\u{8f91}", t)
        } else {
            text_field_row(
                label,
                &value,
                is_selected,
                app.ui.config_editing,
                app.ui.config_edit_cursor,
                t,
            )
        };
        lines.push(line);
        lines.push(Line::from(""));
    }
}

/// Tools tab 内容
fn draw_tab_tools_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
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
            super::components::TOGGLE_ON,
            enabled_count,
            total
        )
    } else {
        format!(
            "  \u{603b}\u{5f00}\u{5173}: {} \u{5173}\u{95ed}",
            super::components::TOGGLE_OFF
        )
    };
    lines.push(Line::from(vec![
        Span::styled(master_text, master_style),
        Span::styled("  (t \u{5207}\u{6362})", Style::default().fg(t.config_dim)),
    ]));
    lines.push(Line::from(""));

    for (i, name) in tool_names.iter().enumerate() {
        field_line_indices.push(lines.len());
        let is_selected = i == app.ui.config_field_idx;
        let is_enabled = !app
            .state
            .agent_config
            .disabled_tools
            .iter()
            .any(|d| d == *name);
        lines.push(toggle_list_item(
            name,
            is_enabled,
            is_selected,
            None,
            None,
            t,
        ));
        lines.push(Line::from(""));
    }
}

/// Skills tab 内容
fn draw_tab_skills_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
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

    for (i, skill) in app.state.loaded_skills.iter().enumerate() {
        field_line_indices.push(lines.len());
        let is_selected = i == app.ui.config_field_idx;
        let name = &skill.frontmatter.name;
        let is_enabled = !app
            .state
            .agent_config
            .disabled_skills
            .iter()
            .any(|d| d == name);
        lines.push(toggle_list_item(
            name,
            is_enabled,
            is_selected,
            Some(&skill.frontmatter.description),
            Some(skill.source.label()),
            t,
        ));
        lines.push(Line::from(""));
    }
}

/// Hooks tab（展示已注册的 hooks）
fn draw_tab_hooks_lines<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;
    let hooks = if let Ok(manager) = app.hook_manager.lock() {
        manager
            .list_hooks()
            .into_iter()
            .map(|(event, def, source)| (event, def.clone(), source.to_string()))
            .collect::<Vec<_>>()
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
            "  \u{7528}\u{6237}\u{7ea7}: ~/.jdata/agent/hooks.yaml",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(Span::styled(
            "  \u{9879}\u{76ee}\u{7ea7}: .jcli/hooks.yaml",
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

    for (event, def, source) in &hooks {
        let source_style = match source.as_str() {
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

        let cmd_display: String = if def.command.chars().count() > 40 {
            let truncated: String = def.command.chars().take(40).collect();
            format!("{truncated}...")
        } else {
            def.command.clone()
        };

        lines.push(Line::from(vec![
            Span::styled(format!("    [{:<7}]  ", source), source_style),
            Span::styled(
                format!("{:<22}  ", event.as_str()),
                Style::default().fg(t.config_label),
            ),
            Span::styled(cmd_display, Style::default().fg(t.config_value)),
            Span::styled(
                format!("  {}s", def.timeout),
                Style::default().fg(t.config_dim),
            ),
        ]));
    }
}

/// Commands tab 内容
fn draw_tab_commands_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
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
        return;
    }

    for (i, cmd) in app.state.loaded_commands.iter().enumerate() {
        field_line_indices.push(lines.len());
        let is_selected = i == app.ui.config_field_idx;
        let name = &cmd.frontmatter.name;
        let is_enabled = !app
            .state
            .agent_config
            .disabled_commands
            .iter()
            .any(|d| d == name);
        lines.push(toggle_list_item(
            name,
            is_enabled,
            is_selected,
            Some(&cmd.frontmatter.description),
            Some(cmd.source.label()),
            t,
        ));
        lines.push(Line::from(""));
    }
}

/// 格式化 Unix 时间戳为人类可读格式
fn format_timestamp(ts: u64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    let dt = UNIX_EPOCH + Duration::from_secs(ts);
    let now = SystemTime::now();
    let elapsed = now.duration_since(dt).unwrap_or_default();
    if elapsed.as_secs() < 60 {
        "\u{521a}\u{521a}".to_string()
    } else if elapsed.as_secs() < 3600 {
        format!("{}\u{5206}\u{949f}\u{524d}", elapsed.as_secs() / 60)
    } else if elapsed.as_secs() < 86400 {
        format!("{}\u{5c0f}\u{65f6}\u{524d}", elapsed.as_secs() / 3600)
    } else if elapsed.as_secs() < 86400 * 30 {
        format!("{}\u{5929}\u{524d}", elapsed.as_secs() / 86400)
    } else {
        let secs = ts;
        let days = secs / 86400;
        let (y, m, d) = days_to_ymd(days);
        format!("{y:04}-{m:02}-{d:02}")
    }
}

/// 将 1970-01-01 起的天数转为 (year, month, day)
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut y = 1970;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1;
    for &md in &month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        m += 1;
    }
    (y, m, remaining + 1)
}

fn is_leap(y: u64) -> bool {
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}

/// Archive tab 内容
fn draw_tab_archive_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
    let t = &app.ui.theme;

    // 确认还原覆盖层
    if app.ui.restore_confirm_needed {
        lines.push(Line::from(Span::styled(
            "  \u{26a0}\u{fe0f}  \u{5f53}\u{524d}\u{4f1a}\u{8bdd}\u{6709}\u{6d88}\u{606f}\u{ff0c}\u{8fd8}\u{539f}\u{5c06}\u{66ff}\u{6362}\u{5f53}\u{524d}\u{5bf9}\u{8bdd}\u{ff08}\u{5f53}\u{524d}\u{4f1a}\u{8bdd}\u{5df2}\u{81ea}\u{52a8}\u{4fdd}\u{5b58}\u{ff09}",
            Style::default()
                .fg(t.config_toggle_off)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "  \u{6309} y/Enter \u{786e}\u{8ba4}\u{8fd8}\u{539f}\u{ff0c}Esc \u{53d6}\u{6d88}",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(""));
    }

    if app.ui.archives.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (\u{6682}\u{65e0}\u{5f52}\u{6863})",
            Style::default().fg(t.config_dim),
        )));
        return;
    }

    lines.push(Line::from(Span::styled(
        format!(
            "  \u{5f52}\u{6863}\u{5217}\u{8868} ({})",
            app.ui.archives.len()
        ),
        Style::default()
            .fg(t.config_label)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (i, archive) in app.ui.archives.iter().enumerate() {
        field_line_indices.push(lines.len());
        let is_selected = i == app.ui.archive_list_index;
        let name_truncated: String = archive.name.chars().take(40).collect();
        let time_str = &archive.created_at;
        let secondary = format!("({} \u{6761}, {})", archive.messages.len(), time_str);
        lines.push(selectable_row(&name_truncated, &secondary, is_selected, t));
        lines.push(Line::from(""));
    }
}

/// Session tab 内容
fn draw_tab_session_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
    let t = &app.ui.theme;

    // 当前会话信息
    let msg_count = app.state.session.messages.len();
    lines.push(Line::from(vec![
        Span::styled(
            "  \u{5f53}\u{524d}\u{4f1a}\u{8bdd}: ",
            Style::default().fg(t.config_label),
        ),
        Span::styled(
            format!(
                "{} ({} \u{6761}\u{6d88}\u{606f})",
                &app.session_id, msg_count
            ),
            Style::default()
                .fg(t.config_toggle_on)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    // 确认恢复覆盖层
    if app.ui.session_restore_confirm {
        lines.push(Line::from(Span::styled(
            "  \u{26a0}\u{fe0f}  \u{5f53}\u{524d}\u{4f1a}\u{8bdd}\u{6709}\u{6d88}\u{606f}\u{ff0c}\u{6062}\u{590d}\u{5c06}\u{5207}\u{6362}\u{5230}\u{5386}\u{53f2}\u{4f1a}\u{8bdd}\u{ff08}\u{5f53}\u{524d}\u{4f1a}\u{8bdd}\u{5df2}\u{81ea}\u{52a8}\u{4fdd}\u{5b58}\u{ff09}",
            Style::default()
                .fg(t.config_toggle_off)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "  \u{6309} y/Enter \u{786e}\u{8ba4}\u{6062}\u{590d}\u{ff0c}Esc \u{53d6}\u{6d88}",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(""));
    }

    if app.ui.session_list.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (\u{6ca1}\u{6709}\u{5386}\u{53f2}\u{4f1a}\u{8bdd})",
            Style::default().fg(t.config_dim),
        )));
        return;
    }

    lines.push(Line::from(Span::styled(
        format!(
            "  \u{5386}\u{53f2}\u{4f1a}\u{8bdd} ({})",
            app.ui.session_list.len()
        ),
        Style::default()
            .fg(t.config_label)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (i, session) in app.ui.session_list.iter().enumerate() {
        field_line_indices.push(lines.len());
        let is_selected = i == app.ui.session_list_index;
        let preview = session
            .first_message_preview
            .as_deref()
            .unwrap_or("(\u{7a7a}\u{4f1a}\u{8bdd})");
        let preview_truncated: String = preview.chars().take(40).collect();
        let time_str = format_timestamp(session.updated_at);
        let secondary = format!("({} \u{6761}, {})", session.message_count, time_str);
        lines.push(selectable_row(
            &preview_truncated,
            &secondary,
            is_selected,
            t,
        ));
        lines.push(Line::from(""));
    }
}
