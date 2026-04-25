use super::theme::Theme;
use crate::command::chat::app::{ChatApp, ChatMode, MsgLinesCache, PerMsgCache, ToolCallStatus};
use crate::command::chat::constants::{
    AGENT_CALL_PROMPT_MAX_LINES, AGENT_RESULT_MAX_LINES, BASH_OUTPUT_MAX_LINES,
    CONFIRM_MSG_MAX_LINES, ERROR_RESULT_MAX_LINES, NORMAL_RESULT_MAX_LINES,
    THINKING_PULSE_MIN_FACTOR, THINKING_PULSE_PERIOD_MS, TOOL_ARG_PREVIEW_MAX_CHARS,
};
use crate::command::chat::markdown::markdown_to_lines;
use crate::command::chat::storage::DisplayType;
use crate::command::chat::storage::ToolCallItem;
use crate::command::chat::storage::config::ThinkingStyle;
use crate::command::chat::tools::classification::{
    ToolCategory, ToolStatus, format_json_value, get_result_summary_for_tool,
};
use crate::command::chat::tools::tool_names;
use crate::command::chat::ui::palette;
use crate::util::safe_lock;
use crate::util::text::{char_width, display_width, wrap_text};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::io::Write;
use std::sync::Arc;

// ── 模块级常量（提取自各渲染函数中的魔法值）──

