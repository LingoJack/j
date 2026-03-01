use super::super::app::{ChatApp, ChatMode, MsgLinesCache, ToolExecStatus};
use super::super::model::agent_config_path;
use super::super::render::{build_message_lines_incremental, char_width, display_width, wrap_text};
use super::archive::{draw_archive_confirm, draw_archive_list};
use super::config::draw_config_screen;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn draw_chat_ui(f: &mut ratatui::Frame, app: &mut ChatApp) {
    let size = f.area();

    // 整体背景
    let bg = Block::default().style(Style::default().bg(app.theme.bg_primary));
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
        draw_help(f, chunks[1], app);
    } else if app.mode == ChatMode::SelectModel {
        draw_model_selector(f, chunks[1], app);
    } else if app.mode == ChatMode::Config {
        draw_config_screen(f, chunks[1], app);
    } else if app.mode == ChatMode::ArchiveConfirm {
        draw_archive_confirm(f, chunks[1], app);
    } else if app.mode == ChatMode::ArchiveList {
        draw_archive_list(f, chunks[1], app);
    } else if app.mode == ChatMode::ToolConfirm {
        draw_messages(f, chunks[1], app);
        draw_tool_confirm(f, size, app);
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
    let t = &app.theme;
    let model_name = app.active_model_name();
    let msg_count = app.session.messages.len();
    let loading = if app.is_loading {
        // 如果有活跃工具调用，显示工具名
        let tool_info = app
            .active_tool_calls
            .iter()
            .find(|tc| matches!(tc.status, ToolExecStatus::Executing))
            .map(|tc| format!(" 🔧 执行 {}...", tc.tool_name));
        if let Some(info) = tool_info {
            info
        } else {
            " ⏳ 思考中...".to_string()
        }
    } else {
        String::new()
    };

    let title_spans = vec![
        Span::styled(" 💬 ", Style::default().fg(t.title_icon)),
        Span::styled(
            "AI Chat",
            Style::default()
                .fg(t.text_white)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(t.title_separator)),
        Span::styled("🤖 ", Style::default()),
        Span::styled(
            model_name,
            Style::default()
                .fg(t.title_model)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(t.title_separator)),
        Span::styled(
            format!("📨 {} 条消息", msg_count),
            Style::default().fg(t.title_count),
        ),
        Span::styled(
            loading,
            Style::default()
                .fg(t.title_loading)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let title_block = Paragraph::new(Line::from(title_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(t.border_title))
            .style(Style::default().bg(t.bg_title)),
    );
    f.render_widget(title_block, area);
}

/// 绘制消息区
pub fn draw_messages(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let t = &app.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(t.border_message))
        .title(Span::styled(
            " 对话记录 ",
            Style::default().fg(t.text_dim).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(ratatui::layout::Alignment::Left)
        .style(Style::default().bg(t.bg_primary));

    // 空消息时显示欢迎界面
    if app.session.messages.is_empty() && !app.is_loading {
        let welcome_lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "  ╭──────────────────────────────────────╮",
                Style::default().fg(t.welcome_border),
            )),
            Line::from(Span::styled(
                "  │                                      │",
                Style::default().fg(t.welcome_border),
            )),
            Line::from(vec![
                Span::styled("  │     ", Style::default().fg(t.welcome_border)),
                Span::styled(
                    "Hi! What can I help you?  ",
                    Style::default().fg(t.welcome_text),
                ),
                Span::styled("     │", Style::default().fg(t.welcome_border)),
            ]),
            Line::from(Span::styled(
                "  │                                      │",
                Style::default().fg(t.welcome_border),
            )),
            Line::from(Span::styled(
                "  │     Type a message, press Enter      │",
                Style::default().fg(t.welcome_hint),
            )),
            Line::from(Span::styled(
                "  │                                      │",
                Style::default().fg(t.welcome_border),
            )),
            Line::from(Span::styled(
                "  ╰──────────────────────────────────────╯",
                Style::default().fg(t.welcome_border),
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

    let cached = app.msg_lines_cache.as_ref().unwrap();
    let all_lines = &cached.lines;
    let total_lines = all_lines.len() as u16;

    f.render_widget(block, area);

    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    let visible_height = inner.height;
    let max_scroll = total_lines.saturating_sub(visible_height);

    if app.mode != ChatMode::Browse {
        if app.scroll_offset == u16::MAX || app.scroll_offset > max_scroll {
            app.scroll_offset = max_scroll;
            app.auto_scroll = true;
        }
    } else {
        if let Some(msg_start) = cached
            .msg_start_lines
            .iter()
            .find(|(idx, _)| *idx == app.browse_msg_index)
            .map(|(_, line)| *line as u16)
        {
            let msg_line_count = cached
                .per_msg_lines
                .get(app.browse_msg_index)
                .map(|c| c.lines.len())
                .unwrap_or(1) as u16;
            let msg_max_scroll = msg_line_count.saturating_sub(visible_height);
            if app.browse_scroll_offset > msg_max_scroll {
                app.browse_scroll_offset = msg_max_scroll;
            }
            app.scroll_offset = (msg_start + app.browse_scroll_offset).min(max_scroll);
        }
    }

    let bg_fill = Block::default().style(Style::default().bg(app.theme.bg_primary));
    f.render_widget(bg_fill, inner);

    let start = app.scroll_offset as usize;
    let end = (start + visible_height as usize).min(all_lines.len());
    let msg_area_bg = Style::default().bg(app.theme.bg_primary);
    for (i, line_idx) in (start..end).enumerate() {
        let line = &all_lines[line_idx];
        let y = inner.y + i as u16;
        let line_area = Rect::new(inner.x, y, inner.width, 1);
        let p = Paragraph::new(line.clone()).style(msg_area_bg);
        f.render_widget(p, line_area);
    }
}

pub fn draw_input(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.theme;
    let usable_width = area.width.saturating_sub(2 + 4) as usize;

    let chars: Vec<char> = app.input.chars().collect();

    let before_all: String = chars[..app.cursor_pos].iter().collect();
    let before_width = display_width(&before_all);

    let scroll_offset_chars = if before_width >= usable_width {
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
        Style::default().fg(t.input_prompt_loading)
    } else {
        Style::default().fg(t.input_prompt)
    };
    let prompt_text = if app.is_loading { " .. " } else { " >  " };

    let full_visible = format!("{}{}{}", before, cursor_ch, after);
    let inner_height = area.height.saturating_sub(2) as usize;
    let wrapped_lines = wrap_text(&full_visible, usable_width);

    let before_len = before.chars().count();
    let cursor_len = cursor_ch.chars().count();
    let cursor_global_pos = before_len;
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
            cursor_line_idx = li;
        }
    }

    let line_scroll = if wrapped_lines.len() <= inner_height {
        0
    } else if cursor_line_idx < inner_height {
        0
    } else {
        cursor_line_idx.saturating_sub(inner_height - 1)
    };

    let mut display_lines: Vec<Line> = Vec::new();
    let mut char_offset: usize = 0;
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
            spans.push(Span::styled("    ", Style::default()));
        }

        let line_chars: Vec<char> = wl.chars().collect();
        let mut seg_start = 0;
        for (ci, &ch) in line_chars.iter().enumerate() {
            let global_idx = char_offset + ci;
            let is_cursor = global_idx >= before_len && global_idx < before_len + cursor_len;

            if is_cursor {
                if ci > seg_start {
                    let seg: String = line_chars[seg_start..ci].iter().collect();
                    spans.push(Span::styled(seg, Style::default().fg(t.text_white)));
                }
                spans.push(Span::styled(
                    ch.to_string(),
                    Style::default().fg(t.cursor_fg).bg(t.cursor_bg),
                ));
                seg_start = ci + 1;
            }
        }
        if seg_start < line_chars.len() {
            let seg: String = line_chars[seg_start..].iter().collect();
            spans.push(Span::styled(seg, Style::default().fg(t.text_white)));
        }

        char_offset += line_chars.len();
        display_lines.push(Line::from(spans));
    }

    if display_lines.is_empty() {
        display_lines.push(Line::from(vec![
            Span::styled(prompt_text, prompt_style),
            Span::styled(" ", Style::default().fg(t.cursor_fg).bg(t.cursor_bg)),
        ]));
    }

    let input_widget = Paragraph::new(display_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(if app.is_loading {
                Style::default().fg(t.border_input_loading)
            } else {
                Style::default().fg(t.border_input)
            })
            .title(Span::styled(" 输入消息 ", Style::default().fg(t.text_dim)))
            .style(Style::default().bg(t.bg_input)),
    );

    f.render_widget(input_widget, area);

    if !app.is_loading {
        let prompt_w: u16 = 4;
        let border_left: u16 = 1;

        let cursor_col_in_line = {
            let mut col = 0usize;
            let mut char_count = 0usize;
            let mut skip_chars = 0usize;
            for wl in wrapped_lines.iter().take(line_scroll) {
                skip_chars += wl.chars().count();
            }
            for wl in wrapped_lines.iter().skip(line_scroll) {
                let line_len = wl.chars().count();
                if skip_chars + char_count + line_len > cursor_global_pos {
                    let pos_in_line = cursor_global_pos - (skip_chars + char_count);
                    col = wl.chars().take(pos_in_line).map(|c| char_width(c)).sum();
                    break;
                }
                char_count += line_len;
            }
            col as u16
        };

        let cursor_row_in_display = (cursor_line_idx - line_scroll) as u16;
        let cursor_x = area.x + border_left + prompt_w + cursor_col_in_line;
        let cursor_y = area.y + 1 + cursor_row_in_display;

        if cursor_x < area.x + area.width && cursor_y < area.y + area.height {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

/// 绘制底部操作提示栏（始终可见）
pub fn draw_hint_bar(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.theme;
    let hints = match app.mode {
        ChatMode::Chat => vec![
            ("Enter", "发送"),
            ("↑↓", "滚动"),
            ("Ctrl+T", "切换模型"),
            ("Ctrl+L", "归档"),
            ("Ctrl+R", "还原"),
            ("Ctrl+Y", "复制"),
            ("Ctrl+B", "浏览"),
            ("Ctrl+S", "流式切换"),
            ("Ctrl+E", "配置"),
            ("?/F1", "帮助"),
            ("Esc", "退出"),
        ],
        ChatMode::SelectModel => vec![("↑↓/jk", "移动"), ("Enter", "确认"), ("Esc", "取消")],
        ChatMode::Browse => vec![("↑↓", "选择消息"), ("y/Enter", "复制"), ("Esc", "返回")],
        ChatMode::Help => vec![("任意键", "返回")],
        ChatMode::Config => vec![
            ("↑↓", "切换字段"),
            ("Enter", "编辑"),
            ("Tab", "切换 Provider"),
            ("a", "新增"),
            ("d", "删除"),
            ("Esc", "保存返回"),
        ],
        ChatMode::ArchiveConfirm => {
            if app.archive_editing_name {
                vec![("Enter", "确认"), ("Esc", "取消")]
            } else {
                vec![
                    ("Enter", "默认名称归档"),
                    ("n", "自定义名称"),
                    ("Esc", "取消"),
                ]
            }
        }
        ChatMode::ArchiveList => {
            if app.restore_confirm_needed {
                vec![("y/Enter", "确认还原"), ("Esc", "取消")]
            } else {
                vec![
                    ("↑↓/jk", "选择"),
                    ("Enter", "还原"),
                    ("d", "删除"),
                    ("Esc", "返回"),
                ]
            }
        }
        ChatMode::ToolConfirm => vec![("Y", "执行工具"), ("N/Esc", "拒绝")],
    };

    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(" ", Style::default()));
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  │  ", Style::default().fg(t.hint_separator)));
        }
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default().fg(t.hint_key_fg).bg(t.hint_key_bg),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(t.hint_desc),
        ));
    }

    let hint_bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(t.bg_primary));
    f.render_widget(hint_bar, area);
}

