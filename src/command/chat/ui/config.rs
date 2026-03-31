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
        ConfigTab::Session,
        ConfigTab::Global,
        ConfigTab::Tools,
        ConfigTab::Skills,
        ConfigTab::Hooks,
        ConfigTab::Commands,
        ConfigTab::Archive,
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
        ConfigTab::Session => {
            draw_tab_session_lines(&mut lines, &mut field_line_indices, app);
        }
        ConfigTab::Archive => {
            draw_tab_archive_lines(&mut lines, &mut field_line_indices, app);
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
        ConfigTab::Model => " ⚙️ 模型配置 ",
        ConfigTab::Global => " 🌐 全局配置 ",
        ConfigTab::Tools => " 🔧 工具开关 ",
        ConfigTab::Skills => " 📦 技能开关 ",
        ConfigTab::Hooks => " 🪝 Hooks ",
        ConfigTab::Commands => " 📋 自定义命令 ",
        ConfigTab::Session => " 💬 会话管理 ",
        ConfigTab::Archive => " 📦 归档管理 ",
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

        let line = if *field == "stream_mode" || *field == "auto_restore_session" {
            let toggle_on = match *field {
                "stream_mode" => app.state.agent_config.stream_mode,
                "auto_restore_session" => app.state.agent_config.auto_restore_session,
                _ => false,
            };
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
            "  (暂无 hooks)",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  用户级: ~/.jdata/agent/hooks.yaml",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(Span::styled(
            "  项目级: .jcli/hooks.yaml",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(Span::styled(
            "  运行时: 通过 RegisterHook 工具注册",
            Style::default().fg(t.config_dim),
        )));
        return;
    }

    lines.push(Line::from(Span::styled(
        format!("  🪝 已注册 Hooks ({})", hooks.len()),
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
            format!("{}...", truncated)
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

/// 格式化 Unix 时间戳为人类可读格式
fn format_timestamp(ts: u64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    let dt = UNIX_EPOCH + Duration::from_secs(ts);
    let now = SystemTime::now();
    let elapsed = now.duration_since(dt).unwrap_or_default();
    if elapsed.as_secs() < 60 {
        "刚刚".to_string()
    } else if elapsed.as_secs() < 3600 {
        format!("{}分钟前", elapsed.as_secs() / 60)
    } else if elapsed.as_secs() < 86400 {
        format!("{}小时前", elapsed.as_secs() / 3600)
    } else if elapsed.as_secs() < 86400 * 30 {
        format!("{}天前", elapsed.as_secs() / 86400)
    } else {
        // 使用简单日期格式
        let secs = ts;
        let days = secs / 86400;
        // 简单计算：1970-01-01 起的天数转日期
        let (y, m, d) = days_to_ymd(days);
        format!("{:04}-{:02}-{:02}", y, m, d)
    }
}

/// 将 1970-01-01 起的天数转为 (year, month, day)
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // 简单算法，足够展示用途
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
            "  ⚠️  当前会话有消息，还原将替换当前对话（当前会话已自动保存）",
            Style::default()
                .fg(t.config_toggle_off)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "  按 y/Enter 确认还原，Esc 取消",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(""));
    }

    if app.ui.archives.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (暂无归档)",
            Style::default().fg(t.config_dim),
        )));
        return;
    }

    lines.push(Line::from(Span::styled(
        format!("  归档列表 ({})", app.ui.archives.len()),
        Style::default()
            .fg(t.config_label)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (i, archive) in app.ui.archives.iter().enumerate() {
        field_line_indices.push(lines.len());
        let is_selected = i == app.ui.archive_list_index;

        let pointer = if is_selected { "  ▸ " } else { "    " };
        let pointer_style = if is_selected {
            Style::default().fg(t.config_pointer)
        } else {
            Style::default()
        };

        let name_style = if is_selected {
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.config_label)
        };

        let name_truncated: String = archive.name.chars().take(40).collect();
        let time_str = &archive.created_at;

        lines.push(Line::from(vec![
            Span::styled(pointer, pointer_style),
            Span::styled(name_truncated, name_style),
            Span::styled(
                format!("  ({} 条, {})", archive.messages.len(), time_str),
                Style::default().fg(t.config_dim),
            ),
        ]));
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
        Span::styled("  当前会话: ", Style::default().fg(t.config_label)),
        Span::styled(
            format!("{} ({} 条消息)", &app.session_id, msg_count),
            Style::default()
                .fg(t.config_toggle_on)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    // 确认恢复覆盖层
    if app.ui.session_restore_confirm {
        lines.push(Line::from(Span::styled(
            "  ⚠️  当前会话有消息，恢复将切换到历史会话（当前会话已自动保存）",
            Style::default()
                .fg(t.config_toggle_off)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "  按 y/Enter 确认恢复，Esc 取消",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(""));
    }

    if app.ui.session_list.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (没有历史会话)",
            Style::default().fg(t.config_dim),
        )));
        return;
    }

    lines.push(Line::from(Span::styled(
        format!("  历史会话 ({})", app.ui.session_list.len()),
        Style::default()
            .fg(t.config_label)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (i, session) in app.ui.session_list.iter().enumerate() {
        field_line_indices.push(lines.len());
        let is_selected = i == app.ui.session_list_index;

        let pointer = if is_selected { "  ▸ " } else { "    " };
        let pointer_style = if is_selected {
            Style::default().fg(t.config_pointer)
        } else {
            Style::default()
        };

        let preview = session
            .first_message_preview
            .as_deref()
            .unwrap_or("(空会话)");
        let preview_truncated: String = preview.chars().take(40).collect();
        let time_str = format_timestamp(session.updated_at);

        let name_style = if is_selected {
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.config_label)
        };

        lines.push(Line::from(vec![
            Span::styled(pointer, pointer_style),
            Span::styled(preview_truncated, name_style),
            Span::styled(
                format!("  ({} 条, {})", session.message_count, time_str),
                Style::default().fg(t.config_dim),
            ),
        ]));
    }
}
