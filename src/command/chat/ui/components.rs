//! Chat UI 组件（薄封装）
//!
//! 通用组件复用 `tui::components`，仅保留 chat 专属组件。

use crate::theme::Theme;
use crate::tui::components::{
    LABEL_WIDTH, TOGGLE_OFF, TOGGLE_ON, cursor_spans, desc_span_with_selected, label_span,
    pointer_span, value_style,
};
use crate::util::text::display_width;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// UI 配置行组件的公共渲染参数
#[derive(Debug)]
pub struct RowContext<'a> {
    pub selected: bool,
    pub theme: &'a Theme,
}

/// Global tab 行的共享渲染参数（label + desc + hint + selected + theme + desc_max_width）
#[derive(Debug)]
pub struct GlobalRowCtx<'a> {
    pub label: String,
    pub desc: String,
    pub hint: String,
    pub selected: bool,
    pub theme: &'a Theme,
    /// 说明文字最大显示宽度（根据屏幕宽度动态计算）
    pub desc_max_width: usize,
}

impl<'a> GlobalRowCtx<'a> {
    /// 从 label/desc/selected/theme 构造，hint 默认 "Enter 切换" 或 "Enter 编辑"
    pub fn new(label: &str, desc: &str, selected: bool, theme: &'a Theme) -> Self {
        Self {
            label: label.to_string(),
            desc: desc.to_string(),
            hint: "Enter \u{5207}\u{6362}".to_string(),
            selected,
            theme,
            desc_max_width: 30,
        }
    }

    /// 根据屏幕宽度设置 desc 最大显示宽度
    ///
    /// 计算逻辑：屏幕宽度减去 pointer(4) + label(16) + gap(2) + value 估算(12) + 右侧呼吸(6)
    pub fn with_screen_width(mut self, screen_width: usize) -> Self {
        const POINTER_W: usize = 4;
        const LABEL_W: usize = 16;
        const GAP_W: usize = 2;
        const VALUE_ESTIMATE: usize = 12;
        const RIGHT_PAD: usize = 6;
        let available =
            screen_width.saturating_sub(POINTER_W + LABEL_W + GAP_W + VALUE_ESTIMATE + RIGHT_PAD);
        // 至少 15，最大 50，避免极端值
        self.desc_max_width = available.clamp(15, 50);
        self
    }
}

// ── 预览值（内部）───────────────────────────────────────────

/// 长文本截断预览（替换换行为空格，超 40 显示宽度截断）
fn render_preview_value(raw: &str) -> String {
    if raw.is_empty() {
        return "(\u{7a7a})".to_string();
    }
    let flat: String = raw
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    const MAX_WIDTH: usize = 40;
    if display_width(&flat) > MAX_WIDTH {
        let truncated: String = flat
            .chars()
            .scan(0usize, |w, ch| {
                *w += crate::util::text::char_width(ch);
                if *w <= MAX_WIDTH { Some(ch) } else { None }
            })
            .collect();
        format!("{truncated}...")
    } else {
        flat
    }
}

// ── API Key 遮罩字段行（chat 专属）─────────────────────────────

/// API Key 字段（未编辑时使用 api_key 颜色）
pub(crate) fn secret_field_row<'a>(
    label: &str,
    value: &str,
    editing: bool,
    cursor: usize,
    ctx: &RowContext<'_>,
) -> Line<'a> {
    let selected = ctx.selected;
    let theme = ctx.theme;
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

// ── Global tab 三列布局行（chat 专属）───────────────────────────

/// Global tab 可编辑文本行（三列: label | value | desc）
pub(crate) fn global_text_row<'a>(
    value: &str,
    editing: bool,
    cursor: usize,
    ctx: &GlobalRowCtx<'_>,
) -> Line<'a> {
    let selected = ctx.selected;
    let theme = ctx.theme;
    let vs = value_style(selected, editing, theme);
    if editing && selected {
        let mut spans = vec![
            pointer_span(selected, theme),
            label_span(&ctx.label, LABEL_WIDTH, selected, theme),
            Span::styled("  ", Style::default()),
        ];
        spans.extend(cursor_spans(value, cursor, vs, theme));
        Line::from(spans)
    } else {
        let display_value = if value.is_empty() {
            "(\u{7a7a})".to_string()
        } else {
            value.to_string()
        };
        Line::from(vec![
            pointer_span(selected, theme),
            label_span(&ctx.label, LABEL_WIDTH, selected, theme),
            Span::styled("  ", Style::default()),
            Span::styled(display_value, vs),
            desc_span_with_selected(&ctx.desc, ctx.desc_max_width, selected, theme),
        ])
    }
}

