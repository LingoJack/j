use super::app::{AppMode, HelpApp};
use crate::assets::HelpEntryKind;
use crate::theme::ThemeName;
use crate::tui::components::{
    CommandItem, CommandPopupConfig, draw_command_popup as render_command_popup,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

/// 绘制 TUI 界面
pub fn draw_ui(f: &mut ratatui::Frame, app: &mut HelpApp) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 标题栏
            Constraint::Min(5),    // 主区域
            Constraint::Length(1), // 帮助栏
        ])
        .split(size);

    // ========== 标题栏 ==========
    render_title_bar(f, app, chunks[0]);

    // ========== 主区域 ==========
    {
        let main_area = chunks[1];
        let left_width = app.compute_left_panel_width(main_area.width as usize) as u16;
        let right_width = main_area.width.saturating_sub(left_width);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(left_width),
                Constraint::Length(right_width),
            ])
            .split(main_area);

        render_list(f, app, main_chunks[0]);
        render_content(f, app, main_chunks[1]);

        // 弹窗浮动在主区域上方
        match app.mode {
            AppMode::CommandPopup => {
                draw_command_popup(f, app, main_area);
            }
            AppMode::ThemeSelect => {
                draw_theme_popup(f, app, main_area);
            }
            AppMode::Normal => {}
        }
    }

    // ========== 帮助栏 ==========
    render_help_bar(f, app, chunks[2]);
}

/// 渲染标题栏
fn render_title_bar(f: &mut ratatui::Frame, _app: &HelpApp, area: Rect) {
    let total = crate::assets::help_file_count();
    let title = format!(" j help — 共 {} 篇文档 ", total);

    let title_block = Paragraph::new(Line::from(vec![Span::styled(
        title,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(title_block, area);
}

/// 渲染左侧目录树
fn render_list(f: &mut ratatui::Frame, app: &mut HelpApp, area: Rect) {
    let inner_width = area.width.saturating_sub(2) as usize;

    let items: Vec<ListItem> = app
        .entries()
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = i == app.selected;

            let indent_style = Style::default().fg(Color::DarkGray);

            match &entry.kind {
                HelpEntryKind::Dir {
                    dir_path: _,
                    name,
                    file_count,
                } => {
                    let dir_style = Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD);
                    let count_str = format!(" ({})", file_count);

                    ListItem::new(Line::from(vec![
                        Span::styled(entry.guide.clone(), indent_style),
                        Span::styled(format!("{}/", name), dir_style),
                        Span::styled(count_str, Style::default().fg(Color::DarkGray)),
                    ]))
                }
                HelpEntryKind::File {
                    path: _,
                    name,
                    content: _,
                } => {
                    let name_style = if is_selected {
                        Style::default()
                            .bg(Color::Cyan)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Reset)
                    };

                    let guide_width = unicode_width::UnicodeWidthStr::width(entry.guide.as_str());
                    let name_display_width = inner_width.saturating_sub(guide_width);
                    let name_text = truncate_name(name, name_display_width);

                    ListItem::new(Line::from(vec![
                        Span::styled(entry.guide.clone(), indent_style),
                        Span::styled(name_text, name_style),
                    ]))
                }
            }
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" 文档列表 ");

    if items.is_empty() {
        let empty_hint = List::new(vec![ListItem::new(Line::from(Span::styled(
            "   (无文档)",
            Style::default().fg(Color::DarkGray),
        )))])
        .block(list_block);
        f.render_widget(empty_hint, area);
    } else {
        // 直接渲染带选中高亮的列表
        let list_widget = List::new(items).block(list_block);
        f.render_widget(list_widget, area);
    }
}

/// 截断文件名以适应显示宽度
fn truncate_name(name: &str, max_width: usize) -> String {
    let char_count = name.chars().count();
    if char_count <= max_width {
        name.to_string()
    } else {
        let truncated: String = name.chars().take(max_width.saturating_sub(2)).collect();
        format!("{}..", truncated)
    }
}

/// 渲染右侧内容区
fn render_content(f: &mut ratatui::Frame, app: &mut HelpApp, area: Rect) {
    let content_width = area.width.saturating_sub(2) as usize; // 减边框

    let lines = app.content_lines(content_width).to_vec();
    let total_lines = app.total_lines;

    // 钳制滚动偏移
    let visible_height = area.height.saturating_sub(2) as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);
    if app.content_scroll > max_scroll {
        app.content_scroll = max_scroll;
    }

    // 取可见范围内的行
    let visible_lines: Vec<Line<'static>> = lines
        .into_iter()
        .skip(app.content_scroll)
        .take(visible_height)
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let content = Paragraph::new(visible_lines).block(block);
    f.render_widget(content, area);
}

