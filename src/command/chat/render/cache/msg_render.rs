//! 基础消息渲染：用户气泡、AI 气泡、思考内容折叠

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::command::chat::markdown::markdown_to_lines;
use crate::command::chat::render::cache::bubble::wrap_md_line_in_bubble_with_margin;
use crate::command::chat::render::cache::{
    ASSISTANT_BUBBLE_LEFT_MARGIN, BUBBLE_MIN_WIDTH, RenderContext, THINKING_FOLDED_MAX_LINES,
    USER_BUBBLE_PAD_LR,
};
use crate::util::text::{display_width, wrap_text};

/// 解析 teammate 消息的 `<AgentName>` 前缀。
/// 返回 `Some((name, rest))` 其中 rest 已去除前导空格。
/// 规则：内容以 `<` 开头，紧跟非 `>` 字符，直到 `>`，后面是消息正文。
/// 支持 `<Type@Name>` 格式（如 `<Teammate@Frontend>`、`<SubAgent@search_auth>`、`<Teammate@Go Advocate>`）。
pub(crate) fn parse_agent_prefix(content: &str) -> Option<(&str, &str)> {
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
pub(crate) fn agent_name_color(name: &str) -> Color {
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
pub(crate) fn render_thinking_block(reasoning: &str, ctx: &mut RenderContext<'_>) {
    let lines = &mut *ctx.lines;
    let theme = ctx.theme;
    let bubble_max_width = ctx.bubble_max_width;
    let expand = ctx.expand;
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

/// 渲染用户消息
pub fn render_user_msg(
    content: &str,
    is_selected: bool,
    inner_width: usize,
    ctx: &mut RenderContext<'_>,
) {
    let lines = &mut *ctx.lines;
    let theme = ctx.theme;
    let bubble_max_width = ctx.bubble_max_width;
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

    // 顶行：label + 气泡背景填充合一，视觉紧凑
    let label_color = if is_selected {
        theme.label_selected
    } else {
        theme.label_user
    };
    {
        let label = if is_selected { "▶ You " } else { "You " };
        let label_w = display_width(label);
        let bubble_fill_w = actual_bubble_w.saturating_sub(label_w);
        let left_pad = inner_width.saturating_sub(actual_bubble_w);
        lines.push(Line::from(vec![
            Span::raw(" ".repeat(left_pad)),
            Span::styled(
                label.to_string(),
                Style::default()
                    .fg(label_color)
                    .bg(user_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ".repeat(bubble_fill_w), Style::default().bg(user_bg)),
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
    ctx: &mut RenderContext<'_>,
) {
    let lines = &mut *ctx.lines;
    let theme = ctx.theme;
    let bubble_max_width = ctx.bubble_max_width;

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

    let bubble_bg = if is_selected {
        theme.bubble_ai_selected
    } else {
        theme.bubble_ai
    };
    let pad_left_w = 3usize;
    let pad_right_w = 3usize;
    let margin = " ".repeat(ASSISTANT_BUBBLE_LEFT_MARGIN);

    // 先用最大宽度渲染 markdown 内容
    let md_content_w =
        bubble_max_width.saturating_sub(pad_left_w + pad_right_w + ASSISTANT_BUBBLE_LEFT_MARGIN);
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

    // 气泡自适应宽度：min(max(实际宽度+padding+margin, 最小宽度), 最大宽度)
    let bubble_total_w =
        (actual_content_max_w + pad_left_w + pad_right_w + ASSISTANT_BUBBLE_LEFT_MARGIN)
            .max(BUBBLE_MIN_WIDTH + ASSISTANT_BUBBLE_LEFT_MARGIN)
            .min(bubble_max_width);

    // 顶行：label + 气泡背景填充合一，视觉紧凑
    let label_text = if is_selected {
        format!("{}▶ {}", margin, agent_name)
    } else {
        format!("{}{}", margin, agent_name)
    };
    let label_color = if is_selected {
        theme.label_selected
    } else if is_teammate {
        agent_name_color(&agent_name)
    } else {
        theme.label_ai
    };
    let label_w = display_width(&label_text);
    let bubble_fill_w = bubble_total_w.saturating_sub(label_w);
    lines.push(Line::from(vec![
        Span::styled(
            label_text,
            Style::default()
                .fg(label_color)
                .bg(bubble_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ".repeat(bubble_fill_w), Style::default().bg(bubble_bg)),
    ]));
    for md_line in md_lines {
        let inner_bubble_w = bubble_total_w.saturating_sub(ASSISTANT_BUBBLE_LEFT_MARGIN);
        let bubble_line = wrap_md_line_in_bubble_with_margin(
            md_line,
            bubble_bg,
            pad_left_w,
            pad_right_w,
            inner_bubble_w,
            &margin,
        );
        lines.push(bubble_line);
    }
    // 下边距
    lines.push(Line::from(vec![
        Span::styled(margin.clone(), Style::default()),
        Span::styled(
            " ".repeat(bubble_total_w.saturating_sub(ASSISTANT_BUBBLE_LEFT_MARGIN)),
            Style::default().bg(bubble_bg),
        ),
    ]));
}
