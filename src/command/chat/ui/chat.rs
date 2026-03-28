use super::super::app::{ChatApp, ChatMode, MsgLinesCache, ToolExecStatus};
use super::super::handler::{get_filtered_files, get_filtered_skills};
use super::super::markdown::image_cache::ImageState;
use super::super::markdown::image_loader::load_image;
use super::super::render_cache::build_message_lines_incremental;
use super::super::storage::agent_config_path;
use super::archive::{draw_archive_confirm, draw_archive_list};
use super::config::{draw_config_screen, draw_skill_toggle, draw_tool_toggle};
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
            Constraint::Length(3), // 标题栏
            Constraint::Min(5),    // 消息区
            Constraint::Length(5), // 输入区
            Constraint::Length(1), // 操作提示栏（始终可见）
        ])
        .split(size);

    // ========== 标题栏 ==========
    draw_title_bar(f, chunks[0], app);

    // ========== 消息区 ==========
    if app.ui.mode == ChatMode::Help {
        draw_help(f, chunks[1], app);
    } else if app.ui.mode == ChatMode::SelectModel {
        draw_model_selector(f, chunks[1], app);
    } else if app.ui.mode == ChatMode::Config {
        draw_config_screen(f, chunks[1], app);
    } else if app.ui.mode == ChatMode::ArchiveConfirm {
        draw_archive_confirm(f, chunks[1], app);
    } else if app.ui.mode == ChatMode::ArchiveList {
        draw_archive_list(f, chunks[1], app);
    } else if app.ui.mode == ChatMode::ToolToggle {
        draw_tool_toggle(f, chunks[1], app);
    } else if app.ui.mode == ChatMode::SkillToggle {
        draw_skill_toggle(f, chunks[1], app);
    } else {
        draw_messages(f, chunks[1], app);
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
}

