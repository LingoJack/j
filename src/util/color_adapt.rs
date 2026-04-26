//! 颜色色阶适配层
//!
//! 解决"hex 真彩色在终端中和背景冲突看不清"的问题。让命令行非 ratatui 输出
//! 路径按用户偏好或终端能力自动降级到 ANSI 256 / 16 / 无色。
//!
//! # 启动流程
//! 1. 在任何 [`crate::theme::Theme`] 加载之前，由 `init_from_config()` 设置全局
//!    [`ColorLevel`]，依据：
//!    - 配置项 `setting.color_mode`（`auto` / `truecolor` / `ansi256` / `ansi16` / `none`）
//!    - `auto` 时通过 `supports-color` 探测（同时尊重 `NO_COLOR` / `CLICOLOR_FORCE`）
//! 2. 后续模块按各自路径消费 ColorLevel —— 见下文「生效路径」。
//!
//! # 生效路径
//!
//! 全局 ColorLevel 通过**两条独立链路**影响最终输出：
//!
//! ## 链路 A：运行时降级（直接调用本模块）
//! - [`apply_fg`] —— `colored` crate 路径，仅 oneshot.rs 使用
//! - [`degrade`] —— ratatui Color → ratatui Color，被 `md_render::map_color` 调用，
//!   覆盖 markdown 终端渲染（`j ls`、oneshot 的 AI 回答重绘等）
//!
//! 这一条只覆盖**非 ratatui** 的输出。chat TUI、todo TUI 这些 ratatui 渲染器
//! **不直接调用本模块的任何函数**。
//!
//! ## 链路 B：JSON 反序列化期解析（间接影响 TUI）
//! 见 [`crate::theme::ColorValue`]：当主题 JSON 字段写成对象形态
//! `{ rgb, ansi256, ansi16 }` 时，`ColorValue::TryFrom` 在反序列化时调用
//! [`current()`] 选择最优级别。结果是 [`crate::theme::Theme`] 里**保存的就是已经
//! 选过的颜色值**——TUI 之后只是把这些值塞进 ratatui Style，不用调任何降级函数，
//! 因为加载阶段就已经决定好了。
//!
//! 现状：内置 7 个主题 JSON 全是字符串形态，不走对象分支，所以 TUI 实际不受
//! `color_mode` 影响。**链路 B 是预留给主题作者**的一个钩子，让他们能对关键色
//! （比如 `welcome_quote` 这类用最近邻容易映射歪的）显式锁定 ansi16 / ansi256
//! 输出，且这种锁定**对 TUI 同样生效**。
//!
//! # 映射策略（链路 A）
//! - TrueColor → 不变
//! - Ansi256 → 通过 `ansi_colours::ansi256_from_rgb` 做感知色差映射，发射 indexed
//! - Ansi16 → 在 16 个标准 xterm 色 RGB 表上做加权欧氏最近邻（Rec.601 亮度权重）
//! - NoColor → 全部转 `Color::Reset`

use ratatui::style::Color as RColor;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorLevel {
    TrueColor,
    Ansi256,
    Ansi16,
    NoColor,
}

static GLOBAL: OnceLock<ColorLevel> = OnceLock::new();

/// 由 main.rs 在加载完 YamlConfig 后调用一次。
/// `mode` 为配置项原文（"auto" / "truecolor" / "ansi256" / "ansi16" / "none" / 空）。
pub fn init_from_config(mode: &str) {
    let level = parse_mode(mode);
    let _ = GLOBAL.set(level);
}

/// 当前生效的色阶。未初始化时按 TrueColor 兜底（保持旧行为）。
pub fn current() -> ColorLevel {
    GLOBAL.get().copied().unwrap_or(ColorLevel::TrueColor)
}

fn parse_mode(s: &str) -> ColorLevel {
    match s.trim().to_lowercase().as_str() {
        "truecolor" | "rgb" | "24bit" | "16m" => ColorLevel::TrueColor,
        "ansi256" | "256" => ColorLevel::Ansi256,
        "ansi16" | "16" | "basic" => ColorLevel::Ansi16,
        "none" | "off" | "no" | "false" => ColorLevel::NoColor,
        // "auto" / "" / 未知值 → 走自动探测
        _ => detect(),
    }
}

fn detect() -> ColorLevel {
    use supports_color::Stream;
    // 探测 stderr —— j-cli 大量诊断信息走 eprintln!
    match supports_color::on_cached(Stream::Stderr) {
        Some(level) if level.has_16m => ColorLevel::TrueColor,
        Some(level) if level.has_256 => ColorLevel::Ansi256,
        Some(level) if level.has_basic => ColorLevel::Ansi16,
        Some(_) => ColorLevel::NoColor,
        None => ColorLevel::NoColor,
    }
}

/// 将 ratatui 颜色按当前色阶降级为另一个 ratatui 颜色。
/// 已经是命名色 / Indexed / Reset 的不会被改写（除非全局是 NoColor）。
pub fn degrade(color: RColor) -> RColor {
    let level = current();
    if let RColor::Reset = color {
        return RColor::Reset;
    }
    if matches!(level, ColorLevel::NoColor) {
        return RColor::Reset;
    }
    match (level, color) {
        (ColorLevel::TrueColor, c) => c,
        (ColorLevel::Ansi256, RColor::Rgb(r, g, b)) => {
            RColor::Indexed(ansi_colours::ansi256_from_rgb((r, g, b)))
        }
        (ColorLevel::Ansi16, RColor::Rgb(r, g, b)) => nearest_ansi16(r, g, b),
        // 已经是命名色或 Indexed —— 任何级别下都直接透传
        (_, c) => c,
    }
}

