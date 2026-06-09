use crate::command::chat::app::ChatApp;
use crate::command::chat::teammate::TeammateStatus;
use crate::command::chat::tools::derived_shared::SubAgentStatus;
use crate::tui::components::ItemList;
use crate::util::text::{display_width, sanitize_single_line_text};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Teammates tab 固定头部（团队摘要 + SubAgents 摘要）
pub(super) fn draw_tab_teammates_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;

    let snapshots = app
        .teammate_manager
        .lock()
        .map(|m| m.teammate_snapshots())
        .unwrap_or_default();
    let sub_snaps = app.sub_agent_tracker.display_snapshots();

    if snapshots.is_empty() && sub_snaps.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (暂无协作者 / 子代理)",
            Style::default().fg(t.config_dim),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  协作者由 AI 通过 Teammate 工具创建，子代理由 Agent 工具触发",
            Style::default().fg(t.config_dim),
        )));
        return;
    }

    // Teammate 汇总行
    if !snapshots.is_empty() {
        let working = snapshots
            .iter()
            .filter(|s| {
                s.status == TeammateStatus::Working || s.status == TeammateStatus::Initializing
            })
            .count();
        let waiting = snapshots
            .iter()
            .filter(|s| s.status == TeammateStatus::WaitingForMessage)
            .count();
        let completed = snapshots
            .iter()
            .filter(|s| s.status == TeammateStatus::Completed)
            .count();
        let errored = snapshots
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    TeammateStatus::Error(_) | TeammateStatus::Cancelled
                )
            })
            .count();

        let mut summary_spans = vec![Span::styled(
            format!("  协作者: {} 个  ", snapshots.len()),
            Style::default()
                .fg(t.config_label)
                .add_modifier(Modifier::BOLD),
        )];

        if working > 0 {
            summary_spans.push(Span::styled(
                format!("● {} 工作中", working),
                Style::default().fg(t.title_loading),
            ));
            summary_spans.push(Span::styled("  ", Style::default()));
        }
        if waiting > 0 {
            summary_spans.push(Span::styled(
                format!("○ {} 等待中", waiting),
                Style::default().fg(t.config_dim),
            ));
            summary_spans.push(Span::styled("  ", Style::default()));
        }
        if completed > 0 {
            summary_spans.push(Span::styled(
                format!("✓ {} 已完成", completed),
                Style::default().fg(t.config_toggle_on),
            ));
            summary_spans.push(Span::styled("  ", Style::default()));
        }
        if errored > 0 {
            summary_spans.push(Span::styled(
                format!("✗ {} 错误", errored),
                Style::default().fg(t.config_toggle_off),
            ));
        }
        lines.push(Line::from(summary_spans));
    }

    // SubAgent 汇总行
    if !sub_snaps.is_empty() {
        let working = sub_snaps
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    SubAgentStatus::Working
                        | SubAgentStatus::Initializing
                        | SubAgentStatus::Retrying { .. }
                )
            })
            .count();
        let completed = sub_snaps
            .iter()
            .filter(|s| s.status == SubAgentStatus::Completed)
            .count();
        let errored = sub_snaps
            .iter()
            .filter(|s| matches!(s.status, SubAgentStatus::Error(_)))
            .count();
        let cancelled = sub_snaps
            .iter()
            .filter(|s| s.status == SubAgentStatus::Cancelled)
            .count();

        let mut summary_spans = vec![Span::styled(
            format!("  子代理: {} 个  ", sub_snaps.len()),
            Style::default()
                .fg(t.config_label)
                .add_modifier(Modifier::BOLD),
        )];
        if working > 0 {
            summary_spans.push(Span::styled(
                format!("● {} 工作中", working),
                Style::default().fg(t.title_loading),
            ));
            summary_spans.push(Span::styled("  ", Style::default()));
        }
        if completed > 0 {
            summary_spans.push(Span::styled(
                format!("✓ {} 已完成", completed),
                Style::default().fg(t.config_toggle_on),
            ));
            summary_spans.push(Span::styled("  ", Style::default()));
        }
        if cancelled > 0 {
            summary_spans.push(Span::styled(
                format!("✗ {} 已取消", cancelled),
                Style::default().fg(t.text_dim),
            ));
            summary_spans.push(Span::styled("  ", Style::default()));
        }
        if errored > 0 {
            summary_spans.push(Span::styled(
                format!("✗ {} 错误", errored),
                Style::default().fg(t.config_toggle_off),
            ));
        }
        lines.push(Line::from(summary_spans));
    }

    lines.push(Line::from(Span::styled(
        "  (s 停止协作者, Enter 查看详情；子代理只读)",
        Style::default().fg(t.config_dim),
    )));
    lines.push(Line::from(""));
}