/// 绘制 Toast 弹窗（右上角浮层）
pub fn draw_toast(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.theme;
    if let Some((ref msg, is_error, _)) = app.toast {
        let text_width = display_width(msg);
        let toast_width = (text_width + 10).min(area.width as usize).max(16) as u16;
        let toast_height: u16 = 3;

        let x = area.width.saturating_sub(toast_width + 1);
        let y: u16 = 1;

        if x + toast_width <= area.width && y + toast_height <= area.height {
            let toast_area = Rect::new(x, y, toast_width, toast_height);

            let clear = Block::default().style(Style::default().bg(if is_error {
                t.toast_error_bg
            } else {
                t.toast_success_bg
            }));
            f.render_widget(clear, toast_area);

            let (icon, border_color, text_color) = if is_error {
                ("❌", t.toast_error_border, t.toast_error_text)
            } else {
                ("✅", t.toast_success_border, t.toast_success_text)
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
                        t.toast_error_bg
                    } else {
                        t.toast_success_bg
                    })),
            );
            f.render_widget(toast_widget, toast_area);
        }
    }
}

/// 绘制模型选择界面
pub fn draw_model_selector(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let t = &app.theme;
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
                    .fg(t.model_sel_active)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.model_sel_inactive)
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
                .border_style(Style::default().fg(t.model_sel_border))
                .title(Span::styled(
                    " 🔄 选择模型 ",
                    Style::default()
                        .fg(t.model_sel_title)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(t.bg_title)),
        )
        .highlight_style(
            Style::default()
                .bg(t.model_sel_highlight_bg)
                .fg(t.text_white)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("  ▸ ");

    f.render_stateful_widget(list, area, &mut app.model_list_state);
}

