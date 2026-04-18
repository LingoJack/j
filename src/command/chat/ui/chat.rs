use super::super::app::{ChatApp, ChatMode, MsgLinesCache, ToolExecStatus};
use super::super::compact::estimate_tokens;
use super::super::handler::{
    AtPopupItem, get_filtered_all_items, get_filtered_command_names, get_filtered_files,
    get_filtered_skill_names, get_filtered_slash_commands,
};
use super::super::markdown::image_cache::ImageState;
use super::super::markdown::image_loader::load_image;
use super::super::render_cache::build_message_lines_incremental;
use super::super::teammate::TeammateStatus;
use super::super::tools::agent_shared::SubAgentStatus;
use super::archive::{draw_archive_confirm, draw_archive_list};
use super::config::draw_config_screen;
use crate::util::safe_lock;
use crate::util::text::{char_width, display_width};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use ratatui_image::{Resize, StatefulImage};

pub fn draw_chat_ui(f: &mut ratatui::Frame, app: &mut ChatApp) {
    let size = f.area();

    // 整体背景
    let bg = Block::default().style(Style::default().bg(app.ui.theme.bg_primary));
    f.render_widget(bg, size);

    // 动态标题栏高度：顶部分割线(1) + 状态行(1) + 可选分割线(1) + 可选 teammate 行 + 可选 subagent 行
    let has_teammates = app
        .teammate_manager
        .lock()
        .map(|m| !m.teammates.is_empty())
        .unwrap_or(false);
    let has_subagents = !app.sub_agent_tracker.display_snapshots().is_empty();
    let status_separator = (has_teammates || has_subagents) as u16; // 状态行与 teammate/subagent 之间的分割线
    let title_height = 2 + status_separator + (has_teammates as u16) + (has_subagents as u16);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(title_height), // 标题栏（顶部分割线 + 内容行 + 可选 teammate 行）
            Constraint::Min(5),               // 消息区
            Constraint::Length(5),            // 输入区
            Constraint::Length(1),            // 操作提示栏（始终可见）
        ])
        .split(size);

    // ========== 标题栏 ==========
    draw_title_bar(f, chunks[0], app, has_teammates, has_subagents);

    // ========== 消息区 ==========
    match app.ui.mode {
        ChatMode::Help => super::help::draw_help(f, chunks[1], app),
        ChatMode::SelectModel => super::selector::draw_model_selector(f, chunks[1], app),
        ChatMode::SelectTheme => super::selector::draw_theme_selector(f, chunks[1], app),
        ChatMode::Config => draw_config_screen(f, chunks[1], app),
        ChatMode::ArchiveConfirm => draw_archive_confirm(f, chunks[1], app),
        ChatMode::ArchiveList => draw_archive_list(f, chunks[1], app),
        _ => draw_messages(f, chunks[1], app),
    }

    // ========== 输入区 ==========
    super::input::draw_input(f, chunks[2], app);

    // ========== 底部操作提示栏（始终可见）==========
    super::hint::draw_hint_bar(f, chunks[3], app);

    // ========== Toast 弹窗覆盖层（右上角）==========
    super::hint::draw_toast(f, size, app);

    // ========== @ 补全弹窗覆盖层 ==========
    if app.ui.at_popup_active {
        draw_at_popup(f, chunks[2], app);
    }

    // ========== 文件补全弹窗覆盖层 ==========
    if app.ui.file_popup_active {
        draw_file_popup(f, chunks[2], app);
    }

    // ========== 技能补全弹窗覆盖层 ==========
    if app.ui.skill_popup_active {
        draw_skill_popup(f, chunks[2], app);
    }

    // ========== 命令补全弹窗覆盖层 ==========
    if app.ui.command_popup_active {
        draw_command_popup(f, chunks[2], app);
    }

    // ========== / 斜杠命令弹窗覆盖层 ==========
    if app.ui.slash_popup_active {
        draw_slash_popup(f, chunks[2], app);
    }
}

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
    let msg_count = app.state.session.messages.len();

    // 估算上下文 tokens：优先使用 agent 实际上下文 token 数，否则从 session.messages 估算
    let estimated_tokens = {
        let agent_tokens = app.context_tokens.lock().ok().map(|ct| *ct).unwrap_or(0);
        if agent_tokens > 0 {
            agent_tokens
        } else {
            estimate_tokens(&app.state.session.messages)
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
                .map(|tc| format!(" 🔧 执行 {}...", tc.tool_name))
                .or_else(|| {
                    app.tool_executor
                        .active_tool_calls
                        .iter()
                        .find(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))
                        .map(|tc| format!(" 🔧 调用 {}...", tc.tool_name))
                });
            if let Some(info) = tool_info {
                info
            } else {
                " ⏳ 思考中...".to_string()
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

    // 第二行：状态信息
    let mut title_spans = vec![
        Span::styled(" 🦞 ", Style::default().fg(t.title_icon)),
        Span::styled(
            "Sprite",
            Style::default()
                .fg(t.text_white)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(t.title_separator)),
        Span::styled("💫 ", Style::default()),
        Span::styled(
            format!("Context: {}", ctx_str),
            Style::default()
                .fg(t.title_model)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(t.title_separator)),
        Span::styled(
            format!("📬 Message: {}", msg_count),
            Style::default().fg(t.title_count),
        ),
        Span::styled("  │  ", Style::default().fg(t.title_separator)),
    ];

    // Loading 状态（始终显示分隔符）
    if !loading.is_empty() {
        title_spans.push(Span::styled(
            loading,
            Style::default()
                .fg(t.title_loading)
                .add_modifier(Modifier::BOLD),
        ));
    }

    // 远程控制连接指示器
    if app.remote_connected {
        title_spans.push(Span::styled(
            "  │  ",
            Style::default().fg(t.title_separator),
        ));
        title_spans.push(Span::styled(
            "📱 远程已连接",
            Style::default()
                .fg(t.title_count)
                .add_modifier(Modifier::BOLD),
        ));
    }

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

    // Teammate 行
    let max_width = area.width as usize;
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
                    TeammateStatus::Working => t.title_loading,
                    TeammateStatus::WaitingForMessage => t.config_dim,
                    TeammateStatus::Completed => t.config_toggle_on,
                    TeammateStatus::Cancelled => t.text_dim,
                    TeammateStatus::Error(_) => t.config_toggle_off,
                    TeammateStatus::Initializing => t.config_dim,
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
                    SubAgentStatus::Working => t.title_loading,
                    SubAgentStatus::Completed => t.config_toggle_on,
                    SubAgentStatus::Cancelled => t.text_dim,
                    SubAgentStatus::Error(_) => t.config_toggle_off,
                    SubAgentStatus::Initializing => t.config_dim,
                };

                // 名字：description 做紧凑显示（≤20 字节）
                let name = short_subagent_label(&snap.description);

                // 状态文字：Working 显示当前工具/轮次；终态显示 icon+label；Error 显示截断错误
                let status_text = match &snap.status {
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
                    _ => format!("{} {}", snap.status.icon(), snap.status.label()),
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
        let s: String = cleaned.chars().take(20).collect();
        format!("{}…", s)
    }
}

