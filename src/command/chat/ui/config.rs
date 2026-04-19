use super::super::ui_helpers::{
    config_field_desc_global, config_field_label_global, config_field_label_model,
    config_field_value_global, config_field_value_model,
};
use super::components::{
    global_preview_row, global_text_row, global_theme_row, global_toggle_row, secret_field_row,
    selectable_row, separator_line, tab_bar, text_field_row, toggle_list_item, toggle_row,
};
use crate::command::chat::app::{ChatApp, ConfigTab};
use crate::command::chat::teammate::TeammateStatus;
use crate::command::chat::tools::derived_shared::SubAgentStatus;
use crate::constants::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS_TAB};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
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
        ConfigTab::Teammates,
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
///
/// 将面板拆分为三层：
///   1. 固定头部：边框标题 + Tab 栏 + 分隔线
///   2. 固定 Tab 头部：每个 Tab 自身的摘要信息（如"当前会话"、"总开关"等）
///   3. 可滚动列表：只有列表项跟随选中项滚动
pub fn draw_config_screen(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    // ── Model Tab: 调整 Provider 水平滚动偏移（在不可变借用之前）──
    if app.ui.config_tab == ConfigTab::Model {
        adjust_provider_scroll_offset(app, area.width.saturating_sub(2) as usize);
    }

    let t = &app.ui.theme;
    let bg = t.bg_primary;

    let title = match app.ui.config_tab {
        ConfigTab::Model => " \u{2699}\u{fe0f} \u{6a21}\u{578b}\u{914d}\u{7f6e} ",
        ConfigTab::Global => " \u{1f310} \u{5168}\u{5c40}\u{914d}\u{7f6e} ",
        ConfigTab::Tools => " \u{1f527} \u{5de5}\u{5177}\u{5f00}\u{5173} ",
        ConfigTab::Skills => " \u{1f4e6} \u{6280}\u{80fd}\u{5f00}\u{5173} ",
        ConfigTab::Hooks => " \u{1fa9d} Hooks ",
        ConfigTab::Commands => " \u{1f4cb} \u{81ea}\u{5b9a}\u{4e49}\u{547d}\u{4ee4} ",
        ConfigTab::Session => " \u{1f4ac} \u{4f1a}\u{8bdd}\u{7ba1}\u{7406} ",
        ConfigTab::Teammates => " Teammates ",
        ConfigTab::Archive => " \u{1f4e6} \u{5f52}\u{6863}\u{7ba1}\u{7406} ",
    };

    // ── 收集每个 Tab 的固定头部行和可滚动列表行 ──
    let mut tab_header_lines: Vec<Line> = Vec::new();
    let mut list_lines: Vec<Line> = Vec::new();
    let mut field_line_indices: Vec<usize> = Vec::new();

    match app.ui.config_tab {
        ConfigTab::Model => {
            draw_tab_model_header(&mut tab_header_lines, app, area.width.saturating_sub(2));
            draw_tab_model_list(&mut list_lines, &mut field_line_indices, app);
        }
        ConfigTab::Global => {
            // Global 没有固定头部，全部是字段列表
            draw_tab_global_lines(&mut list_lines, &mut field_line_indices, app);
        }
        ConfigTab::Tools => {
            draw_tab_tools_header(&mut tab_header_lines, app);
            draw_tab_tools_list(&mut list_lines, &mut field_line_indices, app);
        }
        ConfigTab::Skills => {
            draw_tab_skills_header(&mut tab_header_lines, app);
            draw_tab_skills_list(&mut list_lines, &mut field_line_indices, app);
        }
        ConfigTab::Hooks => {
            // Hooks 没有可选列表，全部固定展示
            draw_tab_hooks_lines(&mut tab_header_lines, app);
        }
        ConfigTab::Commands => {
            draw_tab_commands_header(&mut tab_header_lines, app);
            draw_tab_commands_list(&mut list_lines, &mut field_line_indices, app);
        }
        ConfigTab::Teammates => {
            draw_tab_teammates_header(&mut tab_header_lines, app);
            draw_tab_teammates_list(&mut list_lines, &mut field_line_indices, app);
        }
        ConfigTab::Session => {
            draw_tab_session_header(&mut tab_header_lines, app);
            draw_tab_session_list(&mut list_lines, &mut field_line_indices, app);
        }
        ConfigTab::Archive => {
            draw_tab_archive_header(&mut tab_header_lines, app);
            draw_tab_archive_list(&mut list_lines, &mut field_line_indices, app);
        }
    }

    // Tab 栏行数: 空行 + tab_bar + 空行 + separator = 4
    let tab_bar_lines: u16 = 4;
    // 顶部 border 占 1 行
    let top_border: u16 = 1;
    let tab_header_h = tab_header_lines.len() as u16;
    let fixed_h = top_border + tab_bar_lines + tab_header_h;

    // 如果没有可滚动列表，或终端太小，回退到整体渲染
    if list_lines.is_empty() || area.height <= fixed_h + 1 {
        let mut all_lines: Vec<Line> = vec![
            Line::from(""),
            draw_tab_bar_line(app),
            Line::from(""),
            separator_line(area.width, t),
        ];
        all_lines.append(&mut tab_header_lines);
        all_lines.append(&mut list_lines);
        let widget = Paragraph::new(all_lines)
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
        f.render_widget(widget, area);
        return;
    }

    // ── 三段布局 ──
    let header_area_h = fixed_h; // 顶部 border + tab_bar + tab_header
    let list_area_h = area.height.saturating_sub(header_area_h);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_area_h),
            Constraint::Min(list_area_h),
        ])
        .split(area);

    // ── 固定头部：顶部边框 + 标题 + Tab 栏 + Tab 专属头部 ──
    let mut header_lines: Vec<Line> = vec![
        Line::from(""),
        draw_tab_bar_line(app),
        Line::from(""),
        separator_line(area.width, t),
    ];
    header_lines.append(&mut tab_header_lines);

    let header_block = Block::default()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(t.border_config))
        .title(Span::styled(
            title,
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(bg));
    let header_widget = Paragraph::new(header_lines).block(header_block);
    f.render_widget(header_widget, chunks[0]);

    // ── 可滚动列表区域 ──
    // 可见高度 = list_area_h - 1（底部 border）
    let inner_height = list_area_h.saturating_sub(1) as usize;
    let selected_idx = match app.ui.config_tab {
        ConfigTab::Session => app.ui.session_list_index,
        ConfigTab::Archive => app.ui.archive_list_index,
        ConfigTab::Teammates => app.ui.teammate_list_index,
        ConfigTab::Global if app.ui.compact_exempt_sublist => app.ui.compact_exempt_idx,
        _ => app.ui.config_field_idx,
    };
    if let Some(&selected_line) = field_line_indices.get(selected_idx) {
        let scroll = app.ui.config_scroll_offset as usize;
        let new_scroll = if selected_line < scroll {
            selected_line
        } else if inner_height > 0 && selected_line >= scroll + inner_height {
            selected_line.saturating_sub(inner_height - 1)
        } else {
            scroll
        };
        app.ui.config_scroll_offset = new_scroll as u16;
    }

    let list_block = Block::default()
        .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(t.border_config))
        .style(Style::default().bg(bg));
    let list_widget = Paragraph::new(list_lines)
        .block(list_block)
        .scroll((app.ui.config_scroll_offset, 0));
    f.render_widget(list_widget, chunks[1]);
}

