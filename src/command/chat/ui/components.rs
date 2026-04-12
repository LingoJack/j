use crate::command::chat::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

// ── 常量 ──────────────────────────────────────────────

pub const POINTER_SELECTED: &str = "  ❯ ";
pub const POINTER_EMPTY: &str = "    ";
pub const TOGGLE_ON: &str = "\u{25cf}";
pub const TOGGLE_OFF: &str = "\u{25cb}";
pub const SEPARATOR_V: &str = "\u{2502}";
pub const INDENT: &str = "  ";
pub const LABEL_WIDTH: usize = 10;

// ── 分隔线 ────────────────────────────────────────────

/// 自适应宽度分隔线（替代硬编码 41 字符的 "─────…"）
pub fn separator_line(width: u16, theme: &Theme) -> Line<'static> {
    let w = (width as usize).saturating_sub(4); // 左缩进 2 字符 + 右留 2
    let bar: String = "\u{2500}".repeat(w);
    Line::from(Span::styled(
        format!("{INDENT}{bar}"),
        Style::default().fg(theme.separator),
    ))
}

// ── 章节标题 ──────────────────────────────────────────

/// 章节标题（如 "📖 快捷键帮助"）
pub fn section_header<'a>(icon: &str, title: &str, theme: &Theme) -> Line<'a> {
    Line::from(Span::styled(
        format!("{INDENT}{icon} {title}"),
        Style::default()
            .fg(theme.help_title)
            .add_modifier(Modifier::BOLD),
    ))
}

// ── 指针 / 标签 ──────────────────────────────────────

/// 选中指针 span
pub fn pointer_span<'a>(selected: bool, theme: &Theme) -> Span<'a> {
    if selected {
        Span::styled(POINTER_SELECTED, Style::default().fg(theme.config_pointer))
    } else {
        Span::styled(POINTER_EMPTY, Style::default())
    }
}

/// 标签 span（固定宽度，左对齐）
pub fn label_span<'a>(text: &str, width: usize, selected: bool, theme: &Theme) -> Span<'a> {
    let style = if selected {
        Style::default()
            .fg(theme.config_label_selected)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.config_label)
    };
    Span::styled(format!("{:<width$}", text, width = width), style)
}

/// 值的样式（普通/选中/编辑中）
fn value_style(selected: bool, editing: bool, theme: &Theme) -> Style {
    if editing && selected {
        Style::default()
            .fg(theme.text_white)
            .bg(theme.config_edit_bg)
    } else if selected {
        Style::default().fg(theme.text_white)
    } else {
        Style::default().fg(theme.config_value)
    }
}

// ── 行内光标 ─────────────────────────────────────────

/// 构建带行内光标的 span 列表（从 config.rs render_cursor_spans 迁移）
pub fn cursor_spans<'a>(value: &str, cursor: usize, style: Style, theme: &Theme) -> Vec<Span<'a>> {
    let chars: Vec<char> = value.chars().collect();
    let before: String = chars[..cursor.min(chars.len())].iter().collect();
    let cursor_ch = if cursor < chars.len() {
        chars[cursor].to_string()
    } else {
        " ".to_string()
    };
    let after: String = if cursor < chars.len() {
        chars[cursor + 1..].iter().collect()
    } else {
        String::new()
    };
    vec![
        Span::styled(before, style),
        Span::styled(
            cursor_ch,
            Style::default().fg(theme.cursor_fg).bg(theme.cursor_bg),
        ),
        Span::styled(after, style),
        Span::styled(" \u{270f}\u{fe0f}", Style::default()),
    ]
}

// ── 预览值 ───────────────────────────────────────────

/// 长文本截断预览（替换换行为空格，超 40 字符截断）
fn render_preview_value(raw: &str) -> String {
    if raw.is_empty() {
        return "(\u{7a7a})".to_string();
    }
    let flat: String = raw
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    if flat.chars().count() > 40 {
        let truncated: String = flat.chars().take(40).collect();
        format!("{truncated}...")
    } else {
        flat
    }
}

// ── 开关行 ───────────────────────────────────────────

