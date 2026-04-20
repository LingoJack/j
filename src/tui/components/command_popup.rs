//! 命令面板弹窗组件
//!
//! 提供统一的命令面板弹窗渲染，用于 chat / todo / notebook / help 等模块。

use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

/// 命令面板项
#[derive(Clone, Debug)]
pub struct CommandItem<'a> {
    /// 快捷键（左侧显示，如 "a", "e", "d"）
    pub key: &'a str,
    /// 标签（右侧描述，如 "添加", "编辑"）
    pub label: &'a str,
}

impl<'a> CommandItem<'a> {
    /// 创建命令项
    pub const fn new(key: &'a str, label: &'a str) -> Self {
        Self { key, label }
    }
}

/// 命令面板配置
pub struct CommandPopupConfig<'a> {
    /// 弹窗标题（包含筛选后缀，如 " 命令面板 [filter] "）
    pub title: String,
    /// 命令项列表
    pub items: Vec<CommandItem<'a>>,
    /// 当前选中索引
    pub selected: usize,
    /// 高亮前景色（默认使用 `theme.bg_primary`）
    pub highlight_fg: Option<Color>,
    /// 主题样式
    pub theme: &'a Theme,
}

/// 绘制命令面板弹窗
///
/// 在主区域底部偏左位置渲染一个圆角边框的命令列表弹窗。
/// 宽度根据内容自动计算。
pub fn draw_command_popup(f: &mut Frame, main_area: Rect, config: &CommandPopupConfig<'_>) {
    let item_count = config.items.len();
    if item_count == 0 {
        return;
    }

    let t = config.theme;
    let selected = config.selected.min(item_count.saturating_sub(1));

    // 动态计算宽度：基于最长 "pointer + key + label" 组合
    let max_content_width = config
        .items
        .iter()
        .map(|item| {
            // "  key   label" => 2(spacer) + key_width + padding_to_8 + label_width
            let key_w = unicode_width::UnicodeWidthStr::width(item.key);
            let label_w = unicode_width::UnicodeWidthStr::width(item.label);
            2 + key_w.max(8) + label_w
        })
        .max()
        .unwrap_or(16)
        .max(16);
    let popup_width = (max_content_width as u16 + 2) // 边框
        .min(main_area.width.saturating_sub(4));
    let popup_height = (item_count as u16 + 2).min(main_area.height.saturating_sub(2));

    // 位置：主区域底部偏左
    let x = main_area.x + 2;
    let y = main_area
        .bottom()
        .saturating_sub(popup_height)
        .max(main_area.y);
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // 颜色配置
    let accent = t.md_h1;
    let dim_color = t.text_dim;
    let popup_bg = t.bg_primary;
    let text_color = t.text_normal;
    let highlight_fg = config.highlight_fg.unwrap_or(popup_bg);

    // 构建列表项：pointer + key + label
    let list_items: Vec<ListItem<'_>> = config
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(
                    pointer.to_string(),
                    Style::default().fg(if is_selected { accent } else { text_color }),
                ),
                Span::styled(
                    format!("{:<8}", item.key),
                    Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
                ),
                Span::styled(item.label.to_string(), Style::default().fg(dim_color)),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(accent))
                .title(Span::styled(
                    config.title.as_str(),
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(popup_bg)),
        )
        .highlight_style(
            Style::default()
                .bg(accent)
                .fg(highlight_fg)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut list_state);
}
