use crate::command::chat::app::ChatApp;
use crate::tui::components::{ItemList, TOGGLE_OFF, TOGGLE_ON, pointer_span};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Tools tab 固定头部（总开关）
pub(super) fn draw_tab_tools_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
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
            TOGGLE_ON, enabled_count, total
        )
    } else {
        format!(
            "  \u{603b}\u{5f00}\u{5173}: {} \u{5173}\u{95ed}",
            TOGGLE_OFF
        )
    };
    lines.push(Line::from(vec![
        Span::styled(master_text, master_style),
        Span::styled("  (t \u{5207}\u{6362})", Style::default().fg(t.config_dim)),
    ]));
    lines.push(Line::from(""));
}

/// Tools tab 工具列表（左侧面板）
///
/// 每个工具占一行，使用 pointer_span 指示选中项（与其他 Tab 一致），不带 toggle 圆点。
/// 选中项加粗，非选中项右侧显示 [defer] / [defer·已加载] tag。
/// 选项详情在右侧面板渲染。
pub(super) fn draw_tab_tools_list<'a>(app: &ChatApp) -> ItemList<'a> {
    let t = &app.ui.theme;
    let tool_names = app.tool_registry.tool_names();
    let mut list = ItemList::new(t.bg_primary);

    // 运行时 deferred 状态（LoadTool 可能已从中移除）
    let runtime_deferred = match app.deferred_tools.lock() {
        Ok(guard) => guard,
        Err(e) => e.into_inner(),
    };

    for (i, name) in tool_names.iter().enumerate() {
        let is_selected = i == app.ui.config_field_idx;
        let is_enabled = !app
            .state
            .agent_config
            .disabled_tools
            .iter()
            .any(|d| d == *name);
        // 读配置态（用户配置的 defer 设置）
        let is_config_deferred = app
            .state
            .agent_config
            .deferred_tools
            .iter()
            .any(|d| d == name);
        // 运行时已加载（配置 defer 但运行时不在 deferred_tools 中）
        let is_session_loaded = is_config_deferred && !runtime_deferred.iter().any(|d| d == name);

        let name_style = if is_selected && app.ui.tools_in_options {
            // Tab 进入选项模式：用不同颜色表示正在编辑选项
            Style::default()
                .fg(t.config_toggle_on)
                .add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.config_label)
        };

        let mut spans = vec![
            pointer_span(is_selected, t),
            Span::styled(name.to_string(), name_style),
        ];

        if is_config_deferred && is_enabled {
            if is_session_loaded {
                spans.push(Span::styled(
                    " [defer·已加载]".to_string(),
                    Style::default().fg(t.config_toggle_on),
                ));
            } else {
                spans.push(Span::styled(
                    " [defer]".to_string(),
                    Style::default().fg(t.config_dim),
                ));
            }
        }

        // 非启用的工具名称置灰
        if !is_enabled && !is_selected {
            spans[1] = Span::styled(name.to_string(), Style::default().fg(t.config_dim));
        }

        list.push(Line::from(spans));
    }
    list
}

/// Tools tab 选中工具详情（右侧面板）
///
/// 显示当前选中工具的名称和两个选项（启用 / defer）。
/// `tools_in_options` 控制焦点指示器，`tools_option_idx` 控制哪个选项高亮。
pub(super) fn draw_tab_tools_detail<'a>(app: &ChatApp) -> Vec<Line<'a>> {
    let t = &app.ui.theme;
    let tool_names = app.tool_registry.tool_names();
    let mut lines = Vec::new();

    let selected_idx = app.ui.config_field_idx;
    let name = match tool_names.get(selected_idx) {
        Some(n) => *n,
        None => return lines,
    };

    let is_enabled = !app
        .state
        .agent_config
        .disabled_tools
        .iter()
        .any(|d| d == name);
    // 配置态 defer（用户配置的持久化设置）
    let is_config_deferred = app
        .state
        .agent_config
        .deferred_tools
        .iter()
        .any(|d| d == name);
    // 运行时已加载（配置 defer 但运行时不在 deferred_tools 中）
    let runtime_deferred = match app.deferred_tools.lock() {
        Ok(guard) => guard,
        Err(e) => e.into_inner(),
    };
    let is_session_loaded = is_config_deferred && !runtime_deferred.iter().any(|d| d == name);

    let dim_style = Style::default().fg(t.config_dim);

    // 工具名与选项之间空行
    lines.push(Line::from(""));

    let opt_on_style = |focused: bool| {
        if focused {
            Style::default()
                .fg(t.config_toggle_on)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.config_toggle_on)
        }
    };
    let opt_off_style = |focused: bool| {
        if focused {
            Style::default()
                .fg(t.config_toggle_off)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.config_toggle_off)
        }
    };

    // 选项1：启用
    let enable_focused = app.ui.tools_in_options && app.ui.tools_option_idx == 0;
    let enable_toggle = if is_enabled {
        Span::styled(TOGGLE_ON.to_string(), opt_on_style(enable_focused))
    } else {
        Span::styled(TOGGLE_OFF.to_string(), opt_off_style(enable_focused))
    };
    lines.push(Line::from(vec![
        pointer_span(enable_focused, t),
        Span::styled(
            "启用 ",
            if enable_focused {
                Style::default()
                    .fg(t.config_section)
                    .add_modifier(Modifier::BOLD)
            } else {
                dim_style
            },
        ),
        enable_toggle,
    ]));

    lines.push(Line::from("")); // 选项间空行

    // 选项2：defer
    let defer_focused = app.ui.tools_in_options && app.ui.tools_option_idx == 1;
    let defer_effective = is_enabled;
    let defer_toggle = if is_config_deferred && defer_effective {
        Span::styled(TOGGLE_ON.to_string(), opt_on_style(defer_focused))
    } else if defer_effective {
        Span::styled(TOGGLE_OFF.to_string(), opt_off_style(defer_focused))
    } else {
        Span::styled(TOGGLE_OFF.to_string(), Style::default().fg(t.config_dim))
    };
    let mut defer_spans = vec![
        pointer_span(defer_focused, t),
        Span::styled(
            "defer ",
            if defer_focused {
                Style::default()
                    .fg(t.config_section)
                    .add_modifier(Modifier::BOLD)
            } else {
                dim_style
            },
        ),
        defer_toggle,
    ];
    // 本会话已加载提示
    if is_session_loaded {
        defer_spans.push(Span::styled(
            "  本会话已加载".to_string(),
            Style::default().fg(t.config_toggle_on),
        ));
    }
    lines.push(Line::from(defer_spans));

    lines
}