/// `render_thinking_block` 折叠模式下最大显示行数
const THINKING_FOLDED_MAX_LINES: usize = 5;
/// `render_assistant_msg` 气泡最小宽度（字符列数）
const BUBBLE_MIN_WIDTH: usize = 20;
/// `render_user_msg` 用户气泡左右内边距（字符列数）
const USER_BUBBLE_PAD_LR: usize = 3;
/// `render_tool_result_msg` / `render_bash_result` 普通结果截断显示的行数上限
const TOOL_RESULT_DISPLAY_MAX_LINES: usize = 100;

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

    // ★ UI 渲染从 display_messages 读取（干净文本 + sender_name），不从 session.messages
    let display_msgs = safe_lock(&app.display_messages, "render_cache::display_msgs").clone();
    let msg_count = display_msgs.len();
    let mut current_line_offset: usize = 0;
    let mut msg_start_lines: Vec<(usize, usize)> = Vec::with_capacity(msg_count);
    let mut per_msg_cache: Vec<PerMsgCache> = Vec::with_capacity(msg_count);

    let expand = app.ui.expand_tools;

    // 判断旧缓存中的 per_msg_lines 是否可以复用（bubble_max_width 相同且 expand 一致）
    let can_reuse_per_msg = old_cache
        .map(|c| c.bubble_max_width == bubble_max_width && c.expand_tools == expand)
        .unwrap_or(false);

    // ===== P0 优化：引用 display_messages 克隆，避免重复锁 =====
    // 缓存命中时零拷贝复用，只在缓存未命中时才访问消息内容
    for (idx, m) in display_msgs.iter().enumerate() {
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
        match m.display_type() {
            DisplayType::User => {
                render_user_msg(
                    &m.content,
                    is_selected,
                    inner_width,
                    bubble_max_width,
                    &mut tmp_lines,
                    t,
                );
            }
            DisplayType::AssistantText => {
                // 如果有 reasoning_content，先渲染 thinking 区块
                if let Some(ref reasoning) = m.reasoning_content {
                    render_thinking_block(reasoning, bubble_max_width, &mut tmp_lines, t, expand);
                }
                render_assistant_msg(
                    m.sender_name.as_deref(),
                    &m.content,
                    is_selected,
                    bubble_max_width,
                    &mut tmp_lines,
                    t,
                );
            }
            DisplayType::ToolCallRequest => {
                // 如果有 reasoning_content，先渲染 thinking 区块
                if let Some(ref reasoning) = m.reasoning_content {
                    render_thinking_block(reasoning, bubble_max_width, &mut tmp_lines, t, expand);
                }
                // 先渲染文本内容（如果有）— LLM 可能同时返回文本解释和工具调用
                if !m.content.is_empty() {
                    render_assistant_msg(
                        m.sender_name.as_deref(),
                        &m.content,
                        is_selected,
                        bubble_max_width,
                        &mut tmp_lines,
                        t,
                    );
                }
                // 再渲染工具调用
                if let Some(ref tool_calls) = m.tool_calls {
                    render_tool_call_request_msg(
                        tool_calls,
                        bubble_max_width,
                        &mut tmp_lines,
                        t,
                        expand,
                    );
                }
            }
            DisplayType::ToolResult => {
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
            DisplayType::System => {
                tmp_lines.push(Line::from(""));
                let wrapped = wrap_text(&m.content, inner_width.saturating_sub(8));
                for wl in wrapped {
                    tmp_lines.push(Line::from(Span::styled(
                        format!("    {}  {}", "sys", wl),
                        Style::default().fg(t.text_system),
                    )));
                }
            }
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
            let tick = current_tick();
            let thinking_style = app.state.agent_config.thinking_style;

            let indicator_line = if thinking_style == ThinkingStyle::Comet {
                // ── 彗星逐字符渐变渲染 ──
                // 使用 welcome_palette 调色板实现 RGB 三色分段插值
                comet_gradient_line(tick, t.welcome_palette, t.label_ai)
            } else {
                let pulse_color = thinking_pulse_color(t);
                let frame = thinking_style.frame(tick);
                Line::from(Span::styled(frame, Style::default().fg(pulse_color)))
            };
            let bubble_line = wrap_md_line_in_bubble(
                indicator_line,
                bubble_bg,
                pad_left_w,
                pad_right_w,
                bubble_total_w,
            );
            streaming_lines.push(bubble_line);

            // 如果有 reasoning 内容，在绿点下方渲染 thinking 区块
            let reasoning_str = safe_lock(
                &app.state.streaming_reasoning_content,
                "render::streaming_reasoning",
            )
            .clone();
            if !reasoning_str.is_empty() {
                // Thinking 标签（灰色斜体）
                let thinking_label = Line::from(Span::styled(
                    "  Thinking...",
                    Style::default()
                        .fg(t.text_dim)
                        .add_modifier(Modifier::ITALIC),
                ));
                let label_bubble = wrap_md_line_in_bubble(
                    thinking_label,
                    bubble_bg,
                    pad_left_w,
                    pad_right_w,
                    bubble_total_w,
                );
                streaming_lines.push(label_bubble);

                // Reasoning 内容（灰色文本，带气泡背景）
                let reason_content_w = md_content_w.saturating_sub(2);
                for wrapped_line in wrap_text(&reasoning_str, reason_content_w) {
                    let line = Line::from(Span::styled(
                        format!("  {}", wrapped_line),
                        Style::default().fg(t.text_dim),
                    ));
                    let bubble_line = wrap_md_line_in_bubble(
                        line,
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
    let user_pad_lr = USER_BUBBLE_PAD_LR;
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
/// 规则：内容以 `<` 开头，紧跟非 `>` 字符，直到 `>`，后面是消息正文。
/// 支持 `<Type@Name>` 格式（如 `<Teammate@Frontend>`、`<SubAgent@search_auth>`、`<Teammate@Go Advocate>`）。
fn parse_agent_prefix(content: &str) -> Option<(&str, &str)> {
    if !content.starts_with('<') {
        return None;
    }
    let end = content.find('>')?;
    let name = &content[1..end];
    if name.is_empty() {
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

/// 渲染 thinking 区块（reasoning_content），显示在 AI 气泡上方
/// 折叠模式下（expand=false）仅显示前若干行，避免占用过多屏幕空间
fn render_thinking_block(
    reasoning: &str,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
    expand: bool,
) {
    if reasoning.is_empty() {
        return;
    }

    lines.push(Line::from(""));

    // Thinking 标签（灰色斜体）
    lines.push(Line::from(Span::styled(
        "  >> Thinking...",
        Style::default()
            .fg(theme.text_dim)
            .add_modifier(Modifier::ITALIC),
    )));

    // Reasoning 内容（灰色文本）
    let content_w = bubble_max_width.saturating_sub(6);
    let wrapped = wrap_text(reasoning, content_w);

    // 折叠模式：最多显示 THINKING_FOLDED_MAX_LINES 行，超出时追加省略提示
    let total = wrapped.len();
    let (shown, truncated) = if !expand && total > THINKING_FOLDED_MAX_LINES {
        (&wrapped[..THINKING_FOLDED_MAX_LINES], true)
    } else {
        (&wrapped[..], false)
    };

    for wrapped_line in shown {
        lines.push(Line::from(Span::styled(
            format!("    {}", wrapped_line),
            Style::default().fg(theme.text_dim),
        )));
    }

    if truncated {
        lines.push(Line::from(Span::styled(
            format!(
                "    … (+{} 行, Ctrl+O 展开)",
                total - THINKING_FOLDED_MAX_LINES
            ),
            Style::default()
                .fg(theme.text_dim)
                .add_modifier(Modifier::ITALIC),
        )));
    }
}

/// 渲染 AI 助手消息（含 teammate/subagent 消息）
/// 气泡宽度根据实际内容自适应：最小宽度 20，最大宽度为传入的 bubble_max_width
///
/// # 参数
/// - `sender_name`: 消息发送者名称（如 `Teammate@Frontend`）。优先使用此字段作为气泡标签。
///   若为 None，则尝试从 content 解析 `<Name> ...` 前缀（兼容老 session）。
/// - `content`: 消息正文（不含 sender_name 前缀）
pub fn render_assistant_msg(
    sender_name: Option<&str>,
    content: &str,
    is_selected: bool,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    if content.is_empty() {
        return;
    }

    // 确定 agent_name 和 bubble_content：
    // 优先使用 sender_name 字段；若无则 fallback 解析 content 的 <Name> 前缀（兼容老 session）
    let (agent_name, bubble_content): (String, &str) = if let Some(name) = sender_name {
        (name.to_string(), content)
    } else if let Some((name, rest)) = parse_agent_prefix(content) {
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

    // 先用最大宽度渲染 markdown 内容
    let md_content_w = bubble_max_width.saturating_sub(pad_left_w + pad_right_w);
    let md_lines = markdown_to_lines(bubble_content, md_content_w + 2, theme);

    // 计算实际内容最大宽度：取所有 md_lines 的最大显示宽度
    let actual_content_max_w = md_lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| display_width(&span.content))
                .sum::<usize>()
        })
        .max()
        .unwrap_or(0);

    // 气泡自适应宽度：min(max(实际宽度+padding, 最小宽度), 最大宽度)
    let bubble_total_w = (actual_content_max_w + pad_left_w + pad_right_w)
        .max(BUBBLE_MIN_WIDTH)
        .min(bubble_max_width);

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
    let target_content_w = bubble_max_width.saturating_sub(border_overhead);

    // 溢出钳制：逐 span 逐字符截断，确保内容不超出目标宽度
    let mut clamped_spans: Vec<Span<'static>> = Vec::with_capacity(content_spans.len());
    let mut used: usize = 0;
    for span in content_spans {
        let sw = display_width(&span.content);
        if used + sw <= target_content_w {
            used += sw;
            clamped_spans.push(span);
        } else {
            // 当前 span 需要截断
            let remaining = target_content_w.saturating_sub(used);
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
                    used += tw;
                    clamped_spans.push(Span::styled(truncated, span.style));
                }
            }
            // 后续 span 全部跳过（已溢出）
            break;
        }
    }

    let fill = target_content_w.saturating_sub(used);

    let mut spans = Vec::with_capacity(clamped_spans.len() + 3);
    spans.push(Span::styled(
        "  │ ",
        Style::default().fg(border_color).bg(bg),
    ));
    spans.extend(clamped_spans);
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
        "  🪐 あの、すみません… (〃´∀｀)ゞ"
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

                // 前缀 " ❯ ✏ " 的显示宽度
                let prefix = " ❯ ✏ ";
                let prefix_w = display_width(prefix);
                // 续行缩进宽度（与前缀对齐）
                let indent_w = prefix_w;
                let avail_w = content_w.saturating_sub(prefix_w);
                // 最少保证 4 列可用（光标占 1 + 至少 3 字符余量）
                let avail_w = avail_w.max(4);

                // 拼回完整文本，用 wrap_text 按可用宽度折行
                let full_text = format!("{}{}{}", before, cursor_char, after);
                let wrapped = wrap_text(&full_text, avail_w);

                // 定位光标所在折行：逐行累加字符数，找到 cursor_pos 落在哪一行
                let mut char_idx = 0usize;
                let mut cursor_line = 0usize;
                let mut cursor_offset_in_line = 0usize;
                for (li, line_str) in wrapped.iter().enumerate() {
                    let line_chars: Vec<char> = line_str.chars().collect();
                    if cursor_pos >= char_idx && cursor_pos < char_idx + line_chars.len() {
                        cursor_line = li;
                        cursor_offset_in_line = cursor_pos - char_idx;
                        break;
                    }
                    char_idx += line_chars.len();
                    if li == wrapped.len() - 1 && cursor_pos == char_idx {
                        // 光标在末尾
                        cursor_line = li;
                        cursor_offset_in_line = line_chars.len();
                    }
                }

                for (li, _line_str) in wrapped.iter().enumerate() {
                    let is_first = li == 0;
                    let prefix_span = if is_first {
                        Span::styled(prefix, pointer_style)
                    } else {
                        Span::styled(" ".repeat(indent_w), text_style)
                    };

                    if li == cursor_line {
                        // 光标行：需要拆分 before / cursor_char / after
                        let line_str = &wrapped[li];
                        let line_chars: Vec<char> = line_str.chars().collect();
                        let line_before: String =
                            line_chars[..cursor_offset_in_line].iter().collect();
                        let cc = line_chars
                            .get(cursor_offset_in_line)
                            .copied()
                            .unwrap_or(' ');
                        let line_after: String =
                            line_chars[cursor_offset_in_line + 1..].iter().collect();

                        lines.push(bordered_line(
                            vec![
                                prefix_span,
                                Span::styled(line_before, text_style),
                                Span::styled(cc.to_string(), cursor_style),
                                Span::styled(line_after, text_style),
                            ],
                            bubble_max_width,
                            border_color,
                            confirm_bg,
                        ));
                    } else {
                        // 非光标续行
                        lines.push(bordered_line(
                            vec![prefix_span, Span::styled(wrapped[li].clone(), text_style)],
                            bubble_max_width,
                            border_color,
                            confirm_bg,
                        ));
                    }
                }
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
    tc: &ToolCallStatus,
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
        let max_lines = CONFIRM_MSG_MAX_LINES;
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
                // 前缀 " ❯ type: " 的显示宽度
                let prefix = " ❯ type: ";
                let prefix_w = display_width(prefix);
                // 续行缩进宽度（与前缀对齐）
                let indent_w = prefix_w;
                let avail_w = content_w.saturating_sub(prefix_w);
                // 最少保证 4 列可用（光标占 1 + 至少 3 字符余量）
                let avail_w = avail_w.max(4);

                let input = &app.ui.tool_interact_input;
                let cursor_pos = app.ui.tool_interact_cursor;
                let chars: Vec<char> = input.chars().collect();
                let before: String = chars[..cursor_pos].iter().collect();
                let cursor_char = chars.get(cursor_pos).copied().unwrap_or(' ');
                let after: String = if cursor_pos < chars.len() {
                    chars[cursor_pos + 1..].iter().collect()
                } else {
                    String::new()
                };

                let text_style = Style::default().fg(t.text_white).bg(confirm_bg);
                let cursor_style = Style::default().fg(t.cursor_fg).bg(t.cursor_bg);
                let pointer_style = Style::default()
                    .fg(Color::Cyan)
                    .bg(confirm_bg)
                    .add_modifier(Modifier::BOLD);

                // 将 before / cursor_char / after 拼回完整文本，用 wrap_text 按可用宽度折行
                let full_text = format!("{}{}{}", before, cursor_char, after);
                let wrapped = wrap_text(&full_text, avail_w);

                // 定位光标所在折行：逐行累加宽度，找到 cursor_pos 落在哪一行
                let mut char_idx = 0usize;
                let mut cursor_line = 0usize;
                let mut cursor_offset_in_line = 0usize;
                for (li, line_str) in wrapped.iter().enumerate() {
                    let line_chars: Vec<char> = line_str.chars().collect();
                    if cursor_pos >= char_idx && cursor_pos < char_idx + line_chars.len() {
                        cursor_line = li;
                        cursor_offset_in_line = cursor_pos - char_idx;
                        break;
                    }
                    char_idx += line_chars.len();
                    if li == wrapped.len() - 1 && cursor_pos == char_idx {
                        // 光标在末尾
                        cursor_line = li;
                        cursor_offset_in_line = line_chars.len();
                    }
                }

                for (li, _line_str) in wrapped.iter().enumerate() {
                    let is_first = li == 0;
                    let prefix_span = if is_first {
                        Span::styled(prefix, pointer_style)
                    } else {
                        Span::styled(" ".repeat(indent_w), text_style)
                    };

                    if li == cursor_line {
                        // 光标行：需要拆分 before / cursor_char / after
                        let line_str = &wrapped[li];
                        let line_chars: Vec<char> = line_str.chars().collect();
                        let line_before: String =
                            line_chars[..cursor_offset_in_line].iter().collect();
                        let cc = line_chars
                            .get(cursor_offset_in_line)
                            .copied()
                            .unwrap_or(' ');
                        let line_after: String =
                            line_chars[cursor_offset_in_line + 1..].iter().collect();

                        lines.push(bordered_line(
                            vec![
                                prefix_span,
                                Span::styled(line_before, text_style),
                                Span::styled(cc.to_string(), cursor_style),
                                Span::styled(line_after, text_style),
                            ],
                            bubble_max_width,
                            border_color,
                            confirm_bg,
                        ));
                    } else {
                        // 非光标续行
                        lines.push(bordered_line(
                            vec![prefix_span, Span::styled(wrapped[li].clone(), text_style)],
                            bubble_max_width,
                            border_color,
                            confirm_bg,
                        ));
                    }
                }
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
/// 渲染权限确认区域（子 Agent / Teammate 通用）
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

    // 标题行（支持折行）
    let title = req.title();
    let title_style = Style::default()
        .fg(t.tool_confirm_title)
        .add_modifier(Modifier::BOLD)
        .bg(confirm_bg);
    let title_wrapped = wrap_text(&title, content_w);
    for line_text in title_wrapped {
        lines.push(bordered_line(
            vec![Span::styled(line_text, title_style)],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }

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

    // 标题行（支持折行）
    let title = format!(" Plan 审批请求 [{}] ", req.agent_name);
    let title_style = Style::default()
        .fg(t.tool_confirm_title)
        .add_modifier(Modifier::BOLD)
        .bg(confirm_bg);
    let title_wrapped = wrap_text(&title, content_w);
    for line_text in title_wrapped {
        lines.push(bordered_line(
            vec![Span::styled(line_text, title_style)],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }

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
    tool_calls: &[ToolCallItem],
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
    expand: bool,
) {
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

        if expand {
            // 展开模式：图标 + 工具名 + description（若有）+ 状态（第一行）
            let tool_desc = extract_tool_description_from_args(&tc.name, &tc.arguments);
            let display_name = if let Some(ref desc) = tool_desc {
                format!("{} - {}", tc.name, desc)
            } else {
                tc.name.clone()
            };
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(icon, Style::default().fg(tool_color)),
                Span::styled(" ", Style::default()),
                Span::styled(
                    display_name,
                    Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
                ),
            ]));

            // 参数详情
            if !tc.arguments.is_empty() {
                if matches!(tc.name.as_str(), tool_names::BASH) {
                    // Bash/Shell 工具使用专用渲染：显示命令 + 附加信息
                    if let Some(bash_args) = extract_bash_args(&tc.arguments) {
                        render_bash_call_request_expanded(
                            &bash_args,
                            bubble_max_width,
                            lines,
                            theme,
                        );
                    } else if let Ok(json_value) =
                        serde_json::from_str::<serde_json::Value>(&tc.arguments)
                    {
                        render_json_params_enhanced(&json_value, content_w, lines, theme);
                    }
                // Agent 工具使用专用渲染：边框 + prompt + 元信息
                } else if tc.name.as_str() == tool_names::AGENT {
                    if let Some(agent_args) = extract_agent_args(&tc.arguments) {
                        render_agent_call_request_expanded(
                            &agent_args,
                            bubble_max_width,
                            lines,
                            theme,
                        );
                    }
                // Teammate 工具使用专用渲染：边框 + name/role + prompt
                } else if tc.name.as_str() == tool_names::TEAMMATE {
                    if let Some(tm_args) = extract_teammate_args(&tc.arguments) {
                        render_teammate_call_request_expanded(
                            &tm_args,
                            bubble_max_width,
                            lines,
                            theme,
                        );
                    }
                // ExitPlanMode 工具使用专用渲染：边框显示
                } else if matches!(tc.name.as_str(), tool_names::EXIT_PLAN_MODE) {
                    render_exit_plan_mode_request(bubble_max_width, lines, theme);
                } else if let Ok(json_value) =
                    serde_json::from_str::<serde_json::Value>(&tc.arguments)
                {
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
            // 折叠模式：图标 + 工具名 + description（若有）或参数预览

            // Agent 工具专用折叠渲染：显示 [background] + description
            if tc.name.as_str() == tool_names::AGENT
                && let Some(agent_args) = extract_agent_args(&tc.arguments)
            {
                let mut desc_parts: Vec<String> = Vec::new();
                if agent_args.run_in_background {
                    desc_parts.push("[background]".to_string());
                }
                if let Some(ref desc) = agent_args.description {
                    desc_parts.push(desc.clone());
                }
                if desc_parts.is_empty() {
                    let first_line = agent_args.prompt.lines().next().unwrap_or("");
                    let cw: String = first_line
                        .chars()
                        .take(TOOL_ARG_PREVIEW_MAX_CHARS)
                        .collect();
                    let preview = if first_line.chars().count() > TOOL_ARG_PREVIEW_MAX_CHARS {
                        format!("{}...", cw)
                    } else {
                        cw
                    };
                    desc_parts.push(preview);
                }
                let desc_text = desc_parts.join("  ");
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(icon, Style::default().fg(tool_color)),
                    Span::styled(" ", Style::default()),
                    Span::styled(
                        tc.name.clone(),
                        Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  {}", desc_text),
                        Style::default().fg(theme.text_dim),
                    ),
                ]));
                continue;
            }

            // Teammate 工具专用折叠渲染：显示 name(role) + prompt 预览
            if tc.name.as_str() == tool_names::TEAMMATE
                && let Some(tm_args) = extract_teammate_args(&tc.arguments)
            {
                let mut desc_parts: Vec<String> = Vec::new();
                if tm_args.worktree {
                    desc_parts.push("[worktree]".to_string());
                }
                desc_parts.push(format!("{}({})", tm_args.name, tm_args.role));
                let first_line = tm_args.prompt.lines().next().unwrap_or("");
                let cw: String = first_line
                    .chars()
                    .take(TOOL_ARG_PREVIEW_MAX_CHARS)
                    .collect();
                let preview = if first_line.chars().count() > TOOL_ARG_PREVIEW_MAX_CHARS {
                    format!("{}...", cw)
                } else {
                    cw
                };
                desc_parts.push(preview);
                let desc_text = desc_parts.join("  ");
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(icon, Style::default().fg(tool_color)),
                    Span::styled(" ", Style::default()),
                    Span::styled(
                        tc.name.clone(),
                        Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  {}", desc_text),
                        Style::default().fg(theme.text_dim),
                    ),
                ]));
                continue;
            }

            let tool_desc = extract_tool_description_from_args(&tc.name, &tc.arguments);

            if let Some(desc) = tool_desc {
                // 有 description 时优先展示，替代 raw arguments
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(icon, Style::default().fg(tool_color)),
                    Span::styled(" ", Style::default()),
                    Span::styled(
                        tc.name.clone(),
                        Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("  {}", desc), Style::default().fg(theme.text_dim)),
                ]));
            } else {
                // 无 description，保留原有的参数预览逻辑
                let total_len = tc.arguments.chars().count();
                let truncated = total_len > TOOL_ARG_PREVIEW_MAX_CHARS;

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
                let max_preview = TOOL_ARG_PREVIEW_MAX_CHARS;
                let preview_len = if closing_bracket.is_some() {
                    max_preview - 4
                } else {
                    max_preview
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
}

/// 渲染 JSON 参数（增强版）
fn render_json_params_enhanced(
    json: &serde_json::Value,
    max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
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

    // Todo 工具特殊处理：折叠模式也显示 todo 列表
    let is_todo_tool = tool_name == "TodoRead" || tool_name == "TodoWrite";

    if (!expand && !is_todo_tool) || content.is_empty() {
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

        let error_lines: Vec<&str> = clean.lines().take(ERROR_RESULT_MAX_LINES).collect();
        for line in error_lines {
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("      {}", wrapped),
                    Style::default().fg(theme.toast_error_border),
                )));
            }
        }

        let total_lines = clean.lines().count();
        let max_err_lines = ERROR_RESULT_MAX_LINES;
        if total_lines > max_err_lines {
            lines.push(Line::from(Span::styled(
                format!(
                    "    ... (共 {} 行，显示前 {} 行)",
                    total_lines, max_err_lines
                ),
                Style::default().fg(theme.text_dim),
            )));
        }
    } else if clean.contains("```diff\n") {
        // Diff 块特殊渲染
        render_diff_content(&clean, content_w, lines, theme);
    } else if tool_name == tool_names::AGENT
        || tool_name == tool_names::TEAMMATE
        || tool_name == tool_names::COMPACT
        || tool_name == tool_names::LOAD_SKILL
        || tool_name == tool_names::ENTER_PLAN_MODE
        || tool_name == tool_names::EXIT_PLAN_MODE
    {
        // Agent/Compact/LoadSkill/Plan 结果边框显示
        render_agent_result_nested(&clean, bubble_max_width, lines, theme);
    } else if tool_name == tool_names::BASH {
        // Bash 结果：命令行高亮 + 输出
        render_bash_result(&clean, tool_args, content_w, lines, theme);
    } else if tool_name == tool_names::TODO_READ || tool_name == tool_names::TODO_WRITE {
        // TodoRead/TodoWrite 结果：折叠和展开都显示 todo 列表
        render_todo_result(content, content_w, lines, theme, expand);
    } else {
        // 正常结果
        let all_lines: Vec<&str> = clean.lines().take(NORMAL_RESULT_MAX_LINES).collect();
        for line in all_lines {
            for wrapped in wrap_text(line, content_w) {
                lines.push(Line::from(Span::styled(
                    format!("    {}", wrapped),
                    Style::default().fg(theme.text_dim),
                )));
            }
        }

        let total_lines = clean.lines().count();
        if total_lines > TOOL_RESULT_DISPLAY_MAX_LINES {
            lines.push(Line::from(Span::styled(
                format!(
                    "    ... (共 {} 行，显示前 {} 行)",
                    total_lines, TOOL_RESULT_DISPLAY_MAX_LINES
                ),
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
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let all_lines: Vec<&str> = content.lines().collect();
    let total = all_lines.len();
    let max_display = AGENT_RESULT_MAX_LINES;
    let display_lines = &all_lines[..total.min(max_display)];

    let border_color = theme.text_dim;
    let result_bg = theme.bg_primary;
    // bordered_line: 左 "  │ " (4) + 右 " │" (2) = 6 开销
    let content_w = bubble_max_width.saturating_sub(6);

    // 顶边框
    let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(border_color).bg(result_bg),
    )));

    // 内容行
    for line in display_lines.iter() {
        for wrapped in wrap_text(line, content_w) {
            lines.push(bordered_line(
                vec![Span::styled(
                    wrapped,
                    Style::default().fg(theme.text_dim).bg(result_bg),
                )],
                bubble_max_width,
                border_color,
                result_bg,
            ));
        }
    }

    // 截断提示
    if total > max_display {
        lines.push(bordered_line(
            vec![Span::styled(
                format!("... (共 {} 行)", total),
                Style::default().fg(theme.text_dim).bg(result_bg),
            )],
            bubble_max_width,
            border_color,
            result_bg,
        ));
    }

    // 底边框
    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color).bg(result_bg),
    )));
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
    let output_lines: Vec<&str> = content.lines().take(BASH_OUTPUT_MAX_LINES).collect();
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

