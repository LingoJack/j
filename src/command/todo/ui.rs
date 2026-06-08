use super::app::{AppMode, TodoApp, display_width, truncate_to_width};
use crate::constants::todo_filter;
use crate::tui::components::{
    CommandItem, CommandPopupConfig, ConfirmDialogConfig, StatusInputParams, cursor_wrapped_lines,
    draw_command_popup as render_command_popup, draw_confirm_dialog, draw_status_input,
};
use crate::util::text::wrap_text;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

/// 绘制 TUI 界面
pub fn draw_ui(f: &mut ratatui::Frame, app: &mut TodoApp) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 标题栏
            Constraint::Min(5),    // 列表区
            Constraint::Length(3), // 状态栏
            Constraint::Length(1), // 帮助栏
        ])
        .split(size);

    // ========== 标题栏 ==========
    let filter_label = match app.filter {
        todo_filter::UNDONE => " [未完成]",
        todo_filter::DONE => " [已完成]",
        _ => "",
    };
    let total = app.list.items.len();
    let done = app.list.items.iter().filter(|i| i.done).count();
    let undone = total - done;
    let title = format!(
        " 📝 待办{} — 共 {} 条 | ☑️ {} | ⬜ {} ",
        filter_label, total, done, undone
    );
    let title_block = Paragraph::new(Line::from(vec![Span::styled(
        title,
        Style::default()
            .fg(app.theme.help_title)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.help_title)),
    );
    f.render_widget(title_block, chunks[0]);

    // ========== 列表区 ==========
    if app.mode == AppMode::Help {
        render_help(f, app, chunks[1]);
    } else {
        render_list(f, app, chunks[1]);
    }

    // 命令面板弹窗（浮动在列表区上方）
    if app.mode == AppMode::CommandPopup {
        draw_command_popup(f, app, chunks[1]);
    }

    // ========== 状态栏 ==========
    render_status_bar(f, app, chunks[2]);

    // ========== 帮助栏 ==========
    let help_text = match app.mode {
        AppMode::Normal => {
            " n/↓ 下移 | N/↑ 上移 | 空格/回车 切换完成 | a 添加 | e 编辑 | d 删除 | y 复制 | f 过滤 | s 保存 | / 命令 | ? 帮助 | q 退出"
        }
        AppMode::Adding | AppMode::Editing => {
            " Enter 确认 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾"
        }
        AppMode::ConfirmDelete => " y 确认删除 | n/Esc 取消",
        AppMode::ConfirmReport => " Enter/y 写入日报 | 其他键跳过",
        AppMode::ConfirmCancelInput => " Enter/y 保存 | n/Esc 放弃 | 其他键继续编辑",
        AppMode::Help => " 按任意键返回",
        AppMode::CommandPopup => " ↑↓/jk 选择 | Enter 确认 | Esc 取消",
    };
    let help_widget = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(app.theme.text_dim),
    )));
    f.render_widget(help_widget, chunks[3]);
}

