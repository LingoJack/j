use crate::command::chat::app::ChatApp;
use crate::tui::components::{ItemList, selectable_row};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// 格式化 Unix 时间戳为人类可读格式
fn format_timestamp(ts: u64) -> String {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    let dt = UNIX_EPOCH + Duration::from_secs(ts);
    let now = SystemTime::now();
    let elapsed = now.duration_since(dt).unwrap_or_default();
    if elapsed.as_secs() < 60 {
        "\u{521a}\u{521a}".to_string()
    } else if elapsed.as_secs() < 3600 {
        format!("{}\u{5206}\u{949f}\u{524d}", elapsed.as_secs() / 60)
    } else if elapsed.as_secs() < 86400 {
        format!("{}\u{5c0f}\u{65f6}\u{524d}", elapsed.as_secs() / 3600)
    } else if elapsed.as_secs() < 86400 * 30 {
        format!("{}\u{5929}\u{524d}", elapsed.as_secs() / 86400)
    } else {
        let secs = ts;
        let days = secs / 86400;
        let (y, m, d) = days_to_ymd(days);
        format!("{y:04}-{m:02}-{d:02}")
    }
}

/// 将 1970-01-01 起的天数转为 (year, month, day)
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let mut y = 1970;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1;
    for &md in &month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        m += 1;
    }
    (y, m, remaining + 1)
}

fn is_leap(y: u64) -> bool {
    y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400))
}

/// Session tab 固定头部（当前会话 + 确认恢复 + 历史会话标题）
pub(super) fn draw_tab_session_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;

    // 当前会话信息
    let msg_count = crate::util::safe_lock(&app.display_messages, "session_tab::msg_count").len();
    lines.push(Line::from(vec![
        Span::styled(
            "  \u{5f53}\u{524d}\u{4f1a}\u{8bdd}: ",
            Style::default().fg(t.config_label),
        ),
        Span::styled(
            format!(
                "{} ({} \u{6761}\u{6d88}\u{606f})",
                &app.session_id, msg_count
            ),
            Style::default()
                .fg(t.config_toggle_on)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    // 确认恢复覆盖层
    if app.ui.session_restore_confirm {
        lines.push(Line::from(Span::styled(
            "  \u{26a0}\u{fe0f}  \u{5f53}\u{524d}\u{4f1a}\u{8bdd}\u{6709}\u{6d88}\u{606f}\u{ff0c}\u{6062}\u{590d}\u{5c06}\u{5207}\u{6362}\u{5230}\u{5386}\u{53f2}\u{4f1a}\u{8bdd}\u{ff08}\u{5f53}\u{524d}\u{4f1a}\u{8bdd}\u{5df2}\u{81ea}\u{52a8}\u{4fdd}\u{5b58}\u{ff09}",
            Style::default()
                .fg(t.config_toggle_off)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "  \u{6309} y/Enter \u{786e}\u{8ba4}\u{6062}\u{590d}\u{ff0c}Esc \u{53d6}\u{6d88}",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(""));
    }

    if app.ui.session_list.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (\u{6ca1}\u{6709}\u{5386}\u{53f2}\u{4f1a}\u{8bdd})",
            Style::default().fg(t.config_dim),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            format!(
                "  \u{5386}\u{53f2}\u{4f1a}\u{8bdd} ({})",
                app.ui.session_list.len()
            ),
            Style::default()
                .fg(t.config_label)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
    }
}

/// Session tab 可滚动列表（历史会话列表）
pub(super) fn draw_tab_session_list<'a>(app: &ChatApp) -> ItemList<'a> {
    let t = &app.ui.theme;
    let mut list = ItemList::new(t.bg_primary);

    for (i, session) in app.ui.session_list.iter().enumerate() {
        let is_selected = i == app.ui.session_list_index;
        let preview = session
            .title
            .as_deref()
            .or(session.first_message_preview.as_deref())
            .unwrap_or("(\u{7a7a}\u{4f1a}\u{8bdd})");
        let preview_truncated: String = preview.chars().take(40).collect();
        let time_str = format_timestamp(session.updated_at);
        let secondary = format!("({} \u{6761}, {})", session.message_count, time_str);
        list.push(selectable_row(
            &preview_truncated,
            &secondary,
            is_selected,
            t,
        ));
    }
    list
}
