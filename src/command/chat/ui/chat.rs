use super::archive::{draw_archive_confirm, draw_archive_list};
use super::config::draw_config_screen;
use super::popup;
use super::title_bar;
use crate::command::chat::app::{ChatApp, ChatMode, MouseSelection, MsgLinesCache};
use crate::command::chat::render::cache::build_message_lines_incremental;
use crate::command::chat::render::cache::copy_to_clipboard;
use crate::markdown::image_cache::ImageState;
use crate::markdown::image_loader::load_image;
use crate::tui::components::selection::{
    compute_line_selection_range, extract_content_from_line, is_selectable_line,
    normalize_selection, rebuild_spans_with_selection_style, spans_to_char_offset,
};
use crate::util::safe_lock;

/// 消息气泡宽度占内部可用宽度的百分比。
const BUBBLE_WIDTH_PERCENT: usize = 85;
/// 气泡渲染布局版本：边框样式变化时递增此值以强制缓存失效
const RENDER_VERSION: u32 = 2;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use ratatui_image::{Resize, StatefulImage};

/// 绘制 Chat 主界面：标题栏、消息区、输入区、提示栏及各类弹窗覆盖层
pub fn draw_chat_ui(f: &mut ratatui::Frame, app: &mut ChatApp) {
    let size = f.area();

    // 整体背景：先清除旧内容，再填充背景色。
    // Windows 上 crossterm 差异缓冲区可能不清理旧内容，导致切换模式时残留上一帧的字符。
    f.render_widget(Clear, size);
    let bg = Block::default().style(Style::default().bg(app.ui.theme.bg_primary));
    f.render_widget(bg, size);

    // 动态标题栏高度：状态行(1) + 可选分割线(1) + 可选 teammate 行 + 可选 subagent 行
    let has_teammates = app
        .teammate_manager
        .lock()
        .map(|m| !m.teammates.is_empty())
        .unwrap_or(false);
    let has_subagents = !app.sub_agent_tracker.display_snapshots().is_empty();
    let title_height = title_bar::calc_title_height(app);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(title_height), // 标题栏（内容行 + 可选 teammate 行）
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

    // ========== 右键上下文菜单覆盖层 ==========
    super::context_menu::draw_context_menu(f, app);
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

// ========== 鼠标选区坐标映射 ==========

/// 将屏幕坐标转换为 (全局行号, 行内字符偏移)
/// 返回 None 表示点击在消息区域外、空白区域或不可选行（边框、label、空行等）
pub fn screen_to_text_pos(
    screen_x: u16,
    screen_y: u16,
    inner: Rect,
    scroll_offset: usize,
    cached: &MsgLinesCache,
) -> Option<(usize, usize)> {
    // 1. 计算全局行号
    let local_y = screen_y.saturating_sub(inner.y);
    if local_y >= inner.height {
        return None;
    }
    let global_line = scroll_offset + local_y as usize;
    if global_line >= cached.total_line_count {
        return None;
    }

    // 2. 获取该行的 Line
    let history_total = cached.history_line_count;
    let line = get_line_at(cached, global_line, history_total)?;

    // 3. 检查该行是否可选（跳过边框、label、空行）
    if !is_selectable_line(line) {
        return None;
    }

    // 4. 计算行内字符偏移（考虑 CJK 宽字符）
    let local_x = screen_x.saturating_sub(inner.x) as usize;
    let char_offset = spans_to_char_offset(&line.spans, local_x);

    Some((global_line, char_offset))
}

// `is_selectable_line` / `is_decorative_span` / `extract_content_from_line` /
// `spans_to_char_offset` 已迁移至 `tui/components/selection.rs`，由编辑器和
// chat UI 共用。
/// 根据选区范围，从渲染行中提取纯内容文本（去掉边框和 padding）。
/// anchor/current 的字符偏移是相对于渲染行的，会自动转换为内容偏移。
pub fn extract_selection_text(
    cached: &MsgLinesCache,
    anchor: (usize, usize),
    current: (usize, usize),
) -> String {
    let ((sr, sc), (er, ec)) = normalize_selection(anchor, current);
    let history_total = cached.history_line_count;

    let mut result = String::new();

    for gline in sr..=er {
        let line = match get_line_at(cached, gline, history_total) {
            Some(l) => l,
            None => continue,
        };

        // 跳过不可选行
        if !is_selectable_line(line) {
            continue;
        }

        // 提取纯内容文本和内容起始偏移
        let (content_text, content_start) = extract_content_from_line(line);
        if content_text.is_empty() {
            continue;
        }

        // 将渲染行偏移转换为内容偏移
        let render_start = if gline == sr { sc } else { 0 };
        let render_end = if gline == er { ec } else { usize::MAX };

        // 内容区域：[content_start, content_start + content_len)
        let content_len = content_text.chars().count();
        let content_end = content_start + content_len;

        // 计算交集：渲染选区 ∩ 内容区域
        let intersect_start = render_start.max(content_start);
        let intersect_end = if render_end == usize::MAX {
            content_end
        } else {
            render_end.min(content_end)
        };

        if intersect_start >= intersect_end {
            continue;
        }

        // 转为内容文本内的字符偏移
        let text_start = intersect_start - content_start;
        let text_end = intersect_end - content_start;

        let chars: Vec<char> = content_text.chars().collect();
        let text_end = text_end.min(chars.len());
        if text_start < text_end {
            let slice: String = chars[text_start..text_end].iter().collect();
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&slice);
        }
    }

    result
}