/// Global tab 开关行（三列: label | toggle | desc）
pub(crate) fn global_toggle_row<'a>(is_on: bool, ctx: &GlobalRowCtx<'_>) -> Line<'a> {
    let selected = ctx.selected;
    let theme = ctx.theme;
    let toggle_style = if selected {
        Style::default()
            .fg(theme.text_normal)
            .add_modifier(Modifier::BOLD)
    } else if is_on {
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
        label_span(&ctx.label, LABEL_WIDTH, selected, theme),
        Span::styled("  ", Style::default()),
        Span::styled(toggle_text, toggle_style),
        desc_span_with_selected(&ctx.desc, ctx.desc_max_width, selected, theme),
        Span::styled(
            if selected {
                format!("  ({})", ctx.hint)
            } else {
                String::new()
            },
            if selected {
                Style::default().fg(theme.config_label_selected)
            } else {
                Style::default().fg(theme.config_dim)
            },
        ),
    ])
}

/// Global tab 预览行（三列: label | preview | desc）
pub fn global_preview_row<'a>(raw: &str, ctx: &GlobalRowCtx<'_>) -> Line<'a> {
    let selected = ctx.selected;
    let theme = ctx.theme;
    let vs = value_style(selected, false, theme);
    Line::from(vec![
        pointer_span(selected, theme),
        label_span(&ctx.label, LABEL_WIDTH, selected, theme),
        Span::styled("  ", Style::default()),
        Span::styled(render_preview_value(raw), vs),
        desc_span_with_selected(&ctx.desc, ctx.desc_max_width, selected, theme),
        Span::styled(
            if selected {
                format!("  ({})", ctx.hint)
            } else {
                String::new()
            },
            if selected {
                Style::default().fg(theme.config_label_selected)
            } else {
                Style::default().fg(theme.config_dim)
            },
        ),
    ])
}

/// Global tab 主题行（三列: label | theme | desc）
pub fn global_theme_row<'a>(name: &str, ctx: &GlobalRowCtx<'_>) -> Line<'a> {
    let selected = ctx.selected;
    let theme = ctx.theme;
    Line::from(vec![
        pointer_span(selected, theme),
        label_span(&ctx.label, LABEL_WIDTH, selected, theme),
        Span::styled("  ", Style::default()),
        Span::styled(
            format!("\u{1f3a8} {name}"),
            if selected {
                Style::default()
                    .fg(theme.text_normal)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme.config_toggle_on)
                    .add_modifier(Modifier::BOLD)
            },
        ),
        desc_span_with_selected(&ctx.desc, ctx.desc_max_width, selected, theme),
        Span::styled(
            if selected {
                format!("  ({})", ctx.hint)
            } else {
                String::new()
            },
            if selected {
                Style::default().fg(theme.config_label_selected)
            } else {
                Style::default().fg(theme.config_dim)
            },
        ),
    ])
}

/// Global tab 选择行（三列: label | value | desc）
///
/// 与 `global_theme_row` 类似，但无画板图标，风格更朴素，适用于非主题类枚举选择（如 shell_runtime）。
pub fn global_select_row<'a>(name: &str, ctx: &GlobalRowCtx<'_>) -> Line<'a> {
    let selected = ctx.selected;
    let theme = ctx.theme;
    let vs = value_style(selected, false, theme);
    Line::from(vec![
        pointer_span(selected, theme),
        label_span(&ctx.label, LABEL_WIDTH, selected, theme),
        Span::styled("  ", Style::default()),
        Span::styled(name.to_string(), vs),
        desc_span_with_selected(&ctx.desc, ctx.desc_max_width, selected, theme),
        Span::styled(
            if selected {
                format!("  ({})", ctx.hint)
            } else {
                String::new()
            },
            if selected {
                Style::default().fg(theme.config_label_selected)
            } else {
                Style::default().fg(theme.config_dim)
            },
        ),
    ])
}