/// 渲染 TodoRead/TodoWrite 工具结果（实心点/空心点样式）
/// expand=true 时额外显示完成/未完成条数统计
fn render_todo_result(
    content: &str,
    content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
    expand: bool,
) {
    if let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(content) {
        // 展开模式：先显示统计信息
        if expand {
            let total = items.len();
            let completed = items
                .iter()
                .filter(|i| i.get("status").and_then(|s| s.as_str()) == Some("completed"))
                .count();
            let pending = total.saturating_sub(completed);

            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(
                    format!("完成 {} / 未完成 {}", completed, pending),
                    Style::default().fg(theme.text_dim),
                ),
            ]));
            lines.push(Line::from(""));
        }

        // 列出每个 todo 项
        for item in &items {
            let status = item
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("pending");
            let text = item
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("(empty)");

            // 实心点 ● 表示已完成/进行中，空心点 ○ 表示未开始
            let (dot, color) = match status {
                "completed" => ("●", theme.label_ai),        // 绿色实心点
                "in_progress" => ("◉", theme.title_loading), // 黄色双圈实心点
                "cancelled" => ("◌", theme.text_dim),        // 灰色空心虚圈
                _ => ("○", Color::Yellow),                   // pending: 黄色空心点
            };

            let max_w = content_w.saturating_sub(10); // "    ● " prefix
            for (i, wrapped) in wrap_text(text, max_w).iter().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        Span::styled("    ", Style::default()),
                        Span::styled(dot, Style::default().fg(color)),
                        Span::styled(" ", Style::default()),
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
                        format!("      {}", wrapped),
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

/// 基于 tick（每 100ms 递增 1）计算当前帧序号
fn current_tick() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
        / 100
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
    let period = THINKING_PULSE_PERIOD_MS as f64;
    let phase = (millis % period as u128) as f64 / period;
    let t = (phase * std::f64::consts::TAU).sin() * 0.5 + 0.5; // 0.0 ~ 1.0

    // 从 label_ai 颜色提取 RGB 分量
    if let Color::Rgb(r, g, b) = theme.label_ai {
        // 在 THINKING_PULSE_MIN_FACTOR ~ 100% 亮度之间脉冲
        let min_factor = THINKING_PULSE_MIN_FACTOR;
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

/// 彗星逐字符渐变渲染：每个非空格字符独立着色，头亮尾暗
///
/// - 使用 `palette` 调色板获取三色渐变元组 (start, mid, end)
/// - 彗星头部（██）使用亮色，拖尾（▓▒░·）逐步衰减
/// - 空格保持背景色
/// - 随 tick 做色相偏移，产生流动感
fn comet_gradient_line(tick: u64, palette_idx: u8, fallback_color: Color) -> Line<'static> {
    let frame = ThinkingStyle::Comet.frame(tick);

    // 收集非空格字符的索引，用于计算渐变映射
    let chars: Vec<char> = frame.chars().collect();
    let non_space_count = chars.iter().filter(|&&c| c != ' ').count();

    // 获取渐变色：每 7 个 tick（~700ms）切换一次色相
    let grad_idx = (tick as usize / 7) % 16;
    let (start_c, mid_c, end_c) = palette::get_gradient(palette_idx, grad_idx);

    // 非空格数不足时回退到单色
    if non_space_count < 2 {
        return Line::from(Span::styled(
            frame.to_string(),
            Style::default().fg(fallback_color),
        ));
    }

    let n = non_space_count;
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(chars.len());
    let mut color_idx = 0usize;

    for &ch in &chars {
        if ch == ' ' {
            spans.push(Span::raw(ch.to_string()));
        } else {
            // t: 0.0（头部/最亮）→ 1.0（尾部/最暗）
            let t = color_idx as f32 / (n - 1).max(1) as f32;
            // 三色分段插值：前半段 start→mid，后半段 mid→end
            let (from, to, local_t) = if t <= 0.5 {
                (start_c, mid_c, t * 2.0)
            } else {
                (mid_c, end_c, (t - 0.5) * 2.0)
            };
            let r = (from.0 as f32 * (1.0 - local_t) + to.0 as f32 * local_t).round() as u8;
            let g = (from.1 as f32 * (1.0 - local_t) + to.1 as f32 * local_t).round() as u8;
            let b = (from.2 as f32 * (1.0 - local_t) + to.2 as f32 * local_t).round() as u8;
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(Color::Rgb(r, g, b)),
            ));
            color_idx += 1;
        }
    }

    Line::from(spans)
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