/// 复制选区文本到剪贴板，并显示 toast 提示
pub fn copy_selection_to_clipboard(app: &mut ChatApp) {
    let cached = match app.ui.msg_lines_cache.as_ref() {
        Some(c) => c,
        None => return,
    };

    let sel = match &app.ui.mouse_selection {
        Some(s) => s,
        None => return,
    };

    let text = extract_selection_text(cached, sel.anchor, sel.current);
    if text.is_empty() {
        return;
    }

    if copy_to_clipboard(&text) {
        app.show_toast("已复制到剪贴板", false);
    } else {
        app.show_toast("复制到剪贴板失败", true);
    }
}

/// render_text_pass 的渲染参数（f 单独传）
struct TextPassParams<'a> {
    inner: Rect,
    cached: &'a MsgLinesCache,
    start: usize,
    end: usize,
    history_total: usize,
    msg_area_bg: Style,
    selection_style: Style,
}

/// 文字渲染 pass：遍历可见行渲染文字，同时收集图片标记。
/// 返回 `img_markers`: `(display_row, height, url)` 列表，供后续图片渲染 pass 使用。
/// P1 优化：通过消息范围预计算，避免逐行二分查找，只遍历可见消息。
fn render_text_pass(
    f: &mut ratatui::Frame,
    params: &TextPassParams,
    selection: Option<&MouseSelection>,
) -> Vec<(usize, u16, String)> {
    let mut img_markers: Vec<(usize, u16, String)> = Vec::new();
    let cached = params.cached;
    let history_total = params.history_total;

    // ★ P1 优化：使用二分查找定位第一条可见消息，然后顺序遍历
    // 只遍历 [start, end) 范围内涉及的 per_msg_lines 和 streaming_lines
    let visible_start = params.start;
    let visible_end = params.end;

    // 预计算历史消息的范围
    if visible_start < history_total && !cached.per_msg_lines.is_empty() {
        // 二分查找第一条可见消息
        let first_msg_pos = cached
            .msg_start_lines
            .partition_point(|&(_, start)| start <= visible_start)
            .saturating_sub(1);
        let first_msg_start = cached.msg_start_lines[first_msg_pos].1;

        // 顺序遍历消息，直到超出可见范围
        let mut line_idx = first_msg_start;
        for msg_pos in first_msg_pos..cached.per_msg_lines.len() {
            let per = &cached.per_msg_lines[msg_pos];
            let msg_line_count = per.lines.len();

            // 此消息的所有行
            for local in 0..msg_line_count {
                if line_idx >= visible_end {
                    break;
                }
                if line_idx >= visible_start {
                    let screen_i = line_idx - visible_start;
                    let y = params.inner.y + screen_i as u16;
                    let line_area = Rect::new(params.inner.x, y, params.inner.width, 1);
                    let line = &per.lines[local];

                    render_single_line(
                        f,
                        line,
                        line_area,
                        line_idx,
                        selection,
                        params.msg_area_bg,
                        params.selection_style,
                        &mut img_markers,
                        screen_i,
                    );
                }
                line_idx += 1;
            }
            if line_idx >= visible_end {
                break;
            }
        }
    }

    // 流式内容部分
    if visible_end > history_total {
        let stream_start = visible_start.saturating_sub(history_total);
        let stream_end = visible_end - history_total;
        for (local, line) in cached
            .streaming_lines
            .iter()
            .enumerate()
            .take(stream_end)
            .skip(stream_start)
        {
            let screen_i = history_total + local - visible_start;
            if screen_i >= visible_end - visible_start {
                break;
            }
            let y = params.inner.y + screen_i as u16;
            let line_area = Rect::new(params.inner.x, y, params.inner.width, 1);
            let global_idx = history_total + local;

            render_single_line(
                f,
                line,
                line_area,
                global_idx,
                selection,
                params.msg_area_bg,
                params.selection_style,
                &mut img_markers,
                screen_i,
            );
        }
    }

    img_markers
}

