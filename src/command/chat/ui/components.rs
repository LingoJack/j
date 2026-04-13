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

/// 自适应居中欢迎框（主题感知渐变色）
///
/// 渐变色从 Theme.welcome_gradient_start/mid/end 读取，
/// 并基于 quote_idx 做正弦偏移产生变体，保证每次启动略有不同
/// 但不偏离主题基调。
pub fn welcome_box<'a>(width: u16, theme: &Theme, quote_idx: usize) -> Vec<Line<'a>> {
    use unicode_width::UnicodeWidthStr;

    // 框体内部宽度：取终端内宽的一半，最少 30，最多 50
    let inner = ((width as usize) / 2).clamp(30, 50);
    let box_w = inner + 2;

    let total_w = width as usize;
    let left_pad = if total_w > box_w {
        (total_w - box_w) / 2
    } else {
        0
    };
    let pad: String = " ".repeat(left_pad);

    let border_style = Style::default().fg(theme.welcome_border);

    // ── 渐变色调色板 ──────────────────────────────────────
    // 三色分段插值：前半段 start→mid，后半段 mid→end
    // 与二色线性相比，色彩有"起伏"弧度，视觉上不单调
    type RgbTriple = (u8, u8, u8);

    // 每组 16 个三元组 (start, mid, end)，共 8 组调色板，按 Theme.welcome_palette 索引
    // 色彩设计原则：
    //   - 每组内 16 个渐变色覆盖色轮多个区域，保证每次启动视觉不同
    //   - 色调、饱和度、明度都匹配对应主题的背景和整体氛围
    //   - 相邻三元组（如红→蓝→红）的中间色制造"弧度"，避免单调线性渐变

    #[rustfmt::skip]
    const PALETTE_MIDNIGHT: &[(RgbTriple, RgbTriple, RgbTriple)] = &[
        // Palette 0 — Midnight：古金胭脂，古典暖调，饱和度中等偏高
        ((212,175, 55),(220, 80, 90),(255,230,140)), // 古金 → 胭脂 → 淡金
        ((240,120,130),(100,160,220),(255,190,170)), // 胭脂 → 霁蓝 → 桃粉
        ((100,180,220),(150,210,120),(160,230,210)), // 霁蓝 → 嫩芽 → 碧玉
        ((180, 90,210),(220,160, 60),(220,150,230)), // 紫藤 → 琥珀 → 薰衣草
        (( 80,180,140),(210, 90,110),(160,230,180)), // 青瓷 → 暮红 → 嫩绿
        ((220,130, 70),( 80,180,210),(255,200,120)), // 赭橙 → 湖蓝 → 杏黄
        ((100,160,230),(200,120,200),(180,200,255)), // 远山蓝 → 紫霞 → 月色
        ((200,100,120),( 80,200,170),(240,160,140)), // 暮红 → 碧水 → 霞光
        (( 90,200,180),(210,160, 60),(150,230,220)), // 湖水 → 金橙 → 冰蓝
        ((160,130,210),(120,200,140),(210,180,240)), // 暮霭 → 春草 → 淡紫
        ((230,180, 80),( 80,150,220),(180,210,240)), // 琥珀 → 远蓝 → 月白
        ((120,200,120),(210, 90,130),(180,230,160)), // 春芽 → 胭脂 → 嫩黄绿
        ((255,200,100),(100,190,230),(240,160,200)), // 杏黄 → 天青 → 藕荷
        ((140,100,220),(200,180, 60),(180,130,240)), // 鸢尾 → 秋黄 → 幽紫
        ((200,220,120),( 80,130,210),(140,230,200)), // 黄绿 → 深蓝 → 薄荷
        ((255,150,100),(120, 80,200),(255,200,150)), // 橙红 → 靛蓝 → 浅橙
    ];

    #[rustfmt::skip]
    const PALETTE_DARK: &[(RgbTriple, RgbTriple, RgbTriple)] = &[
        // Palette 1 — Dark：琥珀翡翠，低饱和度柔和暖调
        ((200,180,100),(180,120,100),(190,210,130)), // 淡金 → 赭土 → 嫩绿
        ((180,140,100),(120,170,160),(200,170,130)), // 驼色 → 青灰 → 杏色
        ((140,180,170),(190,160,100),(170,200,180)), // 灰青 → 琥珀 → 薄荷灰
        ((170,140,180),(180,190,110),(185,160,195)), // 灰紫 → 橄榄 → 淡藤
        ((150,190,130),(190,130,120),(175,200,150)), // 苔绿 → 砖红 → 青白
        ((190,150,100),(130,170,180),(210,180,120)), // 沙金 → 灰蓝 → 暮光
        ((130,160,190),(180,150,170),(155,175,205)), // 雾蓝 → 灰粉 → 天青
        ((185,140,130),(140,185,160),(200,165,140)), // 暮色 → 灰碧 → 暮霞
        ((140,185,160),(185,165,100),(155,190,170)), // 灰碧 → 芥黄 → 青灰
        ((165,150,190),(150,185,140),(180,165,205)), // 暮藤 → 苔青 → 淡紫灰
        ((195,175,110),(130,155,185),(175,195,190)), // 琥珀 → 雾蓝 → 灰青
        ((150,185,130),(185,130,130),(170,195,150)), // 嫩叶 → 灰玫 → 青灰
        ((205,185,120),(140,170,195),(195,170,155)), // 暮光 → 青灰 → 暖灰
        ((160,140,195),(185,180,110),(170,150,200)), // 藤紫 → 橄榄 → 幽藤
        ((180,195,130),(130,150,185),(165,195,165)), // 黄灰 → 雾蓝 → 灰绿
        ((200,160,120),(140,140,190),(200,175,135)), // 沙橙 → 灰蓝 → 淡杏
    ];

    #[rustfmt::skip]
    const PALETTE_LIGHT: &[(RgbTriple, RgbTriple, RgbTriple)] = &[
        // Palette 2 — Light：清新明快，在白/米白背景上清晰可辨
        (( 30, 90,180),( 20,150,120),( 50,140,200)), // 海蓝 → 翠绿 → 天蓝
        ((180, 80, 60),( 40,120,180),(200,110, 80)), // 赤陶 → 钴蓝 → 砖橙
        (( 50,150, 90),(170,130, 50),( 80,160,120)), // 翠绿 → 琥珀 → 青翠
        ((130, 60,150),(180,140, 30),(150, 80,170)), // 紫堇 → 金棕 → 紫藤
        (( 40,160,130),(170, 60, 70),( 80,170,140)), // 青 → 朱红 → 翡翠
        ((190,110, 50),( 50,130,170),(210,140, 60)), // 橘黄 → 湖蓝 → 杏黄
        (( 60,120,190),(160, 90,140),(100,150,210)), // 蓝 → 紫红 → 碧蓝
        ((170, 70, 70),( 50,160,130),(190,100, 80)), // 赤红 → 青碧 → 朱砂
        (( 50,160,140),(180,130, 40),( 80,170,150)), // 青 → 琥珀 → 翠青
        ((140, 80,170),( 60,150, 80),(160,100,190)), // 紫菀 → 翠 → 薰衣草
        ((190,140, 40),( 60,110,180),(120,160,190)), // 金棕 → 蓝 → 灰蓝
        (( 60,150, 70),(170, 60, 80),( 90,160, 90)), // 绿 → 绯红 → 嫩绿
        ((200,130, 60),( 50,140,190),(180,100,120)), // 杏 → 天蓝 → 玫红
        ((120, 60,190),(180,150, 30),(140, 80,210)), // 鸢尾 → 金 → 暗紫
        ((160,170, 50),( 50, 90,170),(100,170,140)), // 黄绿 → 深蓝 → 翠
        ((200, 90, 50),( 80, 50,170),(210,120, 70)), // 橙 → 靛 → 赤橙
    ];

    #[rustfmt::skip]
    const PALETTE_NORD: &[(RgbTriple, RgbTriple, RgbTriple)] = &[
        // Palette 3 — Nord：极地冰蓝，低饱和冷调，Nord 色系
        ((136,192,208),(163,190,140),(143,188,187)), // nord8 → nord14 → nord7
        ((180,142,173),( 94,168,174),(191,160,180)), // nord15 → nord8浅 → nord15浅
        (( 94,168,174),(136,192,208),(143,188,187)), // nord8 → nord8 → nord7
        ((163,190,140),(180,142,173),(170,200,160)), // nord14 → nord15 → 浅绿
        ((143,188,187),(210,160,130),(160,195,190)), // nord7 → 暖棕 → 浅青
        ((180,170,150),( 94,168,174),(200,185,160)), // 暖灰 → nord8 → 米色
        ((120,175,200),(170,150,180),(145,190,200)), // 灰蓝 → 灰紫 → 冰蓝
        ((170,155,175),(130,185,175),(185,165,160)), // 灰紫 → 青灰 → 暖灰
        ((130,185,175),(175,165,140),(145,190,180)), // 青灰 → 暖米 → 浅青
        ((160,145,190),(143,188,187),(175,155,200)), // 灰藤 → nord7 → 灰紫
        ((175,165,135),(120,170,195),(160,185,180)), // 米色 → 灰蓝 → 灰青
        ((143,190,155),(175,145,165),(155,195,170)), // 嫩绿 → 灰粉 → 灰绿
        ((185,175,145),(130,175,190),(175,155,170)), // 暖灰 → 灰蓝 → 灰紫
        ((150,140,190),(170,175,140),(160,150,200)), // 藤 → 苔 → 藤紫
        ((165,185,150),(120,160,195),(150,190,170)), // 黄灰 → 灰蓝 → 灰绿
        ((185,160,145),(130,145,190),(190,170,155)), // 暖灰 → 灰蓝 → 米灰
    ];

    #[rustfmt::skip]
    const PALETTE_MONOKAI: &[(RgbTriple, RgbTriple, RgbTriple)] = &[
        // Palette 4 — Monokai：霓虹高对比，经典 Monokai 配色
        ((230,219,116),(249, 38,114),(166,226, 46)), // 黄 → 粉红 → 绿
        ((249, 38,114),(102,217,239),(255, 95,100)), // 粉红 → 青 → 珊瑚
        ((102,217,239),(166,226, 46),(174,220,230)), // 青 → 绿 → 浅青
        ((174,129,255),(230,219,116),(200,150,255)), // 紫 → 黄 → 淡紫
        ((166,226, 46),(249, 38,114),(200,240, 80)), // 绿 → 粉红 → 亮绿
        ((255,151, 50),(102,217,239),(255,200, 80)), // 橙 → 青 → 金黄
        ((102,217,239),(200,100,220),(150,220,250)), // 青 → 紫红 → 天蓝
        ((249, 70,100),(166,226, 46),(255,120, 90)), // 红 → 绿 → 橘红
        ((166,226, 46),(255,151, 50),(200,240,120)), // 绿 → 橙 → 浅绿
        ((200,130,255),(166,226, 46),(220,160,255)), // 紫 → 绿 → 淡紫
        ((255,200, 80),(102,217,239),(230,230,150)), // 金黄 → 青 → 浅黄
        ((166,226, 46),(249, 38,114),(200,240,100)), // 绿 → 粉红 → 黄绿
        ((255,180, 80),(102,217,239),(255,120,180)), // 杏 → 青 → 粉
        ((174,129,255),(230,219,116),(190,160,255)), // 紫 → 黄 → 蓝紫
        ((230,230,120),(102,180,230),(200,240,200)), // 黄绿 → 蓝 → 浅绿
        ((255,120, 80),(174,129,255),(255,160,100)), // 橙红 → 紫 → 橙
    ];

    #[rustfmt::skip]
    const PALETTE_TERMINAL: &[(RgbTriple, RgbTriple, RgbTriple)] = &[
        // Palette 5 — Terminal：灰度低调，适合经典终端
        ((160,160,160),(120,120,120),(175,175,175)), // 灰 → 深灰 → 浅灰
        ((170,170,170),(110,110,110),(180,180,180)), // 浅灰 → 暗灰 → 亮灰
        ((130,130,130),(165,165,165),(145,145,145)), // 中灰 → 浅灰 → 灰
        ((155,155,155),(140,140,140),(170,170,170)), // 灰 → 中灰 → 浅灰
        ((140,140,140),(160,160,160),(150,150,150)), // 灰 → 浅灰 → 中灰
        ((165,165,165),(125,125,125),(175,175,175)), // 浅灰 → 深灰 → 亮灰
        ((120,120,120),(170,170,170),(135,135,135)), // 深灰 → 浅灰 → 中灰
        ((160,150,150),(135,155,155),(170,160,160)), // 暖灰 → 冷灰 → 暖灰
        ((135,155,155),(160,150,150),(145,160,160)), // 冷灰 → 暖灰 → 冷灰
        ((150,140,165),(145,160,150),(160,145,170)), // 灰紫 → 灰绿 → 灰蓝
        ((165,160,140),(125,145,160),(155,165,155)), // 暖灰 → 冷灰 → 灰绿
        ((145,160,145),(160,140,150),(155,165,150)), // 灰绿 → 暖灰 → 灰绿
        ((170,160,145),(135,150,165),(160,155,150)), // 暖灰 → 冷灰 → 中灰
        ((150,140,165),(160,160,140),(155,145,170)), // 灰紫 → 灰 → 灰蓝
        ((160,165,145),(130,140,155),(150,165,155)), // 灰绿 → 冷灰 → 灰绿
        ((165,150,140),(140,140,160),(170,155,145)), // 暖灰 → 冷灰 → 暖灰
    ];

    #[rustfmt::skip]
    const PALETTE_ANTHROPIC_LIGHT: &[(RgbTriple, RgbTriple, RgbTriple)] = &[
        // Palette 6 — Anthropic Light：暖赭陶土，大地色系，白底清晰
        ((180,100, 70),( 70,130, 80),(200,130, 80)), // 赭陶 → 松绿 → 暖橙
        ((160, 80, 60),( 80,100,160),(180,110, 70)), // 赤陶 → 灰蓝 → 砖橙
        (( 70,130, 90),(170,120, 50),( 90,140,100)), // 森绿 → 赭金 → 翠绿
        ((130, 80,120),(170,130, 40),(140, 90,130)), // 灰紫 → 琥珀 → 藤紫
        (( 80,140,100),(160, 70, 60),(100,150,110)), // 翠 → 朱 → 翡翠
        ((180,110, 50),( 60,120,150),(200,130, 60)), // 橘黄 → 灰蓝 → 杏黄
        (( 70,100,150),(140, 80,110),( 90,110,165)), // 灰蓝 → 灰玫 → 蓝灰
        ((160, 70, 60),( 70,140,120),(180, 90, 70)), // 朱 → 青碧 → 赤砂
        (( 70,140,120),(170,120, 40),( 90,150,130)), // 青 → 赭金 → 翠青
        ((130, 80,140),( 70,140, 80),(145, 90,155)), // 紫菀 → 翠 → 暮紫
        ((180,130, 40),( 70, 90,150),(120,140,150)), // 金棕 → 蓝 → 灰蓝
        (( 70,140, 70),(160, 70, 80),( 90,150, 90)), // 绿 → 绯 → 嫩绿
        ((190,120, 50),( 60,120,160),(170, 90,100)), // 杏 → 灰蓝 → 玫红
        ((110, 60,160),(170,140, 30),(120, 70,175)), // 鸢尾 → 金 → 暗紫
        ((150,140, 50),( 50, 80,150),(100,150,120)), // 黄绿 → 深蓝 → 灰绿
        ((190, 80, 40),( 80, 50,140),(200,100, 55)), // 橙 → 靛 → 赤橙
    ];

    #[rustfmt::skip]
    const PALETTE_ANTHROPIC_DARK: &[(RgbTriple, RgbTriple, RgbTriple)] = &[
        // Palette 7 — Anthropic Dark：月蓝幽彩，冷调霓虹
        ((130,170,255),(192,153,255),(140,230,180)), // 月蓝 → 紫 → 翠绿
        ((255,140,120),(100,180,255),(255,170,140)), // 珊瑚 → 天蓝 → 桃
        ((100,200,200),(180,140,255),(130,220,210)), // 青 → 紫 → 浅青
        ((180,140,255),(255,200,100),(200,160,255)), // 紫 → 金 → 淡紫
        ((100,220,160),(255,120,130),(130,230,180)), // 翠 → 粉红 → 嫩翠
        ((255,170, 80),(100,180,230),(255,200,110)), // 橙 → 蓝 → 金
        ((100,180,255),(200,130,220),(140,200,255)), // 蓝 → 紫粉 → 碧蓝
        ((255,120,130),(100,220,200),(255,150,140)), // 粉红 → 青 → 珊瑚
        ((100,220,200),(255,180, 80),(130,230,210)), // 青 → 橙 → 浅青
        ((200,160,255),(100,220,160),(220,180,255)), // 紫 → 翠 → 淡紫
        ((255,200,100),(100,170,255),(200,220,150)), // 金 → 蓝 → 黄绿
        ((100,220,160),(255,120,140),(130,230,180)), // 翠 → 玫红 → 嫩翠
        ((255,180,100),(100,200,220),(255,140,180)), // 杏 → 青 → 粉
        ((170,130,255),(255,200,100),(190,150,255)), // 蓝 → 金 → 紫蓝
        ((200,220,120),(100,140,240),(150,230,200)), // 黄绿 → 蓝 → 薄荷
        ((255,140, 80),(140,100,255),(255,170,100)), // 橙红 → 靛 → 浅橙
    ];

    /// 根据 palette 索引和 quote 索引获取渐变三元组
    fn get_gradient(palette: u8, idx: usize) -> (RgbTriple, RgbTriple, RgbTriple) {
        let i = idx % 16;
        match palette % 8 {
            0 => PALETTE_MIDNIGHT[i],
            1 => PALETTE_DARK[i],
            2 => PALETTE_LIGHT[i],
            3 => PALETTE_NORD[i],
            4 => PALETTE_MONOKAI[i],
            5 => PALETTE_TERMINAL[i],
            6 => PALETTE_ANTHROPIC_LIGHT[i],
            _ => PALETTE_ANTHROPIC_DARK[i],
        }
    }

    let (start_c, mid_c, end_c) = get_gradient(theme.welcome_palette, quote_idx);

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

    // 在标点后断行（且已积累至少半行宽度），超宽时强制断行
    let break_after = ['，', '。', '！', '？', '；', '：', ',', '.', '!', '?'];
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
        // 标点处自然断行（需已累积至半行以上，避免极短行）
        if break_after.contains(&ch) && cur_w * 2 >= text_area {
            lines_chars.push(std::mem::take(&mut cur));
            cur_w = 0;
        }
    }
    if !cur.is_empty() {
        lines_chars.push(cur);
    }

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
