use super::super::ui_helpers::{
    config_field_label_global, config_field_label_model, config_field_value_global,
    config_field_value_model,
};
use crate::command::chat::app::{ChatApp, ConfigTab};
use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS_TAB};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// 构建"带行内光标的可编辑字段"span 列表
fn render_cursor_spans<'a>(
    value: &str,
    cursor: usize,
    value_style: Style,
    cursor_fg: ratatui::style::Color,
    cursor_bg: ratatui::style::Color,
) -> Vec<Span<'a>> {
    let chars: Vec<char> = value.chars().collect();
    let before: String = chars[..cursor.min(chars.len())].iter().collect();
    let cursor_ch = if cursor < chars.len() {
        chars[cursor].to_string()
    } else {
        " ".to_string()
    };
    let after: String = if cursor < chars.len() {
        chars[cursor + 1..].iter().collect()
    } else {
        String::new()
    };
    vec![
        Span::styled(before, value_style),
        Span::styled(cursor_ch, Style::default().fg(cursor_fg).bg(cursor_bg)),
        Span::styled(after, value_style),
        Span::styled(" ✏️", Style::default()),
    ]
}

/// 构建长文本字段的截断预览值（替换换行为空格，超 40 字符截断）
fn render_preview_value(raw: &str) -> String {
    if raw.is_empty() {
        return "(空)".to_string();
    }
    let flat: String = raw
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    if flat.chars().count() > 40 {
        let truncated: String = flat.chars().take(40).collect();
        format!("{}...", truncated)
    } else {
        flat
    }
}

/// 绘制顶部 Tab 栏（支持窄屏水平滚动）
fn draw_tab_bar<'a>(app: &ChatApp) -> Line<'a> {
    let t = &app.ui.theme;
    let current = app.ui.config_tab;
    let all_tabs = [
        ConfigTab::Model,
        ConfigTab::Global,
        ConfigTab::Tools,
        ConfigTab::Skills,
        ConfigTab::Hooks,
        ConfigTab::Commands,
    ];

    let mut spans: Vec<Span<'a>> = vec![Span::styled("  ", Style::default())];

    for (i, tab) in all_tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(t.separator)));
        }
        let label = format!(" {} ", tab.label());
        if *tab == current {
            spans.push(Span::styled(
                label,
                Style::default()
                    .fg(t.config_tab_active_fg)
                    .bg(t.config_tab_active_bg)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                label,
                Style::default().fg(t.config_tab_inactive),
            ));
        }
    }

    spans.push(Span::styled(
        "    (←→ 切换标签)",
        Style::default().fg(t.config_dim),
    ));

    Line::from(spans)
}