/// 根据 config_provider_idx 和可用宽度，自动调整 config_provider_scroll_offset
/// 确保当前选中的 provider 标签始终在可见范围内
fn adjust_provider_scroll_offset(app: &mut ChatApp, max_width: usize) {
    let provider_count = app.state.agent_config.providers.len();
    if provider_count == 0 {
        app.ui.config_provider_scroll_offset = 0;
        return;
    }

    let indent: usize = 2; // "  " 前缀
    let sep_width: usize = 3; // " │ "
    let selected_idx = app.ui.config_provider_idx;

    // 计算每个 provider 标签宽度
    let tab_widths: Vec<usize> = app
        .state
        .agent_config
        .providers
        .iter()
        .map(|p| 4 + p.name.chars().count())
        .collect();

    // 计算所有标签总宽度
    let total_tabs_width: usize =
        tab_widths.iter().sum::<usize>() + sep_width * provider_count.saturating_sub(1);

    // 不需要滚动
    if indent + total_tabs_width <= max_width {
        app.ui.config_provider_scroll_offset = 0;
        return;
    }

    // 需要滚动：从当前 scroll_offset 开始，计算可见范围
    let scroll_offset = app.ui.config_provider_scroll_offset.min(selected_idx);
    let mut used_width = indent;
    let mut visible_end = scroll_offset;

    for (i, &tw) in tab_widths.iter().enumerate().skip(scroll_offset) {
        let w = tw + if i > scroll_offset { sep_width } else { 0 };
        if used_width + w > max_width {
            break;
        }
        used_width += w;
        visible_end = i + 1;
    }

    // 如果 selected 在可见范围内，无需调整
    if selected_idx >= scroll_offset && selected_idx < visible_end {
        app.ui.config_provider_scroll_offset = scroll_offset;
        return;
    }

    // selected 不在可见范围——重新计算 scroll_offset
    let new_offset = if selected_idx < scroll_offset {
        // 在当前窗口左侧，直接跳到 selected
        selected_idx
    } else {
        // 在当前窗口右侧，从 selected 往前回推
        let mut w = indent;
        let mut start = selected_idx;
        for i in (0..=selected_idx).rev() {
            let tw = tab_widths[i] + if i < selected_idx { sep_width } else { 0 };
            if w + tw > max_width {
                start = i + 1;
                break;
            }
            w += tw;
            start = i;
        }
        start
    };

    app.ui.config_provider_scroll_offset = new_offset;
}