/// 渲染帮助页
fn render_help(f: &mut ratatui::Frame, app: &TodoApp, area: ratatui::layout::Rect) {
    use crate::tui::components::{HelpPageConfig, HelpShortcut, draw_help_page};

    let shortcuts = [
        HelpShortcut::new("  n / ↓ / j    ", "向下移动"),
        HelpShortcut::new("  N / ↑ / k    ", "向上移动"),
        HelpShortcut::new("  空格 / 回车   ", "切换完成状态 [x] / [ ]"),
        HelpShortcut::new("  a            ", "添加新待办"),
        HelpShortcut::new("  e            ", "编辑选中待办"),
        HelpShortcut::new("  d            ", "删除待办（需确认）"),
        HelpShortcut::new("  f            ", "过滤切换（全部 / 未完成 / 已完成）"),
        HelpShortcut::new("  J / K        ", "调整待办顺序（下移 / 上移）"),
        HelpShortcut::new("  s            ", "手动保存"),
        HelpShortcut::new("  y            ", "复制选中待办到剪切板"),
        HelpShortcut::new(
            "  q            ",
            "退出（有未保存修改时需先保存或用 q! 强制退出）",
        ),
        HelpShortcut::new("  q!           ", "强制退出（丢弃未保存的修改）"),
        HelpShortcut::new("  Esc          ", "退出（同 q）"),
        HelpShortcut::new("  Ctrl+C       ", "强制退出（不保存）"),
        HelpShortcut::new("  /            ", "命令面板"),
        HelpShortcut::new("  ?            ", "显示此帮助"),
    ];

    draw_help_page(
        f,
        area,
        &HelpPageConfig {
            title: "快捷键帮助",
            title_icon: Some("📖"),
            block_title: Some(" 帮助 "),
            shortcuts: &shortcuts,
            footer_lines: None,
            theme: &app.theme,
        },
    );
}

/// 渲染列表区（含 inline 编辑）
fn render_list(f: &mut ratatui::Frame, app: &mut TodoApp, area: ratatui::layout::Rect) {
    let indices = app.filtered_indices();
    // 指针占 3 列（" ❯ " 或 "   "），加上边框 2 列
    let list_inner_width = area.width.saturating_sub(2 + 3) as usize;
    let checkbox_w = display_width(" [x] ");

    let selected = app.state.selected();

    // 计算最大日期列宽度，确保编辑区折行不会跨到日期区域
    let max_date_col_w = app
        .list
        .items
        .iter()
        .map(|item| {
            let date_str = item
                .created_at
                .get(..10)
                .map(|d| format!(" ({})", d))
                .unwrap_or_default();
            display_width(&date_str)
        })
        .max()
        .unwrap_or(0);

    let mut items: Vec<ListItem> = indices
        .iter()
        .enumerate()
        .map(|(i, &idx)| {
            let is_selected = selected == Some(i);
            // 编辑模式下替换选中项为编辑行
            if app.mode == AppMode::Editing && app.edit_index == Some(idx) {
                let edit_content_width = list_inner_width
                    .saturating_sub(checkbox_w)
                    .saturating_sub(max_date_col_w);
                return build_editing_item(
                    &app.input,
                    app.cursor_pos,
                    edit_content_width,
                    checkbox_w,
                    is_selected,
                    &app.theme,
                );
            }
            build_normal_item(
                &app.list.items[idx],
                list_inner_width,
                checkbox_w,
                is_selected,
                &app.theme,
            )
        })
        .collect();

    // 添加模式：在列表末尾追加编辑行
    if app.mode == AppMode::Adding {
        let is_selected = selected == Some(indices.len());
        let add_content_width = list_inner_width
            .saturating_sub(checkbox_w)
            .saturating_sub(max_date_col_w);
        items.push(build_editing_item(
            &app.input,
            app.cursor_pos,
            add_content_width,
            checkbox_w,
            is_selected,
            &app.theme,
        ));
    }

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.separator))
        .title(" 待办列表 ");

    if items.is_empty() {
        let empty_hint = List::new(vec![ListItem::new(Line::from(Span::styled(
            "   (空) 按 a 添加新待办...",
            Style::default().fg(app.theme.text_dim),
        )))])
        .block(list_block);
        f.render_widget(empty_hint, area);
    } else {
        let list_widget = List::new(items)
            .block(list_block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        f.render_stateful_widget(list_widget, area, &mut app.state);
    }
}

/// 指针 Span（选中时显示彩色 ❯，未选中显示等宽空格）
fn pointer_span(selected: bool, theme: &crate::theme::Theme) -> Span<'static> {
    if selected {
        Span::styled(
            " ❯ ",
            Style::default()
                .fg(theme.config_pointer)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("   ")
    }
}

