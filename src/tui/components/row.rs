//! 行组件（toggle_row / text_field_row / selectable_row / toggle_list_item）

use crate::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::consts::{LABEL_WIDTH, TOGGLE_OFF, TOGGLE_ON};
use super::cursor::cursor_spans;
use super::label::{label_span, value_style};
use super::pointer::pointer_span;

/// 开关字段行（auto_restore_session 等）
pub fn toggle_row<'a>(
    label: &str,
    is_on: bool,
    selected: bool,
    hint: &str,
    theme: &Theme,
) -> Line<'a> {
    let toggle_style = if is_on {
        Style::default()
            .fg(theme.config_toggle_on)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.config_toggle_off)
    };
    let toggle_text = if is_on {
        format!("{TOGGLE_ON} \u{5f00}\u{542f}")
    } else {
        format!("{TOGGLE_OFF} \u{5173}\u{95ed}")
    };
    Line::from(vec![
        pointer_span(selected, theme),
        label_span(label, LABEL_WIDTH, selected, theme),
        Span::styled("  ", Style::default()),
        Span::styled(toggle_text, toggle_style),
        Span::styled(
            if selected {
                format!("  ({hint})")
            } else {
                String::new()
            },
            Style::default().fg(theme.config_dim),
        ),
    ])
}

/// 普通可编辑文本字段行
pub fn text_field_row<'a>(
    label: &str,
    value: &str,
    selected: bool,
    editing: bool,
    cursor: usize,
    theme: &Theme,
) -> Line<'a> {
    let vs = value_style(selected, editing, theme);
    if editing && selected {
        let mut spans = vec![
            pointer_span(selected, theme),
            label_span(label, LABEL_WIDTH, selected, theme),
            Span::styled("  ", Style::default()),
        ];
        spans.extend(cursor_spans(value, cursor, vs, theme));
        Line::from(spans)
    } else {
        Line::from(vec![
            pointer_span(selected, theme),
            label_span(label, LABEL_WIDTH, selected, theme),
            Span::styled("  ", Style::default()),
            Span::styled(
                if value.is_empty() {
                    "(\u{7a7a})".to_string()
                } else {
                    value.to_string()
                },
                vs,
            ),
        ])
    }
}

/// 可选行（主文本 + 次要信息）
pub fn selectable_row<'a>(
    primary: &str,
    secondary: &str,
    selected: bool,
    theme: &Theme,
) -> Line<'a> {
    let name_style = if selected {
        Style::default()
            .fg(theme.config_label_selected)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.config_label)
    };
    Line::from(vec![
        pointer_span(selected, theme),
        Span::styled(primary.to_string(), name_style),
        Span::styled(
            format!("  {secondary}"),
            Style::default().fg(theme.config_dim),
        ),
    ])
}

/// 工具/技能/命令的开关列表项
pub fn toggle_list_item<'a>(
    name: &str,
    enabled: bool,
    selected: bool,
    desc: Option<&str>,
    tag: Option<&str>,
    theme: &Theme,
) -> Line<'a> {
    let toggle_style = if enabled {
        Style::default()
            .fg(theme.config_toggle_on)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.config_toggle_off)
    };
    let toggle_text = if enabled { TOGGLE_ON } else { TOGGLE_OFF };
    let name_style = if selected {
        Style::default()
            .fg(theme.config_label_selected)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.config_label)
    };

    let mut spans = vec![
        pointer_span(selected, theme),
        Span::styled(toggle_text, toggle_style),
        Span::styled(" ", Style::default()),
        Span::styled(name.to_string(), name_style),
    ];
    if let Some(d) = desc {
        spans.push(Span::styled(
            format!("  {d}"),
            Style::default().fg(theme.config_dim),
        ));
    }
    if let Some(t) = tag {
        spans.push(Span::styled(
            format!(" [{t}]"),
            Style::default().fg(theme.config_dim),
        ));
    }
    Line::from(spans)
}
