use crate::command::chat::app::ChatApp;
use crate::command::chat::input::autocomplete::{
    AtPopupItem, get_filtered_all_items, get_filtered_command_names, get_filtered_files,
    get_filtered_skill_names, get_filtered_slash_commands,
};
use crate::util::text::display_width;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

/// 弹窗配置参数（样式 + 行为）
pub(crate) struct PopupConfig {
    pub title: String,
    pub selected: usize,
    pub max_visible: usize,
    pub title_color: ratatui::style::Color,
    pub border_color: ratatui::style::Color,
    pub bg_color: ratatui::style::Color,
}

fn selected_reverse(is_selected: bool) -> Modifier {
    if is_selected {
        Modifier::REVERSED
    } else {
        Modifier::empty()
    }
}

fn popup_item_style(color: Color, is_selected: bool) -> Style {
    Style::default()
        .fg(color)
        .add_modifier(selected_reverse(is_selected))
}

fn popup_item_bold_style(color: Color, is_selected: bool) -> Style {
    popup_item_style(color, is_selected).add_modifier(Modifier::BOLD)
}

/// 通用浮动弹窗列表渲染（输入区上方）
pub(crate) fn draw_popup_list(
    f: &mut ratatui::Frame,
    input_area: Rect,
    items: Vec<ListItem<'static>>,
    item_labels: &[String],
    cfg: &PopupConfig,
) {
    if items.is_empty() {
        return;
    }
    let item_count = items.len();
    let visible_count = item_count.min(cfg.max_visible);
    let popup_height = (visible_count as u16) + 2; // +2 for border
    let max_popup_width = (input_area.width as usize).saturating_sub(2).max(16);
    let popup_width = item_labels
        .iter()
        .map(|n| display_width(n))
        .max()
        .unwrap_or(20)
        .max(16)
        .min(max_popup_width) as u16
        + 2;

    let x = input_area.x + 1;
    let y = input_area.y.saturating_sub(popup_height);
    // 确保弹窗不超出 input_area 右边界，避免 ratatui buffer 越界 panic
    let avail_width = input_area.right().saturating_sub(x);
    let popup_width = popup_width.min(avail_width);
    if popup_width == 0 {
        return;
    }
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    let mut list_state = ListState::default();
    list_state.select(Some(cfg.selected.min(item_count.saturating_sub(1))));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(cfg.border_color))
                .title(Span::styled(
                    &cfg.title,
                    Style::default()
                        .fg(cfg.title_color)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(cfg.bg_color)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED));

    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut list_state);
}