/// 给定全局行号，定位到 per_msg_lines 或 streaming_lines 中对应的行引用
/// history_total 是所有历史消息的总行数（预计算，避免重复求和）
fn get_line_at(
    cached: &MsgLinesCache,
    global_idx: usize,
    history_total: usize,
) -> Option<&Line<'static>> {
    if global_idx < history_total {
        // 二分查找 msg_start_lines 定位所属消息
        let msg_pos = cached
            .msg_start_lines
            .partition_point(|&(_, start)| start <= global_idx);
        if msg_pos == 0 {
            return None;
        }
        let (_msg_idx, start) = cached.msg_start_lines[msg_pos - 1];
        let local = global_idx - start;
        let per = &cached.per_msg_lines[msg_pos - 1];
        per.lines.get(local)
    } else {
        cached.streaming_lines.get(global_idx - history_total)
    }
}

pub fn draw_messages(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let t = &app.ui.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(t.border_message))
        .style(Style::default().bg(t.bg_primary));

    // 空消息时显示欢迎界面
    if app.state.session.messages.is_empty() && !app.state.is_loading {
        let inner_width = area.width.saturating_sub(4);
        let welcome_lines = super::components::welcome_box(inner_width, t, app.ui.quote_idx);
        let empty = Paragraph::new(welcome_lines).block(block);
        f.render_widget(empty, area);
        return;
    }

    // 内部可用宽度（减去边框和左右各1的 padding）
    let inner_width = area.width.saturating_sub(4) as usize;
    // 消息内容最大宽度为可用宽度的 85%
    let bubble_max_width = (inner_width * 85 / 100).max(20);

    let msg_count = app.state.session.messages.len();
    let last_msg_len = app
        .state
        .session
        .messages
        .last()
        .map(|m| m.content.len())
        .unwrap_or(0);
    let streaming_len = if app.state.is_loading {
        // 复用 tui_loop 中已获取的快照长度，避免重复加锁
        // 如果 tui_loop 确定需要重绘（因为 delta 变化），此时 last_rendered_streaming_len 已经是上次值
        // 用一次轻量锁获取（此时 agent 线程可能已经写入更多）
        safe_lock(
            &app.state.streaming_content,
            "draw_messages::streaming_content",
        )
        .len()
    } else {
        0
    };
    let current_browse_index = if app.ui.mode == ChatMode::Browse {
        Some(app.ui.browse_msg_index)
    } else {
        None
    };
    let current_tool_confirm_idx = if app.ui.mode == ChatMode::ToolConfirm {
        Some(app.tool_executor.pending_tool_idx)
    } else {
        None
    };
    let cache_hit = if let Some(ref cache) = app.ui.msg_lines_cache {
        cache.msg_count == msg_count
            && cache.last_msg_len == last_msg_len
            && cache.streaming_len == streaming_len
            && cache.is_loading == app.state.is_loading
            && cache.bubble_max_width == bubble_max_width
            && cache.browse_index == current_browse_index
            && cache.tool_confirm_idx == current_tool_confirm_idx
    } else {
        false
    };

    if !cache_hit {
        let old_cache = app.ui.msg_lines_cache.take();
        let (
            new_msg_start_lines,
            new_per_msg,
            new_streaming_lines,
            new_stable_lines,
            new_stable_offset,
        ) = build_message_lines_incremental(app, inner_width, bubble_max_width, old_cache.as_ref());
        let total_line_count: usize =
            new_per_msg.iter().map(|p| p.lines.len()).sum::<usize>() + new_streaming_lines.len();
        let history_line_count: usize = new_per_msg.iter().map(|p| p.lines.len()).sum();
        app.ui.msg_lines_cache = Some(MsgLinesCache {
            msg_count,
            last_msg_len,
            streaming_len,
            is_loading: app.state.is_loading,
            bubble_max_width,
            browse_index: current_browse_index,
            tool_confirm_idx: current_tool_confirm_idx,
            total_line_count,
            history_line_count,
            msg_start_lines: new_msg_start_lines,
            per_msg_lines: new_per_msg,
            streaming_lines: new_streaming_lines,
            streaming_stable_lines: new_stable_lines,
            streaming_stable_offset: new_stable_offset,
            expand_tools: app.ui.expand_tools,
        });
    }

    let cached = match app.ui.msg_lines_cache.as_ref() {
        Some(c) => c,
        None => return,
    };
    let total_lines = cached.total_line_count as u16;

    f.render_widget(block, area);

    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    let visible_height = inner.height;
    let max_scroll = total_lines.saturating_sub(visible_height);

    if app.ui.mode != ChatMode::Browse {
        if app.ui.mode == ChatMode::ToolConfirm {
            if app.ui.auto_scroll
                || app.ui.scroll_offset == u16::MAX
                || app.ui.scroll_offset > max_scroll
            {
                app.ui.scroll_offset = max_scroll;
            }
        } else if app.ui.scroll_offset == u16::MAX {
            // scroll_offset == u16::MAX 是定位标记（由 send_message/StreamChunk 等设置），
            // 表示"应滚动到底部"，clamp 到 max_scroll 并保持 auto_scroll
            app.ui.scroll_offset = max_scroll;
            app.ui.auto_scroll = true;
        } else if app.ui.scroll_offset >= max_scroll {
            // 用户已位于底部（手动滚下 或 内容缩减使 offset 到达底部）
            // 恢复 auto_scroll：用户在底部时，新消息应自动跟踪
            app.ui.scroll_offset = max_scroll;
            app.ui.auto_scroll = true;
        }
    } else if let Some(msg_start) = cached
        .msg_start_lines
        .iter()
        .find(|(idx, _)| *idx == app.ui.browse_msg_index)
        .map(|(_, line)| *line as u16)
    {
        let msg_line_count = cached
            .per_msg_lines
            .get(app.ui.browse_msg_index)
            .map(|c| c.lines.len())
            .unwrap_or(1) as u16;
        let msg_max_scroll = msg_line_count.saturating_sub(visible_height);
        if app.ui.browse_scroll_offset > msg_max_scroll {
            app.ui.browse_scroll_offset = msg_max_scroll;
        }
        app.ui.scroll_offset = (msg_start + app.ui.browse_scroll_offset).min(max_scroll);
    }

    let bg_fill = Block::default().style(Style::default().bg(app.ui.theme.bg_primary));
    f.render_widget(bg_fill, inner);

    let start = app.ui.scroll_offset as usize;
    let end = (start + visible_height as usize).min(cached.total_line_count);
    let history_total = cached.history_line_count;
    let msg_area_bg = Style::default().bg(app.ui.theme.bg_primary);

    // 单 pass：渲染文字的同时收集图片标记 (display_row, height, url)
    let mut img_markers: Vec<(usize, u16, String)> = Vec::new();
    for (i, line_idx) in (start..end).enumerate() {
        let line = match get_line_at(cached, line_idx, history_total) {
            Some(l) => l,
            None => continue,
        };
        let y = inner.y + i as u16;
        let line_area = Rect::new(inner.x, y, inner.width, 1);

        // 检查是否有图片标记 span
        let img_info: Option<(u16, String)> = line.spans.iter().find_map(|span| {
            span.content.strip_prefix("\x00IMG:").and_then(|rest| {
                rest.find(':').map(|p| {
                    let height: u16 = rest[..p].parse().unwrap_or(20);
                    let url = rest[p + 1..].to_string();
                    (height, url)
                })
            })
        });

        if let Some((height, url)) = img_info {
            // 渲染无标记 span 的气泡行
            let visible_spans: Vec<Span> = line
                .spans
                .iter()
                .filter(|s| !s.content.starts_with("\x00IMG:"))
                .cloned()
                .collect();
            let p = Paragraph::new(Line::from(visible_spans)).style(msg_area_bg);
            f.render_widget(p, line_area);
            img_markers.push((i, height, url));
        } else {
            let p = Paragraph::new(line.clone()).style(msg_area_bg);
            f.render_widget(p, line_area);
        }
    }

    // === 图片渲染 pass（需在文字之后覆盖绘制）===
    let has_picker = safe_lock(&app.ui.image_cache, "draw_messages::image_cache_picker")
        .picker
        .is_some();
    let img_pad = 3u16; // 与气泡 pad_left_w 一致
    let img_render_w = (bubble_max_width as u16).saturating_sub(img_pad * 2);
    for (i, height, url) in img_markers {
        let line_idx = start + i;
        let y = inner.y + i as u16;
        let remaining_h = visible_height.saturating_sub(i as u16);
        let bubble_w = bubble_max_width as u16;

        // 计算实际可用的占位行数：从标记行往下数连续的空行/占位行
        let mut actual_h = 1u16; // 标记行本身占 1 行
        for next_offset in 1..height as usize {
            let next_idx = line_idx + next_offset;
            if next_idx >= cached.total_line_count {
                break;
            }
            let next_line = match get_line_at(cached, next_idx, history_total) {
                Some(l) => l,
                None => break,
            };
            // 占位行要么为空，要么只有气泡背景空格（可能含边框字符 │）
            let is_placeholder = next_line.spans.is_empty()
                || next_line
                    .spans
                    .iter()
                    .all(|s| s.content.replace('│', "").trim().is_empty());
            if is_placeholder {
                actual_h += 1;
            } else {
                break;
            }
        }
        let render_h = actual_h.min(height).min(remaining_h);

        // 如果可见高度不够容纳图片，跳过渲染（避免滚动时缩放）
        if remaining_h < render_h {
            continue;
        }

        // 图片区域在气泡内对齐：左 padding 3，宽度为气泡内容宽度
        let img_x = inner.x + img_pad;
        let img_area = Rect::new(img_x, y, img_render_w, render_h);

        if !has_picker {
            // 终端不支持图形协议，降级为文本链接
            let max_url_w = (bubble_w as usize).saturating_sub(12); // "  [Image: " + "]"
            let display_url = truncate_str(&url, max_url_w);
            let fallback = Paragraph::new(Line::from(Span::styled(
                format!("  [Image: {}]", display_url),
                Style::default()
                    .fg(Color::Cyan)
                    .bg(app.ui.theme.bubble_ai)
                    .add_modifier(Modifier::UNDERLINED),
            )));
            f.render_widget(fallback, Rect::new(inner.x, y, bubble_w, 1));
            continue;
        }

        let mut cache = safe_lock(&app.ui.image_cache, "draw_chat_ui::image_cache");
        match cache.images.get_mut(&url) {
            Some(ImageState::Ready(protocol)) => {
                let widget = StatefulImage::default().resize(Resize::Scale(None));
                f.render_stateful_widget(widget, img_area, protocol);
            }
            Some(ImageState::Failed(err)) => {
                let max_err_w = (bubble_w as usize).saturating_sub(24); // "  [Image load failed: " + "]"
                let display_err = truncate_str(err, max_err_w);
                let err_line = Paragraph::new(Line::from(Span::styled(
                    format!("  [Image load failed: {}]", display_err),
                    Style::default().fg(Color::Red).bg(app.ui.theme.bubble_ai),
                )));
                f.render_widget(err_line, Rect::new(inner.x, y, bubble_w, 1));
            }
            Some(ImageState::Loading) => {
                let max_url_w = (bubble_w as usize).saturating_sub(21); // "  Loading image: " + "..."
                let display_url = truncate_str(&url, max_url_w);
                let loading = Paragraph::new(Line::from(Span::styled(
                    format!("  Loading image: {}...", display_url),
                    Style::default()
                        .fg(Color::DarkGray)
                        .bg(app.ui.theme.bubble_ai),
                )));
                f.render_widget(loading, Rect::new(inner.x, y, bubble_w, 1));
            }
            Some(ImageState::Pending) | None => {
                let max_url_w = (bubble_w as usize).saturating_sub(21);
                let display_url = truncate_str(&url, max_url_w);
                let loading = Paragraph::new(Line::from(Span::styled(
                    format!("  Loading image: {}...", display_url),
                    Style::default()
                        .fg(Color::DarkGray)
                        .bg(app.ui.theme.bubble_ai),
                )));
                f.render_widget(loading, Rect::new(inner.x, y, bubble_w, 1));
                // 标记为加载中
                cache.images.insert(url.clone(), ImageState::Loading);
                // spawn 后台线程加载图片
                let cache_clone = std::sync::Arc::clone(&app.ui.image_cache);
                let url_owned = url.clone();
                std::thread::spawn(move || match load_image(&url_owned) {
                    Ok(dyn_img) => {
                        let mut c = safe_lock(&cache_clone, "image_load::cache_ready");
                        if let Some(ref picker) = c.picker {
                            let protocol: ratatui_image::protocol::StatefulProtocol =
                                picker.new_resize_protocol(dyn_img);
                            c.images.insert(url_owned, ImageState::Ready(protocol));
                        }
                    }
                    Err(e) => {
                        safe_lock(&cache_clone, "image_load::cache_failed")
                            .images
                            .insert(url_owned, ImageState::Failed(e));
                    }
                });
            }
        }
    }
}

