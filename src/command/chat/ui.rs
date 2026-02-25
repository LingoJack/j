use super::app::{CONFIG_FIELDS, CONFIG_GLOBAL_FIELDS, ChatApp, ChatMode, MsgLinesCache};
use super::handler::{config_field_label, config_field_value};
use super::model::agent_config_path;
use super::render::{build_message_lines_incremental, char_width, display_width, wrap_text};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn draw_chat_ui(f: &mut ratatui::Frame, app: &mut ChatApp) {
    let size = f.area();

    // 整体背景
    let bg = Block::default().style(Style::default().bg(Color::Rgb(22, 22, 30)));
    f.render_widget(bg, size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 标题栏
            Constraint::Min(5),    // 消息区
            Constraint::Length(5), // 输入区
            Constraint::Length(1), // 操作提示栏（始终可见）
        ])
        .split(size);

    // ========== 标题栏 ==========
    draw_title_bar(f, chunks[0], app);

    // ========== 消息区 ==========
    if app.mode == ChatMode::Help {
        draw_help(f, chunks[1]);
    } else if app.mode == ChatMode::SelectModel {
        draw_model_selector(f, chunks[1], app);
    } else if app.mode == ChatMode::Config {
        draw_config_screen(f, chunks[1], app);
    } else {
        draw_messages(f, chunks[1], app);
    }

    // ========== 输入区 ==========
    draw_input(f, chunks[2], app);

    // ========== 底部操作提示栏（始终可见）==========
    draw_hint_bar(f, chunks[3], app);

    // ========== Toast 弹窗覆盖层（右上角）==========
    draw_toast(f, size, app);
}

