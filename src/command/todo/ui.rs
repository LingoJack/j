use super::app::{AppMode, TodoApp, display_width, truncate_to_width};
use crate::constants::todo_filter;
use crate::util::text::wrap_text;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
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
        render_help(f, chunks[1]);
    } else {
        render_list(f, app, chunks[1]);
    }

    // ========== 状态栏 ==========
    render_status_bar(f, app, chunks[2]);

    // ========== 帮助栏 ==========
    let help_text = match app.mode {
        AppMode::Normal => {
            " n/↓ 下移 | N/↑ 上移 | 空格/回车 切换完成 | a 添加 | e 编辑 | d 删除 | y 复制 | f 过滤 | s 保存 | ? 帮助 | q 退出"
        }
        AppMode::Adding | AppMode::Editing => {
            " Enter 确认 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾"
        }
        AppMode::ConfirmDelete => " y 确认删除 | n/Esc 取消",
        AppMode::ConfirmReport => " Enter/y 写入日报 | 其他键跳过",
        AppMode::ConfirmCancelInput => " Enter/y 保存 | n/Esc 放弃 | 其他键继续编辑",
        AppMode::Help => " 按任意键返回",
    };
    let help_widget = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(help_widget, chunks[3]);
}

/// 渲染帮助页
fn render_help(f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
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
    ];
    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" 帮助 ");
    let help_widget = Paragraph::new(help_lines).block(help_block);
    f.render_widget(help_widget, area);
}

/// 渲染列表区（含 inline 编辑）
fn render_list(f: &mut ratatui::Frame, app: &mut TodoApp, area: ratatui::layout::Rect) {
    let indices = app.filtered_indices();
    // 指针占 3 列（" ❯ " 或 "   "），加上边框 2 列
    let list_inner_width = area.width.saturating_sub(2 + 3) as usize;
    let checkbox_w = display_width(" [x] ");
    let content_width = list_inner_width.saturating_sub(checkbox_w);

    let selected = app.state.selected();
    let mut items: Vec<ListItem> = indices
        .iter()
        .enumerate()
        .map(|(i, &idx)| {
            let is_selected = selected == Some(i);
            // 编辑模式下替换选中项为编辑行
            if app.mode == AppMode::Editing && app.edit_index == Some(idx) {
                return build_editing_item(
                    &app.input,
                    app.cursor_pos,
                    content_width,
                    checkbox_w,
                    is_selected,
                );
            }
            build_normal_item(
                &app.list.items[idx],
                list_inner_width,
                checkbox_w,
                is_selected,
            )
        })
        .collect();

    // 添加模式：在列表末尾追加编辑行
    if app.mode == AppMode::Adding {
        let is_selected = selected == Some(indices.len());
        items.push(build_editing_item(
            &app.input,
            app.cursor_pos,
            content_width,
            checkbox_w,
            is_selected,
        ));
    }

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
        f.render_widget(empty_hint, area);
    } else {
        let list_widget = List::new(items)
            .block(list_block)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));
        f.render_stateful_widget(list_widget, area, &mut app.state);
    }
}

/// 指针 Span（选中时显示彩色 ❯，未选中显示等宽空格）
fn pointer_span(selected: bool) -> Span<'static> {
    if selected {
        Span::styled(
            " ❯ ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("   ")
    }
}