/// 弹窗配置参数（样式 + 行为）
struct PopupConfig {
    title: String,
    selected: usize,
    max_visible: usize,
    title_color: ratatui::style::Color,
    border_color: ratatui::style::Color,
    bg_color: ratatui::style::Color,
    highlight_bg: ratatui::style::Color,
    highlight_fg: ratatui::style::Color,
}

/// 通用浮动弹窗列表渲染（输入区上方）
fn draw_popup_list(
    f: &mut ratatui::Frame,
    input_area: Rect,
    items: Vec<ListItem<'static>>,
    item_labels: &[String],
    cfg: &PopupConfig,
) {
    if items.is_empty() {
        return;
    }
    let item_count = items.len();
    let visible_count = item_count.min(cfg.max_visible);
    let popup_height = (visible_count as u16) + 2; // +2 for border
    let max_popup_width = (input_area.width as usize).saturating_sub(2).max(16);
    let popup_width = item_labels
        .iter()
        .map(|n| display_width(n))
        .max()
        .unwrap_or(20)
        .max(16)
        .min(max_popup_width) as u16
        + 2;

    let x = input_area.x + 1;
    let y = input_area.y.saturating_sub(popup_height);
    // 确保弹窗不超出 input_area 右边界，避免 ratatui buffer 越界 panic
    let avail_width = input_area.right().saturating_sub(x);
    let popup_width = popup_width.min(avail_width);
    if popup_width == 0 {
        return;
    }
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    let mut list_state = ListState::default();
    list_state.select(Some(cfg.selected.min(item_count.saturating_sub(1))));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(cfg.border_color))
                .title(Span::styled(
                    &cfg.title,
                    Style::default()
                        .fg(cfg.title_color)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(cfg.bg_color)),
        )
        .highlight_style(
            Style::default()
                .bg(cfg.highlight_bg)
                .fg(cfg.highlight_fg)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut list_state);
}

