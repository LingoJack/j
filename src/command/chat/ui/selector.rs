//! 选择器界面（模型选择、主题选择）
//!
//! 提取自 chat.rs，使用通用选择器构建逻辑消除代码重复。

use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::command::chat::app::ChatApp;

use crate::tui::components::{POINTER_EMPTY, POINTER_SELECTED, TOGGLE_OFF, TOGGLE_ON};

/// 绘制模型选择界面
pub fn draw_model_selector(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let t = &app.ui.theme;
    let selected = app.ui.model_list_state.selected().unwrap_or(0);

    let items: Vec<ListItem> = app
        .state
        .agent_config
        .providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_active = i == app.state.agent_config.active_index;
            let is_selected = i == selected;
            let label = format!("{}  ({})", p.name, p.model);
            selector_item(is_active, is_selected, &label, t)
        })
        .collect();

    let list = List::new(items)
        .block(selector_block(
            " \u{1f504} \u{9009}\u{62e9}\u{6a21}\u{578b} ",
            t.model_sel_border,
            t.config_label_selected,
            t.bg_primary,
        ))
        .highlight_style(Style::default())
        .highlight_symbol("");

    f.render_stateful_widget(list, area, &mut app.ui.model_list_state);
}

/// 绘制主题选择界面
pub fn draw_theme_selector(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    use crate::command::chat::render::theme::ThemeName;

    let t = &app.ui.theme;
    let all = ThemeName::all();
    let selected = app.ui.theme_list_state.selected().unwrap_or(0);

    let items: Vec<ListItem> = all
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let is_active = *name == app.state.agent_config.theme;
            let is_selected = i == selected;
            let label = name.display_name().to_string();
            selector_item(is_active, is_selected, &label, t)
        })
        .collect();

    let list = List::new(items)
        .block(selector_block(
            " 🎨 选择主题 ",
            t.model_sel_border,
            t.config_label_selected,
            t.bg_primary,
        ))
        .highlight_style(Style::default())
        .highlight_symbol("");

    f.render_stateful_widget(list, area, &mut app.ui.theme_list_state);
}

/// 通用选择器列表项构建
///
/// 根据 `is_active` 显示开关标记，根据 `is_selected` 显示指针和高亮样式。
fn selector_item<'a>(is_active: bool, is_selected: bool, label: &str, t: &Theme) -> ListItem<'a> {
    let marker = if is_active {
        format!("{} ", TOGGLE_ON)
    } else {
        format!("{} ", TOGGLE_OFF)
    };
    let style = if is_selected {
        Style::default()
            .fg(t.text_white)
            .add_modifier(Modifier::BOLD)
    } else if is_active {
        Style::default()
            .fg(t.model_sel_active)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.model_sel_inactive)
    };
    let pointer = if is_selected {
        POINTER_SELECTED
    } else {
        POINTER_EMPTY
    };
    let detail = format!("{}{}{}", pointer, marker, label);
    ListItem::new(Line::from(Span::styled(detail, style)))
}

/// 通用选择器外框
fn selector_block<'a>(
    title: &'a str,
    border_color: ratatui::style::Color,
    title_color: ratatui::style::Color,
    bg: ratatui::style::Color,
) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(bg))
}