/// 构建普通列表项
///
/// 布局分三个独立 part：
///   pointer+checkbox | content（在自身列宽内折行）| date（固定宽度右对齐，仅首行）
/// 内容折行宽度 = list_inner_width - checkbox_w - date_col_w
fn build_normal_item(
    item: &super::app::TodoItem,
    list_inner_width: usize,
    checkbox_w: usize,
    selected: bool,
    theme: &crate::theme::Theme,
) -> ListItem<'static> {
    let checkbox = if item.done { "[x]" } else { "[ ]" };
    let checkbox_style = if item.done {
        Style::default().fg(theme.config_toggle_on)
    } else {
        Style::default().fg(theme.config_pointer)
    };
    let content_style = if item.done {
        Style::default()
            .fg(theme.text_dim)
            .add_modifier(Modifier::CROSSED_OUT)
    } else {
        Style::default().fg(theme.text_normal)
    };

    let checkbox_str = format!(" {} ", checkbox);
    let date_str = item
        .created_at
        .get(..10)
        .map(|d| format!(" ({})", d))
        .unwrap_or_default();
    let date_display_width = display_width(&date_str);

    // 日期列宽度：至少留出日期的空间，没有日期则为 0
    let date_col_w = if date_display_width > 0 {
        date_display_width
    } else {
        0
    };

    // 内容在自身列宽内折行，不受日期影响
    let content_width = list_inner_width
        .saturating_sub(checkbox_w)
        .saturating_sub(date_col_w);
    let wrapped = wrap_text(&item.content, content_width);
    let indent = " ".repeat(checkbox_w);

    let mut item_lines: Vec<Line> = Vec::new();
    for (i, line_text) in wrapped.iter().enumerate() {
        let mut spans = if i == 0 {
            vec![
                pointer_span(selected, theme),
                Span::styled(checkbox_str.clone(), checkbox_style),
            ]
        } else {
            vec![Span::raw("   "), Span::raw(indent.clone())]
        };
        spans.push(Span::styled(line_text.clone(), content_style));

        if i == 0 && date_display_width > 0 {
            // 首行：用 padding 把日期推到行最右侧
            let line_content_w = checkbox_w + display_width(line_text);
            let padding_w = list_inner_width
                .saturating_sub(line_content_w)
                .saturating_sub(date_display_width);
            spans.push(Span::raw(" ".repeat(padding_w)));
            spans.push(Span::styled(
                date_str.clone(),
                Style::default().fg(theme.text_dim),
            ));
        }

        item_lines.push(Line::from(spans));
    }

    ListItem::new(item_lines)
}

/// 构建 inline 编辑项（添加/编辑模式）
fn build_editing_item(
    input: &str,
    cursor_pos: usize,
    content_width: usize,
    checkbox_w: usize,
    selected: bool,
    theme: &crate::theme::Theme,
) -> ListItem<'static> {
    let indent = " ".repeat(checkbox_w);
    let cursor_lines =
        cursor_wrapped_lines(input, cursor_pos, content_width, Some("输入内容…"), theme);

    let mut item_lines: Vec<Line> = Vec::new();
    for (i, line) in cursor_lines.lines.into_iter().enumerate() {
        let mut spans = if i == 0 {
            vec![pointer_span(selected, theme)]
        } else {
            vec![Span::raw("   ")]
        };
        spans.push(Span::raw(indent.clone()));
        spans.extend(line.spans);
        item_lines.push(Line::from(spans));
    }

    ListItem::new(item_lines)
}

/// 绘制命令面板弹窗（浮动在列表区上方）
pub fn draw_command_popup(f: &mut ratatui::Frame, app: &mut TodoApp, main_area: Rect) {
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
            theme: &app.theme,
        },
    );
}

