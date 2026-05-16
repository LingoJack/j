//! 配置界面 UI 模块
//!
//! 将各 Tab 的渲染逻辑拆分到独立子模块，便于维护和扩展。

mod archive;
mod commands;
mod global;
mod hooks;
mod model;
mod session;
mod skills;
mod teammates;
mod tools;

use crate::command::chat::app::{ChatApp, CommandsMode, ConfigTab, ConfigTabHitBox};
use crate::tui::components::{separator_line, tab_bar};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// 逐行渲染配置页内容，避免单个 Paragraph 在部分终端上发生软换行后污染相邻行。
fn render_block_lines(
    f: &mut ratatui::Frame,
    area: Rect,
    block: Block<'_>,
    bg: Color,
    lines: &[Line<'_>],
    scroll_y: u16,
) {
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    f.render_widget(Block::default().style(Style::default().bg(bg)), inner);

    let start = usize::from(scroll_y).min(lines.len());
    let end = (start + inner.height as usize).min(lines.len());

    for (row, line) in lines[start..end].iter().enumerate() {
        let line_area = Rect::new(inner.x, inner.y + row as u16, inner.width, 1);
        let widget = Paragraph::new(line.clone()).style(Style::default().bg(bg));
        f.render_widget(widget, line_area);
    }
}

/// 绘制顶部 Tab 栏（支持窄屏水平滚动）
fn draw_tab_bar_line<'a>(app: &ChatApp) -> Line<'a> {
    let current = app.ui.config_tab;
    let all_tabs = [
        ConfigTab::Model,
        ConfigTab::Session,
        ConfigTab::Global,
        ConfigTab::Tools,
        ConfigTab::Skills,
        ConfigTab::Hooks,
        ConfigTab::Commands,
        ConfigTab::Teammates,
        ConfigTab::Archive,
    ];
    let tabs: Vec<(&str, bool)> = all_tabs
        .iter()
        .map(|tab| (tab.label(), *tab == current))
        .collect();
    tab_bar(
        &tabs,
        "\u{2190}\u{2192} \u{5207}\u{6362}\u{6807}\u{7b7e}",
        &app.ui.theme,
    )
}

/// 计算 Tab 栏各 Tab 的点击区域（与 `tab_bar` 布局逻辑保持同步）
fn compute_tab_hitboxes() -> Vec<ConfigTabHitBox> {
    use unicode_width::UnicodeWidthStr;

    let all_tabs = [
        ConfigTab::Model,
        ConfigTab::Session,
        ConfigTab::Global,
        ConfigTab::Tools,
        ConfigTab::Skills,
        ConfigTab::Hooks,
        ConfigTab::Commands,
        ConfigTab::Teammates,
        ConfigTab::Archive,
    ];
    // 起始有 "  "（2 列前缀），与 tab_bar 函数一致
    let mut col: u16 = 2;
    let mut hitboxes = Vec::with_capacity(all_tabs.len());
    for (i, tab) in all_tabs.iter().enumerate() {
        if i > 0 {
            // " {SEPARATOR_V} " 分隔符占 3 列
            col += 3;
        }
        // " {label} " = label 显示宽度 + 2（前后空格）
        let label_width = UnicodeWidthStr::width(tab.label()) as u16;
        let start = col;
        let end = col + label_width + 2;
        hitboxes.push(ConfigTabHitBox {
            tab: *tab,
            start_col: start,
            end_col: end,
        });
        col = end;
    }
    hitboxes
}

/// 配置界面主入口（分发器）
///
/// 将面板拆分为三层：
///   1. 固定头部：边框标题 + Tab 栏 + 分隔线
///   2. 固定 Tab 头部：每个 Tab 自身的摘要信息（如"当前会话"、"总开关"等）
///   3. 可滚动列表：只有列表项跟随选中项滚动
pub fn draw_config_screen(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    // ── Model Tab 不再需要水平滚动偏移调整 ──

    let t = &app.ui.theme;
    let bg = t.bg_primary;

    let title = match app.ui.config_tab {
        ConfigTab::Model => " \u{2699}\u{fe0f} \u{6a21}\u{578b}\u{914d}\u{7f6e} ",
        ConfigTab::Global => " \u{1f310} \u{5168}\u{5c40}\u{914d}\u{7f6e} ",
        ConfigTab::Tools => " \u{1f527} \u{5de5}\u{5177}\u{5f00}\u{5173} ",
        ConfigTab::Skills => " \u{1f4e6} \u{6280}\u{80fd}\u{5f00}\u{5173} ",
        ConfigTab::Hooks => " \u{1fa9d} Hooks ",
        ConfigTab::Commands => " \u{1f4cb} \u{81ea}\u{5b9a}\u{4e49}\u{547d}\u{4ee4} ",
        ConfigTab::Session => " \u{1f4ac} \u{4f1a}\u{8bdd}\u{7ba1}\u{7406} ",
        ConfigTab::Teammates => " 👥 协作者 ",
        ConfigTab::Archive => " \u{1f4e6} \u{5f52}\u{6863}\u{7ba1}\u{7406} ",
    };

    // ── 记录每个 Tab 的固定头部行和可滚动列表行 ──
    let mut tab_header_lines: Vec<Line> = Vec::new();
    let mut list_lines: Vec<Line> = Vec::new();
    let mut field_line_indices: Vec<usize> = Vec::new();

    // Model tab 使用左右分栏渲染，需要提前标记
    let is_model_split = app.ui.config_tab == ConfigTab::Model;

    match app.ui.config_tab {
        ConfigTab::Model => {
            // Model tab 使用左右分栏，不需要单独的 header 和 list
        }
        ConfigTab::Global => {
            // Global 没有固定头部，全部是字段列表
            let list = global::draw_tab_global_lines(app);
            let (item_lines, item_indices) = list.into_parts();
            list_lines.extend(item_lines);
            field_line_indices.extend(item_indices);
        }
        ConfigTab::Tools => {
            tools::draw_tab_tools_header(&mut tab_header_lines, app);
            let list = tools::draw_tab_tools_list(app);
            let (item_lines, item_indices) = list.into_parts();
            list_lines.extend(item_lines);
            field_line_indices.extend(item_indices);
        }
        ConfigTab::Skills => {
            skills::draw_tab_skills_header(&mut tab_header_lines, app);
            let list = skills::draw_tab_skills_list(app);
            let (item_lines, item_indices) = list.into_parts();
            list_lines.extend(item_lines);
            field_line_indices.extend(item_indices);
        }
        ConfigTab::Hooks => {
            hooks::draw_tab_hooks_header(&mut tab_header_lines, app);
            let list = hooks::draw_tab_hooks_list(app);
            let (item_lines, item_indices) = list.into_parts();
            list_lines.extend(item_lines);
            field_line_indices.extend(item_indices);
        }
        ConfigTab::Commands => {
            commands::draw_tab_commands_header(&mut tab_header_lines, app);
            // 选择来源模式时不显示列表
            if app.ui.commands_mode != CommandsMode::SelectSource {
                let list = commands::draw_tab_commands_list(app);
                let (item_lines, item_indices) = list.into_parts();
                list_lines.extend(item_lines);
                field_line_indices.extend(item_indices);
            }
        }
        ConfigTab::Teammates => {
            teammates::draw_tab_teammates_header(&mut tab_header_lines, app);
            let list = teammates::draw_tab_teammates_list(app);
            let (item_lines, item_indices) = list.into_parts();
            list_lines.extend(item_lines);
            field_line_indices.extend(item_indices);
        }
        ConfigTab::Session => {
            session::draw_tab_session_header(&mut tab_header_lines, app);
            let list = session::draw_tab_session_list(app);
            let (item_lines, item_indices) = list.into_parts();
            list_lines.extend(item_lines);
            field_line_indices.extend(item_indices);
        }
        ConfigTab::Archive => {
            archive::draw_tab_archive_header(&mut tab_header_lines, app);
            let list = archive::draw_tab_archive_list(app);
            let (item_lines, item_indices) = list.into_parts();
            list_lines.extend(item_lines);
            field_line_indices.extend(item_indices);
        }
    }

    // Tab 栏行数: 空行 + tab_bar + 空行 + separator = 4
    let tab_bar_lines: u16 = 4;
    // 顶部 border 占 1 行
    let top_border: u16 = 1;
    let tab_header_h = tab_header_lines.len() as u16;
    let fixed_h = top_border + tab_bar_lines + tab_header_h;

    // 如果没有可滚动列表，或终端太小，回退到整体渲染
    // 注意：Model tab 虽然在 match 中没有填充 list_lines，但它使用左右分栏渲染，不走回退路径
    if (list_lines.is_empty() && !is_model_split) || area.height <= fixed_h + 1 {
        // Windows 上 crossterm 差异缓冲区可能不清理旧内容，先显式清除区域
        f.render_widget(Clear, area);
        let mut all_lines: Vec<Line> = vec![
            Line::from(""),
            draw_tab_bar_line(app),
            Line::from(""),
            separator_line(area.width, t),
        ];
        all_lines.append(&mut tab_header_lines);
        all_lines.append(&mut list_lines);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(t.border_config))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(t.config_label_selected)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(bg));
        render_block_lines(f, area, block, bg, &all_lines, app.ui.config_scroll_offset);
        // ── 回退模式下也记录布局信息 ──
        // 回退模式的 Block 有 Borders::ALL（含 top border），所以 Tab 栏全局 Y = area.y + 1（top border）+ 1（空行）
        app.ui.config_tab_bar_y = Some(area.y + 2);
        app.ui.config_list_area = None; // 无独立列表区域
        app.ui.config_field_lines = field_line_indices;
        app.ui.config_tab_hitboxes = compute_tab_hitboxes();
        return;
    }

    // ── 三段布局 ──
    let header_area_h = fixed_h; // 顶部 border + tab_bar + tab_header
    let list_area_h = area.height.saturating_sub(header_area_h);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header_area_h),
            Constraint::Min(list_area_h),
        ])
        .split(area);

    // ── 固定头部：顶部边框 + 标题 + Tab 栏 + Tab 专属头部 ──
    // Windows 上 crossterm 差异缓冲区可能不清理旧内容，先显式清除区域
    f.render_widget(Clear, chunks[0]);
    let mut header_lines: Vec<Line> = vec![
        Line::from(""),
        draw_tab_bar_line(app),
        Line::from(""),
        separator_line(area.width, t),
    ];
    header_lines.append(&mut tab_header_lines);

    let header_block = Block::default()
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(t.border_config))
        .title(Span::styled(
            title,
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(bg));
    render_block_lines(f, chunks[0], header_block, bg, &header_lines, 0);

    // ── Tools / Model Tab 特殊处理：左右分栏 ──
    let is_tools_split = app.ui.config_tab == ConfigTab::Tools;

    if is_tools_split {
        // ── 可滚动列表区域 → 左右分栏 ──
        let left_w = (area.width as usize * 45 / 100).max(20) as u16;
        let right_w = area.width.saturating_sub(left_w);

        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(left_w), Constraint::Min(right_w)])
            .split(chunks[1]);

        // ── 左侧：工具列表（可滚动）──
        // Windows 上 crossterm 差异缓冲区可能不清理旧内容，先显式清除区域
        f.render_widget(Clear, h_chunks[0]);
        let inner_height = h_chunks[0].height.saturating_sub(1) as usize;
        let selected_idx = app.ui.config_field_idx;
        if let Some(&selected_line) = field_line_indices.get(selected_idx) {
            let scroll = app.ui.config_scroll_offset as usize;
            let new_scroll = if selected_line < scroll {
                selected_line
            } else if inner_height > 0 && selected_line >= scroll + inner_height {
                selected_line.saturating_sub(inner_height - 1)
            } else {
                scroll
            };
            app.ui.config_scroll_offset = new_scroll as u16;
        }

        let left_block = Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(t.border_config))
            .style(Style::default().bg(bg));
        render_block_lines(
            f,
            h_chunks[0],
            left_block,
            bg,
            &list_lines,
            app.ui.config_scroll_offset,
        );

        // ── 右侧：选中工具详情 ──
        // Windows 上 crossterm 差异缓冲区可能不清理旧内容，先显式清除区域
        f.render_widget(Clear, h_chunks[1]);
        let detail_lines = tools::draw_tab_tools_detail(app);
        let selected_tool_name = app
            .tool_registry
            .tool_names()
            .get(app.ui.config_field_idx)
            .copied()
            .unwrap_or("");
        let right_block = Block::default()
            .borders(Borders::BOTTOM | Borders::RIGHT)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(t.border_config))
            .title(Span::styled(
                format!(" {selected_tool_name} "),
                Style::default()
                    .fg(t.config_label_selected)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(bg));
        render_block_lines(f, h_chunks[1], right_block, bg, &detail_lines, 0);

        // ── 记录布局信息 ──
        app.ui.config_tab_bar_y = Some(chunks[0].y + 2);
        app.ui.config_list_area = Some(h_chunks[0]);
        app.ui.config_field_lines = field_line_indices;
        app.ui.config_tab_hitboxes = compute_tab_hitboxes();
        return;
    }

    if is_model_split {
        // ── Model Tab 左右分栏 ──
        let left_w = model::model_providers_min_width(app);
        let right_w = area.width.saturating_sub(left_w);

        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(left_w), Constraint::Min(right_w)])
            .split(chunks[1]);

        // ── 左侧：Provider 列表（可滚动）──
        f.render_widget(Clear, h_chunks[0]);
        let provider_list = model::draw_tab_model_providers(app);
        let (provider_lines, provider_indices) = provider_list.into_parts();

        // Provider 列表滚动：确保选中项可见
        let inner_height = h_chunks[0].height.saturating_sub(1) as usize;
        if let Some(&selected_line) = provider_indices.get(app.ui.config_provider_idx) {
            let scroll = app.ui.config_scroll_offset as usize;
            let new_scroll = if selected_line < scroll {
                selected_line
            } else if inner_height > 0 && selected_line >= scroll + inner_height {
                selected_line.saturating_sub(inner_height - 1)
            } else {
                scroll
            };
            app.ui.config_scroll_offset = new_scroll as u16;
        }

        let left_block = Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(t.border_config))
            .style(Style::default().bg(bg));
        render_block_lines(
            f,
            h_chunks[0],
            left_block,
            bg,
            &provider_lines,
            app.ui.config_scroll_offset,
        );

        // ── 右侧：配置字段详情 ──
        f.render_widget(Clear, h_chunks[1]);
        let detail_list = model::draw_tab_model_detail(app);
        let (detail_lines, detail_indices) = detail_list.into_parts();

        let selected_provider_name = app
            .state
            .agent_config
            .providers
            .get(app.ui.config_provider_idx)
            .map(|p| p.name.as_str())
            .unwrap_or("");
        let right_block = Block::default()
            .borders(Borders::BOTTOM | Borders::RIGHT)
            .border_type(ratatui::widgets::BorderType::Rounded)
            .border_style(Style::default().fg(t.border_config))
            .title(Span::styled(
                format!(" {selected_provider_name} "),
                Style::default()
                    .fg(t.config_label_selected)
                    .add_modifier(Modifier::BOLD),
            ))
            .style(Style::default().bg(bg));
        render_block_lines(f, h_chunks[1], right_block, bg, &detail_lines, 0);

        // ── 记录布局信息 ──
        app.ui.config_tab_bar_y = Some(chunks[0].y + 2);
        app.ui.config_list_area = Some(h_chunks[1]);
        app.ui.config_field_lines = detail_indices;
        app.ui.config_tab_hitboxes = compute_tab_hitboxes();
        return;
    }

    // ── 可滚动列表区域（非 Tools / Model Tab 的通用路径）──
    // Windows 上 crossterm 差异缓冲区可能不清理旧内容，先显式清除区域
    f.render_widget(Clear, chunks[1]);
    // 可见高度 = list_area_h - 1（底部 border）
    let inner_height = list_area_h.saturating_sub(1) as usize;
    let selected_idx = match app.ui.config_tab {
        ConfigTab::Session => app.ui.session_list_index,
        ConfigTab::Archive => app.ui.archive_list_index,
        ConfigTab::Teammates => app.ui.teammate_list_index,
        ConfigTab::Global if app.ui.compact_exempt_sublist => app.ui.compact_exempt_idx,
        // 这些 Tab 使用 config_field_idx 作为选中索引
        ConfigTab::Model
        | ConfigTab::Tools
        | ConfigTab::Skills
        | ConfigTab::Hooks
        | ConfigTab::Commands
        | ConfigTab::Global => app.ui.config_field_idx,
    };
    if let Some(&selected_line) = field_line_indices.get(selected_idx) {
        let scroll = app.ui.config_scroll_offset as usize;
        let new_scroll = if selected_line < scroll {
            selected_line
        } else if inner_height > 0 && selected_line >= scroll + inner_height {
            selected_line.saturating_sub(inner_height - 1)
        } else {
            scroll
        };
        app.ui.config_scroll_offset = new_scroll as u16;
    }

    let list_block = Block::default()
        .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(t.border_config))
        .style(Style::default().bg(bg));
    render_block_lines(
        f,
        chunks[1],
        list_block,
        bg,
        &list_lines,
        app.ui.config_scroll_offset,
    );

    // ── 记录布局信息供鼠标点击使用 ──
    // header_block 有 Borders::TOP（占 1 行），然后内容第 0 行是空行，第 1 行是 Tab 栏
    // 所以 Tab 栏的全局 Y = chunks[0].y + 1（top border） + 1（空行）= chunks[0].y + 2
    app.ui.config_tab_bar_y = Some(chunks[0].y + 2);
    app.ui.config_list_area = Some(chunks[1]);
    app.ui.config_field_lines = field_line_indices;
    app.ui.config_tab_hitboxes = compute_tab_hitboxes();
}