/// 绘制标题栏
pub fn draw_title_bar(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let model_name = app.active_model_name();
    let msg_count = app.session.messages.len();
    let loading = if app.is_loading {
        " ⏳ 思考中..."
    } else {
        ""
    };

    let title_spans = vec![
        Span::styled(" 💬 ", Style::default().fg(Color::Rgb(120, 180, 255))),
        Span::styled(
            "AI Chat",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(Color::Rgb(60, 60, 80))),
        Span::styled("🤖 ", Style::default()),
        Span::styled(
            model_name,
            Style::default()
                .fg(Color::Rgb(160, 220, 160))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(Color::Rgb(60, 60, 80))),
        Span::styled(
            format!("📨 {} 条消息", msg_count),
            Style::default().fg(Color::Rgb(180, 180, 200)),
        ),
        Span::styled(
            loading,
            Style::default()
                .fg(Color::Rgb(255, 200, 80))
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let title_block = Paragraph::new(Line::from(title_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(80, 100, 140)))
            .style(Style::default().bg(Color::Rgb(28, 28, 40))),
    );
    f.render_widget(title_block, area);
}

/// 绘制消息区
pub fn draw_messages(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(50, 55, 70)))
        .title(Span::styled(
            " 对话记录 ",
            Style::default()
                .fg(Color::Rgb(140, 140, 170))
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(ratatui::layout::Alignment::Left)
        .style(Style::default().bg(Color::Rgb(22, 22, 30)));

    // 空消息时显示欢迎界面
    if app.session.messages.is_empty() && !app.is_loading {
        let welcome_lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "  ╭──────────────────────────────────────╮",
                Style::default().fg(Color::Rgb(60, 70, 90)),
            )),
            Line::from(Span::styled(
                "  │                                      │",
                Style::default().fg(Color::Rgb(60, 70, 90)),
            )),
            Line::from(vec![
                Span::styled("  │     ", Style::default().fg(Color::Rgb(60, 70, 90))),
                Span::styled(
                    "Hi! What can I help you?  ",
                    Style::default().fg(Color::Rgb(120, 140, 180)),
                ),
                Span::styled("     │", Style::default().fg(Color::Rgb(60, 70, 90))),
            ]),
            Line::from(Span::styled(
                "  │                                      │",
                Style::default().fg(Color::Rgb(60, 70, 90)),
            )),
            Line::from(Span::styled(
                "  │     Type a message, press Enter      │",
                Style::default().fg(Color::Rgb(80, 90, 110)),
            )),
            Line::from(Span::styled(
                "  │                                      │",
                Style::default().fg(Color::Rgb(60, 70, 90)),
            )),
            Line::from(Span::styled(
                "  ╰──────────────────────────────────────╯",
                Style::default().fg(Color::Rgb(60, 70, 90)),
            )),
        ];
        let empty = Paragraph::new(welcome_lines).block(block);
        f.render_widget(empty, area);
        return;
    }

    // 内部可用宽度（减去边框和左右各1的 padding）
    let inner_width = area.width.saturating_sub(4) as usize;
    // 消息内容最大宽度为可用宽度的 75%
    let bubble_max_width = (inner_width * 75 / 100).max(20);

    // 计算缓存 key：消息数 + 最后一条消息长度 + 流式内容长度 + is_loading + 气泡宽度 + 浏览模式索引
    let msg_count = app.session.messages.len();
    let last_msg_len = app
        .session
        .messages
        .last()
        .map(|m| m.content.len())
        .unwrap_or(0);
    let streaming_len = app.streaming_content.lock().unwrap().len();
    let current_browse_index = if app.mode == ChatMode::Browse {
        Some(app.browse_msg_index)
    } else {
        None
    };
    let cache_hit = if let Some(ref cache) = app.msg_lines_cache {
        cache.msg_count == msg_count
            && cache.last_msg_len == last_msg_len
            && cache.streaming_len == streaming_len
            && cache.is_loading == app.is_loading
            && cache.bubble_max_width == bubble_max_width
            && cache.browse_index == current_browse_index
    } else {
        false
    };

    if !cache_hit {
        // 缓存未命中，增量构建渲染行
        let old_cache = app.msg_lines_cache.take();
        let (new_lines, new_msg_start_lines, new_per_msg, new_stable_lines, new_stable_offset) =
            build_message_lines_incremental(app, inner_width, bubble_max_width, old_cache.as_ref());
        app.msg_lines_cache = Some(MsgLinesCache {
            msg_count,
            last_msg_len,
            streaming_len,
            is_loading: app.is_loading,
            bubble_max_width,
            browse_index: current_browse_index,
            lines: new_lines,
            msg_start_lines: new_msg_start_lines,
            per_msg_lines: new_per_msg,
            streaming_stable_lines: new_stable_lines,
            streaming_stable_offset: new_stable_offset,
        });
    }

    // 从缓存中借用 lines（零拷贝）
    let cached = app.msg_lines_cache.as_ref().unwrap();
    let all_lines = &cached.lines;
    let total_lines = all_lines.len() as u16;

    // 渲染边框
    f.render_widget(block, area);

    // 计算内部区域（去掉边框）
    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    let visible_height = inner.height;
    let max_scroll = total_lines.saturating_sub(visible_height);

    // 自动滚动到底部（非浏览模式下）
    if app.mode != ChatMode::Browse {
        if app.scroll_offset == u16::MAX || app.scroll_offset > max_scroll {
            app.scroll_offset = max_scroll;
            // 已经在底部，恢复自动滚动
            app.auto_scroll = true;
        }
    } else {
        // 浏览模式：自动滚动到选中消息的位置
        if let Some(target_line) = cached
            .msg_start_lines
            .iter()
            .find(|(idx, _)| *idx == app.browse_msg_index)
            .map(|(_, line)| *line as u16)
        {
            // 确保选中消息在可视区域内
            if target_line < app.scroll_offset {
                app.scroll_offset = target_line;
            } else if target_line >= app.scroll_offset + visible_height {
                app.scroll_offset = target_line.saturating_sub(visible_height / 3);
            }
            // 限制滚动范围
            if app.scroll_offset > max_scroll {
                app.scroll_offset = max_scroll;
            }
        }
    }

    // 填充内部背景色（避免空白行没有背景）
    let bg_fill = Block::default().style(Style::default().bg(Color::Rgb(22, 22, 30)));
    f.render_widget(bg_fill, inner);

    // 只渲染可见区域的行（逐行借用缓存，clone 单行开销极小）
    let start = app.scroll_offset as usize;
    let end = (start + visible_height as usize).min(all_lines.len());
    let msg_area_bg = Style::default().bg(Color::Rgb(22, 22, 30));
    for (i, line_idx) in (start..end).enumerate() {
        let line = &all_lines[line_idx];
        let y = inner.y + i as u16;
        let line_area = Rect::new(inner.x, y, inner.width, 1);
        // 使用 Paragraph 渲染单行，设置背景色确保行尾空余区域颜色一致
        let p = Paragraph::new(line.clone()).style(msg_area_bg);
        f.render_widget(p, line_area);
    }
}

