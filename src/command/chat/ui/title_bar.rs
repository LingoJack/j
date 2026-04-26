use crate::command::chat::app::{ChatApp, ToolExecStatus};
use crate::command::chat::context::compact::estimate_tokens;
use crate::command::chat::teammate::TeammateStatus;
use crate::command::chat::tools::derived_shared::SubAgentStatus;
use crate::util::safe_lock;
use crate::util::text::display_width;

/// 标题栏模型名最大显示字符数。
const TITLE_MODEL_NAME_MAX_CHARS: usize = 20;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// 格式化上下文估算值
fn format_context_tokens(tokens: usize) -> String {
    if tokens >= 1000 {
        format!("{}K", tokens / 1000)
    } else {
        tokens.to_string()
    }
}

/// 绘制标题栏
pub fn draw_title_bar(
    f: &mut ratatui::Frame,
    area: Rect,
    app: &ChatApp,
    has_teammates: bool,
    has_subagents: bool,
) {
    let t = &app.ui.theme;
    let msg_count = safe_lock(&app.display_messages, "title_bar::msg_count").len();

    // 估算上下文 tokens：优先使用 agent 实际上下文 token 数，否则从 context_messages 估算
    let estimated_tokens = {
        let agent_tokens = app.context_tokens.lock().ok().map(|ct| *ct).unwrap_or(0);
        if agent_tokens > 0 {
            agent_tokens
        } else {
            estimate_tokens(&safe_lock(&app.context_messages, "title_bar::est_tokens"))
        }
    };
    let ctx_str = format_context_tokens(estimated_tokens);

    let loading = if app.state.is_loading {
        // 优先级：重试提示 > 工具执行 > 工具等待确认 > 默认思考中
        if let Some(ref hint) = app.state.retry_hint {
            format!(" {}", hint)
        } else {
            let tool_info = app
                .tool_executor
                .active_tool_calls
                .iter()
                .find(|tc| matches!(tc.status, ToolExecStatus::Executing))
                .map(|tc| {
                    if let Some(ref desc) = tc.tool_description {
                        format!(" ⚙ 执行 {} - {}...", tc.tool_name, desc)
                    } else {
                        format!(" ⚙ 执行 {}...", tc.tool_name)
                    }
                })
                .or_else(|| {
                    app.tool_executor
                        .active_tool_calls
                        .iter()
                        .find(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))
                        .map(|tc| {
                            if let Some(ref desc) = tc.tool_description {
                                format!(" ⚙ 调用 {} - {}...", tc.tool_name, desc)
                            } else {
                                format!(" ⚙ 调用 {}...", tc.tool_name)
                            }
                        })
                });
            if let Some(info) = tool_info {
                info
            } else {
                " ⏱ 思考中...".to_string()
            }
        }
    } else {
        String::new()
    };

    // 第一行：顶部分割线
    let top_separator = Paragraph::new(Line::styled(
        "─".repeat(area.width as usize),
        Style::default().fg(t.border_title),
    ))
    .style(Style::default().bg(t.bg_primary));
    f.render_widget(top_separator, Rect::new(area.x, area.y, area.width, 1));

    // 第二行：状态信息（左侧：品牌+指标，右侧：动态状态）
    let icon = if app.ui.auto_approve {
        Span::styled(
            " ▶▶ ",
            Style::default()
                .fg(t.config_toggle_off)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(" 🦞 ", Style::default().fg(t.title_icon))
    };

    let left_spans: Vec<Span> = vec![
        icon,
        Span::styled(
            "Sprite",
            Style::default()
                .fg(t.text_white)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("context({})", ctx_str),
            Style::default()
                .fg(t.title_model)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" · ", Style::default().fg(t.text_dim)),
        Span::styled(
            format!("message({})", msg_count),
            Style::default().fg(t.title_count),
        ),
    ];

    // 思考中状态放在左侧（紧跟 message 后面）
    let mut loading_spans: Vec<Span> = Vec::new();
    if !loading.is_empty() {
        loading_spans.push(Span::styled("  ", Style::default()));
        loading_spans.push(Span::styled(
            loading,
            Style::default()
                .fg(t.title_loading)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // 右侧动态状态（远程连接等）
    let mut right_spans: Vec<Span> = Vec::new();
    if app.remote_connected {
        right_spans.push(Span::styled(
            "◉ 远程",
            Style::default()
                .fg(t.title_count)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // 拼接：左侧 + 思考状态 + 填充空格（右对齐右侧）+ 右侧
    let left_width: usize = left_spans
        .iter()
        .map(|s| display_width(s.content.as_ref()))
        .sum();
    let loading_width: usize = loading_spans
        .iter()
        .map(|s| display_width(s.content.as_ref()))
        .sum();
    let right_width: usize = right_spans
        .iter()
        .map(|s| display_width(s.content.as_ref()))
        .sum();
    let available = area.width as usize;
    let padding = available.saturating_sub(left_width + loading_width + right_width);

    let mut title_spans = left_spans;
    title_spans.extend(loading_spans);
    title_spans.push(Span::raw(" ".repeat(padding)));
    title_spans.extend(right_spans);

    // 渲染内容行
    let content_line =
        Paragraph::new(Line::from(title_spans)).style(Style::default().bg(t.bg_primary));
    f.render_widget(content_line, Rect::new(area.x, area.y + 1, area.width, 1));

    // ========== 分割线 + Teammate + SubAgent 状态 ==========
    let mut next_row = area.y + 2;

    // 状态行与 teammate/subagent 之间的分割线
    if has_teammates || has_subagents {
        let separator = Paragraph::new(Line::styled(
            "─".repeat(area.width as usize),
            Style::default().fg(t.border_title),
        ))
        .style(Style::default().bg(t.bg_primary));
        f.render_widget(separator, Rect::new(area.x, next_row, area.width, 1));
        next_row += 1;
    }

    let max_width = area.width as usize;

    // Teammate 行
    if next_row < area.y + area.height && has_teammates {
        let snapshots = app
            .teammate_manager
            .lock()
            .map(|m| m.teammate_snapshots())
            .unwrap_or_default();

        if !snapshots.is_empty() {
            let mut tm_spans: Vec<Span> = vec![Span::styled(
                " Teammates: ",
                Style::default().fg(t.text_dim).add_modifier(Modifier::BOLD),
            )];
            let mut current_width: usize = 13;

            for (i, snap) in snapshots.iter().enumerate() {
                if i > 0 {
                    if current_width + 3 > max_width {
                        tm_spans.push(Span::styled(" …", Style::default().fg(t.text_dim)));
                        break;
                    }
                    tm_spans.push(Span::styled(" │ ", Style::default().fg(t.title_separator)));
                    current_width += 3;
                }

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

                let status_text = if snap.status == TeammateStatus::Working {
                    if let Some(ref tool) = snap.current_tool {
                        format!("{} {}: {}", snap.status.icon(), snap.status.label(), tool)
                    } else {
                        format!("{} {}", snap.status.icon(), snap.status.label())
                    }
                } else {
                    format!("{} {}", snap.status.icon(), snap.status.label())
                };

                let entry_width = snap.name.len() + 2 + status_text.len() + 1;
                if current_width + entry_width > max_width {
                    tm_spans.push(Span::styled(" …", Style::default().fg(t.text_dim)));
                    break;
                }

                tm_spans.push(Span::styled(
                    snap.name.clone(),
                    Style::default()
                        .fg(t.text_white)
                        .add_modifier(Modifier::BOLD),
                ));
                tm_spans.push(Span::styled(" [", Style::default().fg(t.text_dim)));
                tm_spans.push(Span::styled(status_text, Style::default().fg(status_color)));
                tm_spans.push(Span::styled("]", Style::default().fg(t.text_dim)));
                current_width += entry_width;
            }

            let tm_line =
                Paragraph::new(Line::from(tm_spans)).style(Style::default().bg(t.bg_primary));
            f.render_widget(tm_line, Rect::new(area.x, next_row, area.width, 1));
            next_row += 1;
        }
    }

    // SubAgent 行（仅在 area 还有剩余行时渲染）
    if next_row < area.y + area.height {
        let sub_snaps = app.sub_agent_tracker.display_snapshots();
        if !sub_snaps.is_empty() {
            let mut sa_spans: Vec<Span> = vec![Span::styled(
                " SubAgents: ",
                Style::default().fg(t.text_dim).add_modifier(Modifier::BOLD),
            )];
            let mut current_width: usize = 13;

            for (i, snap) in sub_snaps.iter().enumerate() {
                if i > 0 {
                    if current_width + 3 > max_width {
                        sa_spans.push(Span::styled(" …", Style::default().fg(t.text_dim)));
                        break;
                    }
                    sa_spans.push(Span::styled(" │ ", Style::default().fg(t.title_separator)));
                    current_width += 3;
                }

                let status_color = match &snap.status {
                    SubAgentStatus::Thinking => t.title_loading,
                    SubAgentStatus::Working => t.title_loading,
                    SubAgentStatus::Retrying { .. } => t.title_loading,
                    SubAgentStatus::Completed => t.config_toggle_on,
                    SubAgentStatus::Cancelled => t.text_dim,
                    SubAgentStatus::Error(_) => t.config_toggle_off,
                    SubAgentStatus::Initializing => t.config_dim,
                };

                // 名字：description 做紧凑显示（≤20 字节）
                let name = short_subagent_label(&snap.description);

                // 状态文字：Thinking 显示轮次；Working 显示当前工具/轮次；Retrying 显示重试进度；终态显示 icon+label；Error 显示截断错误
                let status_text = match &snap.status {
                    SubAgentStatus::Thinking => {
                        format!("{} R{}", snap.status.icon(), snap.current_round,)
                    }
                    SubAgentStatus::Working => {
                        if let Some(ref tool) = snap.current_tool {
                            format!("{} R{} {}", snap.status.icon(), snap.current_round, tool)
                        } else {
                            format!(
                                "{} R{}/t{}",
                                snap.status.icon(),
                                snap.current_round,
                                snap.tool_calls_count
                            )
                        }
                    }
                    SubAgentStatus::Retrying {
                        attempt,
                        max_attempts,
                        ..
                    } => {
                        format!("{} {}/{}", snap.status.icon(), attempt, max_attempts)
                    }
                    SubAgentStatus::Error(msg) => {
                        let short = truncate_str(msg, 20);
                        format!("{} {}", snap.status.icon(), short)
                    }
                    SubAgentStatus::Completed => {
                        format!(
                            "{} {} t{}",
                            snap.status.icon(),
                            snap.status.label(),
                            snap.tool_calls_count
                        )
                    }
                    // 这些状态使用默认 icon + label 展示
                    SubAgentStatus::Initializing | SubAgentStatus::Cancelled => {
                        format!("{} {}", snap.status.icon(), snap.status.label())
                    }
                };

                let entry_width = name.chars().count() + 2 + status_text.chars().count() + 1;
                if current_width + entry_width > max_width {
                    sa_spans.push(Span::styled(" …", Style::default().fg(t.text_dim)));
                    break;
                }

                sa_spans.push(Span::styled(
                    name,
                    Style::default()
                        .fg(t.text_white)
                        .add_modifier(Modifier::BOLD),
                ));
                sa_spans.push(Span::styled(" [", Style::default().fg(t.text_dim)));
                sa_spans.push(Span::styled(status_text, Style::default().fg(status_color)));
                sa_spans.push(Span::styled("]", Style::default().fg(t.text_dim)));
                current_width += entry_width;
            }

            let sa_line =
                Paragraph::new(Line::from(sa_spans)).style(Style::default().bg(t.bg_primary));
            f.render_widget(sa_line, Rect::new(area.x, next_row, area.width, 1));
        }
    }
}

/// 将 subagent description 转为紧凑标签（<=20 字符，空白转 _）
fn short_subagent_label(description: &str) -> String {
    let cleaned: String = description
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect();
    if cleaned.chars().count() <= 20 {
        cleaned
    } else {
        let s: String = cleaned.chars().take(TITLE_MODEL_NAME_MAX_CHARS).collect();
        format!("{}…", s)
    }
}

/// 截断字符串到指定显示宽度，超长时加 "..."
pub(crate) fn truncate_str(s: &str, max_w: usize) -> String {
    use crate::util::text::{char_width, display_width};
    let w = display_width(s);
    if w <= max_w {
        return s.to_string();
    }
    let ellipsis = "...";
    let target = max_w.saturating_sub(3);
    let mut cur_w = 0;
    let mut end = 0;
    for c in s.chars() {
        let cw = char_width(c);
        if cur_w + cw > target {
            break;
        }
        cur_w += cw;
        end += c.len_utf8();
    }
    format!("{}{}", &s[..end], ellipsis)
}