/// 绘制 @ 补全弹窗（输入区域上方浮动）
pub fn draw_at_popup(f: &mut ratatui::Frame, input_area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = get_filtered_all_items(app);
    if filtered.is_empty() {
        return;
    }

    let max_items = filtered.len().min(15);
    let selected = app
        .ui
        .at_popup_selected
        .min(filtered.len().saturating_sub(1));

    // 构建显示标签（与渲染内容等宽，用于 popup 宽度计算）
    let labels: Vec<String> = filtered
        .iter()
        .take(max_items)
        .map(|item| match item {
            AtPopupItem::Category(s) => format!("  {}", s),
            AtPopupItem::Skill(s) => format!("  [skill]   {}", s),
            AtPopupItem::Command(s) => format!("  [command] {}", s),
            AtPopupItem::File(s) => format!("  [file]    {}", s),
        })
        .collect();

    // 与 / 弹窗风格一致：pointer + name + desc
    let items: Vec<ListItem<'static>> = filtered
        .iter()
        .take(max_items)
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == selected;

            match item {
                AtPopupItem::Category(s) => {
                    // 分类分隔行：显示为 "skill:" 风格的暗色标签
                    ListItem::new(Line::from(vec![Span::styled(
                        format!("  {s}"),
                        popup_item_style(t.text_dim, is_selected),
                    )]))
                }
                AtPopupItem::Skill(s) => {
                    let pointer = if is_selected { "❯ " } else { "  " };
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            pointer.to_string(),
                            popup_item_style(t.text_normal, is_selected),
                        ),
                        Span::styled(
                            "[skill]   ".to_string(),
                            popup_item_bold_style(t.label_ai, is_selected),
                        ),
                        Span::styled(
                            s.as_str().to_string(),
                            popup_item_style(t.text_white, is_selected),
                        ),
                    ]))
                }
                AtPopupItem::Command(s) => {
                    let pointer = if is_selected { "❯ " } else { "  " };
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            pointer.to_string(),
                            popup_item_style(t.text_normal, is_selected),
                        ),
                        Span::styled(
                            "[command] ".to_string(),
                            popup_item_bold_style(t.label_ai, is_selected),
                        ),
                        Span::styled(
                            s.as_str().to_string(),
                            popup_item_style(t.text_white, is_selected),
                        ),
                    ]))
                }
                AtPopupItem::File(s) => {
                    let pointer = if is_selected { "❯ " } else { "  " };
                    let is_dir = s.ends_with('/');
                    let name_color = if is_dir { Color::Cyan } else { t.text_white };
                    ListItem::new(Line::from(vec![
                        Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                        Span::styled(
                            "[file]    ".to_string(),
                            Style::default()
                                .fg(t.label_user)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(s.as_str().to_string(), Style::default().fg(name_color)),
                    ]))
                }
            }
        })
        .collect();

    let title = if app.ui.at_popup_filter.is_empty() {
        " @ 补全 ".to_string()
    } else {
        format!(" @{} ", app.ui.at_popup_filter)
    };

    let cfg = PopupConfig {
        title,
        selected,
        max_visible: 8,
        title_color: t.md_h1,
        border_color: t.md_h1,
        bg_color: t.bg_primary,
    };
    draw_popup_list(f, input_area, items, &labels, &cfg);
}

/// 绘制文件补全弹窗（输入区域上方浮动）
pub fn draw_file_popup(f: &mut ratatui::Frame, input_area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = get_filtered_files(app);
    if filtered.is_empty() {
        return;
    }
    let max_items = filtered.len().min(15);
    let selected = app
        .ui
        .file_popup_selected
        .min(filtered.len().saturating_sub(1));

    let labels: Vec<String> = filtered
        .iter()
        .take(max_items)
        .map(|n| format!("  {}", n))
        .collect();

    let items: Vec<ListItem<'static>> = filtered
        .iter()
        .take(max_items)
        .enumerate()
        .map(|(i, name)| {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯ " } else { "  " };
            let is_dir = name.ends_with('/');
            let name_color = if is_dir { Color::Cyan } else { t.label_user };
            ListItem::new(Line::from(vec![
                Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                Span::styled(
                    name.clone(),
                    Style::default().fg(name_color).add_modifier(Modifier::BOLD),
                ),
            ]))
        })
        .collect();

    let title = if app.ui.file_popup_filter.is_empty() {
        " Files ".to_string()
    } else {
        format!(" {} ", app.ui.file_popup_filter)
    };
    let cfg = PopupConfig {
        title,
        selected,
        max_visible: 8,
        title_color: t.md_h1,
        border_color: t.md_h1,
        bg_color: t.bg_primary,
    };
    draw_popup_list(f, input_area, items, &labels, &cfg);
}

/// 绘制技能补全弹窗（输入区域上方浮动）
pub fn draw_skill_popup(f: &mut ratatui::Frame, input_area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = get_filtered_skill_names(app);
    if filtered.is_empty() {
        return;
    }
    let max_items = filtered.len().min(8);
    let selected = app
        .ui
        .skill_popup_selected
        .min(filtered.len().saturating_sub(1));

    let labels: Vec<String> = filtered
        .iter()
        .take(max_items)
        .map(|n| format!("  {}", n))
        .collect();

    let items: Vec<ListItem<'static>> = filtered
        .iter()
        .take(max_items)
        .enumerate()
        .map(|(i, name)| {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                Span::styled(
                    name.clone(),
                    Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
                ),
            ]))
        })
        .collect();

    let title = if app.ui.skill_popup_filter.is_empty() {
        " Skills ".to_string()
    } else {
        format!(" {} ", app.ui.skill_popup_filter)
    };
    let cfg = PopupConfig {
        title,
        selected,
        max_visible: 8,
        title_color: t.md_h1,
        border_color: t.md_h1,
        bg_color: t.bg_primary,
    };
    draw_popup_list(f, input_area, items, &labels, &cfg);
}