/// 查找流式内容中最后一个安全的段落边界（双换行），
/// 但要排除代码块内部的双换行（未闭合的 ``` 之后的内容不能拆分）。

pub fn draw_input(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    // 输入区可用宽度（减去边框2 + prompt 4）
    let usable_width = area.width.saturating_sub(2 + 4) as usize;

    let chars: Vec<char> = app.input.chars().collect();

    // 计算光标之前文本的显示宽度，决定是否需要水平滚动
    let before_all: String = chars[..app.cursor_pos].iter().collect();
    let before_width = display_width(&before_all);

    // 如果光标超出可视范围，从光标附近开始显示
    let scroll_offset_chars = if before_width >= usable_width {
        // 往回找到一个合适的起始字符位置
        let target_width = before_width.saturating_sub(usable_width / 2);
        let mut w = 0;
        let mut skip = 0;
        for (i, &ch) in chars.iter().enumerate() {
            if w >= target_width {
                skip = i;
                break;
            }
            w += char_width(ch);
        }
        skip
    } else {
        0
    };

    // 截取可见部分的字符
    let visible_chars = &chars[scroll_offset_chars..];
    let cursor_in_visible = app.cursor_pos - scroll_offset_chars;

    let before: String = visible_chars[..cursor_in_visible].iter().collect();
    let cursor_ch = if cursor_in_visible < visible_chars.len() {
        visible_chars[cursor_in_visible].to_string()
    } else {
        " ".to_string()
    };
    let after: String = if cursor_in_visible < visible_chars.len() {
        visible_chars[cursor_in_visible + 1..].iter().collect()
    } else {
        String::new()
    };

    let prompt_style = if app.is_loading {
        Style::default().fg(Color::Rgb(255, 200, 80))
    } else {
        Style::default().fg(Color::Rgb(100, 200, 130))
    };
    let prompt_text = if app.is_loading { " .. " } else { " >  " };

    // 构建多行输入显示（手动换行）
    let full_visible = format!("{}{}{}", before, cursor_ch, after);
    let inner_height = area.height.saturating_sub(2) as usize; // 减去边框
    let wrapped_lines = wrap_text(&full_visible, usable_width);

    // 找到光标所在的行索引
    let before_len = before.chars().count();
    let cursor_len = cursor_ch.chars().count();
    let cursor_global_pos = before_len; // 光标在全部可见字符中的位置
    let mut cursor_line_idx: usize = 0;
    {
        let mut cumulative = 0usize;
        for (li, wl) in wrapped_lines.iter().enumerate() {
            let line_char_count = wl.chars().count();
            if cumulative + line_char_count > cursor_global_pos {
                cursor_line_idx = li;
                break;
            }
            cumulative += line_char_count;
            cursor_line_idx = li; // 光标恰好在最后一行末尾
        }
    }

    // 计算行滚动：确保光标所在行在可见区域内
    let line_scroll = if wrapped_lines.len() <= inner_height {
        0
    } else if cursor_line_idx < inner_height {
        0
    } else {
        // 让光标行显示在可见区域的最后一行
        cursor_line_idx.saturating_sub(inner_height - 1)
    };

    // 构建带光标高亮的行
    let mut display_lines: Vec<Line> = Vec::new();
    let mut char_offset: usize = 0;
    // 跳过滚动行的字符数
    for wl in wrapped_lines.iter().take(line_scroll) {
        char_offset += wl.chars().count();
    }

    for (_line_idx, wl) in wrapped_lines
        .iter()
        .skip(line_scroll)
        .enumerate()
        .take(inner_height.max(1))
    {
        let mut spans: Vec<Span> = Vec::new();
        if _line_idx == 0 && line_scroll == 0 {
            spans.push(Span::styled(prompt_text, prompt_style));
        } else {
            spans.push(Span::styled("    ", Style::default())); // 对齐 prompt
        }

        // 对该行的每个字符分配样式
        let line_chars: Vec<char> = wl.chars().collect();
        let mut seg_start = 0;
        for (ci, &ch) in line_chars.iter().enumerate() {
            let global_idx = char_offset + ci;
            let is_cursor = global_idx >= before_len && global_idx < before_len + cursor_len;

            if is_cursor {
                // 先把 cursor 前的部分输出
                if ci > seg_start {
                    let seg: String = line_chars[seg_start..ci].iter().collect();
                    spans.push(Span::styled(seg, Style::default().fg(Color::White)));
                }
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default()
                        .fg(Color::Rgb(22, 22, 30))
                        .bg(Color::Rgb(200, 210, 240)),
                ));
                seg_start = ci + 1;
            }
        }
        // 输出剩余部分
        if seg_start < line_chars.len() {
            let seg: String = line_chars[seg_start..].iter().collect();
            spans.push(Span::styled(seg, Style::default().fg(Color::White)));
        }

        char_offset += line_chars.len();
        display_lines.push(Line::from(spans));
    }

    if display_lines.is_empty() {
        display_lines.push(Line::from(vec![
            Span::styled(prompt_text, prompt_style),
            Span::styled(
                " ",
                Style::default()
                    .fg(Color::Rgb(22, 22, 30))
                    .bg(Color::Rgb(200, 210, 240)),
            ),
        ]));
    }

    let input_widget = Paragraph::new(display_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(if app.is_loading {
                Style::default().fg(Color::Rgb(120, 100, 50))
            } else {
                Style::default().fg(Color::Rgb(60, 100, 80))
            })
            .title(Span::styled(
                " 输入消息 ",
                Style::default().fg(Color::Rgb(140, 140, 170)),
            ))
            .style(Style::default().bg(Color::Rgb(26, 26, 38))),
    );

    f.render_widget(input_widget, area);

    // 设置终端光标位置，确保中文输入法 IME 候选窗口在正确位置
    // 计算光标在渲染后的坐标
    if !app.is_loading {
        let prompt_w: u16 = 4; // prompt 宽度
        let border_left: u16 = 1; // 左边框

        // 光标在当前显示行中的列偏移
        let cursor_col_in_line = {
            let mut col = 0usize;
            let mut char_count = 0usize;
            // 跳过 line_scroll 之前的字符
            let mut skip_chars = 0usize;
            for wl in wrapped_lines.iter().take(line_scroll) {
                skip_chars += wl.chars().count();
            }
            // 找到光标在当前行的列
            for wl in wrapped_lines.iter().skip(line_scroll) {
                let line_len = wl.chars().count();
                if skip_chars + char_count + line_len > cursor_global_pos {
                    // 光标在这一行
                    let pos_in_line = cursor_global_pos - (skip_chars + char_count);
                    col = wl.chars().take(pos_in_line).map(|c| char_width(c)).sum();
                    break;
                }
                char_count += line_len;
            }
            col as u16
        };

        // 光标在显示行中的行偏移
        let cursor_row_in_display = (cursor_line_idx - line_scroll) as u16;

        let cursor_x = area.x + border_left + prompt_w + cursor_col_in_line;
        let cursor_y = area.y + 1 + cursor_row_in_display; // +1 跳过上边框

        // 确保光标在区域内
        if cursor_x < area.x + area.width && cursor_y < area.y + area.height {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

/// 绘制底部操作提示栏（始终可见）
pub fn draw_hint_bar(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let hints = match app.mode {
        ChatMode::Chat => {
            vec![
                ("Enter", "发送"),
                ("↑↓", "滚动"),
                ("Ctrl+T", "切换模型"),
                ("Ctrl+L", "清空"),
                ("Ctrl+Y", "复制"),
                ("Ctrl+B", "浏览"),
                ("Ctrl+S", "流式切换"),
                ("Ctrl+E", "配置"),
                ("?/F1", "帮助"),
                ("Esc", "退出"),
            ]
        }
        ChatMode::SelectModel => {
            vec![("↑↓/jk", "移动"), ("Enter", "确认"), ("Esc", "取消")]
        }
        ChatMode::Browse => {
            vec![("↑↓", "选择消息"), ("y/Enter", "复制"), ("Esc", "返回")]
        }
        ChatMode::Help => {
            vec![("任意键", "返回")]
        }
        ChatMode::Config => {
            vec![
                ("↑↓", "切换字段"),
                ("Enter", "编辑"),
                ("Tab", "切换 Provider"),
                ("a", "新增"),
                ("d", "删除"),
                ("Esc", "保存返回"),
            ]
        }
    };

    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(" ", Style::default()));
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                "  │  ",
                Style::default().fg(Color::Rgb(50, 50, 65)),
            ));
        }
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(Color::Rgb(22, 22, 30))
                .bg(Color::Rgb(100, 110, 140)),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(Color::Rgb(120, 120, 150)),
        ));
    }

    let hint_bar =
        Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Rgb(22, 22, 30)));
    f.render_widget(hint_bar, area);
}

