use crate::command::chat::app::ChatApp;
use crate::tui::components::{ItemList, selectable_row};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Archive tab 固定头部（确认还原 + 归档列表标题）
pub(super) fn draw_tab_archive_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;

    // 确认还原覆盖层
    if app.ui.restore_confirm_needed {
        lines.push(Line::from(Span::styled(
            "  \u{26a0}\u{fe0f}  \u{5f53}\u{524d}\u{4f1a}\u{8bdd}\u{6709}\u{6d88}\u{606f}\u{ff0c}\u{8fd8}\u{539f}\u{5c06}\u{66ff}\u{6362}\u{5f53}\u{524d}\u{5bf9}\u{8bdd}\u{ff08}\u{5f53}\u{524d}\u{4f1a}\u{8bdd}\u{5df2}\u{81ea}\u{52a8}\u{4fdd}\u{5b58}\u{ff09}",
            Style::default()
                .fg(t.config_toggle_off)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "  \u{6309} y/Enter \u{786e}\u{8ba4}\u{8fd8}\u{539f}\u{ff0c}Esc \u{53d6}\u{6d88}",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(""));
    }

    if app.ui.archives.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (\u{6682}\u{65e0}\u{5f52}\u{6863})",
            Style::default().fg(t.config_dim),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            format!(
                "  \u{5f52}\u{6863}\u{5217}\u{8868} ({})",
                app.ui.archives.len()
            ),
            Style::default()
                .fg(t.config_label)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
    }
}

/// Archive tab 可滚动列表（归档列表）
pub(super) fn draw_tab_archive_list<'a>(app: &ChatApp) -> ItemList<'a> {
    let t = &app.ui.theme;
    let mut list = ItemList::new(t.bg_primary);

    for (i, archive) in app.ui.archives.iter().enumerate() {
        let is_selected = i == app.ui.archive_list_index;
        let name_truncated: String = archive.name.chars().take(40).collect();
        let time_str = &archive.created_at;
        let secondary = format!("({} \u{6761}, {})", archive.messages.len(), time_str);
        list.push(selectable_row(&name_truncated, &secondary, is_selected, t));
    }
    list
}