/// Model tab 固定头部（Provider sub-tabs，支持水平滚动）
fn draw_tab_model_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp, available_width: u16) {
    let t = &app.ui.theme;

    let provider_count = app.state.agent_config.providers.len();
    if provider_count > 0 {
        let sep_width: usize = 3; // " │ "
        let tab_widths: Vec<usize> = app
            .state
            .agent_config
            .providers
            .iter()
            .map(|p| 4 + p.name.chars().count())
            .collect();

        // 计算所有标签总宽度（含分隔符）
        let total_tabs_width: usize =
            tab_widths.iter().sum::<usize>() + sep_width * provider_count.saturating_sub(1);

        let max_content_width = available_width as usize;
        let indent: usize = 2; // "  " 前缀
        // 提示文字宽度（用于判断是否需要滚动）
        let hint_overhead: usize = 4 + "(● = \u{6d3b}\u{8dc3}\u{6a21}\u{578b}, Tab \u{5207}\u{6362}, s \u{8bbe}\u{4e3a}\u{6d3b}\u{8dc3})".chars().count();

        let need_scroll = indent + total_tabs_width + hint_overhead > max_content_width;
        let selected_idx = app.ui.config_provider_idx;

        if need_scroll {
            // ── 水平滚动模式 ──
            // visible_start 已由 adjust_provider_scroll_offset() 设定
            let visible_start = app.ui.config_provider_scroll_offset;

            // 从 visible_start 开始累加，计算可见的 visible_end
            let mut used_width = indent;
            let mut visible_end = visible_start;
            for (i, &tw) in tab_widths.iter().enumerate().skip(visible_start) {
                let w = tw + if i > visible_start { sep_width } else { 0 };
                if used_width + w > max_content_width {
                    break;
                }
                used_width += w;
                visible_end = i + 1;
            }

            let mut tab_spans: Vec<Span> = vec![Span::styled("  ", Style::default())];

            // 左侧溢出指示
            if visible_start > 0 {
                tab_spans.push(Span::styled(
                    "\u{2026} ".to_string(),
                    Style::default().fg(t.config_dim),
                ));
            }

            for i in visible_start..visible_end {
                let p = &app.state.agent_config.providers[i];
                let is_current = i == selected_idx;
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
                if i < visible_end - 1 {
                    tab_spans.push(Span::styled(
                        format!(" {} ", super::components::SEPARATOR_V),
                        Style::default().fg(t.separator),
                    ));
                }
            }

            // 右侧溢出指示
            if visible_end < provider_count {
                tab_spans.push(Span::styled(
                    " \u{2026}".to_string(),
                    Style::default().fg(t.config_dim),
                ));
            }

            lines.push(Line::from(tab_spans));

            // 第二行：操作提示
            lines.push(Line::from(Span::styled(
                format!(
                    "  ({} = \u{6d3b}\u{8dc3}\u{6a21}\u{578b}, Tab \u{5207}\u{6362}, s \u{8bbe}\u{4e3a}\u{6d3b}\u{8dc3})",
                    super::components::TOGGLE_ON
                ),
                Style::default().fg(t.config_dim),
            )));
        } else {
            // ── 无需滚动：所有 provider 都能放下 ──
            let mut tab_spans: Vec<Span> = vec![Span::styled("  ", Style::default())];
            for (i, p) in app.state.agent_config.providers.iter().enumerate() {
                let is_current = i == selected_idx;
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
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  (\u{65e0} Provider\u{ff0c}\u{6309} a \u{65b0}\u{589e})",
            Style::default().fg(t.config_toggle_off),
        )));
    }
    lines.push(Line::from(""));
}

