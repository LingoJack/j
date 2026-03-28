use super::app::HelpApp;
use crate::util::text::display_width;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// 绘制帮助 TUI 界面
pub fn draw_help_ui(frame: &mut Frame, help_app: &mut HelpApp) {
    let size = frame.area();
    let theme = help_app.theme().clone();

    // 主布局：Tab 栏(1) + 标题栏(3) + 内容区(flex) + 提示栏(1)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tab Bar
            Constraint::Length(3), // Title Bar
            Constraint::Min(1),    // Content
            Constraint::Length(1), // Hint Bar
        ])
        .split(size);

    draw_tab_bar(frame, help_app, chunks[0], &theme);
    draw_title_bar(frame, help_app, chunks[1], &theme);
    draw_content(frame, help_app, chunks[2], &theme);
    draw_hint_bar(frame, chunks[3], &theme);
}

/// 绘制 Tab 栏（可滚动，放不下时显示 ◀ ▶）
fn draw_tab_bar(
    frame: &mut Frame,
    help_app: &HelpApp,
    area: Rect,
    theme: &crate::command::chat::theme::Theme,
) {
    let total_width = area.width as usize;

    // 预计算每个 tab 的标签文本和宽度（含两侧间距）
    let tab_labels: Vec<String> = (0..help_app.tab_count)
        .map(|i| format!(" {}.{} ", i + 1, help_app.tab_name(i)))
        .collect();
    // 每个 tab 占用宽度 = 标签宽度 + 1（右侧间距）
    let tab_widths: Vec<usize> = tab_labels.iter().map(|l| display_width(l) + 1).collect();
    let all_tabs_width: usize = tab_widths.iter().sum::<usize>() + 1; // +1 左侧间距

    // 判断是否需要滚动
    let needs_scroll = all_tabs_width > total_width;
    let arrow_width = 3; // " ◀ " 或 " ▶ "

    // 计算可见 tab 范围
    let (vis_start, vis_end) = if !needs_scroll {
        (0, help_app.tab_count)
    } else {
        // 从 active_tab 向两侧扩展，尽量居中
        let avail = total_width.saturating_sub(arrow_width * 2); // 两侧箭头预留
        let mut start = help_app.active_tab;
        let mut end = help_app.active_tab + 1;
        let mut used = tab_widths[help_app.active_tab] + 1; // +1 左侧间距

        loop {
            let mut expanded = false;
            // 尝试向右扩展
            if end < help_app.tab_count && used + tab_widths[end] <= avail {
                used += tab_widths[end];
                end += 1;
                expanded = true;
            }
            // 尝试向左扩展
            if start > 0 && used + tab_widths[start - 1] <= avail {
                start -= 1;
                used += tab_widths[start];
                expanded = true;
            }
            if !expanded {
                break;
            }
        }
        (start, end)
    };

    let has_left = vis_start > 0;
    let has_right = vis_end < help_app.tab_count;

    let mut spans: Vec<Span> = Vec::new();

    // 左箭头或左间距
    if has_left {
        spans.push(Span::styled(
            " ◀ ",
            Style::default().fg(theme.text_dim).bg(theme.bg_title),
        ));
    } else {
        spans.push(Span::styled(" ", Style::default().bg(theme.bg_title)));
    }

    // 渲染可见 tab
    for i in vis_start..vis_end {
        let label = &tab_labels[i];
        if i == help_app.active_tab {
            spans.push(Span::styled(
                label.clone(),
                Style::default()
                    .fg(theme.config_tab_active_fg)
                    .bg(theme.config_tab_active_bg)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                label.clone(),
                Style::default()
                    .fg(theme.config_tab_inactive)
                    .bg(theme.bg_title),
            ));
        }
        spans.push(Span::styled(" ", Style::default().bg(theme.bg_title)));
    }

    // 右箭头
    if has_right {
        // 先计算已用宽度，填充到右箭头位置
        let used_width: usize = spans.iter().map(|s| display_width(&s.content)).sum();
        let fill = total_width.saturating_sub(used_width + arrow_width);
        if fill > 0 {
            spans.push(Span::styled(
                " ".repeat(fill),
                Style::default().bg(theme.bg_title),
            ));
        }
        spans.push(Span::styled(
            " ▶ ",
            Style::default().fg(theme.text_dim).bg(theme.bg_title),
        ));
    } else {
        // 填充剩余空间
        let used_width: usize = spans.iter().map(|s| display_width(&s.content)).sum();
        let fill = total_width.saturating_sub(used_width);
        if fill > 0 {
            spans.push(Span::styled(
                " ".repeat(fill),
                Style::default().bg(theme.bg_title),
            ));
        }
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(vec![line]), area);
}