/// 开关字段行（auto_restore_session 等）
pub fn toggle_row<'a>(
    label: &str,
    is_on: bool,
    selected: bool,
    hint: &str,
    theme: &Theme,
) -> Line<'a> {
    let toggle_style = if is_on {
        Style::default()
            .fg(theme.config_toggle_on)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.config_toggle_off)
    };
    let toggle_text = if is_on {
        format!("{TOGGLE_ON} \u{5f00}\u{542f}")
    } else {
        format!("{TOGGLE_OFF} \u{5173}\u{95ed}")
    };
    Line::from(vec![
        pointer_span(selected, theme),
        label_span(label, LABEL_WIDTH, selected, theme),
        Span::styled("  ", Style::default()),
        Span::styled(toggle_text, toggle_style),
        Span::styled(
            if selected {
                format!("  ({hint})")
            } else {
                String::new()
            },
            Style::default().fg(theme.config_dim),
        ),
    ])
}

// ── 可编辑文本字段行 ─────────────────────────────────

/// 普通可编辑文本字段行
pub fn text_field_row<'a>(
    label: &str,
    value: &str,
    selected: bool,
    editing: bool,
    cursor: usize,
    theme: &Theme,
) -> Line<'a> {
    let vs = value_style(selected, editing, theme);
    if editing && selected {
        let mut spans = vec![
            pointer_span(selected, theme),
            label_span(label, LABEL_WIDTH, selected, theme),
            Span::styled("  ", Style::default()),
        ];
        spans.extend(cursor_spans(value, cursor, vs, theme));
        Line::from(spans)
    } else {
        Line::from(vec![
            pointer_span(selected, theme),
            label_span(label, LABEL_WIDTH, selected, theme),
            Span::styled("  ", Style::default()),
            Span::styled(
                if value.is_empty() {
                    "(\u{7a7a})".to_string()
                } else {
                    value.to_string()
                },
                vs,
            ),
        ])
    }
}

// ── API Key 遮罩字段行 ──────────────────────────────

/// API Key 字段（未编辑时使用 api_key 颜色）
pub fn secret_field_row<'a>(
    label: &str,
    value: &str,
    selected: bool,
    editing: bool,
    cursor: usize,
    theme: &Theme,
) -> Line<'a> {
    if editing && selected {
        let vs = value_style(selected, editing, theme);
        let mut spans = vec![
            pointer_span(selected, theme),
            label_span(label, LABEL_WIDTH, selected, theme),
            Span::styled("  ", Style::default()),
        ];
        spans.extend(cursor_spans(value, cursor, vs, theme));
        Line::from(spans)
    } else {
        let vs = if selected {
            Style::default().fg(theme.text_white)
        } else {
            Style::default().fg(theme.config_api_key)
        };
        Line::from(vec![
            pointer_span(selected, theme),
            label_span(label, LABEL_WIDTH, selected, theme),
            Span::styled("  ", Style::default()),
            Span::styled(
                if value.is_empty() {
                    "(\u{7a7a})".to_string()
                } else {
                    value.to_string()
                },
                vs,
            ),
        ])
    }
}

// ── 长文本预览字段行 ─────────────────────────────────

/// 长文本预览行（system_prompt / style）
pub fn preview_field_row<'a>(
    label: &str,
    raw: &str,
    selected: bool,
    hint: &str,
    theme: &Theme,
) -> Line<'a> {
    let vs = value_style(selected, false, theme);
    Line::from(vec![
        pointer_span(selected, theme),
        label_span(label, LABEL_WIDTH, selected, theme),
        Span::styled("  ", Style::default()),
        Span::styled(render_preview_value(raw), vs),
        Span::styled(
            if selected {
                format!("  ({hint})")
            } else {
                String::new()
            },
            Style::default().fg(theme.config_dim),
        ),
    ])
}

// ── 主题选择字段行 ───────────────────────────────────

/// 主题选择行
pub fn theme_field_row<'a>(
    label: &str,
    name: &str,
    selected: bool,
    hint: &str,
    theme: &Theme,
) -> Line<'a> {
    Line::from(vec![
        pointer_span(selected, theme),
        label_span(label, LABEL_WIDTH, selected, theme),
        Span::styled("  ", Style::default()),
        Span::styled(
            format!("\u{1f3a8} {name}"),
            Style::default()
                .fg(theme.config_toggle_on)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if selected {
                format!("  ({hint})")
            } else {
                String::new()
            },
            Style::default().fg(theme.config_dim),
        ),
    ])
}