/// Model tab 可滚动列表（配置字段）
fn draw_tab_model_list<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
    let t = &app.ui.theme;
    let provider_count = app.state.agent_config.providers.len();

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
            } else if *provider_field == "supports_vision" {
                let toggle_on = if let Some(p) = app
                    .state
                    .agent_config
                    .providers
                    .get(app.ui.config_provider_idx)
                {
                    p.supports_vision
                } else {
                    false
                };
                toggle_row(label, toggle_on, is_selected, "Enter \u{5207}\u{6362}", t)
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

/// Global tab 内容（三列布局: label | value | desc，--- 分隔分组）
fn draw_tab_global_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
    let t = &app.ui.theme;

    // compact_exempt_tools 子列表模式
    if app.ui.compact_exempt_sublist {
        // 标题行 (lines[0]) + 空行 (lines[1])
        lines.push(Line::from(vec![
            Span::styled(
                "  豁免压缩工具  ",
                Style::default().fg(t.config_label_selected),
            ),
            Span::raw(" "),
            Span::styled(
                "Enter/空格 切换 | Esc 返回",
                Style::default().fg(t.config_dim),
            ),
        ]));
        lines.push(Line::from(""));

        use crate::command::chat::agent::compact::BUILTIN_EXEMPT_TOOLS;
        let tool_names = app.tool_registry.tool_names();
        let exempt = &app.state.agent_config.compact.micro_compact_exempt_tools;

        for (i, name) in tool_names.iter().enumerate() {
            let is_builtin = BUILTIN_EXEMPT_TOOLS.contains(name);
            let is_exempt = is_builtin || exempt.iter().any(|t| t == name);
            let selected = i == app.ui.compact_exempt_idx;

            let label = if is_builtin {
                format!("{} (内置)", name)
            } else {
                name.to_string()
            };

            field_line_indices.push(lines.len());
            lines.push(toggle_list_item(&label, is_exempt, selected, None, None, t));
        }
        return;
    }

    // 顶部留白
    lines.push(Line::from(""));

    // 分组定义: (字段起始索引, 包含字段数)
    let groups: &[(usize, usize)] = &[
        (0, 3), // system_prompt, agent_md, style
        (3, 2), // max_history_messages, max_context_tokens
        (5, 2), // max_tool_rounds, tool_confirm_timeout
        (7, 2), // theme, auto_restore_session
        (9, 4), // compact_enabled, compact_token_threshold, compact_keep_recent, compact_exempt_tools
    ];

    for (gi, &(start, count)) in groups.iter().enumerate() {
        // --- 分隔线 + 空行（首组不画）
        if gi > 0 {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  ────────────────────────────────────",
                Style::default().fg(t.separator),
            )));
            lines.push(Line::from(""));
        }

        for i in start..start + count {
            let field = CONFIG_GLOBAL_FIELDS_TAB.get(i);
            if field.is_none() {
                continue;
            }
            // is_none() 已在上方判断并 continue，此处 field 必为 Some
            let field_name = field.expect("checked is_none() above with continue");

            field_line_indices.push(lines.len());
            let is_selected = app.ui.config_field_idx == i;
            let label = config_field_label_global(i);
            let value = if app.ui.config_editing && is_selected {
                app.ui.config_edit_buf.clone()
            } else {
                config_field_value_global(app, i)
            };
            let desc = config_field_desc_global(i);

            let line = if *field_name == "auto_restore_session" {
                let toggle_on = app.state.agent_config.auto_restore_session;
                global_toggle_row(
                    label,
                    toggle_on,
                    desc,
                    is_selected,
                    "Enter \u{5207}\u{6362}",
                    t,
                )
            } else if *field_name == "compact_enabled" {
                let toggle_on = app.state.agent_config.compact.enabled;
                global_toggle_row(
                    label,
                    toggle_on,
                    desc,
                    is_selected,
                    "Enter \u{5207}\u{6362}",
                    t,
                )
            } else if *field_name == "theme" {
                let theme_name = app.state.agent_config.theme.display_name();
                global_theme_row(
                    label,
                    theme_name,
                    desc,
                    is_selected,
                    "Enter \u{5207}\u{6362}",
                    t,
                )
            } else if *field_name == "system_prompt"
                || *field_name == "agent_md"
                || *field_name == "style"
            {
                global_preview_row(
                    label,
                    &value,
                    desc,
                    is_selected,
                    "Enter \u{7f16}\u{8f91}",
                    t,
                )
            } else {
                global_text_row(
                    label,
                    &value,
                    desc,
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

/// Tools tab 固定头部（总开关）
fn draw_tab_tools_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
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
}

/// Tools tab 可滚动列表
fn draw_tab_tools_list<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
    let t = &app.ui.theme;
    let tool_names = app.tool_registry.tool_names();

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

/// Skills tab 固定头部（已启用计数）
fn draw_tab_skills_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
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
fn draw_tab_skills_list<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
    let t = &app.ui.theme;

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

/// Commands tab 固定头部（已启用计数）
fn draw_tab_commands_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
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
fn draw_tab_commands_list<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
    let t = &app.ui.theme;

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

/// Teammates tab 固定头部（团队摘要 + SubAgents 摘要）
fn draw_tab_teammates_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;

    let snapshots = app
        .teammate_manager
        .lock()
        .map(|m| m.teammate_snapshots())
        .unwrap_or_default();
    let sub_snaps = app.sub_agent_tracker.display_snapshots();

    if snapshots.is_empty() && sub_snaps.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (暂无 Teammate / SubAgent)",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Teammate 由 AI 通过 CreateTeammate 工具创建，SubAgent 由 Agent 工具触发",
            Style::default().fg(t.config_dim),
        )));
        return;
    }

    // Teammate 汇总行
    if !snapshots.is_empty() {
        let working = snapshots
            .iter()
            .filter(|s| {
                s.status == TeammateStatus::Working || s.status == TeammateStatus::Initializing
            })
            .count();
        let waiting = snapshots
            .iter()
            .filter(|s| s.status == TeammateStatus::WaitingForMessage)
            .count();
        let completed = snapshots
            .iter()
            .filter(|s| s.status == TeammateStatus::Completed)
            .count();
        let errored = snapshots
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    TeammateStatus::Error(_) | TeammateStatus::Cancelled
                )
            })
            .count();

        let mut summary_spans = vec![Span::styled(
            format!("  Teammates: {} 个  ", snapshots.len()),
            Style::default()
                .fg(t.config_label)
                .add_modifier(Modifier::BOLD),
        )];

        if working > 0 {
            summary_spans.push(Span::styled(
                format!("● {} 工作中", working),
                Style::default().fg(t.title_loading),
            ));
            summary_spans.push(Span::styled("  ", Style::default()));
        }
        if waiting > 0 {
            summary_spans.push(Span::styled(
                format!("○ {} 等待中", waiting),
                Style::default().fg(t.config_dim),
            ));
            summary_spans.push(Span::styled("  ", Style::default()));
        }
        if completed > 0 {
            summary_spans.push(Span::styled(
                format!("✓ {} 已完成", completed),
                Style::default().fg(t.config_toggle_on),
            ));
            summary_spans.push(Span::styled("  ", Style::default()));
        }
        if errored > 0 {
            summary_spans.push(Span::styled(
                format!("✗ {} 错误", errored),
                Style::default().fg(t.config_toggle_off),
            ));
        }
        lines.push(Line::from(summary_spans));
    }

    // SubAgent 汇总行
    if !sub_snaps.is_empty() {
        let working = sub_snaps
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    SubAgentStatus::Working | SubAgentStatus::Initializing
                )
            })
            .count();
        let completed = sub_snaps
            .iter()
            .filter(|s| s.status == SubAgentStatus::Completed)
            .count();
        let errored = sub_snaps
            .iter()
            .filter(|s| matches!(s.status, SubAgentStatus::Error(_)))
            .count();
        let cancelled = sub_snaps
            .iter()
            .filter(|s| s.status == SubAgentStatus::Cancelled)
            .count();

        let mut summary_spans = vec![Span::styled(
            format!("  SubAgents: {} 个  ", sub_snaps.len()),
            Style::default()
                .fg(t.config_label)
                .add_modifier(Modifier::BOLD),
        )];
        if working > 0 {
            summary_spans.push(Span::styled(
                format!("● {} 工作中", working),
                Style::default().fg(t.title_loading),
            ));
            summary_spans.push(Span::styled("  ", Style::default()));
        }
        if completed > 0 {
            summary_spans.push(Span::styled(
                format!("✓ {} 已完成", completed),
                Style::default().fg(t.config_toggle_on),
            ));
            summary_spans.push(Span::styled("  ", Style::default()));
        }
        if cancelled > 0 {
            summary_spans.push(Span::styled(
                format!("✗ {} 已取消", cancelled),
                Style::default().fg(t.text_dim),
            ));
            summary_spans.push(Span::styled("  ", Style::default()));
        }
        if errored > 0 {
            summary_spans.push(Span::styled(
                format!("✗ {} 错误", errored),
                Style::default().fg(t.config_toggle_off),
            ));
        }
        lines.push(Line::from(summary_spans));
    }

    lines.push(Line::from(Span::styled(
        "  (s 停止 Teammate, Enter 详情；SubAgent 只读)",
        Style::default().fg(t.config_dim),
    )));
    lines.push(Line::from(""));
}

