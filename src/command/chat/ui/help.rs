//! 帮助页面
//!
//! 提取自 chat.rs，显示快捷键帮助信息。
//! 支持鼠标选区复制。

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::command::chat::app::ChatApp;
use crate::command::chat::storage::agent_config_path;
use crate::tui::components;
use crate::tui::components::selection::{
    compute_line_selection_range, rebuild_spans_with_selection,
};

/// 构建帮助页面的渲染行
fn build_help_lines(app: &ChatApp) -> Vec<Line<'static>> {
    let t = &app.ui.theme;
    let sep = components::separator_line(u16::MAX, t);

    let shortcuts: &[(&str, &str)] = &[
        ("Enter", "发送消息"),
        ("Shift/Alt+Enter", "输入框内换行"),
        ("↑ / ↓", "滚动对话记录"),
        ("← / →", "移动输入光标"),
        ("Home / End", "光标跳到行首/行尾"),
        (
            "/",
            "斜杠命令（copy/log/browse/config/model/archive/theme/resume）",
        ),
        ("@", "引用（skill/file/command）"),
        ("Ctrl+O", "展开/折叠工具详情"),
        ("Esc / Ctrl+C", "退出对话"),
        ("? / F1", "显示 / 关闭此帮助"),
    ];

    let mut lines = vec![
        Line::from(Span::styled(
            " 帮助 (按任意键返回)",
            Style::default().fg(t.text_dim),
        )),
        Line::from(""),
        components::section_header("📖", "快捷键帮助", t),
        Line::from(""),
        sep.clone(),
        Line::from(""),
    ];
    for (key, desc) in shortcuts {
        lines.push(components::help_key_row(key, desc, 15, t));
    }
    lines.push(Line::from(""));
    lines.push(sep);
    lines.push(Line::from(""));
    lines.push(components::section_header("📁", "配置文件:", t));
    lines.push(Line::from(Span::styled(
        format!("     {}", agent_config_path().display()),
        Style::default().fg(t.help_path),
    )));

    lines
}

/// 将屏幕坐标转换为 (全局行号, 行内字符偏移)
/// 用于 Help 页面的鼠标选区
pub fn help_screen_to_text_pos(
    screen_x: u16,
    screen_y: u16,
    inner: Rect,
    scroll_offset: usize,
    lines: &[Line<'static>],
) -> Option<(usize, usize)> {
    let local_y = screen_y.saturating_sub(inner.y) as usize;
    if local_y >= inner.height as usize {
        return None;
    }
    let global_line = scroll_offset + local_y;
    if global_line >= lines.len() {
        return None;
    }

    let local_x = screen_x.saturating_sub(inner.x) as usize;
    // 计算 line 内字符偏移：遍历 spans 累加宽度
    let line = &lines[global_line];
    let mut char_offset = 0usize;
    for span in &line.spans {
        let span_len = span.content.chars().count();
        if local_x < char_offset + span_len {
            char_offset = local_x;
            break;
        }
        char_offset += span_len;
    }
    // 如果 x 超出行宽，取行末
    let line_width: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
    char_offset = char_offset.min(line_width);

    Some((global_line, char_offset))
}

/// 从缓存的渲染行提取选区文本
pub fn help_extract_selection_text(
    lines: &[Line<'static>],
    anchor: (usize, usize),
    current: (usize, usize),
) -> String {
    let (start, end) = if anchor.0 <= current.0 {
        (anchor, current)
    } else {
        (current, anchor)
    };

    let end_line = end.0.min(lines.len() - 1);
    let mut result = String::new();
    for (idx, line) in lines[start.0..=end_line].iter().enumerate() {
        let i = start.0 + idx;
        if i > start.0 {
            result.push('\n');
        }
        let line_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        let chars: Vec<char> = line_text.chars().collect();
        let s = if i == start.0 { start.1 } else { 0 };
        let e = if i == end.0 {
            end.1.min(chars.len())
        } else {
            chars.len()
        };
        if s < e {
            let slice: String = chars[s..e].iter().collect();
            result.push_str(&slice);
        }
    }
    result
}

/// 绘制帮助界面
pub fn draw_help(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let t = &app.ui.theme;
    let lines = build_help_lines(app);
    let bg = Style::default().bg(t.help_bg);
    let total_lines = lines.len();
    let visible_height = area.height as usize;

    // 滚动偏移裁剪
    let scroll = app.ui.help_scroll_offset;
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = scroll.min(max_scroll);

    // 应用选区高亮
    let selection = app.ui.mouse_selection.as_ref();
    let display_lines: Vec<Line<'_>> = if let Some(sel) = selection {
        let visible_end = (scroll + visible_height).min(total_lines);
        lines[scroll..visible_end]
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let line_idx = scroll + i;
                let (sel_start, sel_end) =
                    compute_line_selection_range(line_idx, sel.anchor, sel.current);
                if sel_start < sel_end {
                    rebuild_spans_with_selection(
                        &line.spans,
                        0,
                        sel_start,
                        sel_end,
                        Color::White,
                        Color::DarkGray,
                    )
                } else {
                    line.spans.to_vec()
                }
            })
            .map(Line::from)
            .collect()
    } else {
        lines[scroll..(scroll + visible_height).min(total_lines)].to_vec()
    };

    let help_widget = Paragraph::new(display_lines).style(bg);
    f.render_widget(help_widget, area);

    // 缓存渲染数据
    app.ui.help_lines_cache = Some(lines);
    app.ui.help_area_inner = Some(area);
    app.ui.help_scroll_offset = scroll;
}