/// 从工具调用参数 JSON 中提取描述信息
/// - Bash/Shell：提取 description 字段
/// - Read/Write/Edit/Glob/Grep：提取 path 或 file_path 字段
/// - Agent/AgentTeam：提取 description 字段
fn extract_tool_description_from_args(tool_name: &str, arguments: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;

    match tool_name {
        tool_names::BASH => parsed.get("description")?.as_str().map(|s| s.to_string()),
        tool_names::READ
        | tool_names::WRITE
        | tool_names::EDIT
        | tool_names::GLOB
        | tool_names::GREP => parsed
            .get("path")
            .or_else(|| parsed.get("file_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tool_names::AGENT => parsed
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tool_names::TEAMMATE => {
            let name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let role = parsed.get("role").and_then(|v| v.as_str()).unwrap_or(name);
            Some(role.to_string())
        }
        _ => None,
    }
}

/// Teammate 工具参数结构（用于渲染）
struct TeammateCallArgs {
    name: String,
    role: String,
    prompt: String,
    worktree: bool,
}

/// 从 Teammate 工具的 arguments JSON 中提取参数
fn extract_teammate_args(arguments: &str) -> Option<TeammateCallArgs> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;
    Some(TeammateCallArgs {
        name: parsed.get("name")?.as_str()?.to_string(),
        role: parsed
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        prompt: parsed.get("prompt")?.as_str()?.to_string(),
        worktree: parsed
            .get("worktree")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

/// 渲染 Teammate 工具调用请求的展开模式（边框 + name/role + prompt + 元信息）
fn render_teammate_call_request_expanded(
    args: &TeammateCallArgs,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let border_color = theme.text_dim;
    let result_bg = theme.bg_primary;
    let content_w = bubble_max_width.saturating_sub(6);

    // 元信息行：name(role) [worktree]
    let mut meta_parts = vec![format!(
        "{}({})",
        args.name,
        if args.role.is_empty() {
            &args.name
        } else {
            &args.role
        }
    )];
    if args.worktree {
        meta_parts.push("[worktree]".to_string());
    }
    let meta_line = meta_parts.join("  ");
    for wrapped in wrap_text(&meta_line, content_w) {
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default().bg(result_bg)),
            Span::styled(wrapped, Style::default().fg(theme.text_dim).bg(result_bg)),
        ]));
    }

    // Prompt 边框显示
    let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(border_color).bg(result_bg),
    )));

    let prompt_lines: Vec<&str> = args.prompt.lines().collect();
    let total = prompt_lines.len();
    let max_display = AGENT_CALL_PROMPT_MAX_LINES;
    let display_lines = &prompt_lines[..total.min(max_display)];

    for line in display_lines {
        for wrapped in wrap_text(line, content_w) {
            lines.push(bordered_line(
                vec![Span::styled(
                    wrapped,
                    Style::default().fg(theme.text_dim).bg(result_bg),
                )],
                bubble_max_width,
                border_color,
                result_bg,
            ));
        }
    }

    if total > max_display {
        lines.push(bordered_line(
            vec![Span::styled(
                format!("... (共 {} 行)", total),
                Style::default().fg(theme.text_dim).bg(result_bg),
            )],
            bubble_max_width,
            border_color,
            result_bg,
        ));
    }

    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color).bg(result_bg),
    )));
}