// ── 欢迎框（chat 专属）─────────────────────────────────────────

/// 自适应居中欢迎框（主题感知渐变色）
///
/// 渐变色从 Theme.welcome_gradient_start/mid/end 读取，
/// 并基于 quote_idx 做正弦偏移产生变体，保证每次启动略有不同
/// 但不偏离主题基调。
pub fn welcome_box<'a>(
    width: u16,
    theme: &Theme,
    quote_idx: usize,
    show_quote: bool,
) -> Vec<Line<'a>> {
    use unicode_width::UnicodeWidthStr;

    // ── 关闭诗句时显示 J-CLI ASCII Art（无边框） ──
    if !show_quote {
        // ── 渐变色 ──
        use super::palette;
        let triple = palette::get_gradient(theme.welcome_palette, quote_idx);
        let (start_c, mid_c, end_c) = triple;

        // ── J-CLI ASCII Art（6行，与启动页一致） ──
        let art_lines: &[&str] = &[
            "                        ██╗        ██████╗ ██╗     ██╗",
            "                        ██║       ██╔════╝ ██║     ██║",
            "                        ██║ █████╗██║      ██║     ██║",
            "                    ██  ██║ ╚════╝██║      ██║     ██║",
            "                   ╚█████╔╝      ╚██████╗ ███████╗ ██║",
            "                    ╚════╝        ╚═════╝ ╚══════╝ ╚═╝",
        ];

        // 保留各行间相对缩进：减去最小前导空格数
        let min_leading = art_lines
            .iter()
            .map(|l| l.len() - l.trim_start().len())
            .min()
            .unwrap_or(0);
        let trimmed: Vec<String> = art_lines
            .iter()
            .map(|l| {
                if l.len() > min_leading {
                    &l[min_leading..]
                } else {
                    l
                }
                .to_string()
            })
            .collect();
        let art_max_w = trimmed
            .iter()
            .map(|l| UnicodeWidthStr::width(l.as_str()))
            .max()
            .unwrap_or(0);

        // 基于屏幕宽度居中
        let total_w = width as usize;
        let block_pl = if total_w > art_max_w {
            (total_w - art_max_w) / 2
        } else {
            0
        };
        let pad: String = " ".repeat(block_pl);

        let mut result: Vec<Line<'a>> = vec![Line::from(""), Line::from("")];

        for line in &trimmed {
            let chars: Vec<char> = line.chars().collect();
            let total_n = chars.len().max(2);
            let mut spans: Vec<Span<'a>> = vec![Span::raw(pad.clone())];

            for (i, &ch) in chars.iter().enumerate() {
                let t = i as f32 / (total_n - 1) as f32;
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
                    Style::default().fg(ratatui::style::Color::Rgb(r, g, b)),
                ));
            }
            result.push(Line::from(spans));
        }

        return result;
    }

    // 框体内部宽度：取终端内宽的一半，最少 30，最多 55
    let inner = ((width as usize) / 2).clamp(30, 55);
    let box_w = inner + 2;

    let total_w = width as usize;
    let left_pad = if total_w > box_w {
        (total_w - box_w) / 2
    } else {
        0
    };
    let pad: String = " ".repeat(left_pad);

    let border_style = Style::default().fg(theme.welcome_border);

    // ── 渐变色调色板（已迁移至 palette.rs） ──────────────────
    use super::palette;
    let triple = palette::get_gradient(theme.welcome_palette, quote_idx);
    let (start_c, mid_c, end_c) = triple;

    // ── 顶部边框：嵌入 ◈ 装饰符 ──
    // 形如：╭──── ◈ ────╮
    let ornament = " ◈ ";
    let orn_w = UnicodeWidthStr::width(ornament);
    let bar_sides = inner.saturating_sub(orn_w);
    let left_h = bar_sides / 2;
    let right_h = bar_sides - left_h;
    let h_bar_top = format!(
        "\u{256d}{}{}{}\u{256e}",
        "\u{2500}".repeat(left_h),
        ornament,
        "\u{2500}".repeat(right_h),
    );
    let h_bar_bot = format!("\u{2570}{}\u{256f}", "\u{2500}".repeat(inner));
    let empty_row = format!("\u{2502}{}\u{2502}", " ".repeat(inner));

    // ── 诗句自然换行 ──
    // 文字有效宽度：框内减去两侧各 1 格呼吸空间
    let text_area = inner.saturating_sub(2);
    let quote = super::quotes::get_quote(quote_idx);

    // ── 诗句换行逻辑 ──
    // 先判断整句是否能放下：能放下则整行显示，否则按标点断行
    let quote_width = quote
        .chars()
        .map(|c| UnicodeWidthStr::width(c.to_string().as_str()))
        .sum::<usize>();

    let cn_break = ['，', '。', '！', '？', '；', '：'];
    let en_break = [',', '.', '!', '?'];

    // 能放下就不断行，直接返回单行
    let lines_chars: Vec<Vec<char>> = if quote_width <= text_area {
        vec![quote.chars().collect()]
    } else {
        // 放不下才按标点断行
        let mut lines_chars: Vec<Vec<char>> = Vec::new();
        let mut cur: Vec<char> = Vec::new();
        let mut cur_w = 0usize;

        for ch in quote.chars() {
            let cw = UnicodeWidthStr::width(ch.to_string().as_str());
            // 超宽：先把当前行入列，再开新行
            if cur_w + cw > text_area && !cur.is_empty() {
                lines_chars.push(std::mem::take(&mut cur));
                cur_w = 0;
            }
            cur.push(ch);
            cur_w += cw;
            // 中文标点：无条件断行
            if cn_break.contains(&ch) {
                lines_chars.push(std::mem::take(&mut cur));
                cur_w = 0;
            } else if en_break.contains(&ch) && cur_w * 2 >= text_area {
                // 英文标点：需已累积至半行以上
                lines_chars.push(std::mem::take(&mut cur));
                cur_w = 0;
            }
        }
        if !cur.is_empty() {
            lines_chars.push(cur);
        }
        lines_chars
    };

    // ── 全局渐变：整句诗从首字到末字连续插值 ──
    let total_chars: usize = lines_chars.iter().map(|l| l.len()).sum();
    // 至少 2 以避免除零；单字时视为首尾同色
    let total_n = total_chars.max(2);

    let mut quote_lines: Vec<Line<'a>> = Vec::new();
    let mut global_idx = 0usize;

    for line_chars in &lines_chars {
        let line_w: usize = line_chars
            .iter()
            .map(|c| UnicodeWidthStr::width(c.to_string().as_str()))
            .sum();
        // 居中：两侧留白至少 1 格
        let pl = if inner > line_w + 2 {
            (inner - line_w) / 2
        } else {
            1
        };
        let pr = inner.saturating_sub(line_w + pl);

        let mut spans: Vec<Span<'a>> = vec![Span::styled(
            format!("{}\u{2502}{}", pad, " ".repeat(pl)),
            border_style,
        )];

        for (i, &ch) in line_chars.iter().enumerate() {
            let gi = global_idx + i;
            let t = gi as f32 / (total_n - 1) as f32;
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
                Style::default().fg(ratatui::style::Color::Rgb(r, g, b)),
            ));
        }

        spans.push(Span::styled(
            format!("{}\u{2502}", " ".repeat(pr)),
            border_style,
        ));

        quote_lines.push(Line::from(spans));
        global_idx += line_chars.len();

        // 多行诗：行间插入空行，让视觉更通透
        if lines_chars.len() > 1 {
            quote_lines.push(Line::from(Span::styled(
                format!("{pad}{empty_row}"),
                border_style,
            )));
        }
    }

    // 多行诗插入了行间空行，移除最后一个多余的
    if lines_chars.len() > 1 && quote_lines.len() > 1 {
        quote_lines.pop();
    }

    // ── 内边距：单行留两行，多行各留一行 ──
    let pad_rows = if lines_chars.len() == 1 { 2 } else { 1 };
    let make_empty =
        || -> Line<'a> { Line::from(Span::styled(format!("{pad}{empty_row}"), border_style)) };

    let mut result: Vec<Line<'a>> = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(format!("{pad}{h_bar_top}"), border_style)),
    ];
    for _ in 0..pad_rows {
        result.push(make_empty());
    }
    result.extend(quote_lines);
    for _ in 0..pad_rows {
        result.push(make_empty());
    }
    result.push(Line::from(Span::styled(
        format!("{pad}{h_bar_bot}"),
        border_style,
    )));

    result
}
