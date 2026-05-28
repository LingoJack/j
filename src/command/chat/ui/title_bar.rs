use crate::command::chat::app::{ChatApp, ToolExecStatus};
use crate::command::chat::context::compact::estimate_tokens;
use crate::command::chat::teammate::TeammateStatus;
use crate::command::chat::tools::derived_shared::SubAgentStatus;
use crate::util::safe_lock;
use crate::util::text::{char_width, display_width, sanitize_single_line_text};

/// 标题栏模型名最大显示字符数。
const TITLE_MODEL_NAME_MAX_CHARS: usize = 20;
/// Teammate/SubAgent 状态区标签前缀显示宽度
const STATUS_LABEL_WIDTH: usize = 13; // " Teammates: " 或 " SubAgents: "
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

/// 计算标题栏需要的行数（供 draw_chat_ui 预分配高度）
pub fn calc_title_height(app: &ChatApp) -> u16 {
    let has_teammates = app
        .teammate_manager
        .lock()
        .map(|m| !m.teammates.is_empty())
        .unwrap_or(false);
    let has_subagents = !app.sub_agent_tracker.display_snapshots().is_empty();

    if !has_teammates && !has_subagents {
        return 1; // 状态行
    }

    // 需要估算实际行数（包含换行）
    // 保守估计：每个 agent 最多占一行
    let tm_count = if has_teammates {
        app.teammate_manager
            .lock()
            .map(|m| m.teammates.len())
            .unwrap_or(0)
    } else {
        0
    };
    let sa_count = if has_subagents {
        app.sub_agent_tracker.display_snapshots().len()
    } else {
        0
    };

    // 基础：状态行(1) + agent分割线(1)
    let mut height: u16 = 2;

    if tm_count > 0 {
        height += tm_count.min(u16::MAX as usize) as u16;
    }
    if sa_count > 0 {
        height += sa_count.min(u16::MAX as usize) as u16;
    }

    // 上限：不超过 8 行，避免挤占消息区
    height.min(8)
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
            format!(" {}", sanitize_single_line_text(hint))
        } else {
            let tool_info = app
                .tool_executor
                .active_tool_calls
                .iter()
                .find(|tc| matches!(tc.status, ToolExecStatus::Executing))
                .map(|tc| {
                    if let Some(ref desc) = tc.tool_description {
                        format!(
                            " ⚙ 执行 {} - {}...",
                            tc.tool_name,
                            sanitize_single_line_text(desc)
                        )
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
                                format!(
                                    " ⚙ 调用 {} - {}...",
                                    tc.tool_name,
                                    sanitize_single_line_text(desc)
                                )
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

    // 第一行：状态信息（左侧：品牌+指标，右侧：动态状态）
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
    f.render_widget(content_line, Rect::new(area.x, area.y, area.width, 1));

    // ========== 分割线 + Teammate + SubAgent 状态 ==========
    let mut next_row = area.y + 1;

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

    // Teammate 行（支持多行换行）
    if next_row < area.y + area.height && has_teammates {
        let snapshots = app
            .teammate_manager
            .lock()
            .map(|m| m.teammate_snapshots())
            .unwrap_or_default();

        if !snapshots.is_empty() {
            let label_style = Style::default().fg(t.text_dim).add_modifier(Modifier::BOLD);
            let separator_style = Style::default().fg(t.title_separator);

            // 收集每个 teammate 的 entry spans 和宽度
            let entries: Vec<(Vec<Span<'static>>, usize)> = snapshots
                .iter()
                .map(|snap| build_teammate_entry(snap, t))
                .collect();

            let lines = wrap_entries(
                entries,
                max_width,
                " Teammates: ",
                label_style,
                separator_style,
                t,
            );

            for line_spans in lines {
                if next_row >= area.y + area.height {
                    break;
                }
                let line =
                    Paragraph::new(Line::from(line_spans)).style(Style::default().bg(t.bg_primary));
                f.render_widget(line, Rect::new(area.x, next_row, area.width, 1));
                next_row += 1;
            }
        }
    }

    // SubAgent 行（支持多行换行）
    if next_row < area.y + area.height && has_subagents {
        let sub_snaps = app.sub_agent_tracker.display_snapshots();
        if !sub_snaps.is_empty() {
            let label_style = Style::default().fg(t.text_dim).add_modifier(Modifier::BOLD);
            let separator_style = Style::default().fg(t.title_separator);

            let entries: Vec<(Vec<Span<'static>>, usize)> = sub_snaps
                .iter()
                .map(|snap| build_subagent_entry(snap, t))
                .collect();

            let lines = wrap_entries(
                entries,
                max_width,
                " SubAgents: ",
                label_style,
                separator_style,
                t,
            );

            for line_spans in lines {
                if next_row >= area.y + area.height {
                    break;
                }
                let line =
                    Paragraph::new(Line::from(line_spans)).style(Style::default().bg(t.bg_primary));
                f.render_widget(line, Rect::new(area.x, next_row, area.width, 1));
                next_row += 1;
            }
        }
    }
}

/// 构建单个 Teammate 的 spans 和总宽度
fn build_teammate_entry(
    snap: &crate::command::chat::teammate::TeammateSnapshot,
    t: &crate::theme::Theme,
) -> (Vec<Span<'static>>, usize) {
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

    let spans = vec![
        Span::styled(
            safe_name.clone(),
            Style::default()
                .fg(t.text_white)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" [", Style::default().fg(t.text_dim)),
        Span::styled(status_text.clone(), Style::default().fg(status_color)),
        Span::styled("]", Style::default().fg(t.text_dim)),
    ];

    // name + " [" + status_text + "]"
    let width = display_width(&safe_name) + 2 + display_width(&status_text) + 1;
    (spans, width)
}

/// 构建单个 SubAgent 的 spans 和总宽度
fn build_subagent_entry(
    snap: &crate::command::chat::tools::derived_shared::SubAgentDisplay,
    t: &crate::theme::Theme,
) -> (Vec<Span<'static>>, usize) {
    let status_color = match &snap.status {
        SubAgentStatus::Thinking => t.title_loading,
        SubAgentStatus::Working => t.title_loading,
        SubAgentStatus::Retrying { .. } => t.title_loading,
        SubAgentStatus::Completed => t.config_toggle_on,
        SubAgentStatus::Cancelled => t.text_dim,
        SubAgentStatus::Error(_) => t.config_toggle_off,
        SubAgentStatus::Initializing => t.config_dim,
    };

    let name = short_subagent_label(&snap.description);

    let status_text = match &snap.status {
        SubAgentStatus::Thinking => {
            format!("{} R{}", snap.status.icon(), snap.current_round,)
        }
        SubAgentStatus::Working => {
            if let Some(ref tool) = snap.current_tool {
                format!(
                    "{} R{} {}",
                    snap.status.icon(),
                    snap.current_round,
                    sanitize_single_line_text(tool)
                )
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
        SubAgentStatus::Initializing | SubAgentStatus::Cancelled => {
            format!("{} {}", snap.status.icon(), snap.status.label())
        }
    };

    let spans = vec![
        Span::styled(
            name.clone(),
            Style::default()
                .fg(t.text_white)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" [", Style::default().fg(t.text_dim)),
        Span::styled(status_text.clone(), Style::default().fg(status_color)),
        Span::styled("]", Style::default().fg(t.text_dim)),
    ];

    let width = display_width(&name) + 2 + display_width(&status_text) + 1;
    (spans, width)
}

/// 将多个 entry spans 按行宽 wrap 成多行
/// 每行之间用 " │ " 分隔，首行有 label 前缀
fn wrap_entries(
    entries: Vec<(Vec<Span<'static>>, usize)>,
    max_width: usize,
    label: &str,
    label_style: Style,
    separator_style: Style,
    t: &crate::theme::Theme,
) -> Vec<Vec<Span<'static>>> {
    if entries.is_empty() {
        return vec![];
    }

    let separator = " │ ";
    let separator_width = 3;
    let indent = "           "; // 与 label 对齐的缩进（STATUS_LABEL_WIDTH 个字符）

    let mut lines: Vec<Vec<Span<'static>>> = Vec::new();
    let mut current_line: Vec<Span<'static>> = vec![Span::styled(label.to_string(), label_style)];
    let mut current_width = STATUS_LABEL_WIDTH;
    let mut is_first_entry = true;

    for (entry_spans, entry_width) in entries {
        let needed = if is_first_entry {
            entry_width
        } else {
            separator_width + entry_width
        };

        if current_width + needed > max_width {
            // 当前行已满，保存当前行
            lines.push(current_line);

            // 开始新行，带缩进
            current_line = vec![Span::styled(
                indent.to_string(),
                Style::default().fg(t.text_dim),
            )];
            current_width = STATUS_LABEL_WIDTH;

            // 再检查单个 entry 是否能放进新行
            if current_width + entry_width > max_width {
                // 单个 entry 太长，截断并占满一行
                current_line.extend(entry_spans);
                lines.push(current_line);
                current_line = vec![Span::styled(
                    indent.to_string(),
                    Style::default().fg(t.text_dim),
                )];
                current_width = STATUS_LABEL_WIDTH;
                is_first_entry = true;
                continue;
            }
        }

        if !is_first_entry {
            current_line.push(Span::styled(separator.to_string(), separator_style));
            current_width += separator_width;
        }

        current_line.extend(entry_spans);
        current_width += entry_width;
        is_first_entry = false;
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// 将 subagent description 转为紧凑标签（<=20 显示宽度，空白转 _）
fn short_subagent_label(description: &str) -> String {
    let cleaned: String = sanitize_single_line_text(description)
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect();
    if display_width(&cleaned) <= TITLE_MODEL_NAME_MAX_CHARS {
        cleaned
    } else {
        let s: String = cleaned
            .chars()
            .scan(0usize, |w, ch| {
                *w += char_width(ch);
                if *w <= TITLE_MODEL_NAME_MAX_CHARS {
                    Some(ch)
                } else {
                    None
                }
            })
            .collect();
        format!("{}…", s)
    }
}

/// 截断字符串到指定显示宽度，超长时加 "..."
pub(crate) fn truncate_str(s: &str, max_w: usize) -> String {
    use crate::util::text::{char_width, display_width};
    let s = sanitize_single_line_text(s);
    let w = display_width(&s);
    if w <= max_w {
        return s;
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