/// 渲染单行（处理图片标记、选区高亮等）
#[allow(clippy::too_many_arguments)]
fn render_single_line(
    f: &mut ratatui::Frame,
    line: &Line<'static>,
    line_area: Rect,
    line_idx: usize,
    selection: Option<&MouseSelection>,
    msg_area_bg: Style,
    selection_style: Style,
    img_markers: &mut Vec<(usize, u16, String)>,
    screen_i: usize,
) {
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
        img_markers.push((screen_i, height, url));
    } else if let Some(sel) = selection
        && is_selectable_line(line)
    {
        let (sel_start, sel_end) = compute_line_selection_range(line_idx, sel.anchor, sel.current);
        if sel_start < sel_end {
            let highlighted_spans = rebuild_spans_with_selection_style(
                &line.spans,
                0,
                sel_start,
                sel_end,
                selection_style,
            );
            let p = Paragraph::new(Line::from(highlighted_spans)).style(msg_area_bg);
            f.render_widget(p, line_area);
        } else {
            let p = Paragraph::new(line.clone()).style(msg_area_bg);
            f.render_widget(p, line_area);
        }
    } else {
        let p = Paragraph::new(line.clone()).style(msg_area_bg);
        f.render_widget(p, line_area);
    }
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
                        .fg(app.ui.theme.text_normal)
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
                        .fg(app.ui.theme.text_normal)
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
        let welcome_lines = super::components::welcome_box(
            inner_width,
            t,
            app.ui.quote_idx,
            app.state.agent_config.welcome_quote,
        );
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
    // === 分离历史缓存和流式缓存的命中判断 ===
    // 历史缓存：消息数量、内容、气泡宽度变化时才需要重建
    // 流式缓存：每帧重新渲染，但复用 stable_lines 增量缓存
    let history_cache_valid = if let Some(ref cache) = app.ui.msg_lines_cache {
        cache.msg_count == msg_count
            && cache.bubble_max_width == bubble_max_width
            && cache.expand_tools == app.ui.expand_tools
            && cache.browse_index == current_browse_index
            && cache.render_version == RENDER_VERSION
            // 验证每条消息内容长度一致（避免遍历全部，只检查数量和最后一条）
            && cache.per_msg_lines.len() == msg_count
            && cache.last_msg_len == last_msg_len
    } else {
        false
    };

    // 流式内容需要更新的条件
    let streaming_needs_update = app.state.is_loading
        || app
            .ui
            .msg_lines_cache
            .as_ref()
            .map(|c| c.streaming_len != streaming_len)
            .unwrap_or(true)
        || app
            .ui
            .msg_lines_cache
            .as_ref()
            .map(|c| c.tool_confirm_idx != current_tool_confirm_idx)
            .unwrap_or(true);

    // 缓存完全命中：历史有效且流式无需更新
    let cache_hit = history_cache_valid && !streaming_needs_update;

    if !cache_hit {
        if history_cache_valid {
            // ★ P3 核心优化：历史缓存有效，只重建流式内容
            // 避免遍历 1500 条消息、避免 clone Vec<PerMsgCache>
            let old_cache = app.ui.msg_lines_cache.take();
            let (new_streaming_lines, new_stable_lines, new_stable_offset) =
                crate::command::chat::render::cache::rebuild_streaming_only(
                    app,
                    inner_width,
                    bubble_max_width,
                    old_cache.as_ref(),
                );
            let new_streaming_len = new_streaming_lines.len();
            let old = old_cache.unwrap();
            // history_line_count 不变，total_line_count = history + streaming
            let total_line_count = old.history_line_count + new_streaming_len;
            app.ui.msg_lines_cache = Some(MsgLinesCache {
                msg_count: old.msg_count,
                last_msg_len: old.last_msg_len,
                streaming_len,
                bubble_max_width: old.bubble_max_width,
                browse_index: old.browse_index,
                tool_confirm_idx: current_tool_confirm_idx,
                total_line_count,
                history_line_count: old.history_line_count,
                msg_start_lines: old.msg_start_lines,
                per_msg_lines: old.per_msg_lines,
                streaming_lines: new_streaming_lines,
                streaming_stable_lines: new_stable_lines,
                streaming_stable_offset: new_stable_offset,
                expand_tools: old.expand_tools,
                render_version: old.render_version,
            });
        } else {
            // 历史缓存也失效，完整重建
            let old_cache = app.ui.msg_lines_cache.take();
            let (
                new_msg_start_lines,
                new_per_msg,
                new_streaming_lines,
                new_stable_lines,
                new_stable_offset,
            ) = build_message_lines_incremental(
                app,
                inner_width,
                bubble_max_width,
                old_cache.as_ref(),
            );
            let total_line_count: usize = new_per_msg.iter().map(|p| p.lines.len()).sum::<usize>()
                + new_streaming_lines.len();
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
    }

    let cached = match app.ui.msg_lines_cache.as_ref() {
        Some(c) => c,
        None => return,
    };
    // 使用 usize 避免超过 65535 行时溢出
    let total_lines = cached.total_line_count;

    f.render_widget(block, area);

    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    // 缓存 inner rect 供鼠标事件处理使用
    app.ui.msg_area_inner = Some(inner);
    let visible_height = inner.height as usize;
    let max_scroll = total_lines.saturating_sub(visible_height);

    if app.ui.mode != ChatMode::Browse {
        if matches!(
            app.ui.mode,
            ChatMode::ToolConfirm | ChatMode::AgentPermConfirm | ChatMode::PlanApprovalConfirm
        ) {
            if app.ui.auto_scroll
                || app.ui.scroll_offset == usize::MAX
                || app.ui.scroll_offset > max_scroll
            {
                app.ui.scroll_offset = max_scroll;
            }
        } else if app.ui.scroll_offset == usize::MAX || app.ui.scroll_offset >= max_scroll {
            app.ui.scroll_offset = max_scroll;
            app.ui.auto_scroll = true;
        }
    } else if let Some(msg_start) = cached
        .msg_start_lines
        .iter()
        .find(|(idx, _)| *idx == app.ui.browse_msg_index)
        .map(|(_, line)| *line)
    {
        let msg_line_count = cached
            .per_msg_lines
            .get(app.ui.browse_msg_index)
            .map(|c| c.lines.len())
            .unwrap_or(1);
        let msg_max_scroll = msg_line_count.saturating_sub(visible_height);
        if app.ui.browse_scroll_offset > msg_max_scroll {
            app.ui.browse_scroll_offset = msg_max_scroll;
        }
        app.ui.scroll_offset = (msg_start + app.ui.browse_scroll_offset).min(max_scroll);
    }

    // 先清除 inner 区域的旧字符（reset 每个 cell 的 symbol 为空格），
    // 再用背景色填充。ratatui 的 Block/Paragraph 只调用 set_style（不改 symbol），
    // 在窄屏滚动时会导致右侧残留上一帧的字符。
    f.render_widget(ratatui::widgets::Clear, inner);
    let bg_fill = Block::default().style(Style::default().bg(app.ui.theme.bg_primary));
    f.render_widget(bg_fill, inner);

    // === 文字渲染 pass + 图片标记收集 ===
    let (start, img_markers) = {
        let cached = match app.ui.msg_lines_cache.as_ref() {
            Some(c) => c,
            None => return,
        };
        let start = app.ui.scroll_offset;
        let end = (start + visible_height).min(cached.total_line_count);
        let history_total = cached.history_line_count;
        let msg_area_bg = Style::default().bg(app.ui.theme.bg_primary);
        let selection_style = app
            .ui
            .theme
            .popup_highlight_fg
            .apply_fg(app.ui.theme.popup_highlight_bg.apply_bg(Style::default()));
        let selection = app.ui.mouse_selection.as_ref();
        let img_markers = render_text_pass(
            f,
            &TextPassParams {
                inner,
                cached,
                start,
                end,
                history_total,
                msg_area_bg,
                selection_style,
            },
            selection,
        );
        (start, img_markers)
    }; // cached 借用在此释放

    // === 图片渲染 pass（需在文字之后覆盖绘制）===
    render_image_pass(
        f,
        inner,
        img_markers,
        start,
        visible_height as u16,
        bubble_max_width,
        app,
    );
}
