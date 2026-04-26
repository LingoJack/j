//! 动画效果：思考指示器脉冲、彗星渐变

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::command::chat::constants::{THINKING_PULSE_MIN_FACTOR, THINKING_PULSE_PERIOD_MS};
use crate::command::chat::render::theme::Theme;
use crate::command::chat::storage::config::ThinkingStyle;
use crate::command::chat::ui::palette;

/// 基于 tick（每 100ms 递增 1）计算当前帧序号
pub(crate) fn current_tick() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
        / 100
}

/// 计算思考指示器的脉冲颜色：基于 label_ai 颜色在亮暗之间平滑过渡
/// 使用正弦波实现呼吸灯效果，周期约 1.5 秒
pub(crate) fn thinking_pulse_color(theme: &Theme) -> Color {
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
pub(crate) fn comet_gradient_line(
    tick: u64,
    palette_idx: u8,
    fallback_color: Color,
) -> Line<'static> {
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