/// 绘制 Toast 弹窗（右上角浮层）
pub fn draw_toast(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    if let Some((ref msg, is_error, _)) = app.toast {
        let text_width = display_width(msg);
        // toast 宽度 = 文字宽度 + 左右 padding(各2) + emoji(2) + border(2)
        let toast_width = (text_width + 10).min(area.width as usize).max(16) as u16;
        let toast_height: u16 = 3;

        // 定位到右上角
        let x = area.width.saturating_sub(toast_width + 1);
        let y: u16 = 1;

        if x + toast_width <= area.width && y + toast_height <= area.height {
            let toast_area = Rect::new(x, y, toast_width, toast_height);

            // 先清空区域背景
            let clear = Block::default().style(Style::default().bg(if is_error {
                Color::Rgb(60, 20, 20)
            } else {
                Color::Rgb(20, 50, 30)
            }));
            f.render_widget(clear, toast_area);

            let (icon, border_color, text_color) = if is_error {
                ("❌", Color::Rgb(200, 70, 70), Color::Rgb(255, 130, 130))
            } else {
                ("✅", Color::Rgb(60, 160, 80), Color::Rgb(140, 230, 160))
            };

            let toast_widget = Paragraph::new(Line::from(vec![
                Span::styled(format!(" {} ", icon), Style::default()),
                Span::styled(msg.as_str(), Style::default().fg(text_color)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .style(Style::default().bg(if is_error {
                        Color::Rgb(50, 18, 18)
                    } else {
                        Color::Rgb(18, 40, 25)
                    })),
            );
            f.render_widget(toast_widget, toast_area);
        }
    }
}

/// 绘制模型选择界面
pub fn draw_model_selector(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let items: Vec<ListItem> = app
        .agent_config
        .providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_active = i == app.agent_config.active_index;
            let marker = if is_active { " ● " } else { " ○ " };
            let style = if is_active {
                Style::default()
                    .fg(Color::Rgb(120, 220, 160))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(180, 180, 200))
            };
            let detail = format!("{}{}  ({})", marker, p.name, p.model);
            ListItem::new(Line::from(Span::styled(detail, style)))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(180, 160, 80)))
                .title(Span::styled(
                    " 🔄 选择模型 ",
                    Style::default()
                        .fg(Color::Rgb(230, 210, 120))
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(Color::Rgb(28, 28, 40))),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(50, 55, 80))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("  ▸ ");

    f.render_stateful_widget(list, area, &mut app.model_list_state);
}

