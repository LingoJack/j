use super::app::{AppMode, FlatEntryKind, NotebookApp};
use crate::util::text::wrap_text;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
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
    let dir_count = app
        .flat_entries
        .iter()
        .filter(|e| matches!(e.kind, FlatEntryKind::Dir { .. }))
        .count();
    let filter_suffix = match &app.search_filter {
        Some(kw) => format!(" [搜索: {}]", kw),
        None => String::new(),
    };
    let title = if dir_count > 0 {
        format!(
            " 笔记本{} — {} 篇笔记, {} 个文件夹 ",
            filter_suffix, total, dir_count
        )
    } else {
        format!(" 笔记本{} — 共 {} 篇 ", filter_suffix, total)
    };
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
                Constraint::Percentage(app.panel_ratio),       // 笔记列表
                Constraint::Percentage(100 - app.panel_ratio), // 预览区
            ])
            .split(chunks[1]);

        render_list(f, app, main_chunks[0]);
        render_preview(f, app, main_chunks[1]);

        // 命令面板弹窗（浮动在主区域上方）
        if app.mode == AppMode::CommandPopup {
            draw_command_popup(f, app, chunks[1]);
        }
    }

    // ========== 状态栏 ==========
    render_status_bar(f, app, chunks[2]);

    // ========== 帮助栏 ==========
    let help_text = match app.mode {
        AppMode::Normal => {
            " n/↓ 下移 | N/↑ 上移 | Enter/e 编辑 | a 新建 | d 删除 | r 重命名 | Tab 展开/折叠 | p 预览 | / 命令面板 | [ ] 调整比例 | y 复制 | o 打开目录 | s 刷新 | ? 帮助 | q 退出"
        }
        AppMode::Preview => " ↑↓/jk 滚动 | n/N 切换笔记 | p/Esc 退出预览",
        AppMode::Adding => " Enter 确认新建 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
        AppMode::Renaming => " Enter 确认重命名 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
        AppMode::Search => " Enter 搜索 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
        AppMode::ConfirmDelete => " y 确认删除 | n/Esc 取消",
        AppMode::Help => " 按任意键返回",
        AppMode::CommandPopup => " ↑↓/jk 选择 | Enter 确认 | 输入筛选 | Esc 取消",
        AppMode::RatioInput => " Enter 确认 | Esc 取消 | 格式: x:y (如 20:80)",
        AppMode::Mkdir => " Enter 确认创建目录 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
        AppMode::Mv => " Enter 确认移动 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
    };
    let help_widget = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(help_widget, chunks[3]);
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
                return build_rename_item(&app.input, app.cursor_pos, inner_width, is_selected);
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
        ));
    }

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
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
    let text_style = Style::default().fg(Color::Reset);

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
        .border_style(Style::default().fg(Color::DarkGray));

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
            .scroll((app.preview_scroll, 0))
    };
    f.render_widget(content, area);
}

