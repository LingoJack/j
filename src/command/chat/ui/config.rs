use super::super::handler::{config_field_label, config_field_value};
use crate::command::chat::app::ChatApp;
use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn draw_config_screen(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let t = &app.theme;
    let bg = t.bg_title;
    let total_provider_fields = CONFIG_FIELDS.len();

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    lines.push(Line::from(vec![Span::styled(
        "  ⚙️  模型配置",
        Style::default()
            .fg(t.config_title)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let provider_count = app.agent_config.providers.len();
    if provider_count > 0 {
        let mut tab_spans: Vec<Span> = vec![Span::styled("  ", Style::default())];
        for (i, p) in app.agent_config.providers.iter().enumerate() {
            let is_current = i == app.config_provider_idx;
            let is_active = i == app.agent_config.active_index;
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

    lines.push(Line::from(Span::styled(
        "  ─────────────────────────────────────────",
        Style::default().fg(t.separator),
    )));
    lines.push(Line::from(""));

    if provider_count > 0 {
        lines.push(Line::from(Span::styled(
            "  📦 Provider 配置",
            Style::default()
                .fg(t.config_section)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for (i, field) in CONFIG_FIELDS.iter().enumerate().take(total_provider_fields) {
            let is_selected = app.config_field_idx == i;
            let label = config_field_label(i);
            let value = if app.config_editing && is_selected {
                app.config_edit_buf.clone()
            } else {
                config_field_value(app, i)
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
            let value_style = if app.config_editing && is_selected {
                Style::default().fg(t.text_white).bg(t.config_edit_bg)
            } else if is_selected {
                Style::default().fg(t.text_white)
            } else if *field == "api_key" {
                Style::default().fg(t.config_api_key)
            } else {
                Style::default().fg(t.config_value)
            };

            lines.push(Line::from(if app.config_editing && is_selected {
                // 编辑模式：显示带光标的文本
                let mut spans = vec![
                    Span::styled(pointer, pointer_style),
                    Span::styled(format!("{:<10}", label), label_style),
                    Span::styled("  ", Style::default()),
                ];
                let chars: Vec<char> = value.chars().collect();
                let cursor = app.config_edit_cursor;
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
                spans.push(Span::styled(before, value_style));
                spans.push(Span::styled(
                    cursor_ch,
                    Style::default().fg(t.cursor_fg).bg(t.cursor_bg),
                ));
                spans.push(Span::styled(after, value_style));
                spans.push(Span::styled(" ✏️", Style::default()));
                spans
            } else {
                vec![
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
                ]
            }));
        }
    }

    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(
        "  🌐 全局配置",
        Style::default()
            .fg(t.config_section)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (i, field) in CONFIG_GLOBAL_FIELDS.iter().enumerate() {
        let field_idx = total_provider_fields + i;
        let is_selected = app.config_field_idx == field_idx;
        let label = config_field_label(field_idx);
        let value = if app.config_editing && is_selected {
            app.config_edit_buf.clone()
        } else {
            config_field_value(app, field_idx)
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
        let value_style = if app.config_editing && is_selected {
            Style::default().fg(t.text_white).bg(t.config_edit_bg)
        } else if is_selected {
            Style::default().fg(t.text_white)
        } else {
            Style::default().fg(t.config_value)
        };

        if *field == "stream_mode" {
            let toggle_on = app.agent_config.stream_mode;
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
            lines.push(Line::from(vec![
                Span::styled(pointer, pointer_style),
                Span::styled(format!("{:<10}", label), label_style),
                Span::styled("  ", Style::default()),
                Span::styled(toggle_text, toggle_style),
                Span::styled(
                    if is_selected { "  (Enter 切换)" } else { "" },
                    Style::default().fg(t.config_dim),
                ),
            ]));
        } else if *field == "theme" {
            let theme_name = app.agent_config.theme.display_name();
            lines.push(Line::from(vec![
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
            ]));
        } else if *field == "tools_enabled" {
            let toggle_on = app.agent_config.tools_enabled;
            let tool_names = app.tool_registry.tool_names();
            let total = tool_names.len();
            let enabled_count = total
                - app
                    .agent_config
                    .disabled_tools
                    .iter()
                    .filter(|d| tool_names.iter().any(|n| *n == d.as_str()))
                    .count();
            let toggle_style = if toggle_on {
                Style::default()
                    .fg(t.config_toggle_on)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.config_toggle_off)
            };
            let toggle_text = if toggle_on {
                format!("● 开启 ({}/{})", enabled_count, total)
            } else {
                "○ 关闭".to_string()
            };
            lines.push(Line::from(vec![
                Span::styled(pointer, pointer_style),
                Span::styled(format!("{:<10}", label), label_style),
                Span::styled("  ", Style::default()),
                Span::styled(toggle_text, toggle_style),
                Span::styled(
                    if is_selected { "  (Enter 设置)" } else { "" },
                    Style::default().fg(t.config_dim),
                ),
            ]));
        } else if *field == "system_prompt" {
            // system_prompt 特殊处理：截断显示 + Enter 弹出全屏编辑器
            let display_value = if value.is_empty() {
                "(空)".to_string()
            } else {
                // 截断到 40 个字符，替换换行为空格
                let flat: String = value
                    .chars()
                    .map(|c| if c == '\n' { ' ' } else { c })
                    .collect();
                if flat.chars().count() > 40 {
                    let truncated: String = flat.chars().take(40).collect();
                    format!("{}...", truncated)
                } else {
                    flat
                }
            };
            lines.push(Line::from(vec![
                Span::styled(pointer, pointer_style),
                Span::styled(format!("{:<10}", label), label_style),
                Span::styled("  ", Style::default()),
                Span::styled(display_value, value_style),
                Span::styled(
                    if is_selected { "  (Enter 编辑)" } else { "" },
                    Style::default().fg(t.config_dim),
                ),
            ]));
        } else if *field == "style" {
            // style 特殊处理：同 system_prompt 模式
            let display_value = if value.is_empty() {
                "(空)".to_string()
            } else {
                let flat: String = value
                    .chars()
                    .map(|c| if c == '\n' { ' ' } else { c })
                    .collect();
                if flat.chars().count() > 40 {
                    let truncated: String = flat.chars().take(40).collect();
                    format!("{}...", truncated)
                } else {
                    flat
                }
            };
            lines.push(Line::from(vec![
                Span::styled(pointer, pointer_style),
                Span::styled(format!("{:<10}", label), label_style),
                Span::styled("  ", Style::default()),
                Span::styled(display_value, value_style),
                Span::styled(
                    if is_selected { "  (Enter 编辑)" } else { "" },
                    Style::default().fg(t.config_dim),
                ),
            ]));
        } else {
            lines.push(Line::from(if app.config_editing && is_selected {
                // 编辑模式：显示带光标的文本
                let mut spans = vec![
                    Span::styled(pointer, pointer_style),
                    Span::styled(format!("{:<10}", label), label_style),
                    Span::styled("  ", Style::default()),
                ];
                let chars: Vec<char> = value.chars().collect();
                let cursor = app.config_edit_cursor;
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
                spans.push(Span::styled(before, value_style));
                spans.push(Span::styled(
                    cursor_ch,
                    Style::default().fg(t.cursor_fg).bg(t.cursor_bg),
                ));
                spans.push(Span::styled(after, value_style));
                spans.push(Span::styled(" ✏️", Style::default()));
                spans
            } else {
                vec![
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
                ]
            }));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(
        "  ─────────────────────────────────────────",
        Style::default().fg(t.separator),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled(
            "↑↓/jk",
            Style::default()
                .fg(t.config_hint_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 切换字段  ", Style::default().fg(t.config_hint_desc)),
        Span::styled(
            "Enter",
            Style::default()
                .fg(t.config_hint_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 编辑  ", Style::default().fg(t.config_hint_desc)),
        Span::styled(
            "Tab/←→",
            Style::default()
                .fg(t.config_hint_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 切换 Provider  ", Style::default().fg(t.config_hint_desc)),
        Span::styled(
            "a",
            Style::default()
                .fg(t.config_hint_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 新增  ", Style::default().fg(t.config_hint_desc)),
        Span::styled(
            "d",
            Style::default()
                .fg(t.config_hint_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 删除  ", Style::default().fg(t.config_hint_desc)),
        Span::styled(
            "s",
            Style::default()
                .fg(t.config_hint_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 设为活跃  ", Style::default().fg(t.config_hint_desc)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(t.config_hint_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 保存返回", Style::default().fg(t.config_hint_desc)),
    ]));

    let content = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(t.border_config))
                .title(Span::styled(
                    " ⚙️  模型配置编辑 ",
                    Style::default()
                        .fg(t.config_label_selected)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(bg)),
        )
        .scroll((0, 0));
    f.render_widget(content, area);
}

pub fn draw_tool_toggle(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let t = &app.theme;
    let bg = t.bg_title;
    let tool_names = app.tool_registry.tool_names();
    let total = tool_names.len();
    let enabled_count = total
        - app
            .agent_config
            .disabled_tools
            .iter()
            .filter(|d| tool_names.iter().any(|n| *n == d.as_str()))
            .count();

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    lines.push(Line::from(vec![Span::styled(
        "  🔧 工具开关",
        Style::default()
            .fg(t.config_title)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    // 总开关状态
    let master_style = if app.agent_config.tools_enabled {
        Style::default()
            .fg(t.config_toggle_on)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.config_toggle_off)
    };
    let master_text = if app.agent_config.tools_enabled {
        format!("  总开关: ● 开启 ({}/{})", enabled_count, total)
    } else {
        "  总开关: ○ 关闭".to_string()
    };
    lines.push(Line::from(vec![
        Span::styled(master_text, master_style),
        Span::styled("  (t 切换)", Style::default().fg(t.config_dim)),
    ]));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(
        "  ─────────────────────────────────────────",
        Style::default().fg(t.separator),
    )));
    lines.push(Line::from(""));

    for (i, name) in tool_names.iter().enumerate() {
        let is_selected = i == app.tool_toggle_index;
        let is_disabled = app.agent_config.disabled_tools.iter().any(|d| d == *name);
        let is_enabled = !is_disabled;

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

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ─────────────────────────────────────────",
        Style::default().fg(t.separator),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled(
            "↑↓/jk",
            Style::default()
                .fg(t.config_hint_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 选择  ", Style::default().fg(t.config_hint_desc)),
        Span::styled(
            "Enter/空格",
            Style::default()
                .fg(t.config_hint_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 切换  ", Style::default().fg(t.config_hint_desc)),
        Span::styled(
            "t",
            Style::default()
                .fg(t.config_hint_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 总开关  ", Style::default().fg(t.config_hint_desc)),
        Span::styled(
            "a",
            Style::default()
                .fg(t.config_hint_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 全部启用  ", Style::default().fg(t.config_hint_desc)),
        Span::styled(
            "d",
            Style::default()
                .fg(t.config_hint_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 全部禁用  ", Style::default().fg(t.config_hint_desc)),
        Span::styled(
            "Esc",
            Style::default()
                .fg(t.config_hint_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 返回", Style::default().fg(t.config_hint_desc)),
    ]));

    let content = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(t.border_config))
                .title(Span::styled(
                    " 🔧 工具开关设置 ",
                    Style::default()
                        .fg(t.config_label_selected)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(bg)),
        )
        .scroll((0, 0));
    f.render_widget(content, area);
}