/// 绘制 @ 补全弹窗（输入区域上方浮动）
pub fn draw_at_popup(f: &mut ratatui::Frame, input_area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = get_filtered_all_items(app);
    if filtered.is_empty() {
        return;
    }

    let max_items = filtered.len().min(15);
    let selected = app
        .ui
        .at_popup_selected
        .min(filtered.len().saturating_sub(1));

    // 构建显示标签（与渲染内容等宽，用于 popup 宽度计算）
    let labels: Vec<String> = filtered
        .iter()
        .take(max_items)
        .map(|item| match item {
            AtPopupItem::Category(s) => format!("  {}", s),
            AtPopupItem::Skill(s) => format!("  [skill]   {}", s),
            AtPopupItem::Command(s) => format!("  [command] {}", s),
            AtPopupItem::File(s) => format!("  [file]    {}", s),
        })
        .collect();

    // 与 / 弹窗风格一致：pointer + name + desc
    let items: Vec<ListItem<'static>> = filtered
        .iter()
        .take(max_items)
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == selected;

            match item {
                AtPopupItem::Category(s) => {
                    // 分类分隔行：显示为 "skill:" 风格的暗色标签
                    ListItem::new(Line::from(vec![Span::styled(
                        format!("  {s}"),
                        Style::default().fg(t.text_dim),
                    )]))
                }
                AtPopupItem::Skill(s) => {
                    let pointer = if is_selected { "❯ " } else { "  " };
                    ListItem::new(Line::from(vec![
                        Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                        Span::styled(
                            "[skill]   ".to_string(),
                            Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(s.as_str().to_string(), Style::default().fg(t.text_white)),
                    ]))
                }
                AtPopupItem::Command(s) => {
                    let pointer = if is_selected { "❯ " } else { "  " };
                    ListItem::new(Line::from(vec![
                        Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                        Span::styled(
                            "[command] ".to_string(),
                            Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(s.as_str().to_string(), Style::default().fg(t.text_white)),
                    ]))
                }
                AtPopupItem::File(s) => {
                    let pointer = if is_selected { "❯ " } else { "  " };
                    let is_dir = s.ends_with('/');
                    let name_color = if is_dir { Color::Cyan } else { t.text_white };
                    ListItem::new(Line::from(vec![
                        Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                        Span::styled(
                            "[file]    ".to_string(),
                            Style::default()
                                .fg(t.label_user)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(s.as_str().to_string(), Style::default().fg(name_color)),
                    ]))
                }
            }
        })
        .collect();

    let title = if app.ui.at_popup_filter.is_empty() {
        " @ 补全 ".to_string()
    } else {
        format!(" @{} ", app.ui.at_popup_filter)
    };

    let cfg = PopupConfig {
        title,
        selected,
        max_visible: 8,
        title_color: t.md_h1,
        border_color: t.md_h1,
        bg_color: t.bg_primary,
        highlight_bg: t.md_h1,
        highlight_fg: t.bg_primary,
    };
    draw_popup_list(f, input_area, items, &labels, &cfg);
}

/// 绘制文件补全弹窗（输入区域上方浮动）
pub fn draw_file_popup(f: &mut ratatui::Frame, input_area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = get_filtered_files(app);
    if filtered.is_empty() {
        return;
    }
    let max_items = filtered.len().min(15);
    let selected = app
        .ui
        .file_popup_selected
        .min(filtered.len().saturating_sub(1));

    let labels: Vec<String> = filtered
        .iter()
        .take(max_items)
        .map(|n| format!("  {}", n))
        .collect();

    let items: Vec<ListItem<'static>> = filtered
        .iter()
        .take(max_items)
        .enumerate()
        .map(|(i, name)| {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯ " } else { "  " };
            let is_dir = name.ends_with('/');
            let name_color = if is_dir { Color::Cyan } else { t.label_user };
            ListItem::new(Line::from(vec![
                Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                Span::styled(
                    name.clone(),
                    Style::default().fg(name_color).add_modifier(Modifier::BOLD),
                ),
            ]))
        })
        .collect();

    let title = if app.ui.file_popup_filter.is_empty() {
        " Files ".to_string()
    } else {
        format!(" {} ", app.ui.file_popup_filter)
    };
    let cfg = PopupConfig {
        title,
        selected,
        max_visible: 8,
        title_color: t.md_h1,
        border_color: t.md_h1,
        bg_color: t.bg_primary,
        highlight_bg: t.md_h1,
        highlight_fg: t.bg_primary,
    };
    draw_popup_list(f, input_area, items, &labels, &cfg);
}

/// 绘制技能补全弹窗（输入区域上方浮动）
pub fn draw_skill_popup(f: &mut ratatui::Frame, input_area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = get_filtered_skill_names(app);
    if filtered.is_empty() {
        return;
    }
    let max_items = filtered.len().min(8);
    let selected = app
        .ui
        .skill_popup_selected
        .min(filtered.len().saturating_sub(1));

    let labels: Vec<String> = filtered
        .iter()
        .take(max_items)
        .map(|n| format!("  {}", n))
        .collect();

    let items: Vec<ListItem<'static>> = filtered
        .iter()
        .take(max_items)
        .enumerate()
        .map(|(i, name)| {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                Span::styled(
                    name.clone(),
                    Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
                ),
            ]))
        })
        .collect();

    let title = if app.ui.skill_popup_filter.is_empty() {
        " Skills ".to_string()
    } else {
        format!(" {} ", app.ui.skill_popup_filter)
    };
    let cfg = PopupConfig {
        title,
        selected,
        max_visible: 8,
        title_color: t.md_h1,
        border_color: t.md_h1,
        bg_color: t.bg_primary,
        highlight_bg: t.md_h1,
        highlight_fg: t.bg_primary,
    };
    draw_popup_list(f, input_area, items, &labels, &cfg);
}