/// Bash 工具参数结构
struct BashArgs {
    command: Option<String>,
    timeout: Option<u64>,
    run_in_background: bool,
    cwd: Option<String>,
}

/// 从 Bash 工具的 arguments JSON 中提取参数
fn extract_bash_args(arguments: &str) -> Option<BashArgs> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;

    Some(BashArgs {
        command: parsed
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        timeout: parsed.get("timeout").and_then(|v| v.as_u64()),
        run_in_background: parsed
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        cwd: parsed
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}

/// Agent 工具参数结构（用于渲染）
struct AgentCallArgs {
    prompt: String,
    description: Option<String>,
    run_in_background: bool,
}

/// 从 Agent 工具的 arguments JSON 中提取参数
fn extract_agent_args(arguments: &str) -> Option<AgentCallArgs> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;
    Some(AgentCallArgs {
        prompt: parsed.get("prompt")?.as_str()?.to_string(),
        description: parsed
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        run_in_background: parsed
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

/// 渲染 Agent 工具调用请求的展开模式（边框 + prompt + 元信息）
fn render_agent_call_request_expanded(
    args: &AgentCallArgs,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let border_color = theme.text_dim;
    let result_bg = theme.bg_primary;
    let content_w = bubble_max_width.saturating_sub(6);

    // 元信息行：[background] 标识
    if args.run_in_background {
        for wrapped in wrap_text("[background]", content_w) {
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default().bg(result_bg)),
                Span::styled(wrapped, Style::default().fg(theme.text_dim).bg(result_bg)),
            ]));
        }
    }

    // Prompt 边框显示（复用 render_agent_result_nested 的边框风格）
    let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(border_color).bg(result_bg),
    )));

    let prompt_lines: Vec<&str> = args.prompt.lines().collect();
    let total = prompt_lines.len();
    let max_display = AGENT_CALL_PROMPT_MAX_LINES;
    let display_lines = &prompt_lines[..total.min(max_display)];

    for line in display_lines {
        for wrapped in wrap_text(line, content_w) {
            lines.push(bordered_line(
                vec![Span::styled(
                    wrapped,
                    Style::default().fg(theme.text_dim).bg(result_bg),
                )],
                bubble_max_width,
                border_color,
                result_bg,
            ));
        }
    }

    // 截断提示
    if total > max_display {
        lines.push(bordered_line(
            vec![Span::styled(
                format!("... (共 {} 行)", total),
                Style::default().fg(theme.text_dim).bg(result_bg),
            )],
            bubble_max_width,
            border_color,
            result_bg,
        ));
    }

    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color).bg(result_bg),
    )));
}