/// 渲染全屏预览
fn render_preview_full(f: &mut ratatui::Frame, app: &mut NotebookApp, area: Rect) {
    let inner_width = area.width.saturating_sub(2);
    app.render_preview_with_width(inner_width);

    let title = match app.selected_name() {
        Some(name) => format!(" 预览: {} ", name),
        None => " 预览 ".to_string(),
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
            "  快捷键帮助",
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
            Span::styled("  Tab          ", Style::default().fg(Color::Yellow)),
            Span::raw("展开/折叠目录"),
        ]),
        Line::from(vec![
            Span::styled("  / mkdir      ", Style::default().fg(Color::Yellow)),
            Span::raw("新建目录"),
        ]),
        Line::from(vec![
            Span::styled("  / mv         ", Style::default().fg(Color::Yellow)),
            Span::raw("移动笔记到目录"),
        ]),
        Line::from(vec![
            Span::styled("  p            ", Style::default().fg(Color::Yellow)),
            Span::raw("全屏预览当前笔记"),
        ]),
        Line::from(vec![
            Span::styled("  /            ", Style::default().fg(Color::Yellow)),
            Span::raw("打开命令面板"),
        ]),
        Line::from(vec![
            Span::styled("  [            ", Style::default().fg(Color::Yellow)),
            Span::raw("缩小左侧面板 (-5%)"),
        ]),
        Line::from(vec![
            Span::styled("  ]            ", Style::default().fg(Color::Yellow)),
            Span::raw("扩大左侧面板 (+5%)"),
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

/// 状态栏输入框渲染参数
struct InputStatusBarParams<'a> {
    label: &'a str,
    label_color: Color,
    input: &'a str,
    cursor_pos: usize,
    placeholder: &'a str,
    hint: &'a str,
}

/// 在状态栏区域渲染输入框（带标签、文本、光标、占位符）
fn render_input_status_bar(f: &mut ratatui::Frame, area: Rect, params: &InputStatusBarParams<'_>) {
    let cursor_style = Style::default().fg(Color::Black).bg(Color::White);
    let text_style = Style::default().fg(Color::Reset);
    let placeholder_style = Style::default().fg(Color::DarkGray);
    let hint_style = Style::default().fg(Color::DarkGray);

    let _inner_width = area.width.saturating_sub(2) as usize; // 减边框
    let label_display = format!(" {} ", params.label);
    let label_width = unicode_width::UnicodeWidthStr::width(label_display.as_str());

    let mut spans = vec![Span::styled(
        label_display,
        Style::default()
            .fg(params.label_color)
            .add_modifier(Modifier::BOLD),
    )];

    if params.input.is_empty() {
        // 空输入：显示占位符 + 闪烁光标
        spans.push(Span::styled(" ", cursor_style));
        spans.push(Span::styled(
            params.placeholder.to_string(),
            placeholder_style,
        ));
    } else {
        // 有输入：逐字符渲染，光标位置高亮
        let input_chars: Vec<char> = params.input.chars().collect();
        for (i, ch) in input_chars.iter().enumerate() {
            if i == params.cursor_pos {
                spans.push(Span::styled(ch.to_string(), cursor_style));
            } else {
                spans.push(Span::styled(ch.to_string(), text_style));
            }
        }
        if params.cursor_pos >= input_chars.len() {
            spans.push(Span::styled(" ", cursor_style));
        }
    }

    // 添加提示
    spans.push(Span::styled(format!("  {}", params.hint), hint_style));

    let status = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(params.label_color)),
    );
    f.render_widget(status, area);

    // 设置终端光标位置
    let cursor_x_offset = if params.input.is_empty() {
        0
    } else {
        let chars_before_cursor: String = params.input.chars().take(params.cursor_pos).collect();
        unicode_width::UnicodeWidthStr::width(chars_before_cursor.as_str())
    };
    let cursor_x = area.x + 1 + label_width as u16 + cursor_x_offset as u16;
    let cursor_y = area.y + 1; // 3 行状态栏的中间行
    if cursor_x < area.x + area.width.saturating_sub(1) && cursor_y < area.y + area.height {
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

/// 渲染状态栏
fn render_status_bar(f: &mut ratatui::Frame, app: &NotebookApp, area: Rect) {
    match &app.mode {
        AppMode::Adding => {
            let status = Paragraph::new(Line::from(vec![
                Span::styled(
                    " 新建笔记",
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
                    " 重命名",
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
        AppMode::Mkdir => {
            render_input_status_bar(
                f,
                area,
                &InputStatusBarParams {
                    label: "新建目录",
                    label_color: Color::Cyan,
                    input: &app.input,
                    cursor_pos: app.cursor_pos,
                    placeholder: "输入目录名…",
                    hint: "Enter 确认 | Esc 取消",
                },
            );
        }
        AppMode::Mv => {
            render_input_status_bar(
                f,
                area,
                &InputStatusBarParams {
                    label: "移动笔记",
                    label_color: Color::Magenta,
                    input: &app.input,
                    cursor_pos: app.cursor_pos,
                    placeholder: "输入目标路径…",
                    hint: "Enter 确认 | Esc 取消",
                },
            );
        }
        AppMode::Search => {
            render_input_status_bar(
                f,
                area,
                &InputStatusBarParams {
                    label: "搜索",
                    label_color: Color::Cyan,
                    input: &app.input,
                    cursor_pos: app.cursor_pos,
                    placeholder: "输入关键词…",
                    hint: "Enter 搜索 | Esc 取消",
                },
            );
        }
        AppMode::ConfirmDelete => {
            let msg = if let Some(name) = app.selected_name() {
                format!(" 确认删除\"{}\"? (y/n)", name)
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
                    .title(" 确认删除 "),
            );
            f.render_widget(confirm_widget, area);
        }
        AppMode::Preview => {
            let status = Paragraph::new(Line::from(vec![
                Span::styled(
                    " 预览模式",
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
        AppMode::CommandPopup => {
            render_input_status_bar(
                f,
                area,
                &InputStatusBarParams {
                    label: "命令面板",
                    label_color: Color::Magenta,
                    input: &app.cmd_popup_filter,
                    cursor_pos: app.cmd_popup_filter.chars().count(),
                    placeholder: "输入筛选…",
                    hint: "↑↓ 选择 | Enter 确认 | Esc 取消",
                },
            );
        }
        AppMode::RatioInput => {
            render_input_status_bar(
                f,
                area,
                &InputStatusBarParams {
                    label: "比例",
                    label_color: Color::Yellow,
                    input: &app.input,
                    cursor_pos: app.cursor_pos,
                    placeholder: "20:80",
                    hint: "如 20:80",
                },
            );
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

/// 绘制命令面板弹窗（浮动在主区域底部）
fn draw_command_popup(f: &mut ratatui::Frame, app: &mut NotebookApp, main_area: Rect) {
    let items = app.filtered_cmd_items();
    if items.is_empty() {
        return;
    }

    let item_count = items.len();
    let popup_height = (item_count as u16) + 2; // 内容 + 边框

    // 计算宽度：基于最长 "key - 描述" 组合
    let max_label_width = items
        .iter()
        .map(|(_, key, label)| {
            // "  key - 描述"
            2 + unicode_width::UnicodeWidthStr::width(*key)
                + 3
                + unicode_width::UnicodeWidthStr::width(*label)
        })
        .max()
        .unwrap_or(16)
        .max(16);
    let popup_width = (max_label_width as u16 + 2) // 边框
        .min(main_area.width.saturating_sub(4));

    // 位置：主区域底部偏左
    let x = main_area.x + 2;
    let y = main_area
        .bottom()
        .saturating_sub(popup_height)
        .max(main_area.y);
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // 标题
    let title = if app.cmd_popup_filter.is_empty() {
        " 命令面板 ".to_string()
    } else {
        format!(" 命令面板 [{}] ", app.cmd_popup_filter)
    };

    // 使用主题颜色（与 todo 命令面板保持一致）
    let accent = app.theme.md_h1;
    let popup_bg = app.theme.bg_primary;
    let text_color = app.theme.text_normal;
    let dim_color = app.theme.text_dim;
    let label_ai = app.theme.label_ai;
    let highlight_bg = accent;
    let border_color = accent;
    let title_color = accent;

    // 构建列表项
    let selected = app.cmd_popup_selected.min(item_count.saturating_sub(1));
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, (_, key, label))| {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(pointer.to_string(), Style::default().fg(text_color)),
                Span::styled(
                    format!("{:<8}", key),
                    Style::default().fg(label_ai).add_modifier(Modifier::BOLD),
                ),
                Span::styled(label.to_string(), Style::default().fg(dim_color)),
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
                .border_style(Style::default().fg(border_color))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(title_color)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(popup_bg)),
        )
        .highlight_style(
            Style::default()
                .bg(highlight_bg)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut list_state);
}
