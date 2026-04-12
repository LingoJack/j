use super::super::app::{ChatApp, ChatMode, MsgLinesCache, ToolExecStatus};
use super::super::compact::estimate_tokens;
use super::super::handler::{
    AtPopupItem, get_filtered_all_items, get_filtered_command_names, get_filtered_files,
    get_filtered_skill_names, get_filtered_slash_commands,
};
use super::super::markdown::image_cache::ImageState;
use super::super::markdown::image_loader::load_image;
use super::super::render_cache::build_message_lines_incremental;
use super::super::storage::agent_config_path;
use super::archive::{draw_archive_confirm, draw_archive_list};
use super::config::draw_config_screen;
use crate::util::safe_lock;
use crate::util::text::{char_width, display_width, wrap_text};
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 标题栏（顶部分割线 + 内容行 + 底部留白）
            Constraint::Min(5),    // 消息区
            Constraint::Length(5), // 输入区
            Constraint::Length(1), // 操作提示栏（始终可见）
        ])
        .split(size);

    // ========== 标题栏 ==========
    draw_title_bar(f, chunks[0], app);

    // ========== 消息区 ==========
    match app.ui.mode {
        ChatMode::Help => draw_help(f, chunks[1], app),
        ChatMode::SelectModel => draw_model_selector(f, chunks[1], app),
        ChatMode::SelectTheme => draw_theme_selector(f, chunks[1], app),
        ChatMode::Config => draw_config_screen(f, chunks[1], app),
        ChatMode::ArchiveConfirm => draw_archive_confirm(f, chunks[1], app),
        ChatMode::ArchiveList => draw_archive_list(f, chunks[1], app),
        _ => draw_messages(f, chunks[1], app),
    }

    // ========== 输入区 ==========
    draw_input(f, chunks[2], app);

    // ========== 底部操作提示栏（始终可见）==========
    draw_hint_bar(f, chunks[3], app);

    // ========== Toast 弹窗覆盖层（右上角）==========
    draw_toast(f, size, app);

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
pub fn draw_title_bar(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
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
        // 优先显示正在执行中的工具，其次显示等待确认的工具
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
        Span::styled(
            loading,
            Style::default()
                .fg(t.title_loading)
                .add_modifier(Modifier::BOLD),
        ),
    ];

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

    // 第三行：底部留白（与对话记录保持间距）
    let empty_line = Paragraph::new(Line::default()).style(Style::default().bg(t.bg_primary));
    f.render_widget(empty_line, Rect::new(area.x, area.y + 2, area.width, 1));
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
        let welcome_lines = super::components::welcome_box(inner_width, t);
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
            if app.ui.auto_scroll || app.ui.scroll_offset == u16::MAX {
                app.ui.scroll_offset = max_scroll;
                app.ui.auto_scroll = true;
            } else if app.ui.scroll_offset > max_scroll {
                app.ui.scroll_offset = max_scroll;
            }
        } else if app.ui.scroll_offset == u16::MAX || app.ui.scroll_offset > max_scroll {
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

pub fn draw_input(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let t = &app.ui.theme;

    // 提示符逻辑
    let (prompt, prompt_style) = if app.state.is_loading {
        (" + ", Style::default().fg(t.input_prompt_loading))
    } else {
        (" > ", Style::default().fg(t.input_prompt))
    };
    let prompt_width: usize = 3; // " > " 或 " ~> " 都是 3 个显示宽度

    let usable_width = area.width.saturating_sub(2) as usize;
    let input_text = app.ui.input_text();
    let chars: Vec<char> = input_text.chars().collect();

    // 安全检查：cursor_pos 不能超过字符数
    let cursor_pos = app.ui.cursor_char_idx().min(chars.len());

    let before_all: String = chars[..cursor_pos].iter().collect();
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
    let cursor_in_visible = cursor_pos
        .saturating_sub(scroll_offset_chars)
        .min(visible_chars.len());

    let before: String = visible_chars[..cursor_in_visible].iter().collect();
    // 当光标落在 '\n' 上时（如按左键移到行尾），用空格代替 '\n' 作为显示光标字符，
    // 同时把原来的 '\n' 保留到 after 里，避免 wrap_text 将光标空格渲染到下一行。
    let (cursor_ch, after): (String, String) = if cursor_in_visible < visible_chars.len() {
        let ch = visible_chars[cursor_in_visible];
        if ch == '\n' {
            // 光标在换行符上：显示为行尾空格，'\n' 归入 after 保持换行效果
            (
                " ".to_string(),
                visible_chars[cursor_in_visible..].iter().collect(),
            )
        } else {
            (
                ch.to_string(),
                visible_chars[cursor_in_visible + 1..].iter().collect(),
            )
        }
    } else {
        (" ".to_string(), String::new())
    };

    // 占位符逻辑：输入为空时显示，根据加载状态显示不同提示
    let is_empty = chars.is_empty();
    let placeholder = if app.state.is_loading {
        "补充消息，Enter 发送，Esc 打断，Shift/Alt+Enter 换行"
    } else {
        "输入消息，Enter 发送，Esc 退出，Shift/Alt+Enter 换行"
    };

    let full_visible = if is_empty {
        placeholder.to_string()
    } else {
        format!("{}{}{}", before, cursor_ch, after)
    };
    let inner_height = area.height.saturating_sub(2) as usize;
    let wrapped_lines = wrap_text(&full_visible, usable_width);

    // before 中包含 '\n'，但 wrap_text 会剥掉 '\n'，因此渲染循环里的 char_offset
    // 不计 '\n'。需要减去 before 中的 '\n' 数，才能正确定位光标在 wrapped lines 里的位置。
    let newlines_in_before = before.chars().filter(|&c| c == '\n').count();
    let before_len = before.chars().count() - newlines_in_before;
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

    let line_scroll = if inner_height == 0
        || wrapped_lines.len() <= inner_height
        || cursor_line_idx < inner_height
    {
        0
    } else {
        cursor_line_idx.saturating_sub(inner_height - 1)
    };

    // 计算 @mention 高亮范围（缓存：仅 input 变化时重算）
    // mention_ranges 需基于去除 '\n' 的文本计算，与渲染循环中不含 '\n' 的 global_idx 保持一致。
    let input_text_no_nl: String = input_text.chars().filter(|&c| c != '\n').collect();
    let mention_ranges =
        if let Some((ref cached_input, ref cached_ranges)) = app.ui.cached_mention_ranges {
            if cached_input == &input_text {
                cached_ranges.clone()
            } else {
                let ranges = find_at_mention_ranges(&input_text_no_nl);
                app.ui.cached_mention_ranges = Some((input_text.to_string(), ranges.clone()));
                ranges
            }
        } else {
            let ranges = find_at_mention_ranges(&input_text_no_nl);
            app.ui.cached_mention_ranges = Some((input_text.to_string(), ranges.clone()));
            ranges
        };
    // 转换为相对于 scroll_offset_chars 的偏移
    let mention_style = Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD);

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

        // 第一行添加提示符
        if _line_idx == 0 && line_scroll == 0 {
            spans.push(Span::styled(prompt, prompt_style));
        }

        let line_chars: Vec<char> = wl.chars().collect();

        // 空输入时显示占位符
        if is_empty && _line_idx == 0 {
            spans.push(Span::styled(placeholder, Style::default().fg(t.text_dim)));
            display_lines.push(Line::from(spans));
            continue;
        }

        let mut seg_start = 0;
        for (ci, &ch) in line_chars.iter().enumerate() {
            let global_idx = scroll_offset_chars + char_offset + ci;
            let visible_idx = char_offset + ci;
            let is_cursor = visible_idx >= before_len && visible_idx < before_len + cursor_len;
            let is_mention = mention_ranges
                .iter()
                .any(|&(s, e)| global_idx >= s && global_idx < e);

            if is_cursor || is_mention {
                // flush normal or mention segment before this char
                if ci > seg_start {
                    let seg: String = line_chars[seg_start..ci].iter().collect();
                    // check if previous segment was in mention range
                    let prev_global = scroll_offset_chars + char_offset + seg_start;
                    let prev_is_mention = mention_ranges
                        .iter()
                        .any(|&(s, e)| prev_global >= s && prev_global < e);
                    let seg_style = if prev_is_mention {
                        mention_style
                    } else {
                        Style::default().fg(t.text_white)
                    };
                    spans.push(Span::styled(seg, seg_style));
                }
                if is_cursor {
                    spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().fg(t.cursor_fg).bg(t.cursor_bg),
                    ));
                } else {
                    spans.push(Span::styled(ch.to_string(), mention_style));
                }
                seg_start = ci + 1;
            } else if ci > seg_start {
                // check if we just transitioned from mention to non-mention
                let prev_global = scroll_offset_chars + char_offset + (ci - 1);
                let prev_is_mention = mention_ranges
                    .iter()
                    .any(|&(s, e)| prev_global >= s && prev_global < e);
                let curr_is_mention = is_mention;
                if prev_is_mention != curr_is_mention {
                    let seg: String = line_chars[seg_start..ci].iter().collect();
                    let seg_style = if prev_is_mention {
                        mention_style
                    } else {
                        Style::default().fg(t.text_white)
                    };
                    spans.push(Span::styled(seg, seg_style));
                    seg_start = ci;
                }
            }
        }
        if seg_start < line_chars.len() {
            let seg: String = line_chars[seg_start..].iter().collect();
            let seg_global = scroll_offset_chars + char_offset + seg_start;
            let seg_is_mention = mention_ranges
                .iter()
                .any(|&(s, e)| seg_global >= s && seg_global < e);
            let seg_style = if seg_is_mention {
                mention_style
            } else {
                Style::default().fg(t.text_white)
            };
            spans.push(Span::styled(seg, seg_style));
        }

        char_offset += line_chars.len();
        display_lines.push(Line::from(spans));
    }

    if display_lines.is_empty() {
        display_lines.push(Line::from(vec![Span::styled(
            " ",
            Style::default().fg(t.cursor_fg).bg(t.cursor_bg),
        )]));
    }

    let input_widget = Paragraph::new(display_lines).style(Style::default().bg(t.bg_input));

    f.render_widget(input_widget, area);

    // 光标位置计算（考虑提示符偏移）
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
                col = wl.chars().take(pos_in_line).map(char_width).sum();
                break;
            }
            char_count += line_len;
        }
        col as u16
    };

    let cursor_row_in_display = (cursor_line_idx - line_scroll) as u16;
    // 提示符 " > " 只在第一可见行（cursor_row_in_display == 0）渲染，
    // 其余行不加 prompt_width，否则光标 X 会偏右 3 列。
    let cursor_x = if cursor_row_in_display == 0 {
        area.x + prompt_width as u16 + cursor_col_in_line
    } else {
        area.x + cursor_col_in_line
    };
    let cursor_y = area.y + cursor_row_in_display;

    if cursor_x < area.x + area.width && cursor_y < area.y + area.height {
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

/// 绘制底部操作提示栏（始终可见）
pub fn draw_hint_bar(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let hints = match app.ui.mode {
        ChatMode::Chat if app.state.is_loading => vec![], // 加载中时无提示，placeholder 会显示
        ChatMode::Chat => vec![
            ("@", "引用"),
            ("/", "命令"),
            ("Ctrl+O", "工具详情"),
            ("?/F1", "帮助"),
            // Esc 是基础操作，不显示在 hint bar
        ],
        ChatMode::SelectModel => vec![("↑↓/jk", "移动"), ("Enter", "确认"), ("Esc", "取消")],
        ChatMode::Browse => vec![("↑↓", "选择消息"), ("y/Enter", "复制"), ("Esc", "返回")],
        ChatMode::Help => vec![("任意键", "返回")],
        ChatMode::Config => {
            use crate::command::chat::app::ConfigTab;
            match app.ui.config_tab {
                ConfigTab::Model => vec![
                    ("←→", "切换标签"),
                    ("↑↓", "切换字段"),
                    ("Enter", "编辑"),
                    ("Tab", "切换Provider"),
                    ("a", "新增"),
                    ("d", "删除"),
                    ("s", "设为活跃"),
                    ("Esc", "保存返回"),
                ],
                ConfigTab::Global => vec![
                    ("←→", "切换标签"),
                    ("↑↓", "切换字段"),
                    ("Enter", "编辑/切换"),
                    ("Esc", "保存返回"),
                ],
                ConfigTab::Tools => vec![
                    ("←→", "切换标签"),
                    ("↑↓", "选择"),
                    ("Enter/空格", "切换"),
                    ("t", "总开关"),
                    ("a", "全部启用"),
                    ("d", "全部禁用"),
                    ("Esc", "保存返回"),
                ],
                ConfigTab::Skills => vec![
                    ("←→", "切换标签"),
                    ("↑↓", "选择"),
                    ("Enter/空格", "切换"),
                    ("a", "全部启用"),
                    ("d", "全部禁用"),
                    ("Esc", "保存返回"),
                ],
                ConfigTab::Hooks | ConfigTab::Commands => {
                    vec![("←→", "切换标签"), ("Esc", "保存返回")]
                }
                ConfigTab::Archive => {
                    if app.ui.restore_confirm_needed {
                        vec![("y/Enter", "确认还原"), ("Esc", "取消")]
                    } else {
                        vec![
                            ("←→", "切换标签"),
                            ("↑↓", "选择"),
                            ("Enter", "还原"),
                            ("d", "删除"),
                            ("Esc", "保存返回"),
                        ]
                    }
                }
                ConfigTab::Session => {
                    if app.ui.session_restore_confirm {
                        vec![("y/Enter", "确认恢复"), ("Esc", "取消")]
                    } else {
                        vec![
                            ("←→", "切换标签"),
                            ("↑↓", "选择"),
                            ("Enter", "恢复"),
                            ("d", "删除"),
                            ("n", "新建"),
                            ("Esc", "保存返回"),
                        ]
                    }
                }
            }
        }
        ChatMode::ArchiveConfirm => {
            if app.ui.archive_editing_name {
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
            if app.ui.restore_confirm_needed {
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
        ChatMode::ToolConfirm => vec![("↑↓", "选择"), ("Enter", "确认"), ("Esc", "拒绝")],
        ChatMode::SelectTheme => vec![("↑↓/jk", "选择"), ("Enter", "确认"), ("Esc", "返回")],
    };

    // 按终端宽度自适应：依次累加 hint，直到放不下为止
    let avail_width = area.width as usize;
    let sep_w = display_width("  │  ");
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(" ", Style::default()));
    let mut used = 1usize;

    for (i, (key, desc)) in hints.iter().enumerate() {
        let item_w = display_width(&format!(" {} ", key)) + display_width(&format!(" {}", desc));
        let need_w = if i == 0 { item_w } else { sep_w + item_w };
        if used + need_w > avail_width {
            break;
        }
        if i > 0 {
            spans.push(Span::styled("  │  ", Style::default().fg(t.hint_separator)));
        }
        spans.extend(super::components::hint_spans(key, desc, t));
        used += need_w;
    }

    let hint_bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(t.bg_primary));
    f.render_widget(hint_bar, area);
}

/// 绘制 Toast 弹窗（右上角浮层）
pub fn draw_toast(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    if let Some((ref msg, is_error, _)) = app.ui.toast {
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
                ("✖️", t.toast_error_border, t.toast_error_text)
            } else {
                ("☑️", t.toast_success_border, t.toast_success_text)
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
    let t = &app.ui.theme;
    let items: Vec<ListItem> = app
        .state
        .agent_config
        .providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let is_active = i == app.state.agent_config.active_index;
            let marker = if is_active {
                format!("{} ", super::components::TOGGLE_ON)
            } else {
                format!("{} ", super::components::TOGGLE_OFF)
            };
            let is_selected = i == app.ui.model_list_state.selected().unwrap_or(0);
            let style = if is_selected {
                Style::default()
                    .fg(t.text_white)
                    .add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default()
                    .fg(t.model_sel_active)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.model_sel_inactive)
            };
            let pointer = if is_selected {
                super::components::POINTER_SELECTED
            } else {
                super::components::POINTER_EMPTY
            };
            let detail = format!("{}{}{}  ({})", pointer, marker, p.name, p.model);
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
                    " \u{1f504} \u{9009}\u{62e9}\u{6a21}\u{578b} ",
                    Style::default()
                        .fg(t.config_label_selected)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(t.bg_title)),
        )
        .highlight_style(Style::default())
        .highlight_symbol("");

    f.render_stateful_widget(list, area, &mut app.ui.model_list_state);
}