/// 渲染 ExitPlanMode 工具调用请求（边框显示）
fn render_exit_plan_mode_request(
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let border_color = theme.text_dim;
    let result_bg = theme.bg_primary;
    let content_w = bubble_max_width.saturating_sub(6);

    // 顶边框
    let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(border_color).bg(result_bg),
    )));

    // 内容：提交计划审批提示
    let hint = "提交计划审批，等待用户批准后退出计划模式";
    for wrapped in wrap_text(hint, content_w) {
        lines.push(bordered_line(
            vec![Span::styled(
                wrapped,
                Style::default().fg(theme.text_dim).bg(result_bg),
            )],
            bubble_max_width,
            border_color,
            result_bg,
        ));
    }

    // 底边框
    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color).bg(result_bg),
    )));
}

/// 渲染 Bash 工具调用请求的展开模式
fn render_bash_call_request_expanded(
    args: &BashArgs,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let content_w = bubble_max_width.saturating_sub(6);

    // 渲染命令行（$ 前缀）
    if let Some(ref cmd) = args.command {
        let cmd_with_prefix = format!("$ {}", cmd);
        for line in crate::util::text::wrap_text(&cmd_with_prefix, content_w) {
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(line, Style::default().fg(theme.text_normal)),
            ]));
        }
    }

    // 渲染附加信息行（后台运行、超时、工作目录）
    let mut meta_parts: Vec<String> = Vec::new();

    if args.run_in_background {
        meta_parts.push("[background]".to_string());
    }

    if let Some(timeout) = args.timeout {
        meta_parts.push(format!("timeout: {}s", timeout));
    }

    if let Some(ref cwd) = args.cwd {
        meta_parts.push(format!("cwd: {}", cwd));
    }

    if !meta_parts.is_empty() {
        let meta_line = meta_parts.join("  ");
        for line in crate::util::text::wrap_text(&meta_line, content_w) {
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(line, Style::default().fg(theme.text_dim)),
            ]));
        }
    }
}
