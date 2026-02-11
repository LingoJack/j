use crate::constants::time_function;
use crate::{error, info, usage};
use std::io::{self, Write};

/// 处理 time 命令: j time countdown <duration>
/// duration 支持: 30s（秒）、5m（分钟）、1h（小时），不带单位默认为分钟
pub fn handle_time(function: &str, arg: &str) {
    if function != time_function::COUNTDOWN {
        error!("❌ 未知的功能: {}，目前仅支持 countdown", function);
        usage!("j time countdown <duration>");
        info!(
            "  duration 格式: 30s(秒), 5m(分钟), 1h(小时), 不带单位默认为分钟"
        );
        return;
    }

    let duration_secs = parse_duration(arg);
    if duration_secs <= 0 {
        error!("❌ 无效的时长: {}", arg);
        return;
    }

    info!("⏳ Countdown started for {} seconds...", duration_secs);
    run_countdown(duration_secs as u64);
}

/// 解析时长字符串为秒数
fn parse_duration(s: &str) -> i64 {
    let s = s.trim();
    if s.ends_with('s') {
        s[..s.len() - 1].parse::<i64>().unwrap_or(-1)
    } else if s.ends_with('m') {
        s[..s.len() - 1]
            .parse::<i64>()
            .map(|m| m * 60)
            .unwrap_or(-1)
    } else if s.ends_with('h') {
        s[..s.len() - 1]
            .parse::<i64>()
            .map(|h| h * 3600)
            .unwrap_or(-1)
    } else {
        // 默认单位为分钟
        s.parse::<i64>().map(|m| m * 60).unwrap_or(-1)
    }
}

/// 运行倒计时（带进度条和动画）
fn run_countdown(total_secs: u64) {
    let start = std::time::Instant::now();
    let progress_width = 60;

    for remaining in (1..=total_secs).rev() {
        let time_left = format!("⏱️ {:02}:{:02}", remaining / 60, remaining % 60);

        let elapsed_secs = total_secs - remaining;
        let completed = (elapsed_secs * progress_width as u64 / total_secs) as usize;
        let remaining_width = progress_width - completed - 1;

        let bar = format!(
            "[{}>{:width$}]",
            "=".repeat(completed),
            "",
            width = remaining_width
        );

        print!("\r{} {}", time_left, bar);
        let _ = io::stdout().flush();

        // 精确校准每秒
        let next_tick_offset =
            std::time::Duration::from_secs(total_secs - remaining + 1);
        let next_tick = start + next_tick_offset;
        let now = std::time::Instant::now();
        if next_tick > now {
            std::thread::sleep(next_tick - now);
        }
    }

    // 倒计时完成
    println!(
        "\r🎉 Time's up! [{}] 🎉",
        "=".repeat(progress_width) + ">"
    );

    // 结束动画
    display_celebration();
}

/// 结束庆祝动画
fn display_celebration() {
    let frames = [
        "🔔 Ding Ding! Time's Up!🔔",
        "💢😤💢 Stop! Stop! Stop! 💢😤💢",
        "🔥😠🔥 How dare you don't stop! 🔥😠🔥",
    ];

    for i in 0..6 {
        print!("\r{}", frames[i % frames.len()]);
        let _ = io::stdout().flush();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    println!();

    // 系统蜂鸣（macOS）
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("afplay")
            .arg("/System/Library/Sounds/Glass.aiff")
            .spawn();
    }
}