/// 渲染状态栏
fn render_status_bar(f: &mut ratatui::Frame, app: &TodoApp, area: ratatui::layout::Rect) {
    match &app.mode {
        AppMode::Adding => {
            let status = Paragraph::new(Line::from(vec![
                Span::styled(
                    " ✏️  添加模式",
                    Style::default()
                        .fg(app.theme.config_toggle_on)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " — 在列表中输入内容",
                    Style::default().fg(app.theme.text_dim),
                ),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.config_toggle_on)),
            );
            f.render_widget(status, area);
        }
        AppMode::Editing => {
            let status = Paragraph::new(Line::from(vec![
                Span::styled(
                    " ✏️  编辑模式",
                    Style::default()
                        .fg(app.theme.config_pointer)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " — 在列表中修改内容",
                    Style::default().fg(app.theme.text_dim),
                ),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.config_pointer)),
            );
            f.render_widget(status, area);
        }
        AppMode::ConfirmDelete => {
            let msg = if let Some(real_idx) = app.selected_real_index() {
                format!(
                    " 确认删除「{}」？(y 确认 / n 取消)",
                    app.list.items[real_idx].content
                )
            } else {
                " 没有选中的项目".to_string()
            };
            draw_confirm_dialog(
                f,
                area,
                &ConfirmDialogConfig {
                    title: " ⚠️ 确认删除 ",
                    message: msg,
                    color: app.theme.toast_error_text,
                },
            );
        }
        AppMode::ConfirmReport => {
            let inner_width = area.width.saturating_sub(2) as usize;
            let msg = if let Some(ref content) = app.report_pending_content {
                let prefix = " 写入日报: \"";
                let suffix = "\" ？ (Enter/y 写入, 其他跳过)";
                let prefix_w = display_width(prefix);
                let suffix_w = display_width(suffix);
                let budget = inner_width.saturating_sub(prefix_w + suffix_w);
                let truncated = truncate_to_width(content, budget);
                format!("{}{}{}", prefix, truncated, suffix)
            } else {
                " 没有待写入的内容".to_string()
            };
            draw_confirm_dialog(
                f,
                area,
                &ConfirmDialogConfig {
                    title: " 📝 写入日报 ",
                    message: msg,
                    color: app.theme.md_h1,
                },
            );
        }
        AppMode::ConfirmCancelInput => {
            let inner_width = area.width.saturating_sub(2) as usize;
            let prefix = " ⚠️ 是否保存？当前输入: \"";
            let suffix = "\" (Enter/y 保存 / n/Esc 放弃 / 其他键继续编辑)";
            let prefix_w = display_width(prefix);
            let suffix_w = display_width(suffix);
            let budget = inner_width.saturating_sub(prefix_w + suffix_w);
            let truncated = truncate_to_width(&app.input, budget);
            let msg = format!("{}{}{}", prefix, truncated, suffix);
            draw_confirm_dialog(
                f,
                area,
                &ConfirmDialogConfig {
                    title: " ⚠️ 未保存的内容 ",
                    message: msg,
                    color: app.theme.config_pointer,
                },
            );
        }
        AppMode::CommandPopup => {
            draw_status_input(
                f,
                area,
                &StatusInputParams {
                    label: "命令面板",
                    label_color: app.theme.md_h1,
                    input: &app.cmd_popup_filter,
                    cursor_pos: app.cmd_popup_filter.chars().count(),
                    placeholder: "输入筛选…",
                    hint: "↑↓ 选择 | Enter 确认 | Esc 取消",
                },
                &app.theme,
            );
        }
        _ => {
            // Normal / Help
            let msg = app.message.as_deref().unwrap_or("按 ? 查看完整帮助");
            let dirty_indicator = if app.is_dirty() { " [未保存]" } else { "" };
            let status_widget = Paragraph::new(Line::from(vec![
                Span::styled(msg, Style::default().fg(app.theme.text_dim)),
                Span::styled(
                    dirty_indicator,
                    Style::default()
                        .fg(app.theme.toast_error_text)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.separator)),
            );
            f.render_widget(status_widget, area);
        }
    }
}
