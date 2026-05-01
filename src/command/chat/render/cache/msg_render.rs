//! 基础消息渲染：用户气泡、AI 气泡、思考内容折叠

use crate::command::chat::storage::DisplayHint;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::command::chat::render::cache::bubble::wrap_md_line_in_bubble;
use crate::command::chat::render::cache::{
    ASSISTANT_BUBBLE_LEFT_MARGIN, BUBBLE_MIN_WIDTH, RenderContext, THINKING_FOLDED_MAX_LINES,
    USER_BUBBLE_PAD_LR,
};
use crate::markdown::markdown_to_lines;
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
/// 使用引用块风格：左侧竖线 `|` + 柔和背景色，复用 theme.md_blockquote_* 配色
/// 折叠模式下（expand=false）仅显示前若干行，超出部分用省略提示
pub(crate) fn render_thinking_block(reasoning: &str, ctx: &mut RenderContext<'_>) {
    let lines = &mut *ctx.lines;
    let theme = ctx.theme;
    let bubble_max_width = ctx.bubble_max_width;
    let expand = ctx.expand;
    if reasoning.is_empty() {
        return;
    }

    // 引用块配色（复用 markdown blockquote 前景，背景使用 bg_primary）
    let bar_color = theme.md_blockquote_bar;
    let text_color = theme.md_blockquote_text;
    let bg_color = theme.bg_primary;

    // 竖线前缀样式
    let bar_style = Style::default()
        .fg(bar_color)
        .bg(bg_color)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default().fg(text_color).bg(bg_color);

    lines.push(Line::from(""));

    // Thinking 标签行：竖线 + 标签文字（斜体）
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled("| ", bar_style),
        Span::styled(
            "Thinking...",
            Style::default()
                .fg(bar_color)
                .bg(bg_color)
                .add_modifier(Modifier::ITALIC),
        ),
    ]));

    // Reasoning 内容（引用块风格，每行带 `| ` 前缀 + 背景色）
    let content_w = bubble_max_width.saturating_sub(6); // 2空格缩进 + "| " + 1右侧留白
    let wrapped = wrap_text(reasoning, content_w);

    // 折叠模式：最多显示 THINKING_FOLDED_MAX_LINES 行
    let total = wrapped.len();
    let (shown, truncated) = if !expand && total > THINKING_FOLDED_MAX_LINES {
        (&wrapped[..THINKING_FOLDED_MAX_LINES], true)
    } else {
        (&wrapped[..], false)
    };

    for wrapped_line in shown {
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("| ", bar_style),
            Span::styled(wrapped_line.clone(), text_style),
        ]));
    }

    if truncated {
        let hidden = total - THINKING_FOLDED_MAX_LINES;
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("| ", bar_style),
            Span::styled(
                format!("... +{} 行, Ctrl+O 展开", hidden),
                Style::default()
                    .fg(bar_color)
                    .bg(bg_color)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    }
}