/// 绘制帮助界面
pub fn draw_help(f: &mut ratatui::Frame, area: Rect) {
    let separator = Line::from(Span::styled(
        "  ─────────────────────────────────────────",
        Style::default().fg(Color::Rgb(50, 55, 70)),
    ));

    let help_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  📖 快捷键帮助",
            Style::default()
                .fg(Color::Rgb(120, 180, 255))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        separator.clone(),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Enter        ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("发送消息", Style::default().fg(Color::Rgb(200, 200, 220))),
        ]),
        Line::from(vec![
            Span::styled(
                "  ↑ / ↓        ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "滚动对话记录",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  ← / →        ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "移动输入光标",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+T       ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("切换模型", Style::default().fg(Color::Rgb(200, 200, 220))),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+L       ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "清空对话历史",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+Y       ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "复制最后一条 AI 回复",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+B       ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "浏览消息 (↑↓选择, y/Enter复制)",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+S       ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "切换流式/整体输出",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+E       ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "打开配置界面",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Esc / Ctrl+C ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("退出对话", Style::default().fg(Color::Rgb(200, 200, 220))),
        ]),
        Line::from(vec![
            Span::styled(
                "  ? / F1       ",
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "显示 / 关闭此帮助",
                Style::default().fg(Color::Rgb(200, 200, 220)),
            ),
        ]),
        Line::from(""),
        separator,
        Line::from(""),
        Line::from(Span::styled(
            "  📁 配置文件:",
            Style::default()
                .fg(Color::Rgb(120, 180, 255))
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("     {}", agent_config_path().display()),
            Style::default().fg(Color::Rgb(100, 100, 130)),
        )),
    ];

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(80, 100, 140)))
        .title(Span::styled(
            " 帮助 (按任意键返回) ",
            Style::default().fg(Color::Rgb(140, 140, 170)),
        ))
        .style(Style::default().bg(Color::Rgb(24, 24, 34)));
    let help_widget = Paragraph::new(help_lines).block(help_block);
    f.render_widget(help_widget, area);
}