/// Teammates tab 可滚动列表
pub(super) fn draw_tab_teammates_list<'a>(app: &ChatApp) -> ItemList<'a> {
    let t = &app.ui.theme;
    let mut list = ItemList::new(t.bg_primary);

    let snapshots = app
        .teammate_manager
        .lock()
        .map(|m| m.teammate_snapshots())
        .unwrap_or_default();

    for (i, snap) in snapshots.iter().enumerate() {
        let is_selected = i == app.ui.teammate_list_index;

        let pointer = if is_selected { "❯ " } else { "  " };

        // 状态颜色和符号
        let status_color = match &snap.status {
            TeammateStatus::Thinking => t.title_loading,
            TeammateStatus::Working => t.title_loading,
            TeammateStatus::WaitingForMessage => t.config_dim,
            TeammateStatus::Completed => t.config_toggle_on,
            TeammateStatus::Cancelled => t.text_dim,
            TeammateStatus::Error(_) => t.config_toggle_off,
            TeammateStatus::Initializing => t.config_dim,
            TeammateStatus::Retrying { .. } => t.title_loading,
        };

        let safe_name = sanitize_single_line_text(&snap.name);
        let status_text = if snap.status == TeammateStatus::Working {
            if let Some(ref tool) = snap.current_tool {
                format!(
                    "{} {}: {}",
                    snap.status.icon(),
                    snap.status.label(),
                    sanitize_single_line_text(tool)
                )
            } else {
                format!("{} {}", snap.status.icon(), snap.status.label())
            }
        } else {
            format!("{} {}", snap.status.icon(), snap.status.label())
        };

        // 截断角色描述（按显示宽度截断，正确处理 CJK 等宽字符）
        let safe_role = sanitize_single_line_text(&snap.role);
        const ROLE_MAX_WIDTH: usize = 20;
        let role_display: String = if display_width(&safe_role) > ROLE_MAX_WIDTH {
            let truncated: String = safe_role
                .chars()
                .scan(0usize, |w, ch| {
                    *w += crate::util::text::char_width(ch);
                    if *w <= ROLE_MAX_WIDTH { Some(ch) } else { None }
                })
                .collect();
            format!("{truncated}…")
        } else {
            safe_role
        };

        let pointer_style = if is_selected {
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.text_normal)
        };

        let name_style = if is_selected {
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(t.text_white)
                .add_modifier(Modifier::BOLD)
        };

        let detail_style = if is_selected {
            Style::default().fg(t.config_label_selected)
        } else {
            Style::default().fg(t.config_dim)
        };
        let tool_calls_style = if is_selected {
            Style::default().fg(t.config_label_selected)
        } else {
            Style::default().fg(t.text_dim)
        };

        list.push(Line::from(vec![
            Span::styled(pointer.to_string(), pointer_style),
            Span::styled(format!("{:<12}", safe_name), name_style),
            Span::styled(format!("{:<22}", role_display), detail_style),
            Span::styled(
                format!("{:<16}", status_text),
                Style::default().fg(status_color),
            ),
            Span::styled(
                format!("{} 次调用", snap.tool_calls_count),
                tool_calls_style,
            ),
        ]));
    }

    // ── SubAgents 只读分组 ──
    let sub_snaps = app.sub_agent_tracker.display_snapshots();
    if !sub_snaps.is_empty() {
        list.push_raw(Line::from(Span::styled(
            "  ── 子代理 (只读) ──",
            Style::default()
                .fg(t.config_label)
                .add_modifier(Modifier::BOLD),
        )));
        list.push_raw(Line::from(""));

        for snap in sub_snaps.iter() {
            let status_color = match &snap.status {
                SubAgentStatus::Thinking => t.title_loading,
                SubAgentStatus::Working => t.title_loading,
                SubAgentStatus::Retrying { .. } => t.title_loading,
                SubAgentStatus::Completed => t.config_toggle_on,
                SubAgentStatus::Cancelled => t.text_dim,
                SubAgentStatus::Error(_) => t.config_toggle_off,
                SubAgentStatus::Initializing => t.config_dim,
            };

            let status_text = match &snap.status {
                SubAgentStatus::Thinking => {
                    format!("{} 思考中 R{}", snap.status.icon(), snap.current_round)
                }
                SubAgentStatus::Working => {
                    if let Some(ref tool) = snap.current_tool {
                        format!(
                            "{} 工作中 R{} · {}",
                            snap.status.icon(),
                            snap.current_round,
                            sanitize_single_line_text(tool)
                        )
                    } else {
                        format!("{} 工作中 R{}", snap.status.icon(), snap.current_round)
                    }
                }
                SubAgentStatus::Retrying {
                    attempt,
                    max_attempts,
                    ..
                } => {
                    format!("{} 重试 {}/{}", snap.status.icon(), attempt, max_attempts)
                }
                SubAgentStatus::Error(msg) => {
                    let safe_msg = sanitize_single_line_text(msg);
                    const ERR_MAX: usize = 28;
                    let truncated: String = safe_msg
                        .chars()
                        .scan(0usize, |w, ch| {
                            *w += crate::util::text::char_width(ch);
                            if *w <= ERR_MAX { Some(ch) } else { None }
                        })
                        .collect();
                    let suffix = if display_width(&safe_msg) > ERR_MAX {
                        "…"
                    } else {
                        ""
                    };
                    format!("{} 错误: {}{}", snap.status.icon(), truncated, suffix)
                }
                // 这些状态使用默认 icon + label 展示
                SubAgentStatus::Initializing
                | SubAgentStatus::Completed
                | SubAgentStatus::Cancelled => {
                    format!("{} {}", snap.status.icon(), snap.status.label())
                }
            };

            let safe_desc = sanitize_single_line_text(&snap.description);
            const DESC_MAX: usize = 22;
            let desc_display: String = if display_width(&safe_desc) > DESC_MAX {
                let truncated: String = safe_desc
                    .chars()
                    .scan(0usize, |w, ch| {
                        *w += crate::util::text::char_width(ch);
                        if *w <= DESC_MAX { Some(ch) } else { None }
                    })
                    .collect();
                format!("{truncated}…")
            } else {
                safe_desc
            };

            // 时长（秒）显示
            let elapsed = if snap.elapsed_secs < 60 {
                format!("{}s", snap.elapsed_secs)
            } else {
                format!("{}m{}s", snap.elapsed_secs / 60, snap.elapsed_secs % 60)
            };

            list.push_raw(Line::from(vec![
                Span::styled("  ".to_string(), Style::default().fg(t.text_normal)),
                Span::styled(
                    format!("{:<12}", snap.id),
                    Style::default()
                        .fg(t.text_white)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<24}", desc_display),
                    Style::default().fg(t.config_dim),
                ),
                Span::styled(
                    format!("{:<32}", status_text),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    format!(
                        "{} · {} 次调用 · {}",
                        snap.mode, snap.tool_calls_count, elapsed
                    ),
                    Style::default().fg(t.text_dim),
                ),
            ]));
        }
    }
    list
}