pub fn draw_command_popup(f: &mut ratatui::Frame, input_area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = get_filtered_command_names(app);
    if filtered.is_empty() {
        return;
    }
    let max_items = filtered.len().min(8);
    let selected = app
        .ui
        .command_popup_selected
        .min(filtered.len().saturating_sub(1));

    // 构建带 description 的命令列表
    let commands: Vec<(String, String)> = app
        .state
        .loaded_commands
        .iter()
        .filter(|c| {
            !app.state
                .agent_config
                .disabled_commands
                .iter()
                .any(|d| d == &c.frontmatter.name)
        })
        .filter(|c| {
            let f = app.ui.command_popup_filter.to_lowercase();
            f.is_empty() || c.frontmatter.name.to_lowercase().contains(&f)
        })
        .take(max_items)
        .map(|c| {
            (
                c.frontmatter.name.clone(),
                c.frontmatter.description.clone(),
            )
        })
        .collect();

    let labels: Vec<String> = commands
        .iter()
        .map(|(name, desc)| format!("  {:<16}{}", name, desc))
        .collect();

    let items: Vec<ListItem<'static>> = commands
        .iter()
        .enumerate()
        .map(|(i, (name, desc))| {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                Span::styled(
                    format!("{:<16}", name),
                    Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc.clone(), Style::default().fg(t.text_dim)),
            ]))
        })
        .collect();

    let title = if app.ui.command_popup_filter.is_empty() {
        " Commands ".to_string()
    } else {
        format!(" {} ", app.ui.command_popup_filter)
    };
    let cfg = PopupConfig {
        title,
        selected,
        max_visible: 8,
        title_color: t.md_h1,
        border_color: t.md_h1,
        bg_color: t.bg_primary,
        highlight_bg: t.md_h1,
        highlight_fg: t.bg_primary,
    };
    draw_popup_list(f, input_area, items, &labels, &cfg);
}

