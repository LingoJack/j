use super::super::app::{ChatApp, ChatMode, MsgLinesCache, PerMsgCache};
use super::super::markdown::markdown_to_lines;
use super::theme::Theme;
use crate::command::chat::constants::{ROLE_ASSISTANT, ROLE_SYSTEM, ROLE_TOOL, ROLE_USER};
use crate::util::safe_lock;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::io::Write;
use std::sync::Arc;

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

/// 增量构建所有消息的渲染行（P0 + P1 + P2 优化版本）
/// - P0：按消息粒度缓存，历史消息内容未变时直接复用渲染行
/// - P1：流式消息增量段落渲染，只重新解析最后一个不完整段落
/// - P2：不再组装扁平 lines Vec，draw_messages 直接索引 per_msg_lines + streaming_lines
///   返回 (消息起始行号映射, 按消息缓存, 流式渲染行, 流式稳定行缓存, 流式稳定偏移)
#[allow(clippy::type_complexity)]
pub fn build_message_lines_incremental(
    app: &ChatApp,
    inner_width: usize,
    bubble_max_width: usize,
    old_cache: Option<&MsgLinesCache>,
) -> (
    Vec<(usize, usize)>,
    Vec<PerMsgCache>,
    Vec<Line<'static>>,
    Arc<Vec<Line<'static>>>,
    usize,
) {
    // 获取流式内容（只 lock 一次，尽快释放锁）
    let streaming_content_str = if app.state.is_loading {
        let streaming: String = safe_lock(
            &app.state.streaming_content,
            "render_cache::streaming_content",
        )
        .clone();
        if !streaming.is_empty() {
            Some(streaming)
        } else {
            None
        }
    } else {
        None
    };

    let t = &app.ui.theme;
    let is_browse_mode = app.ui.mode == ChatMode::Browse;
    let mut current_line_offset: usize = 0;
    let mut msg_start_lines: Vec<(usize, usize)> = Vec::new();
    let mut per_msg_cache: Vec<PerMsgCache> = Vec::new();

    let expand = app.ui.expand_tools;

    // 判断旧缓存中的 per_msg_lines 是否可以复用（bubble_max_width 相同且 expand 一致）
    let can_reuse_per_msg = old_cache
        .map(|c| c.bubble_max_width == bubble_max_width && c.expand_tools == expand)
        .unwrap_or(false);

    // ===== P0 优化：直接引用 session.messages，避免克隆全部内容 =====
    // 缓存命中时零拷贝复用，只在缓存未命中时才访问消息内容
    let msg_count = app.state.session.messages.len();
    for idx in 0..msg_count {
        let m = &app.state.session.messages[idx];
        let is_selected = is_browse_mode && idx == app.ui.browse_msg_index;

        // 记录消息起始行号
        msg_start_lines.push((idx, current_line_offset));

        // P0 优化：尝试直接按索引复用旧缓存（O(1) 查找代替 O(n) 线性搜索）
        if can_reuse_per_msg
            && let Some(old_c) = old_cache
            && let Some(old_per) = old_c.per_msg_lines.get(idx)
            && old_per.msg_index == idx
            && old_per.content_len == m.content.len()
            && old_per.is_selected == is_selected
        {
            // 直接复用旧缓存（零拷贝：clone PerMsgCache 结构但不重建 flat vec）
            current_line_offset += old_per.lines.len();
            per_msg_cache.push(PerMsgCache {
                content_len: old_per.content_len,
                lines: old_per.lines.clone(),
                msg_index: idx,
                is_selected,
            });
            continue;
        }

        // 缓存未命中 → 重新渲染到临时 Vec
        let mut tmp_lines: Vec<Line<'static>> = Vec::new();
        match m.role.as_str() {
            ROLE_USER => {
                render_user_msg(
                    &m.content,
                    is_selected,
                    inner_width,
                    bubble_max_width,
                    &mut tmp_lines,
                    t,
                );
            }
            ROLE_ASSISTANT => {
                if let Some(ref tool_calls) = m.tool_calls {
                    render_tool_call_request_msg(
                        tool_calls,
                        bubble_max_width,
                        &mut tmp_lines,
                        t,
                        expand,
                    );
                } else {
                    render_assistant_msg(
                        &m.content,
                        is_selected,
                        bubble_max_width,
                        &mut tmp_lines,
                        t,
                    );
                }
            }
            ROLE_TOOL => {
                // 查找对应的工具名：向前搜索 assistant 消息中匹配 tool_call_id 的 ToolCallItem
                let tool_name = m
                    .tool_call_id
                    .as_ref()
                    .and_then(|tid| {
                        app.state.session.messages[..idx]
                            .iter()
                            .rev()
                            .find_map(|prev| {
                                prev.tool_calls.as_ref().and_then(|tcs| {
                                    tcs.iter()
                                        .find(|tc| tc.id == *tid)
                                        .map(|tc| tc.name.clone())
                                })
                            })
                    })
                    .unwrap_or_default();

                // 获取对应的 tool_call arguments（用于特性化渲染）
                let tool_args = m.tool_call_id.as_ref().and_then(|tid| {
                    app.state.session.messages[..idx]
                        .iter()
                        .rev()
                        .find_map(|prev| {
                            prev.tool_calls.as_ref().and_then(|tcs| {
                                tcs.iter()
                                    .find(|tc| tc.id == *tid)
                                    .map(|tc| tc.arguments.clone())
                            })
                        })
                });

                let label = if tool_name.is_empty() {
                    "工具结果".to_string()
                } else {
                    tool_name
                };

                render_tool_result_msg(
                    &m.content,
                    &label,
                    tool_args.as_deref(),
                    bubble_max_width,
                    &mut tmp_lines,
                    t,
                    expand,
                );
            }
            ROLE_SYSTEM => {
                tmp_lines.push(Line::from(""));
                let wrapped = wrap_text(&m.content, inner_width.saturating_sub(8));
                for wl in wrapped {
                    tmp_lines.push(Line::from(Span::styled(
                        format!("    {}  {}", "sys", wl),
                        Style::default().fg(t.text_system),
                    )));
                }
            }
            _ => {}
        }

        // 缓存此历史消息的渲染行（无需额外复制，直接存入）
        current_line_offset += tmp_lines.len();
        per_msg_cache.push(PerMsgCache {
            content_len: m.content.len(),
            lines: tmp_lines,
            msg_index: idx,
            is_selected,
        });
    }

    // ===== 流式消息单独渲染进 streaming_lines =====
    let mut streaming_lines: Vec<Line<'static>> = Vec::new();

    // 获取旧的 stable_lines（Arc::clone O(1) 代替 Vec::clone O(n)）
    let (mut stable_lines, old_stable_offset) = if let Some(old_c) = old_cache {
        if old_c.bubble_max_width == bubble_max_width {
            (
                (*old_c.streaming_stable_lines).clone(),
                old_c.streaming_stable_offset,
            )
        } else {
            (Vec::<Line<'static>>::new(), 0)
        }
    } else {
        (Vec::<Line<'static>>::new(), 0)
    };

    let has_streaming_msg = app.state.is_loading;
    let mut final_stable_offset = old_stable_offset;

    if has_streaming_msg {
        let streaming_text = streaming_content_str.as_deref().unwrap_or("◍");
        // P1 增量段落渲染
        let bubble_bg = t.bubble_ai;
        let pad_left_w = 3usize;
        let pad_right_w = 3usize;
        let md_content_w = bubble_max_width.saturating_sub(pad_left_w + pad_right_w);
        let bubble_total_w = bubble_max_width;

        // AI 标签
        streaming_lines.push(Line::from(""));
        streaming_lines.push(Line::from(Span::styled(
            "Sprite",
            Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
        )));

        // 上边距
        streaming_lines.push(Line::from(vec![Span::styled(
            " ".repeat(bubble_total_w),
            Style::default().bg(bubble_bg),
        )]));

        // 思考指示器：颜色脉冲动画
        if streaming_text == "◍" {
            let pulse_color = thinking_pulse_color(t);
            let indicator_line = Line::from(Span::styled("◍", Style::default().fg(pulse_color)));
            let bubble_line = wrap_md_line_in_bubble(
                indicator_line,
                bubble_bg,
                pad_left_w,
                pad_right_w,
                bubble_total_w,
            );
            streaming_lines.push(bubble_line);

            // 下边距
            streaming_lines.push(Line::from(vec![Span::styled(
                " ".repeat(bubble_total_w),
                Style::default().bg(bubble_bg),
            )]));
        } else {
            let content = streaming_text;
            // 找到当前内容中最后一个安全的段落边界
            let boundary = find_stable_boundary(content);

            // 如果有新的完整段落超过了上次缓存的偏移
            if boundary > old_stable_offset {
                let new_stable_text = &content[old_stable_offset..boundary];
                let new_md_lines = markdown_to_lines(new_stable_text, md_content_w + 2, t);
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
            }
            final_stable_offset = boundary;

            // 追加已缓存的稳定段落行（引用 clone，不再双重 clone）
            streaming_lines.extend(stable_lines.iter().cloned());

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
                    streaming_lines.push(bubble_line);
                }
            }

            // 下边距
            streaming_lines.push(Line::from(vec![Span::styled(
                " ".repeat(bubble_total_w),
                Style::default().bg(bubble_bg),
            )]));
        }
    } else {
        // 非流式状态：stable_lines 不再需要
        stable_lines = Vec::new();
        final_stable_offset = 0;
    }

    // ========== 内联工具确认区（统一交互区域）==========
    if app.ui.mode == ChatMode::ToolConfirm {
        render_tool_confirm_area(app, bubble_max_width, &mut streaming_lines);
    }

    // ========== 子 Agent 权限确认区 ==========
    if app.ui.mode == ChatMode::AgentPermConfirm {
        render_agent_perm_confirm_area(app, bubble_max_width, &mut streaming_lines);
    }

    // ========== Teammate Plan 审批确认区 ==========
    if app.ui.mode == ChatMode::PlanApprovalConfirm {
        render_plan_approval_confirm_area(app, bubble_max_width, &mut streaming_lines);
    }

    // 末尾留白
    streaming_lines.push(Line::from(""));

    (
        msg_start_lines,
        per_msg_cache,
        streaming_lines,
        Arc::new(stable_lines),
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
    // 图片标记行：渲染为纯气泡背景空行，标记信息附加在末尾（不影响可见区域）
    for span in &md_line.spans {
        if span.content.starts_with("\x00IMG:") {
            let marker = span.content.clone();
            let spans: Vec<Span> = vec![
                // 整行用气泡背景色空格填充（与占位行一致）
                Span::styled(" ".repeat(bubble_total_w), Style::default().bg(bubble_bg)),
                // 标记信息附加在行末（超出可见区域，渲染 pass 通过扫描 spans 识别）
                Span::styled(marker, Style::default()),
            ];
            return Line::from(spans);
        }
    }
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

/// 解析 teammate 消息的 `<AgentName>` 前缀。
/// 返回 `Some((name, rest))` 其中 rest 已去除前导空格。
/// 规则：内容以 `<` 开头，紧跟非空白非 `>` 字符，直到 `>`，后面是消息正文。
fn parse_agent_prefix(content: &str) -> Option<(&str, &str)> {
    if !content.starts_with('<') {
        return None;
    }
    let end = content.find('>')?;
    let name = &content[1..end];
    if name.is_empty() || name.contains(char::is_whitespace) {
        return None;
    }
    let rest = content[end + 1..].trim_start();
    Some((name, rest))
}

/// 根据 agent 名字哈希出一个固定颜色（深色/浅色主题均有一定对比度）。
fn agent_name_color(name: &str) -> Color {
    const PALETTE: &[Color] = &[
        Color::Rgb(255, 160, 100), // 橙
        Color::Rgb(100, 200, 255), // 天蓝
        Color::Rgb(255, 110, 180), // 粉红
        Color::Rgb(160, 255, 110), // 草绿
        Color::Rgb(200, 150, 255), // 薰衣草紫
        Color::Rgb(255, 220, 80),  // 琥珀黄
        Color::Rgb(80, 220, 200),  // 青绿
        Color::Rgb(255, 140, 140), // 浅红
    ];
    let hash = name
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    PALETTE[hash as usize % PALETTE.len()]
}

/// 渲染 AI 助手消息（含 teammate 消息）
pub fn render_assistant_msg(
    content: &str,
    is_selected: bool,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    if content.is_empty() {
        return;
    }

    // 检测 teammate 消息：`<AgentName> 正文`
    let (agent_name, bubble_content): (String, &str) =
        if let Some((name, rest)) = parse_agent_prefix(content) {
            (name.to_string(), rest)
        } else {
            ("Sprite".to_string(), content)
        };

    let is_teammate = agent_name != "Sprite";

    lines.push(Line::from(""));

    // 标签行：`  ▶ AgentName` 或 `  AgentName`
    let label = if is_selected {
        format!("  ▶ {}", agent_name)
    } else {
        format!("  {}", agent_name)
    };
    let label_color = if is_selected {
        theme.label_selected
    } else if is_teammate {
        agent_name_color(&agent_name)
    } else {
        theme.label_ai
    };
    lines.push(Line::from(Span::styled(
        label,
        Style::default()
            .fg(label_color)
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
    let md_lines = markdown_to_lines(bubble_content, md_content_w + 2, theme);
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

// 文本工具函数已移至 crate::util::text，此处 re-export 保持兼容
pub use crate::util::text::{char_width, display_width, wrap_text};

/// 构建一行带左右边框的行，自动用空格补齐到 bubble_max_width
/// content_spans: 不含左右边框的内容 spans（会被消费）
/// bubble_max_width: 气泡总宽度
/// border_color: 边框颜色
/// bg: 背景色
fn bordered_line(
    content_spans: Vec<Span<'static>>,
    bubble_max_width: usize,
    border_color: Color,
    bg: Color,
) -> Line<'static> {
    // 左边框 "  │ " 占 4 列，右边框 " │" 占 2 列
    let border_overhead = 4 + 2;
    let content_used: usize = content_spans
        .iter()
        .map(|s| display_width(&s.content))
        .sum();
    let fill = bubble_max_width.saturating_sub(border_overhead + content_used);

    let mut spans = Vec::with_capacity(content_spans.len() + 3);
    spans.push(Span::styled(
        "  │ ",
        Style::default().fg(border_color).bg(bg),
    ));
    spans.extend(content_spans);
    spans.push(Span::styled(" ".repeat(fill), Style::default().bg(bg)));
    spans.push(Span::styled(" │", Style::default().fg(border_color).bg(bg)));
    Line::from(spans)
}

/// 渲染工具确认/Ask 交互区域
fn render_tool_confirm_area(
    app: &ChatApp,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let t = &app.ui.theme;
    let confirm_bg = t.tool_confirm_bg;
    let border_color = t.tool_confirm_border;
    let content_w = bubble_max_width.saturating_sub(6); // 左右各 3 的 padding
    let is_ask = app.ui.tool_ask_mode;

    // 空行
    lines.push(Line::from(""));

    // 标题行
    let title = if is_ask {
        "  🪐 问一下："
    } else {
        "  🔧 工具调用确认"
    };
    lines.push(Line::from(Span::styled(
        title,
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

    if is_ask {
        render_ask_questions(app, bubble_max_width, content_w, lines);
    } else if let Some(tc) = app
        .tool_executor
        .active_tool_calls
        .get(app.tool_executor.pending_tool_idx)
    {
        render_tool_confirm_content(app, tc, bubble_max_width, content_w, lines);
    }

    // 底边框
    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color).bg(confirm_bg),
    )));
}

/// 渲染 Ask 模式的结构化问答内容
fn render_ask_questions(
    app: &ChatApp,
    bubble_max_width: usize,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let t = &app.ui.theme;
    let confirm_bg = t.tool_confirm_bg;
    let border_color = t.tool_confirm_border;

    if let Some(cur_q) = app.ui.tool_ask_questions.get(app.ui.tool_ask_current_idx) {
        let total_q = app.ui.tool_ask_questions.len();
        let cur_idx = app.ui.tool_ask_current_idx;

        // header 标签 + 进度（过长时折行）
        let header_text = if total_q > 1 {
            format!("[{}/{}] {}", cur_idx + 1, total_q, cur_q.header)
        } else {
            cur_q.header.clone()
        };
        {
            // " " 前缀占 1 列，右侧留 1 列 padding
            let header_avail_w = content_w.saturating_sub(2).max(4);
            let header_wrapped = wrap_text(&header_text, header_avail_w);
            for hl in &header_wrapped {
                lines.push(bordered_line(
                    vec![Span::styled(
                        format!(" {}", hl),
                        Style::default().fg(t.tool_confirm_text).bg(confirm_bg),
                    )],
                    bubble_max_width,
                    border_color,
                    confirm_bg,
                ));
            }
        }

        // question 内容（Markdown 渲染）
        {
            let max_msg_w = content_w.saturating_sub(2);
            let md_lines_rendered = markdown_to_lines(&cur_q.question, max_msg_w, t);
            for md_line in md_lines_rendered.iter() {
                let is_img_marker = md_line
                    .spans
                    .iter()
                    .any(|s| s.content.starts_with("\x00IMG:"));
                let is_placeholder = md_line.spans.is_empty()
                    || md_line.spans.iter().all(|s| s.content.trim().is_empty());

                if is_img_marker {
                    let marker = match md_line
                        .spans
                        .iter()
                        .find(|s| s.content.starts_with("\x00IMG:"))
                    {
                        Some(s) => s.content.clone(),
                        None => continue,
                    };
                    let inner_w = bubble_max_width.saturating_sub(8);
                    lines.push(Line::from(vec![
                        Span::styled("  │ ", Style::default().fg(border_color).bg(confirm_bg)),
                        Span::styled(" ".repeat(inner_w), Style::default().bg(confirm_bg)),
                        Span::styled(" │", Style::default().fg(border_color).bg(confirm_bg)),
                        Span::styled(marker, Style::default()),
                    ]));
                } else if is_placeholder {
                    // 空行
                    let inner_w = bubble_max_width.saturating_sub(4);
                    lines.push(Line::from(vec![
                        Span::styled("  │", Style::default().fg(border_color).bg(confirm_bg)),
                        Span::styled(" ".repeat(inner_w), Style::default().bg(confirm_bg)),
                        Span::styled("│", Style::default().fg(border_color).bg(confirm_bg)),
                    ]));
                } else {
                    let mut content_spans =
                        vec![Span::styled(" ", Style::default().bg(confirm_bg))];
                    for span in &md_line.spans {
                        let mut patched = span.clone();
                        patched.style = patched.style.bg(confirm_bg);
                        content_spans.push(patched);
                    }
                    lines.push(bordered_line(
                        content_spans,
                        bubble_max_width,
                        border_color,
                        confirm_bg,
                    ));
                }
            }
        }

        // 空行分隔
        {
            let inner_w = bubble_max_width.saturating_sub(4);
            lines.push(Line::from(vec![
                Span::styled("  │", Style::default().fg(border_color).bg(confirm_bg)),
                Span::styled(" ".repeat(inner_w), Style::default().bg(confirm_bg)),
                Span::styled("│", Style::default().fg(border_color).bg(confirm_bg)),
            ]));
        }

        // 渲染选项列表
        let is_multi = cur_q.multi_select;

        for (i, opt) in cur_q.options.iter().enumerate() {
            let is_cursor = i == app.ui.tool_ask_cursor;
            let is_selected_multi =
                i < app.ui.tool_ask_selections.len() && app.ui.tool_ask_selections[i];

            // 指示器和复选框用多个 span 实现颜色区分
            let pointer_str = if is_cursor { " ❯ " } else { "   " };
            let check_str = if is_multi {
                if is_selected_multi { "☑ " } else { "☐ " }
            } else if is_cursor {
                "● "
            } else {
                "○ "
            };

            let pointer_style = if is_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .bg(confirm_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().bg(confirm_bg)
            };
            let check_style = if is_cursor || is_selected_multi {
                Style::default()
                    .fg(Color::Green)
                    .bg(confirm_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.tool_confirm_label).bg(confirm_bg)
            };
            let label_style = if is_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .bg(confirm_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.tool_confirm_label).bg(confirm_bg)
            };

            // label 折行：pointer + check 占去一段前缀，label 在剩余宽度内自动折行
            {
                let prefix_w = display_width(pointer_str) + display_width(check_str);
                let label_avail_w = content_w.saturating_sub(prefix_w + 2).max(4);
                let label_wrapped = wrap_text(&opt.label, label_avail_w);
                let indent_str = " ".repeat(prefix_w);
                for (li, label_line) in label_wrapped.iter().enumerate() {
                    if li == 0 {
                        lines.push(bordered_line(
                            vec![
                                Span::styled(pointer_str, pointer_style),
                                Span::styled(check_str, check_style),
                                Span::styled(label_line.clone(), label_style),
                            ],
                            bubble_max_width,
                            border_color,
                            confirm_bg,
                        ));
                    } else {
                        // 续行缩进对齐 label 起始列
                        lines.push(bordered_line(
                            vec![
                                Span::styled(indent_str.clone(), Style::default().bg(confirm_bg)),
                                Span::styled(label_line.clone(), label_style),
                            ],
                            bubble_max_width,
                            border_color,
                            confirm_bg,
                        ));
                    }
                }
            }

            // description 行（缩进，灰色）
            if !opt.description.is_empty() {
                let desc_prefix = "       ";
                let desc_max_w = content_w.saturating_sub(display_width(desc_prefix) + 2);
                let desc_wrapped = wrap_text(&opt.description, desc_max_w);
                for dl in &desc_wrapped {
                    let desc_text = format!("{}{}", desc_prefix, dl);
                    lines.push(bordered_line(
                        vec![Span::styled(
                            desc_text,
                            Style::default().fg(t.text_dim).bg(confirm_bg),
                        )],
                        bubble_max_width,
                        border_color,
                        confirm_bg,
                    ));
                }
            }
        }

        // "自由输入" 选项
        {
            let free_idx = cur_q.options.len();
            let is_cursor = free_idx == app.ui.tool_ask_cursor;

            if app.ui.tool_interact_typing {
                let pointer_style = Style::default()
                    .fg(Color::Cyan)
                    .bg(confirm_bg)
                    .add_modifier(Modifier::BOLD);

                // 块状光标渲染
                let input = &app.ui.tool_interact_input;
                let cursor_pos = app.ui.tool_interact_cursor;
                let chars: Vec<char> = input.chars().collect();

                // 光标前的文本
                let before: String = chars[..cursor_pos].iter().collect();
                // 光标处的字符（如果没有则使用空格）
                let cursor_char = chars.get(cursor_pos).copied().unwrap_or(' ');
                // 光标后的文本（光标位置+1 开始）
                let after: String = if cursor_pos < chars.len() {
                    chars[cursor_pos + 1..].iter().collect()
                } else {
                    String::new()
                };

                // 普通文本样式
                let text_style = Style::default().fg(t.text_white).bg(confirm_bg);
                // 块状光标样式（使用主题定义的光标颜色）
                let cursor_style = Style::default().fg(t.cursor_fg).bg(t.cursor_bg);

                lines.push(bordered_line(
                    vec![
                        Span::styled(" ❯ ✏ ", pointer_style),
                        Span::styled(before, text_style),
                        Span::styled(cursor_char.to_string(), cursor_style),
                        Span::styled(after, text_style),
                    ],
                    bubble_max_width,
                    border_color,
                    confirm_bg,
                ));
            } else {
                let pointer_str = if is_cursor { " ❯ " } else { "   " };
                let pointer_style = if is_cursor {
                    Style::default()
                        .fg(Color::Cyan)
                        .bg(confirm_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().bg(confirm_bg)
                };
                let text_style = if is_cursor {
                    Style::default()
                        .fg(Color::Cyan)
                        .bg(confirm_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(t.tool_confirm_label).bg(confirm_bg)
                };
                lines.push(bordered_line(
                    vec![
                        Span::styled(pointer_str, pointer_style),
                        Span::styled("✏ 自由输入...", text_style),
                    ],
                    bubble_max_width,
                    border_color,
                    confirm_bg,
                ));
            }
        }

        // 底部操作提示
        {
            let inner_w = bubble_max_width.saturating_sub(4);
            lines.push(Line::from(vec![
                Span::styled("  │", Style::default().fg(border_color).bg(confirm_bg)),
                Span::styled(" ".repeat(inner_w), Style::default().bg(confirm_bg)),
                Span::styled("│", Style::default().fg(border_color).bg(confirm_bg)),
            ]));
        }
        let hint = if is_multi {
            " Up/Down Move | Space Toggle | Enter OK | PgUp/PgDn Scroll | Esc Cancel"
        } else {
            " Up/Down Move | Enter OK | PgUp/PgDn Scroll | Esc Cancel"
        };
        lines.push(bordered_line(
            vec![Span::styled(
                hint,
                Style::default().fg(t.text_dim).bg(confirm_bg),
            )],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }
}

/// 渲染工具确认模式的内容和选项
fn render_tool_confirm_content(
    app: &ChatApp,
    tc: &super::super::app::ToolCallStatus,
    bubble_max_width: usize,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let t = &app.ui.theme;
    let confirm_bg = t.tool_confirm_bg;
    let border_color = t.tool_confirm_border;

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

    // 确认信息行（折行显示，最多 CONFIRM_MSG_MAX_LINES 行）
    {
        let max_msg_w = content_w.saturating_sub(2);
        let wrapped = wrap_text(&tc.confirm_message, max_msg_w);
        let max_lines = crate::command::chat::constants::CONFIRM_MSG_MAX_LINES;
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

    // 空行 + 选项式交互区域
    {
        let fill = bubble_max_width.saturating_sub(4);
        lines.push(Line::from(vec![
            Span::styled("  │", Style::default().fg(border_color).bg(confirm_bg)),
            Span::styled(" ".repeat(fill), Style::default().bg(confirm_bg)),
            Span::styled("│", Style::default().fg(border_color).bg(confirm_bg)),
        ]));
    }

    // 工具确认选项
    {
        let arrow_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let selected = app.ui.tool_interact_selected;

        let countdown_suffix = if app.state.agent_config.tool_confirm_timeout > 0 {
            let elapsed = app
                .tool_executor
                .tool_confirm_entered_at
                .elapsed()
                .as_secs();
            let remaining = app
                .state
                .agent_config
                .tool_confirm_timeout
                .saturating_sub(elapsed);
            format!(" ({}s)", remaining)
        } else {
            String::new()
        };
        let options: Vec<String> = vec![
            format!("continue: 确认执行{}", countdown_suffix),
            "allow: 允许并记住".to_string(),
            "refuse: 拒绝执行".to_string(),
            "type something...".to_string(),
        ];

        for (i, option) in options.iter().enumerate() {
            let is_selected = i == selected;
            let pointer = if is_selected { "❯" } else { " " };

            if i == 3 && app.ui.tool_interact_typing {
                let input_display = format!("{} type: {}█", pointer, app.ui.tool_interact_input);
                let input_w = display_width(&input_display);
                let fill = content_w.saturating_sub(input_w + 2);
                lines.push(Line::from(vec![
                    Span::styled("  │ ", Style::default().fg(border_color).bg(confirm_bg)),
                    Span::styled(" ", Style::default().bg(confirm_bg)),
                    Span::styled(pointer, arrow_style.bg(confirm_bg)),
                    Span::styled(
                        format!(" type: {}█", app.ui.tool_interact_input),
                        Style::default().fg(t.text_white).bg(confirm_bg),
                    ),
                    Span::styled(
                        " ".repeat(fill.saturating_sub(1).saturating_add(2)),
                        Style::default().bg(confirm_bg),
                    ),
                    Span::styled(" │", Style::default().fg(border_color).bg(confirm_bg)),
                ]));
            } else {
                let full_text = format!("{} {}", pointer, option);
                let text_w = display_width(&full_text);
                let fill = content_w.saturating_sub(text_w + 2);
                let text_style = if is_selected {
                    arrow_style.bg(confirm_bg)
                } else {
                    Style::default().fg(t.tool_confirm_label).bg(confirm_bg)
                };
                lines.push(Line::from(vec![
                    Span::styled("  │ ", Style::default().fg(border_color).bg(confirm_bg)),
                    Span::styled(" ", Style::default().bg(confirm_bg)),
                    Span::styled(
                        pointer,
                        if is_selected {
                            arrow_style.bg(confirm_bg)
                        } else {
                            Style::default().bg(confirm_bg)
                        },
                    ),
                    Span::styled(format!(" {}", option), text_style),
                    Span::styled(
                        " ".repeat(fill.saturating_sub(1).saturating_add(2)),
                        Style::default().bg(confirm_bg),
                    ),
                    Span::styled(" │", Style::default().fg(border_color).bg(confirm_bg)),
                ]));
            }
        }
    }
}
/// 渲染子 Agent 权限确认区域
fn render_agent_perm_confirm_area(
    app: &ChatApp,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let t = &app.ui.theme;
    let confirm_bg = t.tool_confirm_bg;
    let border_color = t.tool_confirm_border;
    let content_w = bubble_max_width.saturating_sub(6);

    let req = match app.ui.pending_agent_perm.as_ref() {
        Some(r) => r,
        None => return,
    };

    // 顶边框
    lines.push(Line::from(Span::styled(
        format!("  ╭{}╮", "─".repeat(bubble_max_width.saturating_sub(4))),
        Style::default().fg(border_color),
    )));

    // 标题行
    let title = format!(" 子 Agent 权限请求 [{}] ", req.agent_name);
    lines.push(bordered_line(
        vec![Span::styled(
            title,
            Style::default()
                .fg(t.tool_confirm_title)
                .add_modifier(Modifier::BOLD)
                .bg(confirm_bg),
        )],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));

    // 工具名行
    lines.push(bordered_line(
        vec![Span::styled(
            format!(" 工具: {}", req.tool_name),
            Style::default()
                .fg(t.tool_confirm_name)
                .add_modifier(Modifier::BOLD)
                .bg(confirm_bg),
        )],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));

    // 确认消息（折行显示）
    for wrapped in wrap_text(&req.confirm_msg, content_w) {
        lines.push(bordered_line(
            vec![Span::styled(
                format!(" {}", wrapped),
                Style::default().fg(t.tool_confirm_text).bg(confirm_bg),
            )],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }

    // 空行间隔
    lines.push(bordered_line(
        vec![Span::styled(" ", Style::default().bg(confirm_bg))],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));

    // Y/N 提示行
    lines.push(bordered_line(
        vec![Span::styled(
            " [Y/Enter] 允许   [N/Esc] 拒绝",
            Style::default()
                .fg(t.text_dim)
                .add_modifier(Modifier::BOLD)
                .bg(confirm_bg),
        )],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));

    // 底边框
    lines.push(Line::from(Span::styled(
        format!("  ╰{}╯", "─".repeat(bubble_max_width.saturating_sub(4))),
        Style::default().fg(border_color),
    )));
}

/// 渲染 Teammate Plan 审批确认区域
fn render_plan_approval_confirm_area(
    app: &ChatApp,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let t = &app.ui.theme;
    let confirm_bg = t.tool_confirm_bg;
    let border_color = t.tool_confirm_border;
    let content_w = bubble_max_width.saturating_sub(6);

    let req = match app.ui.pending_plan_approval.as_ref() {
        Some(r) => r,
        None => return,
    };

    // 顶边框
    lines.push(Line::from(Span::styled(
        format!("  ╭{}╮", "─".repeat(bubble_max_width.saturating_sub(4))),
        Style::default().fg(border_color),
    )));

    // 标题行
    let title = format!(" Plan 审批请求 [{}] ", req.agent_name);
    lines.push(bordered_line(
        vec![Span::styled(
            title,
            Style::default()
                .fg(t.tool_confirm_title)
                .add_modifier(Modifier::BOLD)
                .bg(confirm_bg),
        )],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));

    // Plan 名称行
    lines.push(bordered_line(
        vec![Span::styled(
            format!(" Plan: {}", req.plan_name),
            Style::default()
                .fg(t.tool_confirm_name)
                .add_modifier(Modifier::BOLD)
                .bg(confirm_bg),
        )],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));

    // Plan 内容（折行显示，最多 20 行）
    let plan_lines: Vec<&str> = req.plan_content.lines().take(20).collect();
    for line in &plan_lines {
        for wrapped in wrap_text(line, content_w) {
            lines.push(bordered_line(
                vec![Span::styled(
                    format!(" {}", wrapped),
                    Style::default().fg(t.tool_confirm_text).bg(confirm_bg),
                )],
                bubble_max_width,
                border_color,
                confirm_bg,
            ));
        }
    }
    if req.plan_content.lines().count() > 20 {
        lines.push(bordered_line(
            vec![Span::styled(
                " ... (内容已截断)".to_string(),
                Style::default().fg(t.text_dim).bg(confirm_bg),
            )],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }

    // 空行间隔
    lines.push(bordered_line(
        vec![Span::styled(" ", Style::default().bg(confirm_bg))],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));

    // Y/N 提示行
    lines.push(bordered_line(
        vec![Span::styled(
            " [Y/Enter] 批准   [C] 批准并清空   [N/Esc] 拒绝",
            Style::default()
                .fg(t.text_dim)
                .add_modifier(Modifier::BOLD)
                .bg(confirm_bg),
        )],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));

    // 底边框
    lines.push(Line::from(Span::styled(
        format!("  ╰{}╯", "─".repeat(bubble_max_width.saturating_sub(4))),
        Style::default().fg(border_color),
    )));
}

pub fn render_tool_call_request_msg(
    tool_calls: &[super::super::storage::ToolCallItem],
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
    expand: bool,
) {
    use super::super::tools::classification::{ToolCategory, ToolStatus};

    let content_w = bubble_max_width.saturating_sub(6);

    // 与前一条消息之间留一行间距
    lines.push(Line::from(""));

    for (i, tc) in tool_calls.iter().enumerate() {
        // 多个 tool_call 之间留一行间距
        if i > 0 {
            lines.push(Line::from(""));
        }
        let category = ToolCategory::from_name(&tc.name);
        let icon = category.icon();
        let tool_color = category.color(theme);
        let status = ToolStatus::Pending;
        let status_icon = status.icon();
        let status_color = status.color(theme);

        if expand {
            // 展开模式：图标 + 工具名 + 状态（第一行）
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(icon, Style::default().fg(tool_color)),
                Span::styled(" ", Style::default()),
                Span::styled(
                    tc.name.clone(),
                    Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" ", Style::default()),
                Span::styled(status_icon, Style::default().fg(status_color)),
            ]));

            // 参数详情
            if !tc.arguments.is_empty() {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&tc.arguments) {
                    render_json_params_enhanced(&json_value, content_w, lines, theme);
                } else {
                    // 非 JSON 参数，普通折行显示
                    for line in wrap_text(&tc.arguments, content_w) {
                        lines.push(Line::from(vec![
                            Span::styled("    ", Style::default()),
                            Span::styled(line, Style::default().fg(theme.text_dim)),
                        ]));
                    }
                }
            }
        } else {
            // 折叠模式：图标 + 工具名 + 参数预览
            let total_len = tc.arguments.chars().count();
            let truncated = total_len > crate::command::chat::constants::TOOL_ARG_PREVIEW_MAX_CHARS;

            // 检测 JSON 开括号类型，用于截断时添加闭合括号
            let closing_bracket = if truncated {
                tc.arguments.chars().next().and_then(|c| match c {
                    '{' => Some('}'),
                    '[' => Some(']'),
                    _ => None,
                })
            } else {
                None
            };

            // 如果需要闭合括号，预留 4 字符给 "...}" 或 "...]"
            let preview_len = if closing_bracket.is_some() {
                60 - 4
            } else {
                60
            };

            let args_preview: String = tc.arguments.chars().take(preview_len).collect();

            let suffix = if truncated {
                if let Some(bracket) = closing_bracket {
                    format!("...{}", bracket)
                } else {
                    "…".to_string()
                }
            } else {
                "".to_string()
            };

            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(icon, Style::default().fg(tool_color)),
                Span::styled(" ", Style::default()),
                Span::styled(
                    tc.name.clone(),
                    Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
                ),
                if !args_preview.is_empty() {
                    Span::styled(
                        format!(" {}{}", args_preview, suffix),
                        Style::default().fg(theme.text_dim),
                    )
                } else {
                    Span::raw("")
                },
            ]));
        }
    }
}

/// 渲染 JSON 参数（增强版）
fn render_json_params_enhanced(
    json: &serde_json::Value,
    max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    use super::super::tools::classification::format_json_value;

    if let Some(obj) = json.as_object() {
        for (key, value) in obj {
            let value_str = format_json_value(value);
            let max_val_chars = max_width.saturating_sub(key.chars().count() + 7);

            let value_display = if value_str.chars().count() > max_val_chars {
                let truncated: String = value_str.chars().take(max_val_chars).collect();
                format!("{}…", truncated)
            } else {
                value_str
            };

            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(format!("{}:", key), Style::default().fg(theme.text_dim)),
                Span::styled(" ", Style::default()),
                Span::styled(value_display, Style::default().fg(theme.text_normal)),
            ]));
        }
    } else {
        // 非 JSON 对象，直接显示
        let value_str = format_json_value(json);
        for line in wrap_text(&value_str, max_width) {
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(line, Style::default().fg(theme.text_normal)),
            ]));
        }
    }
}