/// 绘制标题栏
pub fn draw_title_bar(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let model_name = app.active_model_name();
    let msg_count = app.state.session.messages.len();
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

    let title_spans = vec![
        Span::styled(" 🦞 ", Style::default().fg(t.title_icon)),
        Span::styled(
            " Sprite",
            Style::default()
                .fg(t.text_white)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(t.title_separator)),
        Span::styled("💫  ", Style::default()),
        Span::styled(
            model_name,
            Style::default()
                .fg(t.title_model)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(t.title_separator)),
        Span::styled(
            format!("📬  {} 条消息", msg_count),
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
        .title(Span::styled(
            " 对话记录 ",
            Style::default().fg(t.text_dim).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(ratatui::layout::Alignment::Left)
        .style(Style::default().bg(t.bg_primary));

    // 空消息时显示欢迎界面
    if app.state.session.messages.is_empty() && !app.state.is_loading {
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
                Span::styled("       │", Style::default().fg(t.welcome_border)),
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
        app.ui.msg_lines_cache = Some(MsgLinesCache {
            msg_count,
            last_msg_len,
            streaming_len,
            is_loading: app.state.is_loading,
            bubble_max_width,
            browse_index: current_browse_index,
            tool_confirm_idx: current_tool_confirm_idx,
            total_line_count,
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
    let history_total: usize = cached.per_msg_lines.iter().map(|p| p.lines.len()).sum();
    let msg_area_bg = Style::default().bg(app.ui.theme.bg_primary);
    for (i, line_idx) in (start..end).enumerate() {
        let line = match get_line_at(cached, line_idx, history_total) {
            Some(l) => l,
            None => continue,
        };
        let y = inner.y + i as u16;
        let line_area = Rect::new(inner.x, y, inner.width, 1);
        // 图片标记行：只渲染气泡填充 span，跳过末尾的标记 span
        let has_marker = line.spans.iter().any(|s| s.content.starts_with("\x00IMG:"));
        if has_marker {
            let visible_spans: Vec<Span> = line
                .spans
                .iter()
                .filter(|s| !s.content.starts_with("\x00IMG:"))
                .cloned()
                .collect();
            let p = Paragraph::new(Line::from(visible_spans)).style(msg_area_bg);
            f.render_widget(p, line_area);
        } else {
            let p = Paragraph::new(line.clone()).style(msg_area_bg);
            f.render_widget(p, line_area);
        }
    }

    // === 图片渲染 pass ===
    let has_picker = safe_lock(&app.ui.image_cache, "draw_messages::image_cache_picker")
        .picker
        .is_some();
    // 图片渲染宽度与气泡内容区对齐
    let img_pad = 3u16; // 与气泡 pad_left_w 一致
    let img_render_w = (bubble_max_width as u16).saturating_sub(img_pad * 2);
    for (i, line_idx) in (start..end).enumerate() {
        let line = match get_line_at(cached, line_idx, history_total) {
            Some(l) => l,
            None => continue,
        };
        // 在所有 spans 中查找 \x00IMG: 标记（气泡包装会添加 padding spans）
        let img_marker: Option<(u16, String)> = line.spans.iter().find_map(|span| {
            let content = span.content.as_ref();
            content.strip_prefix("\x00IMG:").and_then(|rest| {
                rest.find(':').map(|colon_pos| {
                    let height: u16 = rest[..colon_pos].parse().unwrap_or(20);
                    let url = rest[colon_pos + 1..].to_string();
                    (height, url)
                })
            })
        });
        if let Some((height, url)) = img_marker {
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
}

pub fn draw_input(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let usable_width = area.width.saturating_sub(2 + 4) as usize;

    let chars: Vec<char> = app.ui.input.chars().collect();

    // 安全检查：cursor_pos 不能超过字符数
    let cursor_pos = app.ui.cursor_pos.min(chars.len());

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

    let prompt_style = if app.state.is_loading {
        Style::default().fg(t.input_prompt_loading)
    } else {
        Style::default().fg(t.input_prompt)
    };
    let prompt_text = if app.state.is_loading { " .. " } else { " >  " };

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

    let line_scroll = if wrapped_lines.len() <= inner_height || cursor_line_idx < inner_height {
        0
    } else {
        cursor_line_idx.saturating_sub(inner_height - 1)
    };

    // 计算 @mention 高亮范围（基于全局 char index, 相对于 input 起始）
    let skill_names: Vec<String> = app
        .state
        .loaded_skills
        .iter()
        .map(|s| s.frontmatter.name.clone())
        .collect();
    let mention_ranges = find_at_mention_ranges(&app.ui.input, &skill_names);
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
        if _line_idx == 0 && line_scroll == 0 {
            spans.push(Span::styled(prompt_text, prompt_style));
        } else {
            spans.push(Span::styled("    ", Style::default()));
        }

        let line_chars: Vec<char> = wl.chars().collect();
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
        display_lines.push(Line::from(vec![
            Span::styled(prompt_text, prompt_style),
            Span::styled(" ", Style::default().fg(t.cursor_fg).bg(t.cursor_bg)),
        ]));
    }

    let input_widget = Paragraph::new(display_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(if app.state.is_loading {
                Style::default().fg(t.border_input_loading)
            } else {
                Style::default().fg(t.border_input)
            })
            .title(Span::styled(" 输入消息 ", Style::default().fg(t.text_dim)))
            .style(Style::default().bg(t.bg_input)),
    );

    f.render_widget(input_widget, area);

    if !app.state.is_loading {
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
                    col = wl.chars().take(pos_in_line).map(char_width).sum();
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
    let t = &app.ui.theme;
    let hints = match app.ui.mode {
        ChatMode::Chat if app.state.is_loading => vec![("Esc", "取消请求"), ("↑↓", "滚动")],
        ChatMode::Chat => vec![
            ("Enter", "发送"),
            ("↑↓", "滚动"),
            ("@", "技能"),
            ("Ctrl+T", "切换模型"),
            ("Ctrl+L", "归档"),
            ("Ctrl+R", "还原"),
            ("Ctrl+Y", "复制"),
            ("Ctrl+B", "浏览"),
            ("Ctrl+S", "流式切换"),
            ("Ctrl+E", "配置"),
            ("Ctrl+G", "日志"),
            ("Ctrl+O", "工具详情"),
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
        ChatMode::ToolToggle => vec![
            ("↑↓", "选择"),
            ("Enter/空格", "切换"),
            ("t", "总开关"),
            ("a", "全部启用"),
            ("d", "全部禁用"),
            ("Esc", "返回"),
        ],
        ChatMode::SkillToggle => vec![
            ("↑↓", "选择"),
            ("Enter/空格", "切换"),
            ("a", "全部启用"),
            ("d", "全部禁用"),
            ("Esc", "返回"),
        ],
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

    f.render_stateful_widget(list, area, &mut app.ui.model_list_state);
}

/// 绘制帮助界面
pub fn draw_help(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
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
                "  Ctrl+G       ",
                Style::default().fg(t.help_key).add_modifier(Modifier::BOLD),
            ),
            Span::styled("实时查看日志", Style::default().fg(t.help_desc)),
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

/// 绘制 @ 补全弹窗（输入区域上方浮动）
pub fn draw_at_popup(f: &mut ratatui::Frame, input_area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = get_filtered_skills(app);
    if filtered.is_empty() {
        return;
    }

    let item_count = filtered.len().min(8); // 最多显示 8 项
    let popup_height = (item_count as u16) + 2; // 加上边框
    let popup_width = filtered
        .iter()
        .map(|n| display_width(&format!("  @{}  ", n)))
        .max()
        .unwrap_or(20)
        .max(16)
        .min(input_area.width.saturating_sub(4) as usize) as u16
        + 2; // 加边框

    // 弹窗位置：输入区域上方
    let x = input_area.x + 1;
    let y = input_area.y.saturating_sub(popup_height);

    let popup_area = Rect::new(x, y, popup_width, popup_height);

    let items: Vec<ListItem> = filtered
        .iter()
        .take(item_count)
        .map(|name| {
            let style = Style::default().fg(t.label_ai);
            ListItem::new(Line::from(Span::styled(format!("  @{}  ", name), style)))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(
        app.ui.at_popup_selected.min(item_count.saturating_sub(1)),
    ));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(t.border_title))
                .title(Span::styled(
                    " Skills ",
                    Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(t.bg_title)),
        )
        .highlight_style(
            Style::default()
                .bg(t.model_sel_highlight_bg)
                .fg(t.text_white)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut list_state);
}

/// 绘制文件补全弹窗（输入区域上方浮动）
pub fn draw_file_popup(f: &mut ratatui::Frame, input_area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = get_filtered_files(app);
    if filtered.is_empty() {
        return;
    }

    let item_count = filtered.len().min(12);
    let popup_height = (item_count as u16) + 2;
    let popup_width = filtered
        .iter()
        .map(|n| display_width(&format!("  {}  ", n)))
        .max()
        .unwrap_or(20)
        .max(16)
        .min(input_area.width.saturating_sub(4) as usize) as u16
        + 2;

    let x = input_area.x + 1;
    let y = input_area.y.saturating_sub(popup_height);

    let popup_area = Rect::new(x, y, popup_width, popup_height);

    let items: Vec<ListItem> = filtered
        .iter()
        .take(item_count)
        .map(|name| {
            let is_dir = name.ends_with('/');
            let style = if is_dir {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(t.text_white)
            };
            ListItem::new(Line::from(Span::styled(format!("  {}  ", name), style)))
        })
        .collect();

    let title_text = if app.ui.file_popup_filter.is_empty() {
        " Files ".to_string()
    } else {
        format!(" {} ", app.ui.file_popup_filter)
    };

    let mut list_state = ListState::default();
    list_state.select(Some(
        app.ui.file_popup_selected.min(item_count.saturating_sub(1)),
    ));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(t.border_title))
                .title(Span::styled(
                    title_text,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(t.bg_title)),
        )
        .highlight_style(
            Style::default()
                .bg(t.model_sel_highlight_bg)
                .fg(t.text_white)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut list_state);
}

/// 查找输入文本中所有 @mention 的字符范围 (start_char_idx, end_char_idx)
fn find_at_mention_ranges(text: &str, skill_names: &[String]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '@' {
            let valid_start = i == 0 || chars[i - 1].is_whitespace();
            if valid_start {
                let rest: String = chars[i + 1..].iter().collect();
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
                // 检查 @skill_name
                let mut best_len = 0usize;
                for name in skill_names {
                    if rest.starts_with(name.as_str()) {
                        let after_pos = name.len();
                        let is_boundary = if after_pos >= rest.len() {
                            true
                        } else {
                            rest.chars()
                                .nth(after_pos)
                                .map(|next_ch| {
                                    next_ch.is_whitespace() || !next_ch.is_alphanumeric()
                                })
                                .unwrap_or(true)
                        };
                        if is_boundary && name.len() > best_len {
                            best_len = name.len();
                        }
                    }
                }
                if best_len > 0 {
                    ranges.push((i, i + 1 + best_len));
                    i += 1 + best_len;
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