/// 渲染用户消息
pub fn render_user_msg(
    content: &str,
    is_selected: bool,
    actual_inner_content_width: usize,
    ctx: &mut RenderContext<'_>,
) {
    let lines = &mut *ctx.lines;
    let theme = ctx.theme;
    let bubble_max_width = ctx.bubble_max_width;

    // 与前一条消息之间留一行间距
    lines.push(Line::from(""));

    let user_bg = if is_selected {
        theme.bubble_user_selected
    } else if ctx.flat_bubble {
        theme.bg_primary
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

    // label 行（独立行，无背景）
    // 用户标签右对齐，使其位于气泡的右上方
    let label_color = if is_selected {
        theme.label_selected
    } else {
        theme.label_user
    };
    let label = if is_selected { "▶ You " } else { "You " };
    let label_w = display_width(label);
    let left_pad = actual_inner_content_width.saturating_sub(label_w);
    lines.push(Line::from(vec![
        Span::raw(" ".repeat(left_pad)),
        Span::styled(
            label.to_string(),
            Style::default()
                .fg(label_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // 边框颜色
    let border_color = if is_selected {
        theme.label_selected
    } else {
        theme.border_message
    };
    // 内容行：│ + pad + content + fill + pad + │

    // 顶行：╭─...─╮
    {
        let dash_w = actual_inner_content_w + user_pad_lr * 2;
        let pad = actual_inner_content_width.saturating_sub(dash_w + 2);
        lines.push(Line::from(vec![
            Span::raw(" ".repeat(pad)),
            Span::styled("╭", Style::default().fg(border_color).bg(user_bg)),
            Span::styled(
                "─".repeat(dash_w),
                Style::default().fg(border_color).bg(user_bg),
            ),
            Span::styled("╮", Style::default().fg(border_color).bg(user_bg)),
        ]));
    }
    // 内容行：│ pad content fill pad │
    for wl in &all_wrapped_lines {
        let wl_width = display_width(wl);
        let fill = actual_inner_content_w.saturating_sub(wl_width);
        let pad =
            actual_inner_content_width.saturating_sub(actual_inner_content_w + user_pad_lr * 2 + 2);
        lines.push(Line::from(vec![
            Span::raw(" ".repeat(pad)),
            Span::styled("│", Style::default().fg(border_color).bg(user_bg)),
            Span::styled(" ".repeat(user_pad_lr), Style::default().bg(user_bg)),
            Span::styled(
                wl.clone(),
                Style::default().fg(theme.text_white).bg(user_bg),
            ),
            Span::styled(" ".repeat(fill), Style::default().bg(user_bg)),
            Span::styled(" ".repeat(user_pad_lr), Style::default().bg(user_bg)),
            Span::styled("│", Style::default().fg(border_color).bg(user_bg)),
        ]));
    }
    // 底行：╰─...─╯
    {
        let dash_w = actual_inner_content_w + user_pad_lr * 2;
        let pad = actual_inner_content_width.saturating_sub(dash_w + 2);
        lines.push(Line::from(vec![
            Span::raw(" ".repeat(pad)),
            Span::styled("╰", Style::default().fg(border_color).bg(user_bg)),
            Span::styled(
                "─".repeat(dash_w),
                Style::default().fg(border_color).bg(user_bg),
            ),
            Span::styled("╯", Style::default().fg(border_color).bg(user_bg)),
        ]));
    }
}

/// 渲染 AI 助手消息（含 teammate/subagent 消息）
/// 气泡宽度根据实际内容自适应：最小宽度 20，最大宽度为传入的 bubble_max_width
///
/// # 参数
/// - `sender_name`: 消息发送者名称（如 `Teammate@Frontend`）。优先使用此字段作为气泡标签。
///   若为 None，则尝试从 content 解析 `<Name> ...` 前缀（兼容老 session）。
/// - `recipient_name`: 消息目标接收者名称。若非 None 则在 label 行显示 → Target。
/// - `content`: 消息正文（不含 sender_name 前缀）
/// - `display_hint`: 显示提示（Draft 表示内部思考，渲染为淡化样式）
pub fn render_assistant_msg(
    sender_name: Option<&str>,
    recipient_name: Option<&str>,
    content: &str,
    is_selected: bool,
    display_hint: DisplayHint,
    ctx: &mut RenderContext<'_>,
) {
    let lines = &mut *ctx.lines;
    let theme = ctx.theme;
    let bubble_max_width = ctx.bubble_max_width;

    if content.is_empty() {
        return;
    }

    let pad_left_w = 3usize;
    let pad_right_w = 3usize;
    let margin = " ".repeat(ASSISTANT_BUBBLE_LEFT_MARGIN);

    let is_draft = display_hint == DisplayHint::Draft;

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
    } else if ctx.flat_bubble {
        theme.bg_primary
    } else {
        theme.bubble_ai
    };

    // 与前一条消息之间留一行间距
    lines.push(Line::from(""));

    // label 行（独立行，无背景）
    let recipient_suffix = recipient_name
        .map(|r| format!(" → {}", r))
        .unwrap_or_default();
    let label_text = if is_selected {
        format!("{}▶ {}{}", margin, agent_name, recipient_suffix)
    } else if is_draft {
        format!("{}{} [draft]", margin, agent_name)
    } else {
        format!("{}{}{}", margin, agent_name, recipient_suffix)
    };
    let label_color = if is_selected {
        theme.label_selected
    } else if is_draft {
        theme.text_dim
    } else if is_teammate {
        agent_name_color(&agent_name)
    } else {
        theme.label_ai
    };
    let label_modifier = if is_draft {
        Modifier::ITALIC
    } else {
        Modifier::BOLD
    };
    lines.push(Line::from(Span::styled(
        label_text,
        Style::default()
            .fg(label_color)
            .add_modifier(label_modifier),
    )));

    // 边框颜色
    let border_color = if is_selected {
        theme.label_selected
    } else {
        theme.border_message
    };

    // 先用最大宽度渲染 markdown 内容（减去边框占宽：│1 + space1 + ... + space1 + │1 = 4）
    let md_content_w = bubble_max_width
        .saturating_sub(pad_left_w + pad_right_w + ASSISTANT_BUBBLE_LEFT_MARGIN + 2);
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

    // 气泡内部宽度 = pad + content + pad
    let bubble_actual_inner_content_w = (actual_content_max_w + pad_left_w + pad_right_w)
        .max(BUBBLE_MIN_WIDTH)
        .min(bubble_max_width.saturating_sub(ASSISTANT_BUBBLE_LEFT_MARGIN + 2));
    // 边框顶/底行 ─ 数量
    let dash_w = bubble_actual_inner_content_w;

    // 顶行：margin + ╭─...─╮
    lines.push(Line::from(vec![
        Span::styled(margin.clone(), Style::default()),
        Span::styled("╭", Style::default().fg(border_color).bg(bubble_bg)),
        Span::styled(
            "─".repeat(dash_w),
            Style::default().fg(border_color).bg(bubble_bg),
        ),
        Span::styled("╮", Style::default().fg(border_color).bg(bubble_bg)),
    ]));

    // 内容行：margin + │ + bubble content + │
    for md_line in md_lines {
        let mut bubble_line = wrap_md_line_in_bubble(
            md_line,
            bubble_bg,
            pad_left_w,
            pad_right_w,
            bubble_actual_inner_content_w,
        );
        // 在内容两侧加 │
        let last = bubble_line.spans.pop();
        // 移除 pad_right 并在右侧加 │
        // 实际上 wrap_md_line_in_bubble 末尾是 pad_right span，需要替换
        // 更简单：直接构造 │ + bubble内容（去掉pad_right） + │
        // 但改 wrap_md_line_in_bubble 影响太大，用 bordered_line 思路
        if let Some(pad_span) = last {
            // pad_span 是右侧 padding，保留它
            bubble_line.spans.push(pad_span);
        }
        let mut spans = vec![Span::styled(margin.clone(), Style::default())];
        spans.push(Span::styled(
            "│",
            Style::default().fg(border_color).bg(bubble_bg),
        ));
        spans.extend(bubble_line.spans);
        spans.push(Span::styled(
            "│",
            Style::default().fg(border_color).bg(bubble_bg),
        ));
        lines.push(Line::from(spans));
    }

    // 底行：margin + ╰─...─╯
    lines.push(Line::from(vec![
        Span::styled(margin.clone(), Style::default()),
        Span::styled("╰", Style::default().fg(border_color).bg(bubble_bg)),
        Span::styled(
            "─".repeat(dash_w),
            Style::default().fg(border_color).bg(bubble_bg),
        ),
        Span::styled("╯", Style::default().fg(border_color).bg(bubble_bg)),
    ]));
}