/// 16-color 标准 RGB 表（xterm 默认调色板）
const ANSI16_PALETTE: &[(RColor, (u8, u8, u8))] = &[
    (RColor::Black, (0, 0, 0)),
    (RColor::Red, (170, 0, 0)),
    (RColor::Green, (0, 170, 0)),
    (RColor::Yellow, (170, 85, 0)),
    (RColor::Blue, (0, 0, 170)),
    (RColor::Magenta, (170, 0, 170)),
    (RColor::Cyan, (0, 170, 170)),
    (RColor::Gray, (170, 170, 170)),
    (RColor::DarkGray, (85, 85, 85)),
    (RColor::LightRed, (255, 85, 85)),
    (RColor::LightGreen, (85, 255, 85)),
    (RColor::LightYellow, (255, 255, 85)),
    (RColor::LightBlue, (85, 85, 255)),
    (RColor::LightMagenta, (255, 85, 255)),
    (RColor::LightCyan, (85, 255, 255)),
    (RColor::White, (255, 255, 255)),
];

/// 加权欧氏距离（Rec.601 亮度权重）做最近邻
fn nearest_ansi16(r: u8, g: u8, b: u8) -> RColor {
    ANSI16_PALETTE
        .iter()
        .min_by_key(|(_, (cr, cg, cb))| {
            let dr = (r as i32 - *cr as i32).pow(2) as u32;
            let dg = (g as i32 - *cg as i32).pow(2) as u32;
            let db = (b as i32 - *cb as i32).pow(2) as u32;
            // Rec.601: 0.299 R + 0.587 G + 0.114 B（×1000 转整数）
            299 * dr + 587 * dg + 114 * db
        })
        .map(|(c, _)| *c)
        .unwrap_or(RColor::Reset)
}

/// 给 `colored` crate 路径（oneshot.rs）使用：把任意 ratatui 颜色作为前景，
/// 按当前色阶产出已降级的 ColoredString。
pub fn apply_fg(text: &str, color: RColor) -> colored::ColoredString {
    use colored::Colorize;

    let degraded = degrade(color);
    match degraded {
        RColor::Reset => text.normal(),
        RColor::Rgb(r, g, b) => text.truecolor(r, g, b),
        // ANSI 256：colored 不直接支持 indexed，回查 RGB 后用 truecolor 输出。
        // 终端会按自身能力进一步处理，且这种情况只在 Ansi256 模式下出现，等价于小幅 fidelity 损失。
        RColor::Indexed(i) => {
            let (r, g, b) = ansi_colours::rgb_from_ansi256(i);
            text.truecolor(r, g, b)
        }
        named => match ratatui_to_colored(named) {
            Some(c) => text.color(c),
            None => text.normal(),
        },
    }
}

fn ratatui_to_colored(c: RColor) -> Option<colored::Color> {
    use colored::Color as CColor;
    Some(match c {
        RColor::Black => CColor::Black,
        RColor::Red => CColor::Red,
        RColor::Green => CColor::Green,
        RColor::Yellow => CColor::Yellow,
        RColor::Blue => CColor::Blue,
        RColor::Magenta => CColor::Magenta,
        RColor::Cyan => CColor::Cyan,
        RColor::Gray => CColor::White,
        RColor::DarkGray => CColor::BrightBlack,
        RColor::LightRed => CColor::BrightRed,
        RColor::LightGreen => CColor::BrightGreen,
        RColor::LightYellow => CColor::BrightYellow,
        RColor::LightBlue => CColor::BrightBlue,
        RColor::LightMagenta => CColor::BrightMagenta,
        RColor::LightCyan => CColor::BrightCyan,
        RColor::White => CColor::BrightWhite,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mode_explicit() {
        assert!(matches!(parse_mode("truecolor"), ColorLevel::TrueColor));
        assert!(matches!(parse_mode("ansi16"), ColorLevel::Ansi16));
        assert!(matches!(parse_mode("16"), ColorLevel::Ansi16));
        assert!(matches!(parse_mode("256"), ColorLevel::Ansi256));
        assert!(matches!(parse_mode("none"), ColorLevel::NoColor));
        // "auto" / "" / 未知 → 走 detect()，结果依赖运行环境，不断言具体值
    }

    #[test]
    fn nearest_ansi16_palette_self_map() {
        for (color, (r, g, b)) in ANSI16_PALETTE {
            assert_eq!(
                nearest_ansi16(*r, *g, *b),
                *color,
                "palette entry {:?} should map to itself",
                color
            );
        }
    }

    #[test]
    fn nearest_ansi16_white_high() {
        assert!(matches!(nearest_ansi16(250, 250, 250), RColor::White));
    }

    #[test]
    fn nearest_ansi16_pure_black() {
        assert!(matches!(nearest_ansi16(0, 0, 0), RColor::Black));
    }
}
