use super::app::{AppMode, FlatEntryKind, Focus, NotebookApp};
use crate::tui::components::{
    CommandItem, CommandPopupConfig, ConfirmDialogConfig, StatusInputParams, cursor_wrapped_lines,
    draw_command_popup as render_command_popup, draw_confirm_dialog, draw_status_input,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

/// 绘制 TUI 界面
pub fn draw_ui(f: &mut ratatui::Frame, app: &mut NotebookApp) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 顶部栏（标题 / 状态合一）
            Constraint::Min(5),    // 主区域
            Constraint::Length(1), // 帮助栏
        ])
        .split(size);

    // ========== 顶部栏（在 Normal 模式下是标题；其他模式用作状态/输入栏） ==========
    if matches!(
        app.mode,
        AppMode::Normal | AppMode::Adding | AppMode::Renaming
    ) {
        // Normal: 显示笔记本概览
        // Adding/Renaming: 输入框直接在列表内联渲染，顶部栏继续显示概览即可
        render_title_bar(f, app, chunks[0]);
    } else {
        render_status_bar(f, app, chunks[0]);
    }

    // ========== 主区域 ==========
    {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(app.panel_ratio),       // 笔记列表
                Constraint::Percentage(100 - app.panel_ratio), // 编辑器区
            ])
            .split(chunks[1]);

        render_list(f, app, main_chunks[0]);
        render_editor(f, app, main_chunks[1]);

        // 命令面板弹窗（浮动在主区域上方）
        if app.mode == AppMode::CommandPopup {
            draw_command_popup(f, app, chunks[1]);
        }
    }

    // ========== 帮助栏 ==========
    let help_text = match app.mode {
        AppMode::Normal => match app.focus {
            Focus::Tree => {
                " / 命令面板 | a 新建 | r 改名 | d 删除 | s 刷新 | o 打开目录 | ↑↓/jk 切换 | Enter 编辑 | q/Esc 退出"
            }
            Focus::Editor => " :w 保存 | :wq 保存退出 | :q 退出编辑 | Esc(Normal) 回目录树",
        },
        AppMode::Adding => " Enter 确认新建 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
        AppMode::Renaming => " Enter 确认重命名 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
        AppMode::Search => " Enter 搜索 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
        AppMode::ConfirmDelete => " y 确认删除 | n/Esc 取消",
        AppMode::CommandPopup => " ↑↓/jk 选择 | Enter 确认 | 输入筛选 | Esc 取消",
        AppMode::RatioInput => " Enter 确认 | Esc 取消 | 格式: x:y (如 20:80)",
        AppMode::Mkdir => " Enter 确认创建目录 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
        AppMode::Mv => " Enter 确认移动 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
    };
    let help_widget = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(help_widget, chunks[2]);
}

/// 渲染顶部标题栏（Normal/Adding/Renaming 模式下展示笔记本概览）
fn render_title_bar(f: &mut ratatui::Frame, app: &NotebookApp, area: Rect) {
    let total = app.notes.len();
    let dir_count = app
        .flat_entries
        .iter()
        .filter(|e| matches!(e.kind, FlatEntryKind::Dir { .. }))
        .count();
    let filter_suffix = match &app.search_filter {
        Some(kw) => format!(" [搜索: {}]", kw),
        None => String::new(),
    };
    let mut title = if dir_count > 0 {
        format!(
            " 笔记本{} — {} 篇笔记, {} 个文件夹 ",
            filter_suffix, total, dir_count
        )
    } else {
        format!(" 笔记本{} — 共 {} 篇 ", filter_suffix, total)
    };
    // 当前若有提示消息，附加显示在标题后（含 Normal 下的成功提示与 Renaming 下的错误回显）
    if let Some(msg) = app.message.as_deref()
        && !msg.is_empty()
    {
        title.push_str(&format!("· {} ", msg));
    }
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

/// 渲染笔记列表（树形结构）
fn render_list(f: &mut ratatui::Frame, app: &mut NotebookApp, area: Rect) {
    let inner_width = area.width.saturating_sub(2) as usize; // 减边框
    let selected = app.state.selected();

    let mut items: Vec<ListItem> = app
        .flat_entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = selected == Some(i);

            // 重命名模式：特殊渲染文件条目
            if let FlatEntryKind::File { note_index } = &entry.kind
                && app.mode == AppMode::Renaming
                && app.rename_index == Some(*note_index)
            {
                return build_rename_item(
                    &app.input,
                    app.cursor_pos,
                    inner_width,
                    is_selected,
                    &app.theme,
                );
            }

            // 缩进空格
            let indent_style = Style::default().fg(Color::DarkGray);

            match &entry.kind {
                FlatEntryKind::Dir {
                    name, file_count, ..
                } => {
                    let dir_style = Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD);
                    let count_str = format!(" ({})", file_count);

                    ListItem::new(Line::from(vec![
                        Span::styled(entry.guide.clone(), indent_style),
                        Span::styled(name.clone(), dir_style),
                        Span::styled(count_str, Style::default().fg(Color::DarkGray)),
                    ]))
                }
                FlatEntryKind::File { note_index } => {
                    let note = &app.notes[*note_index];
                    let name_style = Style::default().fg(Color::Reset);
                    let guide_width = unicode_width::UnicodeWidthStr::width(entry.guide.as_str());
                    let name_display_width = inner_width.saturating_sub(guide_width);
                    let display_name = note.display_name();
                    let name_text =
                        if display_name.chars().collect::<Vec<_>>().len() > name_display_width {
                            let mut s: String = display_name
                                .chars()
                                .take(name_display_width.saturating_sub(2))
                                .collect();
                            s.push_str("..");
                            s
                        } else {
                            display_name.to_string()
                        };

                    ListItem::new(Line::from(vec![
                        Span::styled(entry.guide.clone(), indent_style),
                        Span::styled(name_text, name_style),
                    ]))
                }
            }
        })
        .collect();

    // 添加模式：在列表末尾追加输入行
    if app.mode == AppMode::Adding {
        let is_selected = selected == Some(app.flat_entries.len());
        items.push(build_adding_item(
            &app.input,
            app.cursor_pos,
            inner_width,
            is_selected,
            &app.theme,
        ));
    }

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if app.focus == Focus::Tree {
            Color::Cyan
        } else {
            Color::DarkGray
        }))
        .title(" 笔记列表 ");

    if items.is_empty() {
        let empty_hint = List::new(vec![ListItem::new(Line::from(Span::styled(
            "   (空) 按 a 新建笔记...",
            Style::default().fg(Color::DarkGray),
        )))])
        .block(list_block);
        f.render_widget(empty_hint, area);
    } else {
        let list_widget = List::new(items).block(list_block).highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
        f.render_stateful_widget(list_widget, area, &mut app.state);
    }
}

