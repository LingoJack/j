use crate::command::chat::app::ChatApp;
use crate::command::chat::render::helpers::{config_field_label_model, config_field_value_model};
use crate::command::chat::ui::components::{RowContext, secret_field_row};
use crate::constants::CONFIG_FIELDS;
use crate::tui::components::{
    ItemList, SEPARATOR_V, TOGGLE_OFF, TOGGLE_ON, text_field_row, toggle_row,
};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// 根据 config_provider_idx 和可用宽度，自动调整 config_provider_scroll_offset
/// 确保当前选中的 provider 标签始终在可见范围内
pub(super) fn adjust_provider_scroll_offset(app: &mut ChatApp, max_width: usize) {
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
pub(super) fn draw_tab_model_header<'a>(
    lines: &mut Vec<Line<'a>>,
    app: &ChatApp,
    available_width: u16,
) {
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
        let hint_overhead: usize =
            4 + "(● = \u{6d3b}\u{8dc3}\u{6a21}\u{578b}, Tab \u{5207}\u{6362}, s \u{8bbe}\u{4e3a}\u{6d3b}\u{8dc3})"
                .chars()
                .count();

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
                let marker = if is_active { TOGGLE_ON } else { TOGGLE_OFF };
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
                        format!(" {} ", SEPARATOR_V),
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
                    TOGGLE_ON
                ),
                Style::default().fg(t.config_dim),
            )));
        } else {
            // ── 无需滚动：所有 provider 都能放下 ──
            let mut tab_spans: Vec<Span> = vec![Span::styled("  ", Style::default())];
            for (i, p) in app.state.agent_config.providers.iter().enumerate() {
                let is_current = i == selected_idx;
                let is_active = i == app.state.agent_config.active_index;
                let marker = if is_active { TOGGLE_ON } else { TOGGLE_OFF };
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
                        format!(" {} ", SEPARATOR_V),
                        Style::default().fg(t.separator),
                    ));
                }
            }
            tab_spans.push(Span::styled(
                format!(
                    "    ({} = \u{6d3b}\u{8dc3}\u{6a21}\u{578b}, Tab \u{5207}\u{6362}, s \u{8bbe}\u{4e3a}\u{6d3b}\u{8dc3})",
                    TOGGLE_ON
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
pub(super) fn draw_tab_model_list<'a>(app: &ChatApp) -> ItemList<'a> {
    let t = &app.ui.theme;
    let provider_count = app.state.agent_config.providers.len();
    let mut list = ItemList::new(t.bg_primary);

    if provider_count > 0 {
        for (i, provider_field) in CONFIG_FIELDS.iter().enumerate() {
            let is_selected = app.ui.config_field_idx == i;
            let label = config_field_label_model(i);
            let value = if app.ui.config_editing && is_selected {
                app.ui.config_edit_buf.clone()
            } else {
                config_field_value_model(app, i)
            };

            let line = if *provider_field == "api_key" {
                let ctx = RowContext {
                    selected: is_selected,
                    theme: t,
                };
                secret_field_row(
                    label,
                    &value,
                    app.ui.config_editing,
                    app.ui.config_edit_cursor,
                    &ctx,
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
            list.push(line);
        }
    }
    list
}
