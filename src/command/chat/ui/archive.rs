use crate::command::chat::app::ChatApp;
use crate::tui::components::{
    POINTER_EMPTY, POINTER_SELECTED, cursor_spans, help_key_row, section_header, separator_line,
};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

/// 绘制归档确认界面
#[allow(clippy::vec_init_then_push)]
pub fn draw_archive_confirm(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(""));
    lines.push(section_header(
        "\u{1f5c2}\u{fe0f}",
        "\u{5f52}\u{6863}\u{5f53}\u{524d}\u{5bf9}\u{8bdd}",
        t,
    ));
    lines.push(Line::from(""));
    lines.push(separator_line(area.width, t));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  \u{5373}\u{5c06}\u{5f52}\u{6863}\u{5f53}\u{524d}\u{5bf9}\u{8bdd}\u{ff0c}\u{5f52}\u{6863}\u{540e}\u{5f53}\u{524d}\u{4f1a}\u{8bdd}\u{5c06}\u{88ab}\u{6e05}\u{7a7a}\u{3002}",
        Style::default().fg(t.text_dim),
    )));
    lines.push(Line::from(""));

    if app.ui.archive_editing_name {
        lines.push(Line::from(Span::styled(
            "  \u{8bf7}\u{8f93}\u{5165}\u{5f52}\u{6863}\u{540d}\u{79f0}\u{ff1a}",
            Style::default().fg(t.text_white),
        )));
        lines.push(Line::from(""));

        let value_style = Style::default().fg(t.text_white);
        let name_with_cursor = if app.ui.archive_custom_name.is_empty() {
            vec![Span::styled(
                " ",
                Style::default().fg(t.cursor_fg).bg(t.cursor_bg),
            )]
        } else {
            cursor_spans(
                &app.ui.archive_custom_name,
                app.ui.archive_edit_cursor,
                value_style,
                t,
            )
            .into_iter()
            .filter(|s| s.content != " \u{270f}\u{fe0f}") // 去掉编辑图标
            .collect()
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
            "  \u{63d0}\u{793a}\u{ff1a}\u{7559}\u{7a7a}\u{5219}\u{4f7f}\u{7528}\u{9ed8}\u{8ba4}\u{540d}\u{79f0}\u{ff08}\u{5982} archive-2026-02-25\u{ff09}",
            Style::default().fg(t.text_dim),
        )));
        lines.push(Line::from(""));
        lines.push(separator_line(area.width, t));
        lines.push(Line::from(""));
        lines.push(help_key_row(
            "Enter",
            "\u{786e}\u{8ba4}\u{5f52}\u{6863}",
            7,
            t,
        ));
        lines.push(help_key_row("Esc", "\u{53d6}\u{6d88}", 7, t));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "  \u{9ed8}\u{8ba4}\u{540d}\u{79f0}\u{ff1a}",
                Style::default().fg(t.text_dim),
            ),
            Span::styled(
                &app.ui.archive_default_name,
                Style::default()
                    .fg(t.config_toggle_on)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(""));
        lines.push(separator_line(area.width, t));
        lines.push(Line::from(""));
        lines.push(help_key_row(
            "Enter",
            "\u{4f7f}\u{7528}\u{9ed8}\u{8ba4}\u{540d}\u{79f0}\u{5f52}\u{6863}",
            7,
            t,
        ));
        lines.push(help_key_row(
            "n",
            "\u{81ea}\u{5b9a}\u{4e49}\u{540d}\u{79f0}",
            7,
            t,
        ));
        lines.push(help_key_row(
            "d",
            "\u{4ec5}\u{6e05}\u{7a7a}\u{4e0d}\u{5f52}\u{6863}",
            7,
            t,
        ));
        lines.push(help_key_row("Esc", "\u{53d6}\u{6d88}", 7, t));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(t.border_title))
        .title(Span::styled(
            " \u{5f52}\u{6863}\u{786e}\u{8ba4} ",
            Style::default().fg(t.text_dim),
        ))
        .style(Style::default().bg(t.help_bg));
    let widget = Paragraph::new(lines).block(block);
    f.render_widget(widget, area);
}

/// 绘制归档列表界面
#[allow(clippy::vec_init_then_push)]
pub fn draw_archive_list(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;

    if app.ui.restore_confirm_needed {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(""));
        lines.push(section_header(
            "\u{26a0}\u{fe0f}",
            "\u{786e}\u{8ba4}\u{8fd8}\u{539f}",
            t,
        ));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  \u{5f53}\u{524d}\u{5bf9}\u{8bdd}\u{672a}\u{5f52}\u{6863}\u{ff0c}\u{8fd8}\u{539f}\u{5c06}\u{4e22}\u{5931}\u{5f53}\u{524d}\u{5bf9}\u{8bdd}\u{5185}\u{5bb9}\u{ff01}",
            Style::default().fg(t.text_white),
        )));
        lines.push(Line::from(""));
        lines.push(separator_line(area.width, t));
        lines.push(Line::from(""));
        if let Some(archive) = app.ui.archives.get(app.ui.archive_list_index) {
            lines.push(Line::from(vec![
                Span::styled(
                    "  \u{5c06}\u{8fd8}\u{539f}\u{5f52}\u{6863}\u{ff1a}",
                    Style::default().fg(t.text_dim),
                ),
                Span::styled(
                    &archive.name,
                    Style::default()
                        .fg(t.config_toggle_on)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(help_key_row(
            "y/Enter",
            "\u{786e}\u{8ba4}\u{8fd8}\u{539f}",
            8,
            t,
        ));
        lines.push(help_key_row("Esc", "\u{53d6}\u{6d88}", 8, t));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(t.toast_error_border))
            .title(Span::styled(
                " \u{8fd8}\u{539f}\u{786e}\u{8ba4} ",
                Style::default().fg(t.text_dim),
            ))
            .style(Style::default().bg(t.help_bg));
        let widget = Paragraph::new(lines).block(block);
        f.render_widget(widget, area);
        return;
    }

    if app.ui.archives.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "  \u{1f4e6} \u{6682}\u{65e0}\u{5f52}\u{6863}\u{5bf9}\u{8bdd}",
                Style::default().fg(t.text_dim).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  \u{6309} Ctrl+L \u{5f52}\u{6863}\u{5f53}\u{524d}\u{5bf9}\u{8bdd}",
                Style::default().fg(t.text_dim),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  \u{6309} Esc \u{8fd4}\u{56de}\u{804a}\u{5929}",
                Style::default().fg(t.text_dim),
            )),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(t.border_title))
            .title(Span::styled(
                " \u{5f52}\u{6863}\u{5217}\u{8868} ",
                Style::default().fg(t.text_dim),
            ))
            .style(Style::default().bg(t.help_bg));
        let widget = Paragraph::new(lines).block(block);
        f.render_widget(widget, area);
        return;
    }

    let items: Vec<ListItem> = app
        .ui
        .archives
        .iter()
        .enumerate()
        .map(|(i, archive)| {
            let is_selected = i == app.ui.archive_list_index;
            let marker = if is_selected {
                POINTER_SELECTED
            } else {
                POINTER_EMPTY
            };
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
                "{}{}  \u{1f4e8} {} \u{6761}\u{6d88}\u{606f}  \u{1f4c5} {}",
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
                    " \u{1f4e6} \u{5f52}\u{6863}\u{5217}\u{8868} (Enter \u{8fd8}\u{539f}, d \u{5220}\u{9664}, Esc \u{8fd4}\u{56de}) ",
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
    list_state.select(Some(app.ui.archive_list_index));
    f.render_stateful_widget(list, area, &mut list_state);
}