/// Teammates tab 可滚动列表
fn draw_tab_teammates_list<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
    let t = &app.ui.theme;

    let snapshots = app
        .teammate_manager
        .lock()
        .map(|m| m.teammate_snapshots())
        .unwrap_or_default();

    for (i, snap) in snapshots.iter().enumerate() {
        field_line_indices.push(lines.len());
        let is_selected = i == app.ui.teammate_list_index;

        let pointer = if is_selected { "❯ " } else { "  " };

        // 状态颜色和符号
        let status_color = match &snap.status {
            TeammateStatus::Working => t.title_loading,
            TeammateStatus::WaitingForMessage => t.config_dim,
            TeammateStatus::Completed => t.config_toggle_on,
            TeammateStatus::Cancelled => t.text_dim,
            TeammateStatus::Error(_) => t.config_toggle_off,
            TeammateStatus::Initializing => t.config_dim,
        };

        let status_text = if snap.status == TeammateStatus::Working {
            if let Some(ref tool) = snap.current_tool {
                format!("{} {}: {}", snap.status.icon(), snap.status.label(), tool)
            } else {
                format!("{} {}", snap.status.icon(), snap.status.label())
            }
        } else {
            format!("{} {}", snap.status.icon(), snap.status.label())
        };

        // 截断角色描述
        let role_display: String = if snap.role.chars().count() > 20 {
            let truncated: String = snap.role.chars().take(20).collect();
            format!("{truncated}…")
        } else {
            snap.role.clone()
        };

        let pointer_style = if is_selected {
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.text_normal)
        };

        let name_style = if is_selected {
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(t.text_white)
                .add_modifier(Modifier::BOLD)
        };

        lines.push(Line::from(vec![
            Span::styled(pointer.to_string(), pointer_style),
            Span::styled(format!("{:<12}", snap.name), name_style),
            Span::styled(
                format!("{:<22}", role_display),
                Style::default().fg(t.config_dim),
            ),
            Span::styled(
                format!("{:<16}", status_text),
                Style::default().fg(status_color),
            ),
            Span::styled(
                format!("{} 次调用", snap.tool_calls_count),
                Style::default().fg(t.text_dim),
            ),
        ]));
        lines.push(Line::from(""));
    }

    // ── SubAgents 只读分组 ──
    let sub_snaps = app.sub_agent_tracker.display_snapshots();
    if !sub_snaps.is_empty() {
        lines.push(Line::from(Span::styled(
            "  ── SubAgents (只读) ──",
            Style::default()
                .fg(t.config_label)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for snap in sub_snaps.iter() {
            let status_color = match &snap.status {
                SubAgentStatus::Working => t.title_loading,
                SubAgentStatus::Completed => t.config_toggle_on,
                SubAgentStatus::Cancelled => t.text_dim,
                SubAgentStatus::Error(_) => t.config_toggle_off,
                SubAgentStatus::Initializing => t.config_dim,
            };

            let status_text = match &snap.status {
                SubAgentStatus::Working => {
                    if let Some(ref tool) = snap.current_tool {
                        format!(
                            "{} 工作中 R{} · {}",
                            snap.status.icon(),
                            snap.current_round,
                            tool
                        )
                    } else {
                        format!("{} 工作中 R{}", snap.status.icon(), snap.current_round)
                    }
                }
                SubAgentStatus::Error(msg) => {
                    let short: String = msg.chars().take(28).collect();
                    let suffix = if msg.chars().count() > 28 { "…" } else { "" };
                    format!("{} 错误: {}{}", snap.status.icon(), short, suffix)
                }
                _ => format!("{} {}", snap.status.icon(), snap.status.label()),
            };

            let desc_display: String = if snap.description.chars().count() > 22 {
                let truncated: String = snap.description.chars().take(22).collect();
                format!("{truncated}…")
            } else {
                snap.description.clone()
            };

            // 时长（秒）显示
            let elapsed = if snap.elapsed_secs < 60 {
                format!("{}s", snap.elapsed_secs)
            } else {
                format!("{}m{}s", snap.elapsed_secs / 60, snap.elapsed_secs % 60)
            };

            lines.push(Line::from(vec![
                Span::styled("  ".to_string(), Style::default().fg(t.text_normal)),
                Span::styled(
                    format!("{:<12}", snap.id),
                    Style::default()
                        .fg(t.text_white)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<24}", desc_display),
                    Style::default().fg(t.config_dim),
                ),
                Span::styled(
                    format!("{:<32}", status_text),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!(
                        "{} · {} 次调用 · {}",
                        snap.mode, snap.tool_calls_count, elapsed
                    ),
                    Style::default().fg(t.text_dim),
                ),
            ]));
            lines.push(Line::from(""));
        }
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

/// Archive tab 固定头部（确认还原 + 归档列表标题）
fn draw_tab_archive_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
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
    } else {
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
    }
}

/// Archive tab 可滚动列表（归档列表）
fn draw_tab_archive_list<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
    let t = &app.ui.theme;

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

/// Session tab 固定头部（当前会话 + 确认恢复 + 历史会话标题）
fn draw_tab_session_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
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
    } else {
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
    }
}

/// Session tab 可滚动列表（历史会话列表）
fn draw_tab_session_list<'a>(
    lines: &mut Vec<Line<'a>>,
    field_line_indices: &mut Vec<usize>,
    app: &ChatApp,
) {
    let t = &app.ui.theme;

    for (i, session) in app.ui.session_list.iter().enumerate() {
        field_line_indices.push(lines.len());
        let is_selected = i == app.ui.session_list_index;
        let preview = session
            .title
            .as_deref()
            .or(session.first_message_preview.as_deref())
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