/// 构建新建笔记输入行
fn build_adding_item(
    input: &str,
    cursor_pos: usize,
    width: usize,
    selected: bool,
    theme: &crate::theme::Theme,
) -> ListItem<'static> {
    let pointer = if selected {
        Span::styled(
            " ❯ ",
            Style::default()
                .fg(theme.md_h1)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("   ")
    };

    let content_width = width.saturating_sub(3); // pointer
    let cursor_lines =
        cursor_wrapped_lines(input, cursor_pos, content_width, Some("输入标题…"), theme);

    let mut item_lines: Vec<Line<'static>> = Vec::new();
    for (i, line) in cursor_lines.lines.into_iter().enumerate() {
        let mut spans = if i == 0 {
            vec![pointer.clone()]
        } else {
            vec![Span::raw("   ")]
        };
        spans.extend(line.spans);
        item_lines.push(Line::from(spans));
    }

    ListItem::new(item_lines)
}

/// 构建重命名输入行
fn build_rename_item(
    input: &str,
    cursor_pos: usize,
    width: usize,
    selected: bool,
    theme: &crate::theme::Theme,
) -> ListItem<'static> {
    build_adding_item(input, cursor_pos, width, selected, theme)
}

/// 渲染右侧编辑器区域
fn render_editor(f: &mut ratatui::Frame, app: &mut NotebookApp, area: Rect) {
    if let Some(ref mut editor) = app.editor {
        editor.render(f, area);
    } else {
        // 无内容时显示提示
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        let content = Paragraph::new(Line::from(Span::styled(
            "  选择笔记以编辑内容",
            Style::default().fg(Color::DarkGray),
        )))
        .block(block);
        f.render_widget(content, area);
    }
}

/// 渲染顶部输入/确认栏（用于 Mkdir/Mv/Search/ConfirmDelete/CommandPopup/RatioInput 模式）
fn render_status_bar(f: &mut ratatui::Frame, app: &NotebookApp, area: Rect) {
    match &app.mode {
        AppMode::Mkdir => {
            draw_status_input(
                f,
                area,
                &StatusInputParams {
                    label: "新建目录",
                    label_color: Color::Cyan,
                    input: &app.input,
                    cursor_pos: app.cursor_pos,
                    placeholder: "输入目录名…",
                    hint: "Enter 确认 | Esc 取消",
                },
                &app.theme,
            );
        }
        AppMode::Mv => {
            draw_status_input(
                f,
                area,
                &StatusInputParams {
                    label: "移动笔记",
                    label_color: Color::Magenta,
                    input: &app.input,
                    cursor_pos: app.cursor_pos,
                    placeholder: "输入目标路径…",
                    hint: "Enter 确认 | Esc 取消",
                },
                &app.theme,
            );
        }
        AppMode::Search => {
            draw_status_input(
                f,
                area,
                &StatusInputParams {
                    label: "搜索",
                    label_color: Color::Cyan,
                    input: &app.input,
                    cursor_pos: app.cursor_pos,
                    placeholder: "输入关键词…",
                    hint: "Enter 搜索 | Esc 取消",
                },
                &app.theme,
            );
        }
        AppMode::ConfirmDelete => {
            let msg = if let Some(name) = app.selected_name() {
                format!(" 确认删除\"{}\"? (y/n)", name)
            } else {
                " 没有选中的笔记".to_string()
            };
            draw_confirm_dialog(
                f,
                area,
                &ConfirmDialogConfig {
                    title: " 确认删除 ",
                    message: msg,
                    color: Color::Red,
                },
            );
        }
        AppMode::CommandPopup => {
            draw_status_input(
                f,
                area,
                &StatusInputParams {
                    label: "命令面板",
                    label_color: Color::Magenta,
                    input: &app.cmd_popup_filter,
                    cursor_pos: app.cmd_popup_filter.chars().count(),
                    placeholder: "输入筛选…",
                    hint: "↑↓ 选择 | Enter 确认 | Esc 取消",
                },
                &app.theme,
            );
        }
        AppMode::RatioInput => {
            draw_status_input(
                f,
                area,
                &StatusInputParams {
                    label: "比例",
                    label_color: Color::Yellow,
                    input: &app.input,
                    cursor_pos: app.cursor_pos,
                    placeholder: "20:80",
                    hint: "如 20:80",
                },
                &app.theme,
            );
        }
        // Normal/Adding/Renaming 走顶部标题栏，不会进入此分支
        _ => {}
    }
}

/// 绘制命令面板弹窗（浮动在主区域底部）
fn draw_command_popup(f: &mut ratatui::Frame, app: &mut NotebookApp, main_area: Rect) {
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
            theme: &app.theme,
        },
    );
}
