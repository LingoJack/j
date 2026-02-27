use super::app::HelpApp;
use crate::command::chat::render::display_width;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// 绘制帮助 TUI 界面
pub fn draw_help_ui(f: &mut Frame, app: &mut HelpApp) {
    let size = f.area();
    let theme = app.theme().clone();

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

    draw_tab_bar(f, app, chunks[0], &theme);
    draw_title_bar(f, app, chunks[1], &theme);
    draw_content(f, app, chunks[2], &theme);
    draw_hint_bar(f, chunks[3], &theme);
}

/// 绘制 Tab 栏
fn draw_tab_bar(
    f: &mut Frame,
    app: &HelpApp,
    area: Rect,
    theme: &crate::command::chat::theme::Theme,
) {
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(" ", Style::default().bg(theme.bg_title)));

    for i in 0..app.tab_count {
        let num = if i == 9 {
            "0".to_string()
        } else {
            format!("{}", i + 1)
        };
        let label = format!(" {}.{} ", num, app.tab_name(i));

        if i == app.active_tab {
            spans.push(Span::styled(
                label,
                Style::default()
                    .fg(theme.config_tab_active_fg)
                    .bg(theme.config_tab_active_bg)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                label,
                Style::default()
                    .fg(theme.config_tab_inactive)
                    .bg(theme.bg_title),
            ));
        }
        spans.push(Span::styled(" ", Style::default().bg(theme.bg_title)));
    }

    // 填充剩余空间
    let used_width: usize = spans.iter().map(|s| display_width(&s.content)).sum();
    let fill = (area.width as usize).saturating_sub(used_width);
    if fill > 0 {
        spans.push(Span::styled(
            " ".repeat(fill),
            Style::default().bg(theme.bg_title),
        ));
    }

    let line = Line::from(spans);
    f.render_widget(Paragraph::new(vec![line]), area);
}

/// 绘制标题栏
fn draw_title_bar(
    f: &mut Frame,
    app: &HelpApp,
    area: Rect,
    theme: &crate::command::chat::theme::Theme,
) {
    let title_text = format!("  📖 j help — {}", app.tab_name(app.active_tab));
    let page_info = format!("{}/{}  ", app.active_tab + 1, app.tab_count);

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
    f.render_widget(Paragraph::new(vec![Line::from("")]), inner_chunks[0]);

    // 标题内容
    f.render_widget(Paragraph::new(vec![Line::from(spans)]), inner_chunks[1]);

    // 分隔线
    let sep_width = area.width as usize;
    let sep_line = Line::from(Span::styled(
        "─".repeat(sep_width),
        Style::default().fg(theme.separator),
    ));
    f.render_widget(Paragraph::new(vec![sep_line]), inner_chunks[2]);
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
        ("PgUp/Dn", "翻页"),
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