/// 格式化 JSON 参数字符串，返回适合显示的行列表
/// 如果是有效的 JSON，则美化格式化；否则原样折行显示
/// 渲染工具执行结果消息：展开时完整内容，折叠时只显示标签
pub fn render_tool_result_msg(
    content: &str,
    label: &str,
    tool_args: Option<&str>,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
    expand: bool,
) {
    use super::super::tools::classification::{
        ToolCategory, ToolStatus, get_result_summary_for_tool,
    };

    // 与前一条消息（tool_call）之间留一行间距
    lines.push(Line::from(""));

    // 解析 label，格式为 "工具名..." 或 "工具名[id]..."
    let (tool_name, is_error) = parse_tool_label(label);
    let category = ToolCategory::from_name(&tool_name);
    let tool_color = category.color(theme);
    // tool_result 统一使用 🔧 图标，与 tool_call_request 的分类图标区分
    let icon = "🔧";

    let status = if is_error {
        ToolStatus::Failed
    } else {
        ToolStatus::Success
    };
    let status_icon = status.icon();
    let status_color = status.color(theme);

    // 获取结果摘要
    let summary = get_result_summary_for_tool(content, is_error, &tool_name, tool_args);

    // 第一行：图标 + 工具名 + 状态 + 摘要
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(icon, Style::default().fg(tool_color)),
        Span::styled(" ", Style::default()),
        Span::styled(
            tool_name.clone(),
            Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ", Style::default()),
        Span::styled(status_icon, Style::default().fg(status_color)),
        Span::styled(" ", Style::default()),
        Span::styled(summary, Style::default().fg(theme.text_dim)),
    ]));

    if !expand || content.is_empty() {
        return;
    }

    // 展开模式：缩进显示内容
    let clean = crate::util::text::sanitize_tool_output(content);
    let content_w = bubble_max_width.saturating_sub(6);

    // 错误结果特殊处理
    if is_error {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(
                "Error:",
                Style::default()
                    .fg(theme.toast_error_border)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        let error_lines: Vec<&str> = clean
            .lines()
            .take(crate::command::chat::constants::ERROR_RESULT_MAX_LINES)
            .collect();
        for line in error_lines {
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("      {}", wrapped),
                    Style::default().fg(theme.toast_error_border),
                )));
            }
        }

        let total_lines = clean.lines().count();
        if total_lines > 20 {
            lines.push(Line::from(Span::styled(
                format!("    ... (共 {} 行，显示前 20 行)", total_lines),
                Style::default().fg(theme.text_dim),
            )));
        }
    } else if clean.contains("```diff\n") {
        // Diff 块特殊渲染
        render_diff_content(&clean, content_w, lines, theme);
    } else if tool_name == "Agent" {
        // Agent 结果嵌套显示
        render_agent_result_nested(&clean, content_w, lines, theme);
    } else if tool_name == "Bash" {
        // Bash 结果：命令行高亮 + 输出
        render_bash_result(&clean, tool_args, content_w, lines, theme);
    } else if tool_name == "TodoRead" || tool_name == "TodoWrite" {
        // TodoRead/TodoWrite 结果：checkbox 样式
        render_todo_result(content, content_w, lines, theme);
    } else {
        // 正常结果
        let all_lines: Vec<&str> = clean
            .lines()
            .take(crate::command::chat::constants::NORMAL_RESULT_MAX_LINES)
            .collect();
        for line in all_lines {
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_dim),
                )));
            }
        }

        let total_lines = clean.lines().count();
        if total_lines > 100 {
            lines.push(Line::from(Span::styled(
                format!("    ... (共 {} 行，显示前 100 行)", total_lines),
                Style::default().fg(theme.text_dim),
            )));
        }
    }
}

