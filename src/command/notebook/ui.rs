use super::app::{AppMode, NotebookApp, format_time};
use crate::util::text::wrap_text;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

/// 绘制 TUI 界面
pub fn draw_ui(f: &mut ratatui::Frame, app: &mut NotebookApp) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 标题栏
            Constraint::Min(5),    // 主区域
            Constraint::Length(3), // 状态栏
            Constraint::Length(1), // 帮助栏
        ])
        .split(size);

    // ========== 标题栏 ==========
    let total = app.notes.len();
    let filter_suffix = match &app.search_filter {
        Some(kw) => format!(" [搜索: {}]", kw),
        None => String::new(),
    };
    let title = format!(" 📓 笔记本{} — 共 {} 篇 ", filter_suffix, total);
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

    // ========== 主区域 ==========
    if app.mode == AppMode::Help {
        render_help(f, chunks[1]);
    } else if app.mode == AppMode::Preview {
        render_preview_full(f, app, chunks[1]);
    } else {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // 笔记列表
                Constraint::Percentage(70), // 预览区
            ])
            .split(chunks[1]);

        render_list(f, app, main_chunks[0]);
        render_preview(f, app, main_chunks[1]);
    }

    // ========== 状态栏 ==========
    render_status_bar(f, app, chunks[2]);

    // ========== 帮助栏 ==========
    let help_text = match app.mode {
        AppMode::Normal => {
            " n/↓ 下移 | N/↑ 上移 | Enter/e 编辑 | a 新建 | d 删除 | r 重命名 | p 预览 | / 搜索 | y 复制 | o 打开目录 | s 刷新 | ? 帮助 | q 退出"
        }
        AppMode::Preview => " ↑↓/jk 滚动 | n/N 切换笔记 | p/Esc 退出预览",
        AppMode::Adding => " Enter 确认新建 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
        AppMode::Renaming => " Enter 确认重命名 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
        AppMode::Search => " Enter 搜索 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
        AppMode::ConfirmDelete => " y 确认删除 | n/Esc 取消",
        AppMode::Help => " 按任意键返回",
    };
    let help_widget = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(help_widget, chunks[3]);
}