/// 绘制标题栏
fn draw_title_bar(
    frame: &mut Frame,
    help_app: &HelpApp,
    area: Rect,
    theme: &crate::command::chat::theme::Theme,
) {
    let title_text = format!("  📖 j help — {}", help_app.tab_name(help_app.active_tab));
    let page_info = format!("{}/{}  ", help_app.active_tab + 1, help_app.tab_count);

    let title_w = display_width(&title_text);
    let page_w = display_width(&page_info);
    let fill = (area.width as usize).saturating_sub(title_w + page_w);

    let spans = vec![
        Span::styled(
            title_text,
            Style::default()
                .fg(theme.help_title)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ".repeat(fill), Style::default()),
        Span::styled(page_info, Style::default().fg(theme.text_dim)),
    ];

    // 标题栏占 3 行：空行 + 标题内容 + 分隔线
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    // 空行
    frame.render_widget(Paragraph::new(vec![Line::from("")]), inner_chunks[0]);

    // 标题内容
    frame.render_widget(Paragraph::new(vec![Line::from(spans)]), inner_chunks[1]);

    // 分隔线
    let sep_width = area.width as usize;
    let sep_line = Line::from(Span::styled(
        "─".repeat(sep_width),
        Style::default().fg(theme.separator),
    ));
    frame.render_widget(Paragraph::new(vec![sep_line]), inner_chunks[2]);
}

/// 绘制内容区（带滚动）
fn draw_content(
    f: &mut Frame,
    app: &mut HelpApp,
    area: Rect,
    _theme: &crate::command::chat::theme::Theme,
) {
    let content_width = area.width.saturating_sub(4) as usize; // 左右各留 2 字符
    let visible_height = area.height as usize;

    // 获取渲染行（带缓存）
    let all_lines = app.current_tab_lines(content_width).to_vec();

    // 更新 total_lines 并钳制滚动
    app.clamp_scroll(visible_height);

    let scroll_offset = app.scroll_offset();

    // 给每行加左边距 "  "
    let display_lines: Vec<Line<'static>> = all_lines
        .into_iter()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|line| {
            let mut spans = vec![Span::raw("  ")];
            spans.extend(line.spans);
            Line::from(spans)
        })
        .collect();

    let paragraph = Paragraph::new(display_lines);
    f.render_widget(paragraph, area);
}

/// 绘制底部提示栏
fn draw_hint_bar(f: &mut Frame, area: Rect, theme: &crate::command::chat::theme::Theme) {
    let hints: &[(&str, &str)] = &[
        ("←→", "切换"),
        ("1-0", "跳转"),
        ("↑↓", "滚动"),
        ("q", "退出"),
    ];

    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(" ", Style::default().bg(theme.bg_title)));

    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" ", Style::default().fg(theme.hint_separator)));
        }
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default().fg(theme.hint_key_fg).bg(theme.hint_key_bg),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(theme.hint_desc),
        ));
    }

    // 填充剩余空间
    let used_width: usize = spans.iter().map(|s| display_width(&s.content)).sum();
    let fill = (area.width as usize).saturating_sub(used_width);
    if fill > 0 {
        spans.push(Span::raw(" ".repeat(fill)));
    }

    let line = Line::from(spans);
    f.render_widget(Paragraph::new(vec![line]), area);
}