/// 绘制主题选择界面
pub fn draw_theme_selector(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    use super::super::theme::ThemeName;

    let t = &app.ui.theme;
    let all = ThemeName::all();
    let items: Vec<ListItem> = all
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let is_active = *name == app.state.agent_config.theme;
            let marker = if is_active {
                format!("{} ", super::components::TOGGLE_ON)
            } else {
                format!("{} ", super::components::TOGGLE_OFF)
            };
            let is_selected = i == app.ui.theme_list_state.selected().unwrap_or(0);
            let style = if is_selected {
                Style::default()
                    .fg(t.text_white)
                    .add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default()
                    .fg(t.model_sel_active)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.model_sel_inactive)
            };
            let pointer = if is_selected {
                super::components::POINTER_SELECTED
            } else {
                super::components::POINTER_EMPTY
            };
            let detail = format!("{}{}{}", pointer, marker, name.display_name());
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
                    " 🎨 选择主题 ",
                    Style::default()
                        .fg(t.config_label_selected)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(t.bg_title)),
        )
        .highlight_style(Style::default())
        .highlight_symbol("");

    f.render_stateful_widget(list, area, &mut app.ui.theme_list_state);
}

/// 绘制帮助界面
pub fn draw_help(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let sep = super::components::separator_line(area.width, t);

    let shortcuts: &[(&str, &str)] = &[
        ("Enter", "发送消息"),
        ("Shift/Alt+Enter", "输入框内换行"),
        ("↑ / ↓", "滚动对话记录"),
        ("← / →", "移动输入光标"),
        ("Home / End", "光标跳到行首/行尾"),
        (
            "/",
            "斜杠命令（copy/log/browse/config/model/archive/theme）",
        ),
        ("@", "引用（skill/file/command）"),
        ("Ctrl+O", "展开/折叠工具详情"),
        ("Esc / Ctrl+C", "退出对话"),
        ("? / F1", "显示 / 关闭此帮助"),
    ];

    let mut help_lines = vec![
        Line::from(""),
        super::components::section_header("📖", "快捷键帮助", t),
        Line::from(""),
        sep.clone(),
        Line::from(""),
    ];
    for (key, desc) in shortcuts {
        help_lines.push(super::components::help_key_row(key, desc, 15, t));
    }
    help_lines.push(Line::from(""));
    help_lines.push(sep);
    help_lines.push(Line::from(""));
    help_lines.push(super::components::section_header("📁", "配置文件:", t));
    help_lines.push(Line::from(Span::styled(
        format!("     {}", agent_config_path().display()),
        Style::default().fg(t.help_path),
    )));

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

/// 通用浮动弹窗列表渲染（输入区上方）
#[allow(clippy::too_many_arguments)]
fn draw_popup_list(
    f: &mut ratatui::Frame,
    input_area: Rect,
    items: Vec<ListItem<'static>>,
    item_labels: &[String],
    title: String,
    title_color: ratatui::style::Color,
    border_color: ratatui::style::Color,
    bg_color: ratatui::style::Color,
    highlight_bg: ratatui::style::Color,
    highlight_fg: ratatui::style::Color,
    selected: usize,
) {
    if items.is_empty() {
        return;
    }
    let item_count = items.len();
    let popup_height = (item_count as u16) + 2;
    let popup_width = item_labels
        .iter()
        .map(|n| display_width(n))
        .max()
        .unwrap_or(20)
        .max(16)
        .min(input_area.width.saturating_sub(4) as usize) as u16
        + 2;

    let x = input_area.x + 1;
    let y = input_area.y.saturating_sub(popup_height);
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    let mut list_state = ListState::default();
    list_state.select(Some(selected.min(item_count.saturating_sub(1))));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(title_color)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(bg_color)),
        )
        .highlight_style(
            Style::default()
                .bg(highlight_bg)
                .fg(highlight_fg)
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

    // 构建显示标签（用于 popup 计算宽度等）
    let labels: Vec<String> = filtered
        .iter()
        .take(max_items)
        .map(|item| match item {
            AtPopupItem::Category(s) => format!("[cat] {}", s),
            AtPopupItem::Skill(s) => format!("[skill] {}", s),
            AtPopupItem::Command(s) => format!("[cmd] {}", s),
            AtPopupItem::File(s) => format!("[file] {}", s),
        })
        .collect();

    // 三段式渲染：pointer + [type] 名称 + 描述（与 / 弹窗风格一致）
    let items: Vec<ListItem<'static>> = filtered
        .iter()
        .take(max_items)
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯ " } else { "  " };

            let (tag, tag_color, name, desc) = match item {
                AtPopupItem::Category(s) => ("cat", t.text_dim, "", s.as_str()),
                AtPopupItem::Skill(s) => ("skill", t.label_ai, s.as_str(), "技能"),
                AtPopupItem::Command(s) => ("cmd", t.text_system, s.as_str(), "命令"),
                AtPopupItem::File(s) => ("file", t.label_user, s.as_str(), "文件"),
            };

            let name_part = if name.is_empty() {
                // Category: 显示分类标签本身作为主要名称
                let cat_name = match item {
                    AtPopupItem::Category(s) => s.as_str(),
                    _ => "",
                };
                format!("{:<12}", cat_name.trim_end_matches(':'))
            } else {
                format!("{:<12}", name)
            };

            ListItem::new(Line::from(vec![
                Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                Span::styled(
                    format!("[{:<5}]", tag),
                    Style::default().fg(tag_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    name_part,
                    Style::default()
                        .fg(t.text_white)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc.to_string(), Style::default().fg(t.text_dim)),
            ]))
        })
        .collect();

    let title = if app.ui.at_popup_filter.is_empty() {
        " @ 补全 ".to_string()
    } else {
        format!(" @{} ", app.ui.at_popup_filter)
    };

    draw_popup_list(
        f,
        input_area,
        items,
        &labels,
        title,
        t.md_h1,
        t.md_h1,
        t.bg_primary,
        t.md_h1,
        t.bg_primary,
        selected,
    );
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

    let labels: Vec<String> = filtered.iter().take(max_items).cloned().collect();

    let items: Vec<ListItem<'static>> = filtered
        .iter()
        .take(max_items)
        .enumerate()
        .map(|(i, name)| {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯ " } else { "  " };
            let is_dir = name.ends_with('/');
            let name_color = if is_dir { Color::Cyan } else { t.text_white };
            let desc = if is_dir { "目录" } else { "文件" };
            ListItem::new(Line::from(vec![
                Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                Span::styled(
                    format!("{:<24}", name),
                    Style::default().fg(name_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc.to_string(), Style::default().fg(t.text_dim)),
            ]))
        })
        .collect();

    let title = if app.ui.file_popup_filter.is_empty() {
        " Files ".to_string()
    } else {
        format!(" {} ", app.ui.file_popup_filter)
    };
    draw_popup_list(
        f,
        input_area,
        items,
        &labels,
        title,
        t.md_h1,
        t.md_h1,
        t.bg_primary,
        t.md_h1,
        t.bg_primary,
        selected,
    );
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

    let labels: Vec<String> = filtered.iter().take(max_items).map(|n| n.clone()).collect();

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
                    format!("{:<16}", name),
                    Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
                ),
                Span::styled("skill".to_string(), Style::default().fg(t.text_dim)),
            ]))
        })
        .collect();

    let title = if app.ui.skill_popup_filter.is_empty() {
        " Skills ".to_string()
    } else {
        format!(" {} ", app.ui.skill_popup_filter)
    };
    draw_popup_list(
        f,
        input_area,
        items,
        &labels,
        title,
        t.md_h1,
        t.md_h1,
        t.bg_primary,
        t.md_h1,
        t.bg_primary,
        selected,
    );
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
        .map(|(name, desc)| format!("  {}  - {}", name, desc))
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
                    format!("{:<12}", name),
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
    draw_popup_list(
        f,
        input_area,
        items,
        &labels,
        title,
        t.md_h1,
        t.md_h1,
        t.bg_primary,
        t.md_h1,
        t.bg_primary,
        selected,
    );
}

