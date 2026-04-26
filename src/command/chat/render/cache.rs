//! 消息渲染缓存模块
//!
//! 负责增量构建所有消息的渲染行，按职责拆分为以下子模块：
//! - `bubble` — 气泡布局工具
//! - `msg_render` — 用户/AI 消息渲染
//! - `confirm_render` — 确认/交互区域渲染
//! - `tool_call_render` — 工具调用请求渲染
//! - `tool_result_render` — 工具结果渲染
//! - `animation` — 动画效果
//! - `clipboard` — 剪贴板操作

pub mod animation;
pub mod bubble;
pub mod clipboard;
pub mod confirm_render;
pub mod msg_render;
pub mod tool_call_render;
pub mod tool_result_render;

// ── Re-export 公共 API（保持外部引用路径不变）──
pub use clipboard::copy_to_clipboard;
pub use msg_render::{render_assistant_msg, render_user_msg};
pub use tool_call_render::render_tool_call_request_msg;
pub use tool_result_render::render_tool_result_msg;

use super::theme::Theme;
use crate::command::chat::app::{ChatApp, ChatMode, MsgLinesCache, PerMsgCache};
use crate::command::chat::markdown::markdown_to_lines;
use crate::command::chat::storage::DisplayType;
use crate::command::chat::storage::config::ThinkingStyle;
use crate::util::safe_lock;
use crate::util::text::wrap_text;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use std::sync::Arc;

use animation::{comet_gradient_line, current_tick, thinking_pulse_color};
use bubble::{wrap_md_line_in_bubble, wrap_md_line_in_bubble_with_margin};
use confirm_render::{
    render_agent_perm_confirm_area, render_plan_approval_confirm_area, render_tool_confirm_area,
};
// pub use 在上方已导入 render_assistant_msg, render_user_msg, render_tool_call_request_msg, render_tool_result_msg
// 此处只导入额外需要的内部函数
use msg_render::render_thinking_block;

// ── 模块级常量（提取自各渲染函数中的魔法值）──

/// `render_thinking_block` 折叠模式下最大显示行数
pub(crate) const THINKING_FOLDED_MAX_LINES: usize = 5;
/// `render_assistant_msg` 气泡最小宽度（字符列数）
pub(crate) const BUBBLE_MIN_WIDTH: usize = 20;
/// `render_assistant_msg` 气泡左边距（字符列数），避免气泡贴近消息区左边界
pub(crate) const ASSISTANT_BUBBLE_LEFT_MARGIN: usize = 2;
/// `render_user_msg` 用户气泡左右内边距（字符列数）
pub(crate) const USER_BUBBLE_PAD_LR: usize = 3;
/// `render_tool_result_msg` / `render_bash_result` 普通结果截断显示的行数上限
pub(crate) const TOOL_RESULT_DISPLAY_MAX_LINES: usize = 100;
/// Plan 内容折叠时最大显示行数。
pub(crate) const PLAN_DISPLAY_MAX_LINES: usize = 20;

// ── 渲染上下文结构体（提取自多参数渲染函数的公共参数）──

/// 消息渲染的公共上下文
pub struct RenderContext<'a> {
    pub bubble_max_width: usize,
    pub lines: &'a mut Vec<Line<'static>>,
    pub theme: &'a Theme,
    pub expand: bool,
}

/// 内容渲染的公共上下文（用于工具结果等）
pub(crate) struct ContentContext<'a> {
    pub content_w: usize,
    pub lines: &'a mut Vec<Line<'static>>,
    pub theme: &'a Theme,
    pub expand: bool,
}