/// 渲染包含 diff 块的工具结果内容
fn render_diff_content(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let mut in_diff = false;
    for line in content.lines() {
        if line.starts_with("```diff") {
            in_diff = true;
            continue;
        }
        if in_diff && line.starts_with("```") {
            in_diff = false;
            continue;
        }
        if in_diff {
            let color = if line.starts_with("- ")
                || line.starts_with('-') && !line.starts_with("---")
            {
                theme.diff_del
            } else if line.starts_with("+ ") || line.starts_with('+') && !line.starts_with("+++") {
                theme.diff_add
            } else if line.starts_with("@@ ") {
                theme.diff_header
            } else {
                theme.text_dim
            };
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(color),
                )));
            }
        } else {
            // diff 块外的文本正常渲染
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_dim),
                )));
            }
        }
    }
}

/// 渲染 Agent 工具结果（嵌套缩进显示）
fn render_agent_result_nested(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let all_lines: Vec<&str> = content.lines().collect();
    let total = all_lines.len();
    let max_display = crate::command::chat::constants::AGENT_RESULT_MAX_LINES;
    let display_lines = &all_lines[..total.min(max_display)];

    for (i, line) in display_lines.iter().enumerate() {
        let prefix = if i == display_lines.len() - 1 && total <= max_display {
            "  └─ "
        } else {
            "  ├─ "
        };
        let available_w = content_w.saturating_sub(5);
        for (j, wrapped) in wrap_text(line, available_w).iter().enumerate() {
            let p = if j == 0 { prefix } else { "  │  " };
            lines.push(Line::from(Span::styled(
                format!("    {}{}", p, wrapped),
                Style::default().fg(theme.text_dim),
            )));
        }
    }

    if total > max_display {
        lines.push(Line::from(Span::styled(
            format!("    ... (共 {} 行)", total),
            Style::default().fg(theme.text_dim),
        )));
    }
}