/// 构建普通列表项
fn build_normal_item(
    item: &super::app::TodoItem,
    list_inner_width: usize,
    checkbox_w: usize,
    selected: bool,
) -> ListItem<'static> {
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
    let date_str = item
        .created_at
        .get(..10)
        .map(|d| format!("  ({})", d))
        .unwrap_or_default();
    let date_display_width = display_width(&date_str);

    // 所有行统一按完整内容宽度折行，日期不影响折行
    let content_width = list_inner_width.saturating_sub(checkbox_w);
    let wrapped = wrap_text(&item.content, content_width);
    let indent = " ".repeat(checkbox_w);

    let mut item_lines: Vec<Line> = Vec::new();
    for (i, line_text) in wrapped.iter().enumerate() {
        if i == 0 {
            item_lines.push(Line::from(vec![
                pointer_span(selected),
                Span::styled(checkbox_str.clone(), checkbox_style),
                Span::styled(line_text.clone(), content_style),
            ]));
        } else {
            item_lines.push(Line::from(vec![
                Span::raw("   "),
                Span::raw(indent.clone()),
                Span::styled(line_text.clone(), content_style),
            ]));
        }
    }

    // 日期追加在最后一行末尾（放得下）或另起一行
    let last_content_w = wrapped.last().map(|s| display_width(s)).unwrap_or(0);
    let last_line_total = checkbox_w + last_content_w + date_display_width;

    if last_line_total <= list_inner_width {
        // 追加到最后一行
        if let Some(last) = item_lines.last_mut() {
            let padding_w =
                list_inner_width.saturating_sub(checkbox_w + last_content_w + date_display_width);
            last.spans.push(Span::raw(" ".repeat(padding_w)));
            last.spans
                .push(Span::styled(date_str, Style::default().fg(Color::DarkGray)));
        }
    } else {
        // 另起一行，右对齐
        let padding_w = list_inner_width.saturating_sub(date_display_width);
        item_lines.push(Line::from(vec![
            Span::raw(" ".repeat(padding_w)),
            Span::styled(date_str, Style::default().fg(Color::DarkGray)),
        ]));
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
) -> ListItem<'static> {
    let indent = " ".repeat(checkbox_w);
    let cursor_lines = build_cursor_wrapped_lines(input, cursor_pos, content_width);

    let mut item_lines: Vec<Line> = Vec::new();
    for (i, line) in cursor_lines.into_iter().enumerate() {
        let mut spans = if i == 0 {
            vec![pointer_span(selected)]
        } else {
            vec![Span::raw("   ")]
        };
        spans.push(Span::raw(indent.clone()));
        spans.extend(line.spans);
        item_lines.push(Line::from(spans));
    }

    ListItem::new(item_lines)
}

/// 将输入文本折行并在正确位置渲染光标
fn build_cursor_wrapped_lines(input: &str, cursor_pos: usize, width: usize) -> Vec<Line<'static>> {
    let cursor_style = Style::default().fg(Color::Black).bg(Color::White);
    let text_style = Style::default().fg(Color::White);

    if input.is_empty() {
        return vec![Line::from(vec![
            Span::styled(" ".to_string(), cursor_style),
            Span::styled(
                " 输入内容…".to_string(),
                Style::default().fg(Color::DarkGray),
            ),
        ])];
    }

    let wrapped = wrap_text(input, width);
    let mut char_offset = 0;
    let mut result = Vec::new();
    let mut cursor_placed = false;

    for (line_idx, line_str) in wrapped.iter().enumerate() {
        let line_chars: Vec<char> = line_str.chars().collect();
        let line_len = line_chars.len();
        let is_last = line_idx == wrapped.len() - 1;

        let cursor_on_this_line = !cursor_placed
            && (cursor_pos < char_offset + line_len
                || (is_last && cursor_pos == char_offset + line_len));

        if cursor_on_this_line {
            cursor_placed = true;
            let pos_in_line = cursor_pos - char_offset;
            let before: String = line_chars[..pos_in_line].iter().collect();
            let (cursor_ch, after) = if pos_in_line < line_len {
                (
                    line_chars[pos_in_line].to_string(),
                    line_chars[pos_in_line + 1..].iter().collect::<String>(),
                )
            } else {
                (" ".to_string(), String::new())
            };

            result.push(Line::from(vec![
                Span::styled(before, text_style),
                Span::styled(cursor_ch, cursor_style),
                Span::styled(after, text_style),
            ]));
        } else {
            result.push(Line::from(Span::styled(line_str.clone(), text_style)));
        }

        char_offset += line_len;
    }

    result
}

/// 渲染状态栏
fn render_status_bar(f: &mut ratatui::Frame, app: &TodoApp, area: ratatui::layout::Rect) {
    match &app.mode {
        AppMode::Adding => {
            let status = Paragraph::new(Line::from(vec![
                Span::styled(
                    " ✏️  添加模式",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" — 在列表中输入内容", Style::default().fg(Color::DarkGray)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
            );
            f.render_widget(status, area);
        }
        AppMode::Editing => {
            let status = Paragraph::new(Line::from(vec![
                Span::styled(
                    " ✏️  编辑模式",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" — 在列表中修改内容", Style::default().fg(Color::DarkGray)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
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
            f.render_widget(confirm_widget, area);
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
            f.render_widget(confirm_widget, area);
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
            f.render_widget(confirm_widget, area);
        }
        _ => {
            // Normal / Help
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
            f.render_widget(status_widget, area);
        }
    }
}