/// 在 Markdown 内容中查找一个安全的截断边界，确保不会在代码围栏中间截断。
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

    // ★ UI 渲染从 display_messages 读取（干净文本 + sender_name）
    let display_msgs = safe_lock(&app.display_messages, "render_cache::display_msgs");
    let msg_count = display_msgs.len();
    let mut current_line_offset: usize = 0;
    let mut msg_start_lines: Vec<(usize, usize)> = Vec::with_capacity(msg_count);
    let mut per_msg_cache: Vec<PerMsgCache> = Vec::with_capacity(msg_count);

    let expand = app.ui.expand_tools;

    // 判断旧缓存中的 per_msg_lines 是否可以复用（bubble_max_width 相同且 expand 一致）
    let can_reuse_per_msg = old_cache
        .map(|c| c.bubble_max_width == bubble_max_width && c.expand_tools == expand)
        .unwrap_or(false);

    // ===== P0 优化：直接引用 display_messages，避免整 Vec clone =====
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
                let mut ctx = RenderContext {
                    bubble_max_width,
                    lines: &mut tmp_lines,
                    theme: t,
                    expand,
                };
                render_user_msg(&m.content, is_selected, inner_width, &mut ctx);
            }
            DisplayType::AssistantText => {
                let mut ctx = RenderContext {
                    bubble_max_width,
                    lines: &mut tmp_lines,
                    theme: t,
                    expand,
                };
                // 如果有 reasoning_content，先渲染 thinking 区块
                if let Some(ref reasoning) = m.reasoning_content {
                    render_thinking_block(reasoning, &mut ctx);
                }
                render_assistant_msg(m.sender_name.as_deref(), &m.content, is_selected, &mut ctx);
            }
            DisplayType::ToolCallRequest => {
                let mut ctx = RenderContext {
                    bubble_max_width,
                    lines: &mut tmp_lines,
                    theme: t,
                    expand,
                };
                // 如果有 reasoning_content，先渲染 thinking 区块
                if let Some(ref reasoning) = m.reasoning_content {
                    render_thinking_block(reasoning, &mut ctx);
                }
                // 先渲染文本内容（如果有）— LLM 可能同时返回文本解释和工具调用
                if !m.content.is_empty() {
                    render_assistant_msg(
                        m.sender_name.as_deref(),
                        &m.content,
                        is_selected,
                        &mut ctx,
                    );
                }
                // 再渲染工具调用
                if let Some(ref tool_calls) = m.tool_calls {
                    render_tool_call_request_msg(m.sender_name.as_deref(), tool_calls, &mut ctx);
                }
            }
            DisplayType::ToolResult => {
                // 查找对应的工具名：向前搜索 assistant 消息中匹配 tool_call_id 的 ToolCallItem
                let tool_name = m
                    .tool_call_id
                    .as_ref()
                    .and_then(|tid| {
                        display_msgs[..idx].iter().rev().find_map(|prev| {
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
                    display_msgs[..idx].iter().rev().find_map(|prev| {
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
                    m.sender_name.as_deref(),
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
        let margin_str = " ".repeat(ASSISTANT_BUBBLE_LEFT_MARGIN);
        let md_content_w = bubble_max_width
            .saturating_sub(pad_left_w + pad_right_w + ASSISTANT_BUBBLE_LEFT_MARGIN);
        let inner_bubble_w = bubble_max_width.saturating_sub(ASSISTANT_BUBBLE_LEFT_MARGIN);

        // AI 标签
        streaming_lines.push(Line::from(""));
        streaming_lines.push(Line::from(Span::styled(
            format!("{}Sprite", margin_str),
            Style::default().fg(t.label_ai).add_modifier(Modifier::BOLD),
        )));

        // 上边距
        streaming_lines.push(Line::from(vec![
            Span::styled(margin_str.clone(), Style::default()),
            Span::styled(" ".repeat(inner_bubble_w), Style::default().bg(bubble_bg)),
        ]));

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
            let bubble_line = wrap_md_line_in_bubble_with_margin(
                indicator_line,
                bubble_bg,
                pad_left_w,
                pad_right_w,
                inner_bubble_w,
                &margin_str,
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
                let label_bubble = wrap_md_line_in_bubble_with_margin(
                    thinking_label,
                    bubble_bg,
                    pad_left_w,
                    pad_right_w,
                    inner_bubble_w,
                    &margin_str,
                );
                streaming_lines.push(label_bubble);

                // Reasoning 内容（灰色文本，带气泡背景）
                let reason_content_w = md_content_w.saturating_sub(2);
                for wrapped_line in wrap_text(&reasoning_str, reason_content_w) {
                    let line = Line::from(Span::styled(
                        format!("  {}", wrapped_line),
                        Style::default().fg(t.text_dim),
                    ));
                    let bubble_line = wrap_md_line_in_bubble_with_margin(
                        line,
                        bubble_bg,
                        pad_left_w,
                        pad_right_w,
                        inner_bubble_w,
                        &margin_str,
                    );
                    streaming_lines.push(bubble_line);
                }
            }

            // 下边距
            streaming_lines.push(Line::from(vec![
                Span::styled(margin_str.clone(), Style::default()),
                Span::styled(" ".repeat(inner_bubble_w), Style::default().bg(bubble_bg)),
            ]));
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
                        inner_bubble_w,
                    );
                    stable_lines.push(bubble_line);
                }
            }
            final_stable_offset = boundary;

            // 追加已缓存的稳定段落行（带 margin 前缀）
            for sl in stable_lines.iter() {
                let mut line = sl.clone();
                line.spans
                    .insert(0, Span::styled(margin_str.clone(), Style::default()));
                streaming_lines.push(line);
            }

            // 只对最后一个不完整段落做全量 Markdown 解析
            let tail = &content[boundary..];
            if !tail.is_empty() {
                let tail_md_lines = markdown_to_lines(tail, md_content_w + 2, t);
                for md_line in tail_md_lines {
                    let bubble_line = wrap_md_line_in_bubble_with_margin(
                        md_line,
                        bubble_bg,
                        pad_left_w,
                        pad_right_w,
                        inner_bubble_w,
                        &margin_str,
                    );
                    streaming_lines.push(bubble_line);
                }
            }

            // 下边距
            streaming_lines.push(Line::from(vec![
                Span::styled(margin_str.clone(), Style::default()),
                Span::styled(" ".repeat(inner_bubble_w), Style::default().bg(bubble_bg)),
            ]));
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
