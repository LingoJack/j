//! 输入框渲染
//!
//! 提取自 chat.rs，处理用户输入框的渲染、折行、光标位置计算和 @mention 高亮。

use crate::theme::Theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::command::chat::app::{ChatApp, ChatMode};
use crate::util::text::{char_width, wrap_text};

/// 绘制输入框（支持多行、光标、@mention 高亮）
pub fn draw_input(f: &mut ratatui::Frame, area: Rect, app: &mut ChatApp) {
    let t = &app.ui.theme;

    // 浏览模式：在输入区显示过滤状态
    if app.ui.mode == ChatMode::Browse {
        draw_browse_filter(f, area, app);
        return;
    }

    // 提示符逻辑
    let (prompt, prompt_style, prompt_width) = if app.state.is_loading {
        (" + ", Style::default().fg(t.input_prompt_loading), 3)
    } else if app.ui.auto_approve {
        (" bypass > ", Style::default().fg(t.config_toggle_on), 10)
    } else {
        (" > ", Style::default().fg(t.input_prompt), 3)
    };

    let usable_width = area.width.saturating_sub(2) as usize;
    let input_text = app.ui.input_text();
    let chars: Vec<char> = input_text.chars().collect();

    // 安全检查：cursor_pos 不能超过字符数
    let cursor_pos = app.ui.cursor_char_idx().min(chars.len());

    let cursor_in_visible = cursor_pos.min(chars.len());

    let before: String = chars[..cursor_in_visible].iter().collect();
    // 当光标落在 '\n' 上时，用空格代替 '\n' 作为显示光标字符
    let (cursor_ch, after): (String, String) = if cursor_in_visible < chars.len() {
        let ch = chars[cursor_in_visible];
        if ch == '\n' {
            (" ".to_string(), chars[cursor_in_visible..].iter().collect())
        } else {
            (
                ch.to_string(),
                chars[cursor_in_visible + 1..].iter().collect(),
            )
        }
    } else {
        (" ".to_string(), String::new())
    };

    // 占位符逻辑
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
    let wrap_width = usable_width.saturating_sub(prompt_width);
    // 缓存 wrap_width 供 handler 判断视觉折行
    app.ui.input_wrap_width = wrap_width;
    let wrapped_lines = wrap_text(&full_visible, wrap_width);

    // 计算光标在 wrapped lines 中的位置
    let newlines_in_before = before.chars().filter(|&c| c == '\n').count();
    let before_len = before.chars().count() - newlines_in_before;
    let cursor_len = cursor_ch.chars().count();
    let cursor_global_pos = before_len;
    let cursor_line_idx = compute_cursor_line_idx(&wrapped_lines, cursor_global_pos);

    let line_scroll = compute_line_scroll(inner_height, &wrapped_lines, cursor_line_idx);

    // 计算 @mention 高亮范围
    let input_text_no_nl: String = input_text.chars().filter(|&c| c != '\n').collect();
    let mention_ranges = compute_mention_ranges(app, &input_text, &input_text_no_nl);
    let mention_style = Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD);

    let mut display_lines: Vec<Line> = Vec::new();
    let mut char_offset: usize = 0;
    for wl in wrapped_lines.iter().take(line_scroll) {
        char_offset += wl.chars().count();
    }

    for (line_idx, wl) in wrapped_lines
        .iter()
        .skip(line_scroll)
        .enumerate()
        .take(inner_height.max(1))
    {
        let mut spans: Vec<Span> = Vec::new();

        // 第一行显示提示符，续行显示等宽空格缩进
        if line_idx == 0 && line_scroll == 0 {
            spans.push(Span::styled(prompt, prompt_style));
        } else {
            spans.push(Span::styled("   ", Style::default()));
        }

        let line_chars: Vec<char> = wl.chars().collect();

        // 空输入时显示占位符
        if is_empty && line_idx == 0 {
            spans.push(Span::styled(placeholder, Style::default().fg(t.text_dim)));
            display_lines.push(Line::from(spans));
            continue;
        }

        let segments = build_line_segments(
            &line_chars,
            char_offset,
            before_len,
            cursor_len,
            &mention_ranges,
            mention_style,
            t,
        );
        spans.extend(segments);

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

    // 光标位置计算
    let cursor_col_in_line =
        compute_cursor_col_in_line(&wrapped_lines, line_scroll, cursor_global_pos);
    let cursor_row_in_display = (cursor_line_idx - line_scroll) as u16;
    let cursor_x = area.x + prompt_width as u16 + cursor_col_in_line;
    let cursor_y = area.y + cursor_row_in_display;

    if cursor_x < area.x + area.width && cursor_y < area.y + area.height {
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

/// 计算 @mention 高亮范围（缓存）
fn compute_mention_ranges(
    app: &ChatApp,
    input_text: &str,
    input_text_no_nl: &str,
) -> Vec<(usize, usize)> {
    if let Some((ref cached_input, ref cached_ranges)) = app.ui.cached_mention_ranges
        && cached_input == input_text
    {
        return cached_ranges.clone();
    }
    find_at_mention_ranges(input_text_no_nl)
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

/// 计算光标所在行索引
fn compute_cursor_line_idx(wrapped_lines: &[String], cursor_global_pos: usize) -> usize {
    let mut cursor_line_idx: usize = 0;
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
    cursor_line_idx
}

/// 计算行滚动偏移
fn compute_line_scroll(
    inner_height: usize,
    wrapped_lines: &[String],
    cursor_line_idx: usize,
) -> usize {
    if inner_height == 0 || wrapped_lines.len() <= inner_height || cursor_line_idx < inner_height {
        0
    } else {
        cursor_line_idx.saturating_sub(inner_height - 1)
    }
}

/// 计算光标在当前行的列位置
fn compute_cursor_col_in_line(
    wrapped_lines: &[String],
    line_scroll: usize,
    cursor_global_pos: usize,
) -> u16 {
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
}

/// 构建一行中的 Span 片段
fn build_line_segments(
    line_chars: &[char],
    char_offset: usize,
    before_len: usize,
    cursor_len: usize,
    mention_ranges: &[(usize, usize)],
    mention_style: Style,
    t: &Theme,
) -> Vec<Span<'static>> {
    let mut spans: Vec<Span> = Vec::new();
    let mut seg_start = 0;

    for (ci, &ch) in line_chars.iter().enumerate() {
        let global_idx = char_offset + ci;
        let is_cursor = global_idx >= before_len && global_idx < before_len + cursor_len;
        let is_mention = mention_ranges
            .iter()
            .any(|&(s, e)| global_idx >= s && global_idx < e);

        if is_cursor || is_mention {
            // flush segment before this char
            if ci > seg_start {
                let seg: String = line_chars[seg_start..ci].iter().collect();
                let prev_global = char_offset + seg_start;
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
            // check transition from mention to non-mention
            let prev_global = char_offset + (ci - 1);
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

    // flush remaining segment
    if seg_start < line_chars.len() {
        let seg: String = line_chars[seg_start..].iter().collect();
        let seg_global = char_offset + seg_start;
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

    spans
}

/// 浏览模式：在输入区显示过滤状态
fn draw_browse_filter(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let filtered = app.browse_filtered_indices();
    let pos = filtered.iter().position(|&i| i == app.ui.browse_msg_index);

    let mut spans: Vec<Span> = Vec::new();

    let role_label = match &app.ui.browse_role_filter {
        Some(r) if r == "ai" => "AI",
        Some(r) if r == "user" => "用户",
        _ => "全部",
    };

    spans.push(Span::styled(
        " 浏览 ",
        Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
    ));

    if let Some(p) = pos {
        spans.push(Span::styled(
            format!("{}{}/{} ", role_label, p + 1, filtered.len()),
            Style::default().fg(t.text_white),
        ));
    } else if !filtered.is_empty() {
        spans.push(Span::styled(
            format!(
                "{}{}/{} ",
                role_label,
                filtered.len(),
                app.state.session.messages.len()
            ),
            Style::default().fg(t.text_dim),
        ));
    } else {
        spans.push(Span::styled(
            format!("{}无匹配 ", role_label),
            Style::default().fg(t.toast_error_text),
        ));
    }

    if !app.ui.browse_filter.is_empty() {
        spans.push(Span::styled(
            format!("过滤: {}", app.ui.browse_filter),
            Style::default().fg(t.label_user),
        ));
        spans.push(Span::styled(
            "█",
            Style::default().fg(t.cursor_fg).bg(t.cursor_bg),
        ));
    } else {
        spans.push(Span::styled(
            "输入关键词过滤消息...",
            Style::default().fg(t.text_dim),
        ));
    }

    let widget = Paragraph::new(Line::from(spans)).style(Style::default().bg(t.bg_input));
    f.render_widget(widget, area);
}
