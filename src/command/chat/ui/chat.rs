use super::super::app::{ChatApp, ChatMode, MsgLinesCache};
use super::super::markdown::image_cache::ImageState;
use super::super::markdown::image_loader::load_image;
use super::super::render_cache::build_message_lines_incremental;
use super::archive::{draw_archive_confirm, draw_archive_list};
use super::config::draw_config_screen;
use super::popup;
use super::title_bar;
use crate::util::safe_lock;

/// 消息气泡宽度占内部可用宽度的百分比。
const BUBBLE_WIDTH_PERCENT: usize = 85;
/// 气泡渲染布局版本：边框样式变化时递增此值以强制缓存失效
const RENDER_VERSION: u32 = 2;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_image::{Resize, StatefulImage};

/// 绘制 Chat 主界面：标题栏、消息区、输入区、提示栏及各类弹窗覆盖层
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
    title_bar::draw_title_bar(f, chunks[0], app, has_teammates, has_subagents);

    // ========== 消息区 ==========
    match app.ui.mode {
        ChatMode::Help => super::help::draw_help(f, chunks[1], app),
        ChatMode::SelectModel => super::selector::draw_model_selector(f, chunks[1], app),
        ChatMode::SelectTheme => super::selector::draw_theme_selector(f, chunks[1], app),
        ChatMode::Config => draw_config_screen(f, chunks[1], app),
        ChatMode::ArchiveConfirm => draw_archive_confirm(f, chunks[1], app),
        ChatMode::ArchiveList => draw_archive_list(f, chunks[1], app),
        // 这些模式的主区域均显示消息列表
        ChatMode::Chat
        | ChatMode::Browse
        | ChatMode::ToolConfirm
        | ChatMode::AgentPermConfirm
        | ChatMode::PlanApprovalConfirm => draw_messages(f, chunks[1], app),
    }

    // ========== 输入区 ==========
    super::input::draw_input(f, chunks[2], app);

    // ========== 底部操作提示栏（始终可见）==========
    super::hint::draw_hint_bar(f, chunks[3], app);

    // ========== Toast 弹窗覆盖层（右上角）==========
    super::hint::draw_toast(f, size, app);

    // ========== @ 补全弹窗覆盖层 ==========
    if app.ui.at_popup_active {
        popup::draw_at_popup(f, chunks[2], app);
    }

    // ========== 文件补全弹窗覆盖层 ==========
    if app.ui.file_popup_active {
        popup::draw_file_popup(f, chunks[2], app);
    }

    // ========== 技能补全弹窗覆盖层 ==========
    if app.ui.skill_popup_active {
        popup::draw_skill_popup(f, chunks[2], app);
    }

    // ========== 命令补全弹窗覆盖层 ==========
    if app.ui.command_popup_active {
        popup::draw_command_popup(f, chunks[2], app);
    }

    // ========== / 斜杠命令弹窗覆盖层 ==========
    if app.ui.slash_popup_active {
        popup::draw_slash_popup(f, chunks[2], app);
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

/// 文字渲染 pass：遍历可见行渲染文字，同时收集图片标记。
/// 返回 `img_markers`: `(display_row, height, url)` 列表，供后续图片渲染 pass 使用。
fn render_text_pass(
    f: &mut ratatui::Frame,
    inner: Rect,
    cached: &MsgLinesCache,
    start: usize,
    end: usize,
    history_total: usize,
    msg_area_bg: Style,
) -> Vec<(usize, u16, String)> {
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
    img_markers
}

/// 图片渲染 pass：根据图片标记处理各状态的图片（Ready/Failed/Loading/Pending），
/// 并在 Pending 时启动异步加载线程。
fn render_image_pass(
    f: &mut ratatui::Frame,
    inner: Rect,
    img_markers: Vec<(usize, u16, String)>,
    start: usize,
    visible_height: u16,
    bubble_max_width: usize,
    app: &mut ChatApp,
) {
    let cached = match app.ui.msg_lines_cache.as_ref() {
        Some(c) => c,
        None => return,
    };
    let has_picker = safe_lock(&app.ui.image_cache, "draw_messages::image_cache_picker")
        .picker
        .is_some();
    let img_pad = 3u16; // 与气泡 pad_left_w 一致
    let img_render_w = (bubble_max_width as u16).saturating_sub(img_pad * 2);
    let history_total = cached.history_line_count;

    for (i, height, url) in img_markers {
        let line_idx = start + i;
        let y = inner.y + i as u16;
        let remaining_h = visible_height.saturating_sub(i as u16);
        let bubble_w = bubble_max_width as u16;

        // 计算实际可用的占位行数：从标记行往下数连续的空行/占位行
        let mut actual_h = 1u16;
        for next_offset in 1..height as usize {
            let next_idx = line_idx + next_offset;
            if next_idx >= cached.total_line_count {
                break;
            }
            let next_line = match get_line_at(cached, next_idx, history_total) {
                Some(l) => l,
                None => break,
            };
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

        if remaining_h < render_h {
            continue;
        }

        let img_x = inner.x + img_pad;
        let img_area = Rect::new(img_x, y, img_render_w, render_h);

        if !has_picker {
            let max_url_w = (bubble_w as usize).saturating_sub(12);
            let display_url = title_bar::truncate_str(&url, max_url_w);
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
                let max_err_w = (bubble_w as usize).saturating_sub(24);
                let display_err = title_bar::truncate_str(err, max_err_w);
                let err_line = Paragraph::new(Line::from(Span::styled(
                    format!("  [Image load failed: {}]", display_err),
                    Style::default().fg(Color::Red).bg(app.ui.theme.bubble_ai),
                )));
                f.render_widget(err_line, Rect::new(inner.x, y, bubble_w, 1));
            }
            Some(ImageState::Loading) => {
                let max_url_w = (bubble_w as usize).saturating_sub(21);
                let display_url = title_bar::truncate_str(&url, max_url_w);
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
                let display_url = title_bar::truncate_str(&url, max_url_w);
                let loading = Paragraph::new(Line::from(Span::styled(
                    format!("  Loading image: {}...", display_url),
                    Style::default()
                        .fg(Color::DarkGray)
                        .bg(app.ui.theme.bubble_ai),
                )));
                f.render_widget(loading, Rect::new(inner.x, y, bubble_w, 1));
                cache.images.insert(url.clone(), ImageState::Loading);
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

/// 绘制消息列表区域，支持缓存增量渲染、图片加载和滚动定位
pub fn draw_messages(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let t = &app.ui.theme;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(t.border_message))
        .style(Style::default().bg(t.bg_primary));

    // 空消息时显示欢迎界面（以 display_messages 为准，它是 UI 的数据源）
    // 快速检查是否为空，锁住后立即释放
    let is_empty = {
        let display = crate::util::safe_lock(&app.display_messages, "draw_messages::is_empty");
        display.is_empty()
    };
    if is_empty && !app.state.is_loading {
        let inner_width = area.width.saturating_sub(4);
        let welcome_lines = super::components::welcome_box(inner_width, t, app.ui.quote_idx);
        let empty = Paragraph::new(welcome_lines).block(block);
        f.render_widget(empty, area);
        return;
    }

    // 内部可用宽度（减去边框和左右各1的 padding）
    let inner_width = area.width.saturating_sub(4) as usize;
    // 消息内容最大宽度为可用宽度的指定百分比
    let bubble_max_width = (inner_width * BUBBLE_WIDTH_PERCENT / 100).max(20);

    let (msg_count, last_msg_len) = {
        let display = crate::util::safe_lock(&app.display_messages, "draw_messages::msg_stats");
        (
            display.len(),
            display.last().map(|m| m.content.len()).unwrap_or(0),
        )
    };
    let streaming_len = if app.state.is_loading {
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
    let cache_hit = if !app.state.is_loading {
        if let Some(ref cache) = app.ui.msg_lines_cache {
            cache.msg_count == msg_count
                && cache.last_msg_len == last_msg_len
                && cache.streaming_len == streaming_len
                && cache.bubble_max_width == bubble_max_width
                && cache.browse_index == current_browse_index
                && cache.tool_confirm_idx == current_tool_confirm_idx
                && cache.render_version == RENDER_VERSION
        } else {
            false
        }
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
            render_version: RENDER_VERSION,
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
        } else if app.ui.scroll_offset == u16::MAX || app.ui.scroll_offset >= max_scroll {
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

    // 背景填充 inner 区域
    let bg_fill = Block::default().style(Style::default().bg(app.ui.theme.bg_primary));
    f.render_widget(bg_fill, inner);

    // === 文字渲染 pass + 图片标记收集 ===
    let (start, img_markers) = {
        let cached = match app.ui.msg_lines_cache.as_ref() {
            Some(c) => c,
            None => return,
        };
        let start = app.ui.scroll_offset as usize;
        let end = (start + visible_height as usize).min(cached.total_line_count);
        let history_total = cached.history_line_count;
        let msg_area_bg = Style::default().bg(app.ui.theme.bg_primary);
        let img_markers =
            render_text_pass(f, inner, cached, start, end, history_total, msg_area_bg);
        (start, img_markers)
    }; // cached 借用在此释放

    // === 图片渲染 pass（需在文字之后覆盖绘制）===
    render_image_pass(
        f,
        inner,
        img_markers,
        start,
        visible_height,
        bubble_max_width,
        app,
    );
}