/// 绘制 / 斜杠命令弹窗（输入区域上方浮动）
pub fn draw_slash_popup(f: &mut ratatui::Frame, input_area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = get_filtered_slash_commands(&app.ui.slash_popup_filter);
    if filtered.is_empty() {
        return;
    }

    // 构建显示项：命令 + 描述
    let max_items = filtered.len().min(8);
    let labels: Vec<String> = filtered
        .iter()
        .take(max_items)
        .map(|cmd| format!("{} - {}", cmd.display_label(), cmd.description()))
        .collect();

    let selected = app
        .ui
        .slash_popup_selected
        .min(filtered.len().saturating_sub(1));
    let items: Vec<ListItem<'static>> = filtered
        .iter()
        .take(max_items)
        .enumerate()
        .map(|(i, cmd)| {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯ " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(pointer.to_string(), Style::default().fg(t.text_normal)),
                Span::styled(
                    format!("{:<10}", cmd.display_label()),
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
        title,
        t.md_h1,
        t.md_h1,
        t.bg_primary,
        t.md_h1,
        t.bg_primary,
        selected,
    );
}

/// 查找输入文本中所有 @mention 的字符范围 (start_char_idx, end_char_idx)
fn find_at_mention_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '@' {
            let valid_start = i == 0 || chars[i - 1].is_whitespace();
            if valid_start {
                let rest: String = chars[i + 1..].iter().collect();
                // 检查 @skill:name 模式
                if rest.starts_with("skill:") {
                    let mut end = i + 1 + 6; // @skill: 的末尾
                    while end < len && !chars[end].is_whitespace() {
                        end += 1;
                    }
                    ranges.push((i, end));
                    i = end;
                    continue;
                }
                // 检查 @command:name 模式
                if rest.starts_with("command:") {
                    let mut end = i + 1 + 8; // @command: 的末尾
                    while end < len && !chars[end].is_whitespace() {
                        end += 1;
                    }
                    ranges.push((i, end));
                    i = end;
                    continue;
                }
                // 检查 @file:xxx 模式
                if rest.starts_with("file:") {
                    // 找到 @file: 之后直到空白字符为止
                    let mut end = i + 1 + 5; // @file: 的末尾
                    while end < len && !chars[end].is_whitespace() {
                        end += 1;
                    }
                    ranges.push((i, end));
                    i = end;
                    continue;
                }
            }
        }
        i += 1;
    }

    ranges
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