// ── 开关列表项 ──────────────────────────────────────

/// 工具/技能/命令的开关列表项
pub fn toggle_list_item<'a>(
    name: &str,
    enabled: bool,
    selected: bool,
    desc: Option<&str>,
    tag: Option<&str>,
    theme: &Theme,
) -> Line<'a> {
    let toggle_style = if enabled {
        Style::default()
            .fg(theme.config_toggle_on)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.config_toggle_off)
    };
    let toggle_text = if enabled { TOGGLE_ON } else { TOGGLE_OFF };
    let name_style = if selected {
        Style::default()
            .fg(theme.config_label_selected)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.config_label)
    };

    let mut spans = vec![
        pointer_span(selected, theme),
        Span::styled(toggle_text, toggle_style),
        Span::styled(" ", Style::default()),
        Span::styled(name.to_string(), name_style),
    ];
    if let Some(d) = desc {
        spans.push(Span::styled(
            format!("  {d}"),
            Style::default().fg(theme.config_dim),
        ));
    }
    if let Some(t) = tag {
        spans.push(Span::styled(
            format!(" [{t}]"),
            Style::default().fg(theme.config_dim),
        ));
    }
    Line::from(spans)
}

// ── 可选行（session / archive 列表） ────────────────

/// 可选行（主文本 + 次要信息）
pub fn selectable_row<'a>(
    primary: &str,
    secondary: &str,
    selected: bool,
    theme: &Theme,
) -> Line<'a> {
    let name_style = if selected {
        Style::default()
            .fg(theme.config_label_selected)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.config_label)
    };
    Line::from(vec![
        pointer_span(selected, theme),
        Span::styled(primary.to_string(), name_style),
        Span::styled(
            format!("  {secondary}"),
            Style::default().fg(theme.config_dim),
        ),
    ])
}

// ── Tab 栏 ──────────────────────────────────────────

/// Tab 栏（支持任意 tab 列表）
pub fn tab_bar<'a>(tabs: &[(&str, bool)], hint: &str, theme: &Theme) -> Line<'a> {
    let mut spans: Vec<Span<'a>> = vec![Span::styled("  ", Style::default())];
    for (i, (label, active)) in tabs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                format!(" {SEPARATOR_V} "),
                Style::default().fg(theme.separator),
            ));
        }
        let text = format!(" {label} ");
        if *active {
            spans.push(Span::styled(
                text,
                Style::default()
                    .fg(theme.config_tab_active_fg)
                    .bg(theme.config_tab_active_bg)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                text,
                Style::default().fg(theme.config_tab_inactive),
            ));
        }
    }
    if !hint.is_empty() {
        spans.push(Span::styled(
            format!("    ({hint})"),
            Style::default().fg(theme.config_dim),
        ));
    }
    Line::from(spans)
}

// ── 帮助页快捷键行 ──────────────────────────────────

/// 帮助页快捷键行
pub fn help_key_row<'a>(key: &str, desc: &str, key_width: usize, theme: &Theme) -> Line<'a> {
    Line::from(vec![
        Span::styled(INDENT, Style::default()),
        Span::styled(
            format!("{:<width$}", key, width = key_width),
            Style::default()
                .fg(theme.help_key)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_string(), Style::default().fg(theme.help_desc)),
    ])
}

// ── 底部提示栏单项 ──────────────────────────────────

/// 底部提示栏单项 spans
pub fn hint_spans<'a>(key: &str, desc: &str, theme: &Theme) -> Vec<Span<'a>> {
    vec![
        Span::styled(
            format!(" {key} "),
            Style::default().fg(theme.hint_key_fg).bg(theme.hint_key_bg),
        ),
        Span::styled(format!(" {desc}"), Style::default().fg(theme.hint_desc)),
    ]
}

// ── 欢迎框 ──────────────────────────────────────────