/// 配置界面主入口（分发器）
pub fn draw_config_screen(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let t = &app.ui.theme;
    let bg = t.bg_title;

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        draw_tab_bar(app),
        Line::from(""),
        Line::from(Span::styled(
            "  ─────────────────────────────────────────",
            Style::default().fg(t.separator),
        )),
        Line::from(""),
    ];

    let mut field_line_indices: Vec<usize> = Vec::new();

    match app.ui.config_tab {
        ConfigTab::Model => {
            draw_tab_model_lines(&mut lines, &mut field_line_indices, app);
        }
        ConfigTab::Global => {
            draw_tab_global_lines(&mut lines, &mut field_line_indices, app);
        }
        ConfigTab::Tools => {
            draw_tab_tools_lines(&mut lines, &mut field_line_indices, app);
        }
        ConfigTab::Skills => {
            draw_tab_skills_lines(&mut lines, &mut field_line_indices, app);
        }
        ConfigTab::Hooks => {
            draw_tab_hooks_lines(&mut lines, app);
        }
        ConfigTab::Commands => {
            draw_tab_commands_lines(&mut lines, app);
        }
    }

    // 滚动：确保选中字段始终可见
    let inner_height = area.height.saturating_sub(2) as usize;
    if let Some(&selected_line) = field_line_indices.get(app.ui.config_field_idx) {
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
        ConfigTab::Model => " ⚙️  模型配置 ",
        ConfigTab::Global => " 🌐 全局配置 ",
        ConfigTab::Tools => " 🔧 工具开关 ",
        ConfigTab::Skills => " 📦 技能开关 ",
        ConfigTab::Hooks => " 🪝 Hooks ",
        ConfigTab::Commands => " 📋 自定义命令 ",
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
        let mut tab_spans: Vec<Span> = vec![Span::styled("  ", Style::default())];
        for (i, p) in app.state.agent_config.providers.iter().enumerate() {
            let is_current = i == app.ui.config_provider_idx;
            let is_active = i == app.state.agent_config.active_index;
            let marker = if is_active { "● " } else { "○ " };
            let label = format!(" {}{} ", marker, p.name);
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
                tab_spans.push(Span::styled(" │ ", Style::default().fg(t.separator)));
            }
        }
        tab_spans.push(Span::styled(
            "    (● = 活跃模型, Tab 切换, s 设为活跃)",
            Style::default().fg(t.config_dim),
        ));
        lines.push(Line::from(tab_spans));
    } else {
        lines.push(Line::from(Span::styled(
            "  (无 Provider，按 a 新增)",
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

            let pointer = if is_selected { "  ▸ " } else { "    " };
            let pointer_style = if is_selected {
                Style::default().fg(t.config_pointer)
            } else {
                Style::default()
            };
            let label_style = if is_selected {
                Style::default()
                    .fg(t.config_label_selected)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.config_label)
            };
            let value_style = if app.ui.config_editing && is_selected {
                Style::default().fg(t.text_white).bg(t.config_edit_bg)
            } else if is_selected {
                Style::default().fg(t.text_white)
            } else if *provider_field == "api_key" {
                Style::default().fg(t.config_api_key)
            } else {
                Style::default().fg(t.config_value)
            };

            let line = if app.ui.config_editing && is_selected {
                let mut spans = vec![
                    Span::styled(pointer, pointer_style),
                    Span::styled(format!("{:<10}", label), label_style),
                    Span::styled("  ", Style::default()),
                ];
                spans.extend(render_cursor_spans(
                    &value,
                    app.ui.config_edit_cursor,
                    value_style,
                    t.cursor_fg,
                    t.cursor_bg,
                ));
                Line::from(spans)
            } else {
                Line::from(vec![
                    Span::styled(pointer, pointer_style),
                    Span::styled(format!("{:<10}", label), label_style),
                    Span::styled("  ", Style::default()),
                    Span::styled(
                        if value.is_empty() {
                            "(空)".to_string()
                        } else {
                            value
                        },
                        value_style,
                    ),
                ])
            };
            lines.push(line);
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

        let pointer = if is_selected { "  ▸ " } else { "    " };
        let pointer_style = if is_selected {
            Style::default().fg(t.config_pointer)
        } else {
            Style::default()
        };
        let label_style = if is_selected {
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.config_label)
        };
        let value_style = if app.ui.config_editing && is_selected {
            Style::default().fg(t.text_white).bg(t.config_edit_bg)
        } else if is_selected {
            Style::default().fg(t.text_white)
        } else {
            Style::default().fg(t.config_value)
        };

        let line = if *field == "stream_mode" {
            let toggle_on = app.state.agent_config.stream_mode;
            let toggle_style = if toggle_on {
                Style::default()
                    .fg(t.config_toggle_on)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.config_toggle_off)
            };
            let toggle_text = if toggle_on {
                "● 开启"
            } else {
                "○ 关闭"
            };
            Line::from(vec![
                Span::styled(pointer, pointer_style),
                Span::styled(format!("{:<10}", label), label_style),
                Span::styled("  ", Style::default()),
                Span::styled(toggle_text, toggle_style),
                Span::styled(
                    if is_selected { "  (Enter 切换)" } else { "" },
                    Style::default().fg(t.config_dim),
                ),
            ])
        } else if *field == "theme" {
            let theme_name = app.state.agent_config.theme.display_name();
            Line::from(vec![
                Span::styled(pointer, pointer_style),
                Span::styled(format!("{:<10}", label), label_style),
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("🎨 {}", theme_name),
                    Style::default()
                        .fg(t.config_toggle_on)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    if is_selected { "  (Enter 切换)" } else { "" },
                    Style::default().fg(t.config_dim),
                ),
            ])
        } else if *field == "system_prompt" || *field == "style" {
            Line::from(vec![
                Span::styled(pointer, pointer_style),
                Span::styled(format!("{:<10}", label), label_style),
                Span::styled("  ", Style::default()),
                Span::styled(render_preview_value(&value), value_style),
                Span::styled(
                    if is_selected {
                        "  (Enter 编辑)".to_string()
                    } else {
                        String::new()
                    },
                    Style::default().fg(t.config_dim),
                ),
            ])
        } else if app.ui.config_editing && is_selected {
            let mut spans = vec![
                Span::styled(pointer, pointer_style),
                Span::styled(format!("{:<10}", label), label_style),
                Span::styled("  ", Style::default()),
            ];
            spans.extend(render_cursor_spans(
                &value,
                app.ui.config_edit_cursor,
                value_style,
                t.cursor_fg,
                t.cursor_bg,
            ));
            Line::from(spans)
        } else {
            Line::from(vec![
                Span::styled(pointer, pointer_style),
                Span::styled(format!("{:<10}", label), label_style),
                Span::styled("  ", Style::default()),
                Span::styled(
                    if value.is_empty() {
                        "(空)".to_string()
                    } else {
                        value
                    },
                    value_style,
                ),
            ])
        };
        lines.push(line);
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
        format!("  总开关: ● 开启 ({}/{})", enabled_count, total)
    } else {
        "  总开关: ○ 关闭".to_string()
    };
    lines.push(Line::from(vec![
        Span::styled(master_text, master_style),
        Span::styled("  (t 切换)", Style::default().fg(t.config_dim)),
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

        let pointer = if is_selected { "  ▸ " } else { "    " };
        let pointer_style = if is_selected {
            Style::default().fg(t.config_pointer)
        } else {
            Style::default()
        };
        let toggle_style = if is_enabled {
            Style::default()
                .fg(t.config_toggle_on)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.config_toggle_off)
        };
        let toggle_text = if is_enabled { "●" } else { "○" };
        let name_style = if is_selected {
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.config_label)
        };

        lines.push(Line::from(vec![
            Span::styled(pointer, pointer_style),
            Span::styled(toggle_text, toggle_style),
            Span::styled(" ", Style::default()),
            Span::styled(name.to_string(), name_style),
        ]));
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
        format!("  已启用: {}/{}", enabled_count, total),
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

        let pointer = if is_selected { "  ▸ " } else { "    " };
        let pointer_style = if is_selected {
            Style::default().fg(t.config_pointer)
        } else {
            Style::default()
        };
        let toggle_style = if is_enabled {
            Style::default()
                .fg(t.config_toggle_on)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.config_toggle_off)
        };
        let toggle_text = if is_enabled { "●" } else { "○" };
        let name_style = if is_selected {
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.config_label)
        };

        lines.push(Line::from(vec![
            Span::styled(pointer, pointer_style),
            Span::styled(toggle_text, toggle_style),
            Span::styled(" ", Style::default()),
            Span::styled(name.to_string(), name_style),
            Span::styled(
                format!("  {}", skill.frontmatter.description),
                Style::default().fg(t.config_dim),
            ),
        ]));
    }
}

/// Hooks tab（占位）
fn draw_tab_hooks_lines<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  🪝 Hooks（即将推出）",
        Style::default().fg(t.config_dim),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Hook 系统允许你在特定事件发生时自动执行自定义操作。",
        Style::default().fg(t.config_dim),
    )));
}

/// Commands tab（占位）
fn draw_tab_commands_lines<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  📋 自定义命令（即将推出）",
        Style::default().fg(t.config_dim),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  自定义命令允许你创建常用操作的快捷方式。",
        Style::default().fg(t.config_dim),
    )));
}