/// 渲染笔记列表
fn render_list(f: &mut ratatui::Frame, app: &mut NotebookApp, area: Rect) {
    let indices = app.filtered_indices();
    let inner_width = area.width.saturating_sub(2) as usize; // 减边框

    let selected = app.state.selected();

    let mut items: Vec<ListItem> = indices
        .iter()
        .enumerate()
        .map(|(i, &idx)| {
            let note = &app.notes[idx];
            let is_selected = selected == Some(i);

            // 输入模式下的特殊渲染
            if app.mode == AppMode::Renaming && app.rename_index == Some(idx) {
                return build_rename_item(&app.input, app.cursor_pos, inner_width, is_selected);
            }

            let pointer = if is_selected {
                Span::styled(
                    " ❯ ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw("   ")
            };

            let name_style = if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let time_str = format_time(note.mtime);
            let name_display_width = inner_width.saturating_sub(3 + 17); // pointer + time
            let name_text = if note.name.chars().collect::<Vec<_>>().len() > name_display_width {
                let mut s: String = note
                    .name
                    .chars()
                    .take(name_display_width.saturating_sub(2))
                    .collect();
                s.push_str("..");
                s
            } else {
                note.name.clone()
            };

            let padding = name_display_width
                .saturating_sub(unicode_width::UnicodeWidthStr::width(name_text.as_str()));

            ListItem::new(Line::from(vec![
                pointer,
                Span::styled(name_text, name_style),
                Span::raw(" ".repeat(padding)),
                Span::styled(time_str, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    // 添加模式：在列表末尾追加输入行
    if app.mode == AppMode::Adding {
        let is_selected = selected == Some(indices.len());
        items.push(build_adding_item(
            &app.input,
            app.cursor_pos,
            inner_width,
            is_selected,
        ));
    }

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title(" 笔记列表 ");

    if items.is_empty() {
        let empty_hint = List::new(vec![ListItem::new(Line::from(Span::styled(
            "   (空) 按 a 新建笔记...",
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

/// 构建新建笔记输入行
fn build_adding_item(
    input: &str,
    cursor_pos: usize,
    width: usize,
    selected: bool,
) -> ListItem<'static> {
    let pointer = if selected {
        Span::styled(
            " ❯ ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("   ")
    };

    let content_width = width.saturating_sub(3); // pointer

    if input.is_empty() {
        return ListItem::new(Line::from(vec![
            pointer,
            Span::styled(
                "输入标题…".to_string(),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(" ", Style::default().fg(Color::Black).bg(Color::White)),
        ]));
    }

    let cursor_style = Style::default().fg(Color::Black).bg(Color::White);
    let text_style = Style::default().fg(Color::White);

    let wrapped = wrap_text(input, content_width);
    let mut char_offset = 0;
    let mut cursor_placed = false;

    let mut lines = Vec::new();
    for (line_idx, line_str) in wrapped.iter().enumerate() {
        let line_chars: Vec<char> = line_str.chars().collect();
        let line_len = line_chars.len();
        let is_last = line_idx == wrapped.len() - 1;

        let cursor_on_this_line = !cursor_placed
            && (cursor_pos < char_offset + line_len
                || (is_last && cursor_pos == char_offset + line_len));

        if line_idx == 0 {
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
                lines.push(Line::from(vec![
                    pointer.clone(),
                    Span::styled(before, text_style),
                    Span::styled(cursor_ch, cursor_style),
                    Span::styled(after, text_style),
                ]));
            } else {
                lines.push(Line::from(vec![
                    pointer.clone(),
                    Span::styled(line_str.clone(), text_style),
                ]));
            }
        } else {
            let indent = Span::raw("   ");
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
                lines.push(Line::from(vec![
                    indent,
                    Span::styled(before, text_style),
                    Span::styled(cursor_ch, cursor_style),
                    Span::styled(after, text_style),
                ]));
            } else {
                lines.push(Line::from(vec![
                    indent,
                    Span::styled(line_str.clone(), text_style),
                ]));
            }
        }
        char_offset += line_len;
    }

    ListItem::new(lines)
}

/// 构建重命名输入行
fn build_rename_item(
    input: &str,
    cursor_pos: usize,
    width: usize,
    selected: bool,
) -> ListItem<'static> {
    // 复用 adding 逻辑
    build_adding_item(input, cursor_pos, width, selected)
}

/// 渲染右侧预览区
fn render_preview(f: &mut ratatui::Frame, app: &mut NotebookApp, area: Rect) {
    let inner_width = area.width.saturating_sub(2); // 减边框
    app.render_preview_with_width(inner_width);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title(" 预览 ");

    let content = if app.preview_lines.is_empty() {
        match &app.preview_content {
            Some(_) => Paragraph::new(Line::from(Span::styled(
                "  (空笔记)",
                Style::default().fg(Color::DarkGray),
            )))
            .block(block),
            None => Paragraph::new(Line::from(Span::styled(
                "  选择笔记以预览内容",
                Style::default().fg(Color::DarkGray),
            )))
            .block(block),
        }
    } else {
        Paragraph::new(app.preview_lines.clone())
            .block(block)
            .wrap(Wrap { trim: false })
    };
    f.render_widget(content, area);
}

/// 渲染全屏预览
fn render_preview_full(f: &mut ratatui::Frame, app: &mut NotebookApp, area: Rect) {
    let inner_width = area.width.saturating_sub(2);
    app.render_preview_with_width(inner_width);

    let title = match app.selected_name() {
        Some(name) => format!(" 📖 {} ", name),
        None => " 📖 预览 ".to_string(),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    if app.preview_lines.is_empty() {
        let content = Paragraph::new(Line::from(Span::styled(
            "  (空)",
            Style::default().fg(Color::DarkGray),
        )))
        .block(block);
        f.render_widget(content, area);
    } else {
        let scroll = app.preview_scroll as usize;
        let visible_lines: Vec<Line> = app.preview_lines.iter().skip(scroll).cloned().collect();
        if visible_lines.is_empty() {
            let content = Paragraph::new(Line::from(Span::styled(
                "  (已到末尾)",
                Style::default().fg(Color::DarkGray),
            )))
            .block(block);
            f.render_widget(content, area);
        } else {
            let content = Paragraph::new(visible_lines).block(block);
            f.render_widget(content, area);
        }
    }
}

/// 渲染帮助页
fn render_help(f: &mut ratatui::Frame, area: Rect) {
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
            Span::styled("  Enter / e    ", Style::default().fg(Color::Yellow)),
            Span::raw("编辑笔记（Markdown 编辑器）"),
        ]),
        Line::from(vec![
            Span::styled("  a            ", Style::default().fg(Color::Yellow)),
            Span::raw("新建笔记"),
        ]),
        Line::from(vec![
            Span::styled("  d            ", Style::default().fg(Color::Yellow)),
            Span::raw("删除笔记（需确认）"),
        ]),
        Line::from(vec![
            Span::styled("  r            ", Style::default().fg(Color::Yellow)),
            Span::raw("重命名笔记"),
        ]),
        Line::from(vec![
            Span::styled("  p            ", Style::default().fg(Color::Yellow)),
            Span::raw("全屏预览当前笔记"),
        ]),
        Line::from(vec![
            Span::styled("  /            ", Style::default().fg(Color::Yellow)),
            Span::raw("搜索笔记（标题+内容）"),
        ]),
        Line::from(vec![
            Span::styled("  y            ", Style::default().fg(Color::Yellow)),
            Span::raw("复制笔记名到剪切板"),
        ]),
        Line::from(vec![
            Span::styled("  o            ", Style::default().fg(Color::Yellow)),
            Span::raw("在 Finder 中打开 notebook 目录"),
        ]),
        Line::from(vec![
            Span::styled("  s            ", Style::default().fg(Color::Yellow)),
            Span::raw("刷新笔记列表"),
        ]),
        Line::from(vec![
            Span::styled("  Esc          ", Style::default().fg(Color::Yellow)),
            Span::raw("清除搜索 / 退出"),
        ]),
        Line::from(vec![
            Span::styled("  q            ", Style::default().fg(Color::Yellow)),
            Span::raw("退出"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+C       ", Style::default().fg(Color::Yellow)),
            Span::raw("强制退出"),
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

/// 渲染状态栏
fn render_status_bar(f: &mut ratatui::Frame, app: &NotebookApp, area: Rect) {
    match &app.mode {
        AppMode::Adding => {
            let status = Paragraph::new(Line::from(vec![
                Span::styled(
                    " ✏️  新建笔记",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " — 输入标题后按 Enter 创建",
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
            );
            f.render_widget(status, area);
        }
        AppMode::Renaming => {
            let status = Paragraph::new(Line::from(vec![
                Span::styled(
                    " ✏️  重命名",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " — 输入新名称后按 Enter 确认",
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );
            f.render_widget(status, area);
        }
        AppMode::Search => {
            let status = Paragraph::new(Line::from(vec![
                Span::styled(
                    " 🔍 搜索",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " — 输入关键词后按 Enter 搜索",
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );
            f.render_widget(status, area);
        }
        AppMode::ConfirmDelete => {
            let msg = if let Some(name) = app.selected_name() {
                format!(" 确认删除「{}」？(y 确认 / n 取消)", name)
            } else {
                " 没有选中的笔记".to_string()
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
        AppMode::Preview => {
            let status = Paragraph::new(Line::from(vec![
                Span::styled(
                    " 📖 预览模式",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " — ↑↓/jk 滚动 | p/Esc 退出预览",
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );
            f.render_widget(status, area);
        }
        _ => {
            // Normal / Help
            let msg = app.message.as_deref().unwrap_or("按 ? 查看完整帮助");
            let status_widget = Paragraph::new(Line::from(Span::styled(
                format!(" {}", msg),
                Style::default().fg(Color::Gray),
            )))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            f.render_widget(status_widget, area);
        }
    }
}
