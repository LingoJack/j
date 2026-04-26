/// 在终端中渲染 Markdown 文本（使用 Rust 原生 Markdown 渲染器）
#[macro_export]
macro_rules! md {
    ($($arg:tt)*) => {{
        let text = format!($($arg)*);
        $crate::util::md_render::render_md(&text);
    }};
}

/// 在终端中渲染单行 Markdown（不换行，用于内联场景）
#[macro_export]
macro_rules! md_inline {
    ($($arg:tt)*) => {{
        let text = format!($($arg)*);
        termimad::print_inline(&text);
    }};
}

/// 将 ratatui Line/Span 输出为 ANSI 彩色终端文本
fn print_lines_to_terminal(lines: &[ratatui::text::Line]) {
    use crossterm::ExecutableCommand;
    use crossterm::style::{Attribute, ContentStyle, Print, ResetColor, SetStyle};
    use ratatui::style::Modifier;
    use std::io::{Write, stdout};

    let mut out = stdout();

    for line in lines {
        for span in &line.spans {
            let style = span.style;
            let mut ct_style = ContentStyle::new();

            // 映射前景色
            if let Some(fg) = style.fg {
                ct_style.foreground_color = Some(map_color(fg));
            }

            // 映射背景色
            if let Some(bg) = style.bg {
                ct_style.background_color = Some(map_color(bg));
            }

            // 映射 modifier
            let mods = style.add_modifier;
            if mods.contains(Modifier::BOLD) {
                ct_style.attributes.set(Attribute::Bold);
            }
            if mods.contains(Modifier::ITALIC) {
                ct_style.attributes.set(Attribute::Italic);
            }
            if mods.contains(Modifier::UNDERLINED) {
                ct_style.attributes.set(Attribute::Underlined);
            }
            if mods.contains(Modifier::CROSSED_OUT) {
                ct_style.attributes.set(Attribute::CrossedOut);
            }
            if mods.contains(Modifier::DIM) {
                ct_style.attributes.set(Attribute::Dim);
            }

            let _ = out.execute(SetStyle(ct_style));
            let _ = out.execute(Print(&span.content));
            let _ = out.execute(ResetColor);
        }
        let _ = writeln!(out);
    }
    let _ = out.flush();
}

/// 映射 ratatui Color → crossterm Color。
///
/// 入口先经 [`crate::util::color_adapt::degrade`]，让全局色阶设置（`color_mode` 配置项）
/// 生效——这样在 ANSI16 / 256 / NoColor 模式下，主题里的 hex 真彩色会被降级到对应等级。
fn map_color(color: ratatui::style::Color) -> crossterm::style::Color {
    use crossterm::style::Color as CtColor;
    use ratatui::style::Color as RColor;

    let color = crate::util::color_adapt::degrade(color);

    match color {
        RColor::Rgb(r, g, b) => CtColor::Rgb { r, g, b },
        RColor::Indexed(i) => CtColor::AnsiValue(i),
        RColor::Black => CtColor::Black,
        RColor::Red => CtColor::DarkRed,
        RColor::Green => CtColor::DarkGreen,
        RColor::Yellow => CtColor::DarkYellow,
        RColor::Blue => CtColor::DarkBlue,
        RColor::Magenta => CtColor::DarkMagenta,
        RColor::Cyan => CtColor::DarkCyan,
        RColor::Gray => CtColor::Grey,
        RColor::DarkGray => CtColor::DarkGrey,
        RColor::LightRed => CtColor::Red,
        RColor::LightGreen => CtColor::Green,
        RColor::LightYellow => CtColor::Yellow,
        RColor::LightBlue => CtColor::Blue,
        RColor::LightMagenta => CtColor::Magenta,
        RColor::LightCyan => CtColor::Cyan,
        RColor::White => CtColor::White,
        _ => CtColor::Reset,
    }
}

/// 渲染 Markdown 文本到终端
pub fn render_md(text: &str) {
    use crate::command::chat::markdown::markdown_to_lines;
    use crate::theme::Theme;

    let width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80);
    // 使用终端原生 ANSI 主题，不依赖 AI 模式的 agent 配置
    let theme = Theme::terminal();
    let lines = markdown_to_lines(text, width, &theme);
    print_lines_to_terminal(&lines);
}