/// 渲染 Bash 工具结果（命令行高亮 + 输出）
fn render_bash_result(
    content: &str,
    tool_args: Option<&str>,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    // 提取命令
    let command = tool_args
        .and_then(|args| serde_json::from_str::<serde_json::Value>(args).ok())
        .and_then(|v| {
            v.get("command")
                .and_then(|c| c.as_str().map(|s| s.to_string()))
        });

    if let Some(cmd) = command {
        // 命令行用高亮颜色显示
        let cmd_w = content_w.saturating_sub(6); // "    $ " 前缀
        for (i, cmd_line) in cmd.lines().enumerate() {
            let prefix = if i == 0 { "    $ " } else { "      " };
            for wrapped in wrap_text(cmd_line, cmd_w) {
                lines.push(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(theme.label_ai)),
                    Span::styled(
                        wrapped,
                        Style::default()
                            .fg(theme.text_white)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
        }
    }

    // 输出内容（灰色）
    let output_lines: Vec<&str> = content
        .lines()
        .take(crate::command::chat::constants::BASH_OUTPUT_MAX_LINES)
        .collect();
    for line in &output_lines {
        for wrapped in wrap_text(line, content_w) {
            lines.push(Line::from(Span::styled(
                format!("    {}", wrapped),
                Style::default().fg(theme.text_dim),
            )));
        }
    }

    let total_lines = content.lines().count();
    if total_lines > 100 {
        lines.push(Line::from(Span::styled(
            format!("    ... (共 {} 行，显示前 100 行)", total_lines),
            Style::default().fg(theme.text_dim),
        )));
    }
}

/// 渲染 TodoRead/TodoWrite 工具结果（checkbox 样式，与 todo TUI 一致）
fn render_todo_result(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(content) {
        for item in &items {
            let status = item
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("pending");
            let text = item
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("(empty)");
            let id = item.get("id").and_then(|i| i.as_str()).unwrap_or("");

            // 与 todo TUI 保持一致的 checkbox 样式和颜色
            let (checkbox, color) = match status {
                "completed" => ("[x]", theme.label_ai), // 绿色，与 todo UI 一致
                "in_progress" => ("[~]", theme.title_loading), // 黄色
                "cancelled" => ("[-]", theme.text_dim), // 灰色
                _ => ("[ ]", Color::Yellow),            // pending: 黄色，与 todo UI 一致
            };

            let id_display = if !id.is_empty() {
                format!("{} ", id)
            } else {
                String::new()
            };

            let item_text = format!("{}{}", id_display, text);
            let max_w = content_w.saturating_sub(10); // "    [x] " prefix
            for (i, wrapped) in wrap_text(&item_text, max_w).iter().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(format!("{} ", checkbox), Style::default().fg(color)),
                        Span::styled(
                            wrapped.clone(),
                            if status == "completed" {
                                Style::default()
                                    .fg(theme.text_dim)
                                    .add_modifier(Modifier::CROSSED_OUT)
                            } else {
                                Style::default().fg(theme.text_white)
                            },
                        ),
                    ]));
                } else {
                    lines.push(Line::from(Span::styled(
                        format!("        {}", wrapped),
                        Style::default().fg(theme.text_dim),
                    )));
                }
            }
        }
    } else {
        // 非 JSON，回退到普通显示
        let all_lines: Vec<&str> = content.lines().take(100).collect();
        for line in all_lines {
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_dim),
                )));
            }
        }
    }
}

