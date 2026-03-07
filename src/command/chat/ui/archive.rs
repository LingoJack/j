use super::super::app::ChatApp;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

/// 绘制归档确认界面
pub fn draw_archive_confirm(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.theme;
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  📦 归档当前对话",
        Style::default()
            .fg(t.help_title)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ─────────────────────────────────────────",
        Style::default().fg(t.separator),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  即将归档当前对话，归档后当前会话将被清空。",
        Style::default().fg(t.text_dim),
    )));
    lines.push(Line::from(""));

    if app.archive_editing_name {
        lines.push(Line::from(Span::styled(
            "  请输入归档名称：",
            Style::default().fg(t.text_white),
        )));
        lines.push(Line::from(""));

        let name_with_cursor = if app.archive_custom_name.is_empty() {
            vec![Span::styled(
                " ",
                Style::default().fg(t.cursor_fg).bg(t.cursor_bg),
            )]
        } else {
            let chars: Vec<char> = app.archive_custom_name.chars().collect();
            let mut spans: Vec<Span> = Vec::new();
            for (i, &ch) in chars.iter().enumerate() {
                if i == app.archive_edit_cursor {
                    spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().fg(t.cursor_fg).bg(t.cursor_bg),
                    ));
                } else {
                    spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().fg(t.text_white),
                    ));
                }
            }
            if app.archive_edit_cursor >= chars.len() {
                spans.push(Span::styled(
                    " ",
                    Style::default().fg(t.cursor_fg).bg(t.cursor_bg),
                ));
            }
            spans
        };

        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(
                format!("archive-{}", chrono::Local::now().format("%Y-%m-%d")),
                Style::default().fg(t.text_dim),
            ),
        ]));
        lines.push(Line::from(
            std::iter::once(Span::styled("    ", Style::default()))
                .chain(name_with_cursor)
                .collect::<Vec<_>>(),
        ));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  提示：留空则使用默认名称（如 archive-2026-02-25）",
            Style::default().fg(t.text_dim),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ─────────────────────────────────────────",
            Style::default().fg(t.separator),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "Enter",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  确认归档", Style::default().fg(t.help_desc)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "Esc",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("    取消", Style::default().fg(t.help_desc)),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("  默认名称：", Style::default().fg(t.text_dim)),
            Span::styled(
                &app.archive_default_name,
                Style::default()
                    .fg(t.config_toggle_on)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ─────────────────────────────────────────",
            Style::default().fg(t.separator),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "Enter",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  使用默认名称归档", Style::default().fg(t.help_desc)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "n",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("      自定义名称", Style::default().fg(t.help_desc)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "d",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("      仅清空不归档", Style::default().fg(t.help_desc)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "Esc",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("    取消", Style::default().fg(t.help_desc)),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(t.border_title))
        .title(Span::styled(" 归档确认 ", Style::default().fg(t.text_dim)))
        .style(Style::default().bg(t.help_bg));
    let widget = Paragraph::new(lines).block(block);
    f.render_widget(widget, area);
}

/// 绘制归档列表界面
pub fn draw_archive_list(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.theme;

    if app.restore_confirm_needed {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ⚠️  确认还原",
            Style::default()
                .fg(t.toast_error_text)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  当前对话未归档，还原将丢失当前对话内容！",
            Style::default().fg(t.text_white),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  ─────────────────────────────────────────",
            Style::default().fg(t.separator),
        )));
        lines.push(Line::from(""));
        if let Some(archive) = app.archives.get(app.archive_list_index) {
            lines.push(Line::from(vec![
                Span::styled("  将还原归档：", Style::default().fg(t.text_dim)),
                Span::styled(
                    &archive.name,
                    Style::default()
                        .fg(t.config_toggle_on)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "y/Enter",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  确认还原", Style::default().fg(t.help_desc)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                "Esc",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("     取消", Style::default().fg(t.help_desc)),
        ]));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(t.toast_error_border))
            .title(Span::styled(" 还原确认 ", Style::default().fg(t.text_dim)))
            .style(Style::default().bg(t.help_bg));
        let widget = Paragraph::new(lines).block(block);
        f.render_widget(widget, area);
        return;
    }

    if app.archives.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "  📦 暂无归档对话",
                Style::default().fg(t.text_dim).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  按 Ctrl+L 归档当前对话",
                Style::default().fg(t.text_dim),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  按 Esc 返回聊天",
                Style::default().fg(t.text_dim),
            )),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(t.border_title))
            .title(Span::styled(" 归档列表 ", Style::default().fg(t.text_dim)))
            .style(Style::default().bg(t.help_bg));
        let widget = Paragraph::new(lines).block(block);
        f.render_widget(widget, area);
        return;
    }

    let items: Vec<ListItem> = app
        .archives
        .iter()
        .enumerate()
        .map(|(i, archive)| {
            let is_selected = i == app.archive_list_index;
            let marker = if is_selected { "  ▸ " } else { "    " };
            let msg_count = archive.messages.len();
            let created_at = archive
                .created_at
                .split('T')
                .next()
                .unwrap_or(&archive.created_at);
            let style = if is_selected {
                Style::default()
                    .fg(t.model_sel_active)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.model_sel_inactive)
            };
            let detail = format!(
                "{}{}  📨 {} 条消息  📅 {}",
                marker, archive.name, msg_count, created_at
            );
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
                    " 📦 归档列表 (Enter 还原, d 删除, Esc 返回) ",
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
        .highlight_symbol("");

    let mut list_state = ListState::default();
    list_state.select(Some(app.archive_list_index));
    f.render_stateful_widget(list, area, &mut list_state);
}