/// 渲染帮助栏
fn render_help_bar(f: &mut ratatui::Frame, app: &HelpApp, area: Rect) {
    let help_text = match app.mode {
        AppMode::Normal => " ↑↓/jk 移动 | Enter 展开/折叠 | [ ] 调整比例 | / 命令 | q 退出",
        AppMode::CommandPopup => " ↑↓ 选择 | Enter 确认 | 输入筛选 | Esc 取消",
        AppMode::ThemeSelect => " ↑↓ 选择 | Enter 确认 | Esc 取消",
    };

    let help_widget = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(help_widget, area);
}

/// 绘制命令面板弹窗
fn draw_command_popup(f: &mut ratatui::Frame, app: &HelpApp, main_area: Rect) {
    let items = app.filtered_cmd_items();
    let cmd_items: Vec<CommandItem<'_>> = items
        .iter()
        .map(|(_, key, label)| CommandItem::new(key, label))
        .collect();

    let title = if app.cmd_popup_filter.is_empty() {
        " 命令面板 ".to_string()
    } else {
        format!(" 命令面板 [{}] ", app.cmd_popup_filter)
    };

    render_command_popup(
        f,
        main_area,
        &CommandPopupConfig {
            title,
            items: cmd_items,
            selected: app.cmd_popup_selected,
            highlight_fg: Some(Color::Black),
            theme: app.theme(),
        },
    );
}

/// 绘制主题选择弹窗
fn draw_theme_popup(f: &mut ratatui::Frame, app: &HelpApp, main_area: Rect) {
    let themes = ThemeName::all();
    let item_count = themes.len();
    if item_count == 0 {
        return;
    }

    let popup_height = (item_count as u16 + 2).min(main_area.height.saturating_sub(2));
    let popup_width = 36u16.min(main_area.width.saturating_sub(4));

    let x = main_area.x + 2;
    let y = main_area
        .bottom()
        .saturating_sub(popup_height)
        .max(main_area.y);
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    let theme = app.theme();
    let accent = theme.md_h1;
    let popup_bg = theme.bg_primary;
    let text_color = theme.text_normal;
    let current_color = theme.md_link;

    let current_idx = themes
        .iter()
        .position(|t| t == &app.theme_name)
        .unwrap_or(0);
    let selected = app.theme_popup_selected.min(item_count - 1);

    let list_items: Vec<ListItem> = themes
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let is_selected = i == selected;
            let is_current = i == current_idx;
            let pointer = if is_selected { "> " } else { "  " };
            let check = if is_current { " *" } else { "" };
            let name_style = if is_selected {
                Style::default().fg(text_color).add_modifier(Modifier::BOLD)
            } else if is_current {
                Style::default().fg(current_color)
            } else {
                Style::default().fg(text_color)
            };
            ListItem::new(Line::from(vec![
                Span::styled(pointer.to_string(), name_style),
                Span::styled(format!("{}{}", name.display_name(), check), name_style),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(accent))
                .title(Span::styled(
                    " 选择主题 ",
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(popup_bg)),
        )
        .highlight_style(
            Style::default()
                .bg(accent)
                .fg(popup_bg)
                .add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, popup_area, &mut list_state);
}