/// 解析工具标签，提取工具名和错误状态
fn parse_tool_label(label: &str) -> (String, bool) {
    let is_error = label.contains("错误") || label.contains("失败") || label.contains("error");
    // 兼容旧格式 "工具 xxx" 和新格式直接工具名
    let tool_name = if label.starts_with("工具 ") {
        label
            .chars()
            .skip(3)
            .collect::<String>()
            .split(['.', ' '])
            .next()
            .unwrap_or(label)
            .to_string()
    } else {
        label.split(['.', ' ']).next().unwrap_or(label).to_string()
    };
    (tool_name, is_error)
}

/// 计算思考指示器的脉冲颜色：基于 label_ai 颜色在亮暗之间平滑过渡
/// 使用正弦波实现呼吸灯效果，周期约 1.5 秒
fn thinking_pulse_color(theme: &Theme) -> Color {
    use std::time::{SystemTime, UNIX_EPOCH};

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    // 周期由 THINKING_PULSE_PERIOD_MS 定义，正弦波映射到 [0.0, 1.0]
    let period = crate::command::chat::constants::THINKING_PULSE_PERIOD_MS as f64;
    let phase = (millis % period as u128) as f64 / period;
    let t = (phase * std::f64::consts::TAU).sin() * 0.5 + 0.5; // 0.0 ~ 1.0

    // 从 label_ai 颜色提取 RGB 分量
    if let Color::Rgb(r, g, b) = theme.label_ai {
        // 在 THINKING_PULSE_MIN_FACTOR ~ 100% 亮度之间脉冲
        let min_factor = crate::command::chat::constants::THINKING_PULSE_MIN_FACTOR;
        let factor = min_factor + (1.0 - min_factor) * t;
        let pr = (r as f64 * factor).round().min(255.0) as u8;
        let pg = (g as f64 * factor).round().min(255.0) as u8;
        let pb = (b as f64 * factor).round().min(255.0) as u8;
        Color::Rgb(pr, pg, pb)
    } else {
        // 非 RGB 颜色的回退：简单交替
        if t > 0.5 {
            theme.label_ai
        } else {
            theme.text_dim
        }
    }
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