/// 绘制 / 斜杠命令弹窗（输入区域上方浮动）
pub fn draw_slash_popup(f: &mut ratatui::Frame, input_area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = get_filtered_slash_commands(&app.ui.slash_popup_filter);
    if filtered.is_empty() {
        return;
    }

    // 构建显示项：命令 + 描述
    // labels 用于计算弹窗宽度，需与实际渲染内容一致
    // 动态计算标签列宽度，取最长 display_label + 2 格间距
    let label_width: usize = filtered
        .iter()
        .map(|cmd| display_width(&cmd.display_label()))
        .max()
        .unwrap_or(10)
        + 2;
    let labels: Vec<String> = filtered
        .iter()
        .map(|cmd| {
            format!(
                "❯ {:<width$} {}",
                cmd.display_label(),
                cmd.description(),
                width = label_width
            )
        })
        .collect();

    let selected = app
        .ui
        .slash_popup_selected
        .min(filtered.len().saturating_sub(1));
    let items: Vec<ListItem<'static>> = filtered
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯ " } else { "  " };
            let padded_label = {
                let label = cmd.display_label();
                let label_dw = display_width(&label);
                let padding = label_width.saturating_sub(label_dw);
                format!("{}{}", label, " ".repeat(padding))
            };
            ListItem::new(Line::from(vec![
                Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                Span::styled(
                    padded_label,
                    Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
                ),
                Span::styled(cmd.description(), Style::default().fg(t.text_dim)),
            ]))
        })
        .collect();

    let title = if app.ui.slash_popup_filter.is_empty() {
        " / 命令 ".to_string()
    } else {
        format!(" /{} ", app.ui.slash_popup_filter)
    };

    draw_popup_list(
        f,
        input_area,
        items,
        &labels,
        &PopupConfig {
            title,
            selected,
            max_visible: 8,
            title_color: t.md_h1,
            border_color: t.md_h1,
            bg_color: t.bg_primary,
            highlight_bg: t.md_h1,
            highlight_fg: t.bg_primary,
        },
    );
}

/// 截断字符串到指定显示宽度，超长时加 "..."
fn truncate_str(s: &str, max_w: usize) -> String {
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