/// 绘制命令补全弹窗（输入区域上方浮动），显示命令名称和描述
pub fn draw_command_popup(f: &mut ratatui::Frame, input_area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = get_filtered_command_names(app);
    if filtered.is_empty() {
        return;
    }
    let max_items = filtered.len().min(8);
    let selected = app
        .ui
        .command_popup_selected
        .min(filtered.len().saturating_sub(1));

    // 构建带 description 的命令列表
    let commands: Vec<(String, String)> = app
        .state
        .loaded_commands
        .iter()
        .filter(|c| {
            !app.state
                .agent_config
                .disabled_commands
                .iter()
                .any(|d| d == &c.frontmatter.name)
        })
        .filter(|c| {
            let f = app.ui.command_popup_filter.to_lowercase();
            f.is_empty() || c.frontmatter.name.to_lowercase().contains(&f)
        })
        .take(max_items)
        .map(|c| {
            (
                c.frontmatter.name.clone(),
                c.frontmatter.description.clone(),
            )
        })
        .collect();

    let labels: Vec<String> = commands
        .iter()
        .map(|(name, desc)| format!("  {:<16}{}", name, desc))
        .collect();

    let items: Vec<ListItem<'static>> = commands
        .iter()
        .enumerate()
        .map(|(i, (name, desc))| {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                Span::styled(
                    format!("{:<16}", name),
                    Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc.clone(), Style::default().fg(t.text_dim)),
            ]))
        })
        .collect();

    let title = if app.ui.command_popup_filter.is_empty() {
        " Commands ".to_string()
    } else {
        format!(" {} ", app.ui.command_popup_filter)
    };
    let cfg = PopupConfig {
        title,
        selected,
        max_visible: 8,
        title_color: t.md_h1,
        border_color: t.md_h1,
        bg_color: t.bg_primary,
    };
    draw_popup_list(f, input_area, items, &labels, &cfg);
}

/// 绘制 / 斜杠命令弹窗（输入区域上方浮动）
pub fn draw_slash_popup(f: &mut ratatui::Frame, input_area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = get_filtered_slash_commands(&app.ui.slash_popup_filter);
    if filtered.is_empty() {
        return;
    }

    // 构建显示项：命令 + 描述
    // labels 用于计算弹窗宽度，需与实际渲染内容一致
    // 动态计算标签列宽度，取最长 display_label + 2 格间距
    let label_width: usize = filtered
        .iter()
        .map(|cmd| display_width(&cmd.display_label()))
        .max()
        .unwrap_or(10)
        + 2;
    let labels: Vec<String> = filtered
        .iter()
        .map(|cmd| {
            format!(
                "❯ {:<width$} {}",
                cmd.display_label(),
                cmd.description(),
                width = label_width
            )
        })
        .collect();

    let selected = app
        .ui
        .slash_popup_selected
        .min(filtered.len().saturating_sub(1));
    let items: Vec<ListItem<'static>> = filtered
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯ " } else { "  " };
            let padded_label = {
                let label = cmd.display_label();
                let label_dw = display_width(&label);
                let padding = label_width.saturating_sub(label_dw);
                format!("{}{}", label, " ".repeat(padding))
            };
            ListItem::new(Line::from(vec![
                Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                Span::styled(
                    padded_label,
                    Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
                ),
                Span::styled(cmd.description(), Style::default().fg(t.text_dim)),
            ]))
        })
        .collect();

    let title = if app.ui.slash_popup_filter.is_empty() {
        " / 命令 ".to_string()
    } else {
        format!(" /{} ", app.ui.slash_popup_filter)
    };

    draw_popup_list(
        f,
        input_area,
        items,
        &labels,
        &PopupConfig {
            title,
            selected,
            max_visible: 8,
            title_color: t.md_h1,
            border_color: t.md_h1,
            bg_color: t.bg_primary,
        },
    );
}
