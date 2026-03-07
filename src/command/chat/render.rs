use super::app::{ChatApp, ChatMode, MsgLinesCache, PerMsgCache};
use super::markdown::markdown_to_lines;
use super::theme::Theme;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::io::Write;

pub fn find_stable_boundary(content: &str) -> usize {
    // 统计 ``` 出现次数，奇数说明有未闭合的代码块
    let mut fence_count = 0usize;
    let mut last_safe_boundary = 0usize;
    let mut i = 0;
    let bytes = content.as_bytes();
    while i < bytes.len() {
        // 检测 ``` 围栏
        if i + 2 < bytes.len() && bytes[i] == b'`' && bytes[i + 1] == b'`' && bytes[i + 2] == b'`' {
            fence_count += 1;
            i += 3;
            // 跳过同行剩余内容（语言标识等）
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // 检测 \n\n 段落边界
        if i + 1 < bytes.len() && bytes[i] == b'\n' && bytes[i + 1] == b'\n' {
            // 只有在代码块外才算安全边界
            if fence_count.is_multiple_of(2) {
                last_safe_boundary = i + 2; // 指向下一段的起始位置
            }
            i += 2;
            continue;
        }
        i += 1;
    }
    last_safe_boundary
}

/// 增量构建所有消息的渲染行（P0 + P1 优化版本）
/// - P0：按消息粒度缓存，历史消息内容未变时直接复用渲染行
/// - P1：流式消息增量段落渲染，只重新解析最后一个不完整段落
///   返回 (渲染行列表, 消息起始行号映射, 按消息缓存, 流式稳定行缓存, 流式稳定偏移)
#[allow(clippy::type_complexity)]
pub fn build_message_lines_incremental(
    app: &ChatApp,
    inner_width: usize,
    bubble_max_width: usize,
    old_cache: Option<&MsgLinesCache>,
) -> (
    Vec<Line<'static>>,
    Vec<(usize, usize)>,
    Vec<PerMsgCache>,
    Vec<Line<'static>>,
    usize,
) {
    struct RenderMsg {
        role: String,
        content: String,
        msg_index: Option<usize>,
        tool_calls: Option<Vec<super::model::ToolCallItem>>,
        role_label: Option<String>,
    }
    let mut render_msgs: Vec<RenderMsg> = app
        .session
        .messages
        .iter()
        .enumerate()
        .map(|(i, m)| RenderMsg {
            role: m.role.clone(),
            content: m.content.clone(),
            msg_index: Some(i),
            tool_calls: m.tool_calls.clone(),
            role_label: m
                .tool_call_id
                .as_ref()
                .map(|id| format!("工具 {}", &id[..id.len().min(8)])),
        })
        .collect();

    // 如果正在流式接收，添加一条临时的 assistant 消息
    let streaming_content_str = if app.is_loading {
        let streaming = app.streaming_content.lock().unwrap().clone();
        if !streaming.is_empty() {
            render_msgs.push(RenderMsg {
                role: "assistant".to_string(),
                content: streaming.clone(),
                msg_index: None,
                tool_calls: None,
                role_label: None,
            });
            Some(streaming)
        } else {
            render_msgs.push(RenderMsg {
                role: "assistant".to_string(),
                content: "◍".to_string(),
                msg_index: None,
                tool_calls: None,
                role_label: None,
            });
            None
        }
    } else {
        None
    };

    let t = &app.theme;
    let is_browse_mode = app.mode == ChatMode::Browse;
    let mut lines: Vec<Line> = Vec::new();
    let mut msg_start_lines: Vec<(usize, usize)> = Vec::new();
    let mut per_msg_cache: Vec<PerMsgCache> = Vec::new();

    // 判断旧缓存中的 per_msg_lines 是否可以复用（bubble_max_width 相同且浏览模式状态一致）
    let can_reuse_per_msg = old_cache
        .map(|c| c.bubble_max_width == bubble_max_width)
        .unwrap_or(false);

    for msg in &render_msgs {
        let is_selected = is_browse_mode
            && msg.msg_index.is_some()
            && msg.msg_index.unwrap() == app.browse_msg_index;

        // 记录消息起始行号
        if let Some(idx) = msg.msg_index {
            msg_start_lines.push((idx, lines.len()));
        }

        // P0 优化：对于有 msg_index 的历史消息，尝试复用旧缓存
        if let Some(idx) = msg.msg_index
            && can_reuse_per_msg
            && let Some(old_c) = old_cache
        {
            // 查找旧缓存中同索引的消息
            if let Some(old_per) = old_c.per_msg_lines.iter().find(|p| p.msg_index == idx) {
                // 内容长度相同 → 消息内容未变，且浏览选中状态一致
                // 使用缓存中记录的 is_selected 字段来判断
                if old_per.content_len == msg.content.len() && old_per.is_selected == is_selected {
                    // 直接复用旧缓存的渲染行
                    lines.extend(old_per.lines.iter().cloned());
                    per_msg_cache.push(PerMsgCache {
                        content_len: old_per.content_len,
                        lines: old_per.lines.clone(),
                        msg_index: idx,
                        is_selected,
                    });
                    continue;
                }
            }
        }

        // 缓存未命中 / 流式消息 → 重新渲染
        let msg_lines_start = lines.len();
        match msg.role.as_str() {
            "user" => {
                render_user_msg(
                    &msg.content,
                    is_selected,
                    inner_width,
                    bubble_max_width,
                    &mut lines,
                    t,
                );
            }
            "assistant" => {
                if msg.msg_index.is_none() {
                    // 流式消息：P1 增量段落渲染（在后面单独处理）
                    // 这里先跳过，后面统一处理
                    // 先标记位置
                } else if msg.tool_calls.is_some() {
                    // assistant 发起工具调用的消息
                    render_tool_call_request_msg(
                        msg.tool_calls.as_ref().unwrap(),
                        bubble_max_width,
                        &mut lines,
                        t,
                    );
                } else {
                    // 已完成的 assistant 消息：完整 Markdown 渲染
                    render_assistant_msg(
                        &msg.content,
                        is_selected,
                        bubble_max_width,
                        &mut lines,
                        t,
                    );
                }
            }
            "tool" => {
                render_tool_result_msg(
                    &msg.content,
                    msg.role_label.as_deref().unwrap_or("工具结果"),
                    &mut lines,
                    t,
                );
            }
            "system" => {
                lines.push(Line::from(""));
                let wrapped = wrap_text(&msg.content, inner_width.saturating_sub(8));
                for wl in wrapped {
                    lines.push(Line::from(Span::styled(
                        format!("    {}  {}", "sys", wl),
                        Style::default().fg(t.text_system),
                    )));
                }
            }
            _ => {}
        }

        // 流式消息的渲染在 assistant 分支中被跳过了，这里处理
        if msg.role == "assistant" && msg.msg_index.is_none() {
            // P1 增量段落渲染
            let bubble_bg = t.bubble_ai;
            let pad_left_w = 3usize;
            let pad_right_w = 3usize;
            let md_content_w = bubble_max_width.saturating_sub(pad_left_w + pad_right_w);
            let bubble_total_w = bubble_max_width;

            // AI 标签
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  AI",
                Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
            )));

            // 上边距
            lines.push(Line::from(vec![Span::styled(
                " ".repeat(bubble_total_w),
                Style::default().bg(bubble_bg),
            )]));

            // 增量段落渲染：取旧缓存中的 stable_lines 和 stable_offset
            let (mut stable_lines, mut stable_offset) = if let Some(old_c) = old_cache {
                if old_c.bubble_max_width == bubble_max_width {
                    (
                        old_c.streaming_stable_lines.clone(),
                        old_c.streaming_stable_offset,
                    )
                } else {
                    (Vec::<Line<'static>>::new(), 0)
                }
            } else {
                (Vec::<Line<'static>>::new(), 0)
            };

            let content = &msg.content;
            // 找到当前内容中最后一个安全的段落边界
            let boundary = find_stable_boundary(content);

            // 如果有新的完整段落超过了上次缓存的偏移
            if boundary > stable_offset {
                // 增量解析：从上次偏移到新边界的新完成段落
                let new_stable_text = &content[stable_offset..boundary];
                let new_md_lines = markdown_to_lines(new_stable_text, md_content_w + 2, t);
                // 将新段落的渲染行包装成气泡样式并追加到 stable_lines
                for md_line in new_md_lines {
                    let bubble_line = wrap_md_line_in_bubble(
                        md_line,
                        bubble_bg,
                        pad_left_w,
                        pad_right_w,
                        bubble_total_w,
                    );
                    stable_lines.push(bubble_line);
                }
                stable_offset = boundary;
            }

            // 追加已缓存的稳定段落行
            lines.extend(stable_lines.iter().cloned());

            // 只对最后一个不完整段落做全量 Markdown 解析
            let tail = &content[boundary..];
            if !tail.is_empty() {
                let tail_md_lines = markdown_to_lines(tail, md_content_w + 2, t);
                for md_line in tail_md_lines {
                    let bubble_line = wrap_md_line_in_bubble(
                        md_line,
                        bubble_bg,
                        pad_left_w,
                        pad_right_w,
                        bubble_total_w,
                    );
                    lines.push(bubble_line);
                }
            }

            // 下边距
            lines.push(Line::from(vec![Span::styled(
                " ".repeat(bubble_total_w),
                Style::default().bg(bubble_bg),
            )]));

            // 记录最终的 stable 状态用于返回
            // （在函数末尾统一返回）
            // 先用局部变量暂存
            let _ = (stable_lines.clone(), stable_offset);

            // 构建末尾留白和返回值时统一处理
        } else if let Some(idx) = msg.msg_index {
            // 缓存此历史消息的渲染行
            let msg_lines_end = lines.len();
            let this_msg_lines: Vec<Line<'static>> = lines[msg_lines_start..msg_lines_end].to_vec();
            let is_selected = is_browse_mode
                && msg.msg_index.is_some()
                && msg.msg_index.unwrap() == app.browse_msg_index;
            per_msg_cache.push(PerMsgCache {
                content_len: msg.content.len(),
                lines: this_msg_lines,
                msg_index: idx,
                is_selected,
            });
        }
    }

    // ========== 内联工具确认区 ==========
    if (app.mode == ChatMode::ToolConfirm || app.mode == ChatMode::ToolRejectInput)
        && let Some(tc) = app.active_tool_calls.get(app.pending_tool_idx)
    {
        let t = &app.theme;
        let confirm_bg = t.tool_confirm_bg;
        let border_color = t.tool_confirm_border;
        let content_w = bubble_max_width.saturating_sub(6); // 左右各 3 的 padding

        // 空行
        lines.push(Line::from(""));

        // 标题行
        lines.push(Line::from(Span::styled(
            "  🔧 工具调用确认",
            Style::default()
                .fg(t.tool_confirm_title)
                .add_modifier(Modifier::BOLD),
        )));

        // 顶边框
        let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
        lines.push(Line::from(Span::styled(
            top_border,
            Style::default().fg(border_color).bg(confirm_bg),
        )));

        // 工具名行
        {
            let label = "工具: ";
            let name = &tc.tool_name;
            let text_content = format!("{}{}", label, name);
            let fill = content_w.saturating_sub(display_width(&text_content));
            lines.push(Line::from(vec![
                Span::styled("  │ ", Style::default().fg(border_color).bg(confirm_bg)),
                Span::styled(" ".to_string(), Style::default().bg(confirm_bg)),
                Span::styled(
                    label,
                    Style::default().fg(t.tool_confirm_label).bg(confirm_bg),
                ),
                Span::styled(
                    name.clone(),
                    Style::default()
                        .fg(t.tool_confirm_name)
                        .bg(confirm_bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " ".repeat(fill.saturating_sub(1)),
                    Style::default().bg(confirm_bg),
                ),
                Span::styled(" │", Style::default().fg(border_color).bg(confirm_bg)),
            ]));
        }

        // 确认信息行（折行显示，最多 10 行）
        {
            let max_msg_w = content_w.saturating_sub(2);
            let wrapped = wrap_text(&tc.confirm_message, max_msg_w);
            let max_lines = 10;
            let show_lines = wrapped.len().min(max_lines);
            for (i, line_text) in wrapped.iter().enumerate().take(show_lines) {
                let display_text = if i == max_lines - 1 && wrapped.len() > max_lines {
                    format!("{}...", line_text)
                } else {
                    line_text.clone()
                };
                let msg_w = display_width(&display_text);
                let fill = content_w.saturating_sub(msg_w + 2);
                lines.push(Line::from(vec![
                    Span::styled("  │ ", Style::default().fg(border_color).bg(confirm_bg)),
                    Span::styled(" ".to_string(), Style::default().bg(confirm_bg)),
                    Span::styled(
                        display_text,
                        Style::default().fg(t.tool_confirm_text).bg(confirm_bg),
                    ),
                    Span::styled(
                        " ".repeat(fill.saturating_sub(1).saturating_add(2)),
                        Style::default().bg(confirm_bg),
                    ),
                    Span::styled(" │", Style::default().fg(border_color).bg(confirm_bg)),
                ]));
            }
        }

        // 空行
        {
            let fill = bubble_max_width.saturating_sub(4);
            lines.push(Line::from(vec![
                Span::styled("  │", Style::default().fg(border_color).bg(confirm_bg)),
                Span::styled(" ".repeat(fill), Style::default().bg(confirm_bg)),
                Span::styled("│", Style::default().fg(border_color).bg(confirm_bg)),
            ]));
        }

        // 操作提示行（根据模式显示不同内容）
        {
            if app.mode == ChatMode::ToolRejectInput {
                // 拒绝输入模式：提示用户输入原因
                let hint = "输入拒绝原因（可为空）  [Enter] 发送  /  [Esc] 取消";
                let hint_text_w = display_width(hint);
                let fill = content_w.saturating_sub(hint_text_w + 2);
                lines.push(Line::from(vec![
                    Span::styled("  │ ", Style::default().fg(border_color).bg(confirm_bg)),
                    Span::styled(" ".to_string(), Style::default().bg(confirm_bg)),
                    Span::styled(
                        hint,
                        Style::default()
                            .fg(t.title_loading)
                            .bg(confirm_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " ".repeat(fill.saturating_sub(1).saturating_add(2)),
                        Style::default().bg(confirm_bg),
                    ),
                    Span::styled(" │", Style::default().fg(border_color).bg(confirm_bg)),
                ]));
            } else {
                // 确认模式：Y/N/Esc + 倒计时
                let countdown_text = if app.agent_config.tool_confirm_timeout > 0 {
                    let elapsed = app.tool_confirm_entered_at.elapsed().as_secs();
                    let remaining = app
                        .agent_config
                        .tool_confirm_timeout
                        .saturating_sub(elapsed);
                    format!("  ({}s 后自动执行)", remaining)
                } else {
                    String::new()
                };
                let base_hint = "[Y/Enter] 执行  /  [N] 拒绝  /  [Esc] 直接拒绝";
                let full_hint = format!("{}{}", base_hint, countdown_text);
                let hint_text_w = display_width(&full_hint);
                let fill = content_w.saturating_sub(hint_text_w + 2);
                let mut spans = vec![
                    Span::styled("  │ ", Style::default().fg(border_color).bg(confirm_bg)),
                    Span::styled(" ".to_string(), Style::default().bg(confirm_bg)),
                    Span::styled(
                        "[Y/Enter] 执行",
                        Style::default()
                            .fg(t.toast_success_border)
                            .bg(confirm_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "  /  ",
                        Style::default().fg(t.tool_confirm_label).bg(confirm_bg),
                    ),
                    Span::styled(
                        "[N] 拒绝",
                        Style::default()
                            .fg(t.toast_error_border)
                            .bg(confirm_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "  /  ",
                        Style::default().fg(t.tool_confirm_label).bg(confirm_bg),
                    ),
                    Span::styled(
                        "[Esc] 直接拒绝",
                        Style::default()
                            .fg(t.toast_error_border)
                            .bg(confirm_bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                ];
                if !countdown_text.is_empty() {
                    spans.push(Span::styled(
                        countdown_text,
                        Style::default()
                            .fg(t.title_loading)
                            .bg(confirm_bg)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                spans.push(Span::styled(
                    " ".repeat(fill.saturating_sub(1).saturating_add(2)),
                    Style::default().bg(confirm_bg),
                ));
                spans.push(Span::styled(
                    " │",
                    Style::default().fg(border_color).bg(confirm_bg),
                ));
                lines.push(Line::from(spans));
            }
        }

        // 底边框
        let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
        lines.push(Line::from(Span::styled(
            bottom_border,
            Style::default().fg(border_color).bg(confirm_bg),
        )));
    }

    // 末尾留白
    lines.push(Line::from(""));

    // 计算最终的流式稳定缓存
    let (final_stable_lines, final_stable_offset) = if let Some(sc) = &streaming_content_str {
        let boundary = find_stable_boundary(sc);
        let bubble_bg = t.bubble_ai;
        let pad_left_w = 3usize;
        let pad_right_w = 3usize;
        let md_content_w = bubble_max_width.saturating_sub(pad_left_w + pad_right_w);
        let bubble_total_w = bubble_max_width;

        let (mut s_lines, s_offset) = if let Some(old_c) = old_cache {
            if old_c.bubble_max_width == bubble_max_width {
                (
                    old_c.streaming_stable_lines.clone(),
                    old_c.streaming_stable_offset,
                )
            } else {
                (Vec::<Line<'static>>::new(), 0)
            }
        } else {
            (Vec::<Line<'static>>::new(), 0)
        };

        if boundary > s_offset {
            let new_text = &sc[s_offset..boundary];
            let new_md_lines = markdown_to_lines(new_text, md_content_w + 2, t);
            for md_line in new_md_lines {
                let bubble_line = wrap_md_line_in_bubble(
                    md_line,
                    bubble_bg,
                    pad_left_w,
                    pad_right_w,
                    bubble_total_w,
                );
                s_lines.push(bubble_line);
            }
        }
        (s_lines, boundary)
    } else {
        (Vec::new(), 0)
    };

    (
        lines,
        msg_start_lines,
        per_msg_cache,
        final_stable_lines,
        final_stable_offset,
    )
}

/// 将一行 Markdown 渲染结果包装成气泡样式行（左右内边距 + 背景色 + 填充到统一宽度）
pub fn wrap_md_line_in_bubble(
    md_line: Line<'static>,
    bubble_bg: Color,
    pad_left_w: usize,
    pad_right_w: usize,
    bubble_total_w: usize,
) -> Line<'static> {
    let pad_left = " ".repeat(pad_left_w);
    let pad_right = " ".repeat(pad_right_w);
    let mut styled_spans: Vec<Span> = Vec::new();
    styled_spans.push(Span::styled(pad_left, Style::default().bg(bubble_bg)));
    let target_content_w = bubble_total_w.saturating_sub(pad_left_w + pad_right_w);
    let mut content_w: usize = 0;
    for span in md_line.spans {
        let sw = display_width(&span.content);
        if content_w + sw > target_content_w {
            // 安全钳制：逐字符截断以适应目标宽度
            let remaining = target_content_w.saturating_sub(content_w);
            if remaining > 0 {
                let mut truncated = String::new();
                let mut tw = 0;
                for ch in span.content.chars() {
                    let cw = char_width(ch);
                    if tw + cw > remaining {
                        break;
                    }
                    truncated.push(ch);
                    tw += cw;
                }
                if !truncated.is_empty() {
                    content_w += tw;
                    let merged_style = span.style.bg(bubble_bg);
                    styled_spans.push(Span::styled(truncated, merged_style));
                }
            }
            // 跳过后续 span（已溢出）
            break;
        }
        content_w += sw;
        let merged_style = span.style.bg(bubble_bg);
        styled_spans.push(Span::styled(span.content.to_string(), merged_style));
    }
    let fill = target_content_w.saturating_sub(content_w);
    if fill > 0 {
        styled_spans.push(Span::styled(
            " ".repeat(fill),
            Style::default().bg(bubble_bg),
        ));
    }
    styled_spans.push(Span::styled(pad_right, Style::default().bg(bubble_bg)));
    Line::from(styled_spans)
}

/// 渲染用户消息
pub fn render_user_msg(
    content: &str,
    is_selected: bool,
    inner_width: usize,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    lines.push(Line::from(""));
    let label = if is_selected { "▶ You " } else { "You " };
    let pad = inner_width.saturating_sub(display_width(label) + 2);
    lines.push(Line::from(vec![
        Span::raw(" ".repeat(pad)),
        Span::styled(
            label,
            Style::default()
                .fg(if is_selected {
                    theme.label_selected
                } else {
                    theme.label_user
                })
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    let user_bg = if is_selected {
        theme.bubble_user_selected
    } else {
        theme.bubble_user
    };
    let user_pad_lr = 3usize;
    let user_content_w = bubble_max_width.saturating_sub(user_pad_lr * 2);
    let mut all_wrapped_lines: Vec<String> = Vec::new();
    for content_line in content.lines() {
        let wrapped = wrap_text(content_line, user_content_w);
        all_wrapped_lines.extend(wrapped);
    }
    if all_wrapped_lines.is_empty() {
        all_wrapped_lines.push(String::new());
    }
    let actual_content_w = all_wrapped_lines
        .iter()
        .map(|l| display_width(l))
        .max()
        .unwrap_or(0);
    let actual_bubble_w = (actual_content_w + user_pad_lr * 2)
        .min(bubble_max_width)
        .max(user_pad_lr * 2 + 1);
    let actual_inner_content_w = actual_bubble_w.saturating_sub(user_pad_lr * 2);
    // 上边距
    {
        let bubble_text = " ".repeat(actual_bubble_w);
        let pad = inner_width.saturating_sub(actual_bubble_w);
        lines.push(Line::from(vec![
            Span::raw(" ".repeat(pad)),
            Span::styled(bubble_text, Style::default().bg(user_bg)),
        ]));
    }
    for wl in &all_wrapped_lines {
        let wl_width = display_width(wl);
        let fill = actual_inner_content_w.saturating_sub(wl_width);
        let text = format!(
            "{}{}{}{}",
            " ".repeat(user_pad_lr),
            wl,
            " ".repeat(fill),
            " ".repeat(user_pad_lr),
        );
        let text_width = display_width(&text);
        let pad = inner_width.saturating_sub(text_width);
        lines.push(Line::from(vec![
            Span::raw(" ".repeat(pad)),
            Span::styled(text, Style::default().fg(theme.text_white).bg(user_bg)),
        ]));
    }
    // 下边距
    {
        let bubble_text = " ".repeat(actual_bubble_w);
        let pad = inner_width.saturating_sub(actual_bubble_w);
        lines.push(Line::from(vec![
            Span::raw(" ".repeat(pad)),
            Span::styled(bubble_text, Style::default().bg(user_bg)),
        ]));
    }
}

/// 渲染 AI 助手消息
pub fn render_assistant_msg(
    content: &str,
    is_selected: bool,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    lines.push(Line::from(""));
    let ai_label = if is_selected { "  ▶ AI" } else { "  AI" };
    lines.push(Line::from(Span::styled(
        ai_label,
        Style::default()
            .fg(if is_selected {
                theme.label_selected
            } else {
                theme.label_ai
            })
            .add_modifier(Modifier::BOLD),
    )));
    let bubble_bg = if is_selected {
        theme.bubble_ai_selected
    } else {
        theme.bubble_ai
    };
    let pad_left_w = 3usize;
    let pad_right_w = 3usize;
    let md_content_w = bubble_max_width.saturating_sub(pad_left_w + pad_right_w);
    let md_lines = markdown_to_lines(content, md_content_w + 2, theme);
    let bubble_total_w = bubble_max_width;
    // 上边距
    lines.push(Line::from(vec![Span::styled(
        " ".repeat(bubble_total_w),
        Style::default().bg(bubble_bg),
    )]));
    for md_line in md_lines {
        let bubble_line =
            wrap_md_line_in_bubble(md_line, bubble_bg, pad_left_w, pad_right_w, bubble_total_w);
        lines.push(bubble_line);
    }
    // 下边距
    lines.push(Line::from(vec![Span::styled(
        " ".repeat(bubble_total_w),
        Style::default().bg(bubble_bg),
    )]));
}

/// 将 Markdown 文本解析为 ratatui 的 Line 列表
/// 支持：标题（去掉 # 标记）、加粗、斜体、行内代码、代码块（语法高亮）、列表、分隔线
/// content_width：内容区可用宽度（不含外层 "  " 缩进和右侧填充）
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    // 最小宽度保证至少能放下一个字符（中文字符宽度2），避免无限循环或不截断
    let max_width = max_width.max(2);
    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
        let ch_width = char_width(ch);
        if current_width + ch_width > max_width && !current_line.is_empty() {
            result.push(current_line.clone());
            current_line.clear();
            current_width = 0;
        }
        current_line.push(ch);
        current_width += ch_width;
    }
    if !current_line.is_empty() {
        result.push(current_line);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

/// 计算字符串的显示宽度（使用 unicode-width crate，比手动范围匹配更准确）
pub fn display_width(s: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    UnicodeWidthStr::width(s)
}

/// 计算单个字符的显示宽度（使用 unicode-width crate）
pub fn char_width(c: char) -> usize {
    use unicode_width::UnicodeWidthChar;
    UnicodeWidthChar::width(c).unwrap_or(0)
}

/// 渲染工具调用请求消息（AI 发起）：黄色标签 + 工具名和参数摘要
pub fn render_tool_call_request_msg(
    tool_calls: &[super::model::ToolCallItem],
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  🔧 AI 调用工具",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    let bubble_bg = Color::Rgb(40, 35, 10);
    let pad = 3usize;
    let content_w = bubble_max_width.saturating_sub(pad * 2);
    lines.push(Line::from(vec![Span::styled(
        " ".repeat(bubble_max_width),
        Style::default().bg(bubble_bg),
    )]));
    for tc in tool_calls {
        let args_preview: String = tc.arguments.chars().take(50).collect();
        let args_display = if tc.arguments.len() > 50 {
            format!("{}...", args_preview)
        } else {
            args_preview
        };
        let text = format!("{} ({})", tc.name, args_display);
        let wrapped = wrap_text(&text, content_w);
        for wl in wrapped {
            let fill = content_w.saturating_sub(display_width(&wl));
            lines.push(Line::from(vec![
                Span::styled(" ".repeat(pad), Style::default().bg(bubble_bg)),
                Span::styled(wl, Style::default().fg(Color::Yellow).bg(bubble_bg)),
                Span::styled(" ".repeat(fill), Style::default().bg(bubble_bg)),
                Span::styled(" ".repeat(pad), Style::default().bg(bubble_bg)),
            ]));
        }
    }
    let _ = theme; // 保留参数以便未来扩展
    lines.push(Line::from(vec![Span::styled(
        " ".repeat(bubble_max_width),
        Style::default().bg(bubble_bg),
    )]));
}

/// 渲染工具执行结果消息：绿色标签 + 截断内容（最多 5 行）
pub fn render_tool_result_msg(
    content: &str,
    label: &str,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  ✅ {}", label),
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )));
    let bubble_bg = Color::Rgb(10, 40, 15);
    let pad = 3usize;
    let content_w = 60usize;
    let bubble_w = content_w + pad * 2;
    lines.push(Line::from(vec![Span::styled(
        " ".repeat(bubble_w),
        Style::default().bg(bubble_bg),
    )]));
    let display_content = if content.len() > 200 {
        let mut end = 200;
        while !content.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &content[..end])
    } else {
        content.to_string()
    };
    let all_lines: Vec<String> = display_content
        .lines()
        .flat_map(|l| wrap_text(l, content_w))
        .take(5)
        .collect();
    for wl in all_lines {
        let fill = content_w.saturating_sub(display_width(&wl));
        lines.push(Line::from(vec![
            Span::styled(" ".repeat(pad), Style::default().bg(bubble_bg)),
            Span::styled(
                wl,
                Style::default().fg(Color::Rgb(180, 255, 180)).bg(bubble_bg),
            ),
            Span::styled(" ".repeat(fill), Style::default().bg(bubble_bg)),
            Span::styled(" ".repeat(pad), Style::default().bg(bubble_bg)),
        ]));
    }
    let _ = theme;
    lines.push(Line::from(vec![Span::styled(
        " ".repeat(bubble_w),
        Style::default().bg(bubble_bg),
    )]));
}

pub fn copy_to_clipboard(content: &str) -> bool {
    use std::process::{Command, Stdio};

    let (cmd, args): (&str, Vec<&str>) = if cfg!(target_os = "macos") {
        ("pbcopy", vec![])
    } else if cfg!(target_os = "linux") {
        if Command::new("which")
            .arg("xclip")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            ("xclip", vec!["-selection", "clipboard"])
        } else {
            ("xsel", vec!["--clipboard", "--input"])
        }
    } else {
        return false;
    };

    let child = Command::new(cmd).args(&args).stdin(Stdio::piped()).spawn();

    match child {
        Ok(mut child) => {
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(content.as_bytes());
            }
            child.wait().map(|s| s.success()).unwrap_or(false)
        }
        Err(_) => false,
    }
}