/// 绘制帮助界面
pub fn draw_help(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.theme;
    let separator = Line::from(Span::styled(
        "  ─────────────────────────────────────────",
        Style::default().fg(t.separator),
    ));

    let help_lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  📖 快捷键帮助",
            Style::default()
                .fg(t.help_title)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        separator.clone(),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Enter        ",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("发送消息", Style::default().fg(t.help_desc)),
        ]),
        Line::from(vec![
            Span::styled(
                "  ↑ / ↓        ",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("滚动对话记录", Style::default().fg(t.help_desc)),
        ]),
        Line::from(vec![
            Span::styled(
                "  ← / →        ",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("移动输入光标", Style::default().fg(t.help_desc)),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+T       ",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("切换模型", Style::default().fg(t.help_desc)),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+L       ",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("归档当前对话", Style::default().fg(t.help_desc)),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+R       ",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("还原归档对话", Style::default().fg(t.help_desc)),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+Y       ",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("复制最后一条 AI 回复", Style::default().fg(t.help_desc)),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+B       ",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "浏览消息 (↑↓选择, y/Enter复制)",
                Style::default().fg(t.help_desc),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+S       ",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("切换流式/整体输出", Style::default().fg(t.help_desc)),
        ]),
        Line::from(vec![
            Span::styled(
                "  Ctrl+E       ",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("打开配置界面", Style::default().fg(t.help_desc)),
        ]),
        Line::from(vec![
            Span::styled(
                "  Esc / Ctrl+C ",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("退出对话", Style::default().fg(t.help_desc)),
        ]),
        Line::from(vec![
            Span::styled(
                "  ? / F1       ",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("显示 / 关闭此帮助", Style::default().fg(t.help_desc)),
        ]),
        Line::from(""),
        separator,
        Line::from(""),
        Line::from(Span::styled(
            "  📁 配置文件:",
            Style::default()
                .fg(t.help_title)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("     {}", agent_config_path().display()),
            Style::default().fg(t.help_path),
        )),
    ];

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(t.border_title))
        .title(Span::styled(
            " 帮助 (按任意键返回) ",
            Style::default().fg(t.text_dim),
        ))
        .style(Style::default().bg(t.help_bg));
    let help_widget = Paragraph::new(help_lines).block(help_block);
    f.render_widget(help_widget, area);
}

/// 绘制工具调用确认浮层（居中弹窗）
pub fn draw_tool_confirm(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let idx = app.pending_tool_idx;
    let tc = match app.active_tool_calls.get(idx) {
        Some(tc) => tc,
        None => return,
    };

    let t = &app.theme;

    // 计算弹窗尺寸
    let dialog_width = (area.width as usize).min(70).max(40) as u16;
    let dialog_height: u16 = 9;
    let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
    let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;

    if x + dialog_width > area.width || y + dialog_height > area.height {
        return;
    }

    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    // 背景
    let bg_block = Block::default().style(Style::default().bg(Color::Rgb(30, 25, 10)));
    f.render_widget(bg_block, dialog_area);

    let tool_name_line = Line::from(vec![
        Span::styled("  工具: ", Style::default().fg(Color::Gray)),
        Span::styled(
            tc.tool_name.clone(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    // 截断确认消息
    let confirm_msg = if tc.confirm_message.len() > (dialog_width as usize).saturating_sub(4) {
        let mut end = (dialog_width as usize).saturating_sub(7);
        while !tc.confirm_message.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &tc.confirm_message[..end])
    } else {
        tc.confirm_message.clone()
    };

    let hint_line = Line::from(vec![
        Span::styled(
            "  [Y] 执行",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  /  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "[N] 拒绝",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
    ]);

    let dialog_lines = vec![
        Line::from(""),
        tool_name_line,
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", confirm_msg),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  ─────────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        hint_line,
    ];

    let dialog_widget = Paragraph::new(dialog_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(Color::Yellow))
            .title(Span::styled(
                " 🔧 工具调用确认 ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(Color::Rgb(30, 25, 10))),
    );

    f.render_widget(dialog_widget, dialog_area);

    let _ = t;
}
