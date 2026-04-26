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

/// 开关字段行的上下文参数
#[derive(Debug)]
pub struct ToggleRowCtx<'a> {
    pub label: String,
    pub is_on: bool,
    pub selected: bool,
    pub hint: String,
    pub theme: &'a Theme,
}

/// 开关字段行（auto_restore_session 等）
pub fn toggle_row<'a>(ctx: &ToggleRowCtx<'_>) -> Line<'a> {
    let toggle_style = if ctx.is_on {
        Style::default()
            .fg(ctx.theme.config_toggle_on)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ctx.theme.config_toggle_off)
    };
    let toggle_text = if ctx.is_on {
        format!("{TOGGLE_ON} \u{5f00}\u{542f}")
    } else {
        format!("{TOGGLE_OFF} \u{5173}\u{95ed}")
    };
    Line::from(vec![
        pointer_span(ctx.selected, ctx.theme),
        label_span(&ctx.label, LABEL_WIDTH, ctx.selected, ctx.theme),
        Span::styled("  ", Style::default()),
        Span::styled(toggle_text, toggle_style),
        Span::styled(
            if ctx.selected {
                format!("  ({})", ctx.hint)
            } else {
                String::new()
            },
            Style::default().fg(ctx.theme.config_dim),
        ),
    ])
}

/// 文本字段行的上下文参数
#[derive(Debug)]
pub struct TextFieldRowCtx<'a> {
    pub label: String,
    pub value: String,
    pub selected: bool,
    pub editing: bool,
    pub cursor: usize,
    pub theme: &'a Theme,
}

/// 普通可编辑文本字段行
pub fn text_field_row<'a>(ctx: &TextFieldRowCtx<'_>) -> Line<'a> {
    let vs = value_style(ctx.selected, ctx.editing, ctx.theme);
    if ctx.editing && ctx.selected {
        let mut spans = vec![
            pointer_span(ctx.selected, ctx.theme),
            label_span(&ctx.label, LABEL_WIDTH, ctx.selected, ctx.theme),
            Span::styled("  ", Style::default()),
        ];
        spans.extend(cursor_spans(&ctx.value, ctx.cursor, vs, ctx.theme));
        Line::from(spans)
    } else {
        Line::from(vec![
            pointer_span(ctx.selected, ctx.theme),
            label_span(&ctx.label, LABEL_WIDTH, ctx.selected, ctx.theme),
            Span::styled("  ", Style::default()),
            Span::styled(
                if ctx.value.is_empty() {
                    "(\u{7a7a})".to_string()
                } else {
                    ctx.value.clone()
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

/// 开关列表项的上下文参数
#[derive(Debug)]
pub struct ToggleListItemCtx<'a> {
    pub name: String,
    pub enabled: bool,
    pub selected: bool,
    pub desc: Option<String>,
    pub tag: Option<String>,
    pub theme: &'a Theme,
}

/// 工具/技能/命令的开关列表项
pub fn toggle_list_item<'a>(ctx: &ToggleListItemCtx<'_>) -> Line<'a> {
    let toggle_style = if ctx.enabled {
        Style::default()
            .fg(ctx.theme.config_toggle_on)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ctx.theme.config_toggle_off)
    };
    let toggle_text = if ctx.enabled { TOGGLE_ON } else { TOGGLE_OFF };
    let name_style = if ctx.selected {
        Style::default()
            .fg(ctx.theme.config_label_selected)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ctx.theme.config_label)
    };

    let mut spans = vec![
        pointer_span(ctx.selected, ctx.theme),
        Span::styled(toggle_text, toggle_style),
        Span::styled(" ", Style::default()),
        Span::styled(ctx.name.clone(), name_style),
    ];
    if let Some(d) = &ctx.desc {
        spans.push(Span::styled(
            format!("  {d}"),
            Style::default().fg(ctx.theme.config_dim),
        ));
    }
    if let Some(t) = &ctx.tag {
        spans.push(Span::styled(
            format!(" [{t}]"),
            Style::default().fg(ctx.theme.config_dim),
        ));
    }
    Line::from(spans)
}