/// 对话模式按键处理，返回 true 表示退出

pub fn draw_config_screen(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let bg = Color::Rgb(28, 28, 40);
    let total_provider_fields = CONFIG_FIELDS.len();

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    // 标题
    lines.push(Line::from(vec![Span::styled(
        "  ⚙️  模型配置",
        Style::default()
            .fg(Color::Rgb(120, 180, 255))
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    // Provider 标签栏
    let provider_count = app.agent_config.providers.len();
    if provider_count > 0 {
        let mut tab_spans: Vec<Span> = vec![Span::styled("  ", Style::default())];
        for (i, p) in app.agent_config.providers.iter().enumerate() {
            let is_current = i == app.config_provider_idx;
            let is_active = i == app.agent_config.active_index;
            let marker = if is_active { "● " } else { "○ " };
            let label = format!(" {}{} ", marker, p.name);
            if is_current {
                tab_spans.push(Span::styled(
                    label,
                    Style::default()
                        .fg(Color::Rgb(22, 22, 30))
                        .bg(Color::Rgb(120, 180, 255))
                        .add_modifier(Modifier::BOLD),
                ));
            } else {
                tab_spans.push(Span::styled(
                    label,
                    Style::default().fg(Color::Rgb(150, 150, 170)),
                ));
            }
            if i < provider_count - 1 {
                tab_spans.push(Span::styled(
                    " │ ",
                    Style::default().fg(Color::Rgb(50, 55, 70)),
                ));
            }
        }
        tab_spans.push(Span::styled(
            "    (● = 活跃模型, Tab 切换, s 设为活跃)",
            Style::default().fg(Color::Rgb(80, 80, 100)),
        ));
        lines.push(Line::from(tab_spans));
    } else {
        lines.push(Line::from(Span::styled(
            "  (无 Provider，按 a 新增)",
            Style::default().fg(Color::Rgb(180, 120, 80)),
        )));
    }
    lines.push(Line::from(""));

    // 分隔线
    lines.push(Line::from(Span::styled(
        "  ─────────────────────────────────────────",
        Style::default().fg(Color::Rgb(50, 55, 70)),
    )));
    lines.push(Line::from(""));

    // Provider 字段
    if provider_count > 0 {
        lines.push(Line::from(Span::styled(
            "  📦 Provider 配置",
            Style::default()
                .fg(Color::Rgb(160, 220, 160))
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for i in 0..total_provider_fields {
            let is_selected = app.config_field_idx == i;
            let label = config_field_label(i);
            let value = if app.config_editing && is_selected {
                // 编辑模式下显示编辑缓冲区
                app.config_edit_buf.clone()
            } else {
                config_field_value(app, i)
            };

            let pointer = if is_selected { "  ▸ " } else { "    " };
            let pointer_style = if is_selected {
                Style::default().fg(Color::Rgb(255, 200, 80))
            } else {
                Style::default()
            };

            let label_style = if is_selected {
                Style::default()
                    .fg(Color::Rgb(230, 210, 120))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(140, 140, 160))
            };

            let value_style = if app.config_editing && is_selected {
                Style::default().fg(Color::White).bg(Color::Rgb(50, 55, 80))
            } else if is_selected {
                Style::default().fg(Color::White)
            } else {
                // API Key 特殊处理
                if CONFIG_FIELDS[i] == "api_key" {
                    Style::default().fg(Color::Rgb(100, 100, 120))
                } else {
                    Style::default().fg(Color::Rgb(180, 180, 200))
                }
            };

            let edit_indicator = if app.config_editing && is_selected {
                " ✏️"
            } else {
                ""
            };

            lines.push(Line::from(vec![
                Span::styled(pointer, pointer_style),
                Span::styled(format!("{:<10}", label), label_style),
                Span::styled("  ", Style::default()),
                Span::styled(
                    if value.is_empty() {
                        "(空)".to_string()
                    } else {
                        value
                    },
                    value_style,
                ),
                Span::styled(edit_indicator, Style::default()),
            ]));
        }
    }

    lines.push(Line::from(""));
    // 分隔线
    lines.push(Line::from(Span::styled(
        "  ─────────────────────────────────────────",
        Style::default().fg(Color::Rgb(50, 55, 70)),
    )));
    lines.push(Line::from(""));

    // 全局配置
    lines.push(Line::from(Span::styled(
        "  🌐 全局配置",
        Style::default()
            .fg(Color::Rgb(160, 220, 160))
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for i in 0..CONFIG_GLOBAL_FIELDS.len() {
        let field_idx = total_provider_fields + i;
        let is_selected = app.config_field_idx == field_idx;
        let label = config_field_label(field_idx);
        let value = if app.config_editing && is_selected {
            app.config_edit_buf.clone()
        } else {
            config_field_value(app, field_idx)
        };

        let pointer = if is_selected { "  ▸ " } else { "    " };
        let pointer_style = if is_selected {
            Style::default().fg(Color::Rgb(255, 200, 80))
        } else {
            Style::default()
        };

        let label_style = if is_selected {
            Style::default()
                .fg(Color::Rgb(230, 210, 120))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(140, 140, 160))
        };

        let value_style = if app.config_editing && is_selected {
            Style::default().fg(Color::White).bg(Color::Rgb(50, 55, 80))
        } else if is_selected {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Rgb(180, 180, 200))
        };

        let edit_indicator = if app.config_editing && is_selected {
            " ✏️"
        } else {
            ""
        };

        // stream_mode 用 toggle 样式
        if CONFIG_GLOBAL_FIELDS[i] == "stream_mode" {
            let toggle_on = app.agent_config.stream_mode;
            let toggle_style = if toggle_on {
                Style::default()
                    .fg(Color::Rgb(120, 220, 160))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Rgb(200, 100, 100))
            };
            let toggle_text = if toggle_on {
                "● 开启"
            } else {
                "○ 关闭"
            };

            lines.push(Line::from(vec![
                Span::styled(pointer, pointer_style),
                Span::styled(format!("{:<10}", label), label_style),
                Span::styled("  ", Style::default()),
                Span::styled(toggle_text, toggle_style),
                Span::styled(
                    if is_selected { "  (Enter 切换)" } else { "" },
                    Style::default().fg(Color::Rgb(80, 80, 100)),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(pointer, pointer_style),
                Span::styled(format!("{:<10}", label), label_style),
                Span::styled("  ", Style::default()),
                Span::styled(
                    if value.is_empty() {
                        "(空)".to_string()
                    } else {
                        value
                    },
                    value_style,
                ),
                Span::styled(edit_indicator, Style::default()),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(""));

    // 操作提示
    lines.push(Line::from(Span::styled(
        "  ─────────────────────────────────────────",
        Style::default().fg(Color::Rgb(50, 55, 70)),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("    ", Style::default()),
        Span::styled(
            "↑↓/jk",
            Style::default()
                .fg(Color::Rgb(230, 210, 120))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " 切换字段  ",
            Style::default().fg(Color::Rgb(120, 120, 150)),
        ),
        Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Rgb(230, 210, 120))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 编辑  ", Style::default().fg(Color::Rgb(120, 120, 150))),
        Span::styled(
            "Tab/←→",
            Style::default()
                .fg(Color::Rgb(230, 210, 120))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " 切换 Provider  ",
            Style::default().fg(Color::Rgb(120, 120, 150)),
        ),
        Span::styled(
            "a",
            Style::default()
                .fg(Color::Rgb(230, 210, 120))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 新增  ", Style::default().fg(Color::Rgb(120, 120, 150))),
        Span::styled(
            "d",
            Style::default()
                .fg(Color::Rgb(230, 210, 120))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 删除  ", Style::default().fg(Color::Rgb(120, 120, 150))),
        Span::styled(
            "s",
            Style::default()
                .fg(Color::Rgb(230, 210, 120))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " 设为活跃  ",
            Style::default().fg(Color::Rgb(120, 120, 150)),
        ),
        Span::styled(
            "Esc",
            Style::default()
                .fg(Color::Rgb(230, 210, 120))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 保存返回", Style::default().fg(Color::Rgb(120, 120, 150))),
    ]));

    let content = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(80, 80, 110)))
                .title(Span::styled(
                    " ⚙️  模型配置编辑 ",
                    Style::default()
                        .fg(Color::Rgb(230, 210, 120))
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(bg)),
        )
        .scroll((0, 0));
    f.render_widget(content, area);
}