/// 自适应居中欢迎框
pub fn welcome_box<'a>(width: u16, theme: &Theme, quote_idx: usize) -> Vec<Line<'a>> {
    use unicode_width::UnicodeWidthStr;

    // 框体内部宽度：取终端内宽的一半，最少 30，最多 50
    let inner = ((width as usize) / 2).clamp(30, 50);
    let box_w = inner + 2; // 包含左右边框字符

    let h_bar_top: String = format!("\u{256d}{}\u{256e}", "\u{2500}".repeat(inner));
    let h_bar_bot: String = format!("\u{2570}{}\u{256f}", "\u{2500}".repeat(inner));
    let empty_row: String = format!("\u{2502}{}\u{2502}", " ".repeat(inner));

    // 总框宽度用于左侧缩进使其居中
    let total_w = width as usize;
    let left_pad = if total_w > box_w {
        (total_w - box_w) / 2
    } else {
        0
    };
    let pad: String = " ".repeat(left_pad);

    let border_style = Style::default().fg(theme.welcome_border);
    // 渐变色对：每对颜色之间有内在关联，平滑过渡
    const GRADIENT_PAIRS: &[((u8, u8, u8), (u8, u8, u8))] = &[
        ((212, 175, 55), (255, 230, 140)),  // 古金 → 淡金
        ((240, 120, 130), (255, 190, 170)), // 胭脂 → 桃粉
        ((100, 180, 220), (160, 230, 210)), // 霁蓝 → 碧玉
        ((180, 90, 210), (220, 150, 230)),  // 紫藤 → 薰衣草
        ((80, 180, 140), (160, 230, 180)),  // 青瓷 → 嫩绿
        ((220, 130, 70), (255, 200, 120)),  // 赭橙 → 杏黄
        ((100, 160, 230), (180, 200, 255)), // 远山蓝 → 月色
        ((200, 100, 120), (240, 160, 140)), // 暮红 → 霞光
        ((90, 200, 180), (150, 230, 220)),  // 湖水 → 冰蓝
        ((160, 130, 210), (210, 180, 240)), // 暮霭 → 淡紫
        ((230, 180, 80), (200, 150, 60)),   // 琥珀 → 深金
        ((120, 200, 120), (180, 230, 160)), // 春芽 → 嫩黄绿
    ];

    let quote = super::quotes::get_quote(quote_idx);
    // 按显示宽度截断诗句，不超过 inner
    let mut quote_chars: Vec<char> = Vec::new();
    let mut w = 0;
    for ch in quote.chars() {
        let cw = UnicodeWidthStr::width(ch.to_string().as_str());
        if w + cw > inner {
            break;
        }
        quote_chars.push(ch);
        w += cw;
    }
    let text_display_width = w;
    let pl = (inner - text_display_width) / 2;
    let pr = inner - text_display_width - pl;

    // 双色渐变：在选定的颜色对之间按字符位置线性插值
    let (start_c, end_c) = GRADIENT_PAIRS[quote_idx % GRADIENT_PAIRS.len()];
    let n = quote_chars.len().max(1);
    let colored_spans: Vec<Span<'a>> = quote_chars
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let t = i as f32 / (n - 1).max(1) as f32;
            let r = (start_c.0 as f32 + (end_c.0 as f32 - start_c.0 as f32) * t) as u8;
            let g = (start_c.1 as f32 + (end_c.1 as f32 - start_c.1 as f32) * t) as u8;
            let b = (start_c.2 as f32 + (end_c.2 as f32 - start_c.2 as f32) * t) as u8;
            Span::styled(
                ch.to_string(),
                Style::default().fg(ratatui::style::Color::Rgb(r, g, b)),
            )
        })
        .collect();

    let mut quote_line_spans: Vec<Span<'a>> = vec![Span::styled(
        format!("{pad}\u{2502}{}", " ".repeat(pl)),
        border_style,
    )];
    quote_line_spans.extend(colored_spans);
    quote_line_spans.push(Span::styled(
        format!("{}\u{2502}", " ".repeat(pr)),
        border_style,
    ));

    vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(format!("{pad}{h_bar_top}"), border_style)),
        Line::from(Span::styled(format!("{pad}{empty_row}"), border_style)),
        Line::from(Span::styled(format!("{pad}{empty_row}"), border_style)),
        // 诗句行：逐字彩色居中显示
        Line::from(quote_line_spans),
        Line::from(Span::styled(format!("{pad}{empty_row}"), border_style)),
        Line::from(Span::styled(format!("{pad}{empty_row}"), border_style)),
        Line::from(Span::styled(format!("{pad}{h_bar_bot}"), border_style)),
    ]
}
