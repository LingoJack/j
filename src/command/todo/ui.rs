use super::app::{
    AppMode, TodoApp, count_wrapped_lines, cursor_wrapped_line, display_width,
    split_input_at_cursor, truncate_to_width,
};
use crate::constants::todo_filter;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

/// 绘制 TUI 界面
pub fn draw_ui(f: &mut ratatui::Frame, app: &mut TodoApp) {
    let size = f.area();

    let needs_preview = if app.mode == AppMode::Adding || app.mode == AppMode::Editing {
        !app.input.is_empty()
    } else {
        false
    };

    let constraints = if needs_preview {
        vec![
            Constraint::Length(3),
            Constraint::Percentage(55),
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(2),
        ]
    } else {
        vec![
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
            Constraint::Length(2),
        ]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
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
        " 📋 待办备忘录{} — 共 {} 条 | ✅ {} | ⬜ {} ",
        filter_label, total, done, undone
    );
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
    f.render_widget(title_block, chunks[0]);

    // ========== 列表区 ==========
    if app.mode == AppMode::Help {
        let help_lines = vec![
            Line::from(Span::styled(
                "  📖 快捷键帮助",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("  n / ↓ / j    ", Style::default().fg(Color::Yellow)),
                Span::raw("向下移动"),
            ]),
            Line::from(vec![
                Span::styled("  N / ↑ / k    ", Style::default().fg(Color::Yellow)),
                Span::raw("向上移动"),
            ]),
            Line::from(vec![
                Span::styled("  空格 / 回车   ", Style::default().fg(Color::Yellow)),
                Span::raw("切换完成状态 [x] / [ ]"),
            ]),
            Line::from(vec![
                Span::styled("  a            ", Style::default().fg(Color::Yellow)),
                Span::raw("添加新待办"),
            ]),
            Line::from(vec![
                Span::styled("  e            ", Style::default().fg(Color::Yellow)),
                Span::raw("编辑选中待办"),
            ]),
            Line::from(vec![
                Span::styled("  d            ", Style::default().fg(Color::Yellow)),
                Span::raw("删除待办（需确认）"),
            ]),
            Line::from(vec![
                Span::styled("  f            ", Style::default().fg(Color::Yellow)),
                Span::raw("过滤切换（全部 / 未完成 / 已完成）"),
            ]),
            Line::from(vec![
                Span::styled("  J / K        ", Style::default().fg(Color::Yellow)),
                Span::raw("调整待办顺序（下移 / 上移）"),
            ]),
            Line::from(vec![
                Span::styled("  s            ", Style::default().fg(Color::Yellow)),
                Span::raw("手动保存"),
            ]),
            Line::from(vec![
                Span::styled("  y            ", Style::default().fg(Color::Yellow)),
                Span::raw("复制选中待办到剪切板"),
            ]),
            Line::from(vec![
                Span::styled("  q            ", Style::default().fg(Color::Yellow)),
                Span::raw("退出（有未保存修改时需先保存或用 q! 强制退出）"),
            ]),
            Line::from(vec![
                Span::styled("  q!           ", Style::default().fg(Color::Yellow)),
                Span::raw("强制退出（丢弃未保存的修改）"),
            ]),
            Line::from(vec![
                Span::styled("  Esc          ", Style::default().fg(Color::Yellow)),
                Span::raw("退出（同 q）"),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+C       ", Style::default().fg(Color::Yellow)),
                Span::raw("强制退出（不保存）"),
            ]),
            Line::from(vec![
                Span::styled("  ?            ", Style::default().fg(Color::Yellow)),
                Span::raw("显示此帮助"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  添加/编辑模式下：",
                Style::default().fg(Color::Gray),
            )),
            Line::from(vec![
                Span::styled("  Alt+↓/↑      ", Style::default().fg(Color::Yellow)),
                Span::raw("预览区滚动（长文本输入时）"),
            ]),
        ];
        let help_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" 帮助 ");
        let help_widget = Paragraph::new(help_lines).block(help_block);
        f.render_widget(help_widget, chunks[1]);
    } else {
        let indices = app.filtered_indices();
        let list_inner_width = chunks[1].width.saturating_sub(2 + 3) as usize;
        let items: Vec<ListItem> = indices
            .iter()
            .map(|&idx| {
                let item = &app.list.items[idx];
                let checkbox = if item.done { "[x]" } else { "[ ]" };
                let checkbox_style = if item.done {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Yellow)
                };
                let content_style = if item.done {
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::CROSSED_OUT)
                } else {
                    Style::default().fg(Color::White)
                };

                let checkbox_str = format!(" {} ", checkbox);
                let checkbox_display_width = display_width(&checkbox_str);

                let date_str = item
                    .created_at
                    .get(..10)
                    .map(|d| format!("  ({})", d))
                    .unwrap_or_default();
                let date_display_width = display_width(&date_str);

                let content_max_width = list_inner_width
                    .saturating_sub(checkbox_display_width)
                    .saturating_sub(date_display_width);

                let content_display = truncate_to_width(&item.content, content_max_width);
                let content_actual_width = display_width(&content_display);

                let padding_width = content_max_width.saturating_sub(content_actual_width);
                let padding = " ".repeat(padding_width);

                ListItem::new(Line::from(vec![
                    Span::styled(checkbox_str, checkbox_style),
                    Span::styled(content_display, content_style),
                    Span::raw(padding),
                    Span::styled(date_str, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .title(" 待办列表 ");

        if items.is_empty() {
            let empty_hint = List::new(vec![ListItem::new(Line::from(Span::styled(
                "   (空) 按 a 添加新待办...",
                Style::default().fg(Color::DarkGray),
            )))])
            .block(list_block);
            f.render_widget(empty_hint, chunks[1]);
        } else {
            let list_widget = List::new(items)
                .block(list_block)
                .highlight_style(
                    Style::default()
                        .bg(Color::Indexed(24))
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(" ▶ ");
            f.render_stateful_widget(list_widget, chunks[1], &mut app.state);
        };
    }

    // ========== 预览区 ==========
    let (_preview_chunk_idx, status_chunk_idx, help_chunk_idx) = if needs_preview {
        let input_content = &app.input;
        let preview_inner_w = (chunks[2].width.saturating_sub(2)) as usize;
        let preview_inner_h = chunks[2].height.saturating_sub(2) as u16;

        let total_wrapped = count_wrapped_lines(input_content, preview_inner_w) as u16;
        let max_scroll = total_wrapped.saturating_sub(preview_inner_h);

        // 自动滚动到光标所在行可见
        let cursor_line = cursor_wrapped_line(input_content, app.cursor_pos, preview_inner_w);
        let auto_scroll = if cursor_line < app.preview_scroll {
            cursor_line
        } else if cursor_line >= app.preview_scroll + preview_inner_h {
            cursor_line.saturating_sub(preview_inner_h - 1)
        } else {
            app.preview_scroll
        };
        let clamped_scroll = auto_scroll.min(max_scroll);
        app.preview_scroll = clamped_scroll;

        let mode_label = match app.mode {
            AppMode::Adding => "新待办",
            AppMode::Editing => "编辑中",
            _ => "预览",
        };
        let title = if total_wrapped > preview_inner_h {
            format!(
                " 📖 {} 预览 [{}/{}行] Alt+↓/↑滚动 ",
                mode_label,
                clamped_scroll + preview_inner_h,
                total_wrapped
            )
        } else {
            format!(" 📖 {} 预览 ", mode_label)
        };

        let preview_block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .border_style(Style::default().fg(Color::Cyan));

        // 构建带光标高亮的预览文本
        let (before, cursor_ch, after) = split_input_at_cursor(input_content, app.cursor_pos);
        let cursor_style = Style::default().fg(Color::Black).bg(Color::White);
        let preview_text = vec![Line::from(vec![
            Span::styled(before, Style::default().fg(Color::White)),
            Span::styled(cursor_ch, cursor_style),
            Span::styled(after, Style::default().fg(Color::White)),
        ])];

        use ratatui::widgets::Wrap;
        let preview = Paragraph::new(preview_text)
            .block(preview_block)
            .wrap(Wrap { trim: false })
            .scroll((clamped_scroll, 0));
        f.render_widget(preview, chunks[2]);
        (2, 3, 4)
    } else {
        (1, 2, 3)
    };

    // ========== 状态/输入栏 ==========
    match &app.mode {
        AppMode::Adding => {
            let (before, cursor_ch, after) = split_input_at_cursor(&app.input, app.cursor_pos);
            let input_widget = Paragraph::new(Line::from(vec![
                Span::styled(" 新待办: ", Style::default().fg(Color::Green)),
                Span::raw(before),
                Span::styled(
                    cursor_ch,
                    Style::default().fg(Color::Black).bg(Color::White),
                ),
                Span::raw(after),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green))
                    .title(" 添加模式 (Enter 确认 / Esc 取消 / ←→ 移动光标) "),
            );
            f.render_widget(input_widget, chunks[status_chunk_idx]);
        }
        AppMode::Editing => {
            let (before, cursor_ch, after) = split_input_at_cursor(&app.input, app.cursor_pos);
            let input_widget = Paragraph::new(Line::from(vec![
                Span::styled(" 编辑: ", Style::default().fg(Color::Yellow)),
                Span::raw(before),
                Span::styled(
                    cursor_ch,
                    Style::default().fg(Color::Black).bg(Color::White),
                ),
                Span::raw(after),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" 编辑模式 (Enter 确认 / Esc 取消 / ←→ 移动光标) "),
            );
            f.render_widget(input_widget, chunks[status_chunk_idx]);
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
            let confirm_widget = Paragraph::new(Line::from(Span::styled(
                msg,
                Style::default().fg(Color::Red),
            )))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red))
                    .title(" ⚠️ 确认删除 "),
            );
            f.render_widget(confirm_widget, chunks[2]);
        }
        AppMode::ConfirmReport => {
            let inner_width = chunks[status_chunk_idx].width.saturating_sub(2) as usize;
            let msg = if let Some(ref content) = app.report_pending_content {
                // 预留前缀和后缀的显示宽度
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
            let confirm_widget = Paragraph::new(Line::from(Span::styled(
                msg,
                Style::default().fg(Color::Cyan),
            )))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(" 📝 写入日报 "),
            );
            f.render_widget(confirm_widget, chunks[status_chunk_idx]);
        }
        AppMode::ConfirmCancelInput => {
            let inner_width = chunks[status_chunk_idx].width.saturating_sub(2) as usize;
            let prefix = " ⚠️ 是否保存？当前输入: \"";
            let suffix = "\" (Enter/y 保存 / n/Esc 放弃 / 其他键继续编辑)";
            let prefix_w = display_width(prefix);
            let suffix_w = display_width(suffix);
            let budget = inner_width.saturating_sub(prefix_w + suffix_w);
            let truncated = truncate_to_width(&app.input, budget);
            let msg = format!("{}{}{}", prefix, truncated, suffix);
            let confirm_widget = Paragraph::new(Line::from(Span::styled(
                msg,
                Style::default().fg(Color::Yellow),
            )))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" ⚠️ 未保存的内容 "),
            );
            f.render_widget(confirm_widget, chunks[status_chunk_idx]);
        }
        AppMode::Normal | AppMode::Help => {
            let msg = app.message.as_deref().unwrap_or("按 ? 查看完整帮助");
            let dirty_indicator = if app.is_dirty() { " [未保存]" } else { "" };
            let status_widget = Paragraph::new(Line::from(vec![
                Span::styled(msg, Style::default().fg(Color::Gray)),
                Span::styled(
                    dirty_indicator,
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            f.render_widget(status_widget, chunks[2]);
        }
    }

    // ========== 帮助栏 ==========
    let help_text = match app.mode {
        AppMode::Normal => {
            " n/↓ 下移 | N/↑ 上移 | 空格/回车 切换完成 | a 添加 | e 编辑 | d 删除 | y 复制 | f 过滤 | s 保存 | ? 帮助 | q 退出"
        }
        AppMode::Adding | AppMode::Editing => {
            " Enter 确认 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾 | Alt+↓/↑ 预览滚动"
        }
        AppMode::ConfirmDelete => " y 确认删除 | n/Esc 取消",
        AppMode::ConfirmReport => " Enter/y 写入日报并保存 | 其他键 跳过",
        AppMode::ConfirmCancelInput => " Enter/y 保存 | n/Esc 放弃 | 其他键 继续编辑",
        AppMode::Help => " 按任意键返回",
    };
    let help_widget = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(help_widget, chunks[help_chunk_idx]);
}
