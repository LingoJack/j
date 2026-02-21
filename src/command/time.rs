use crate::constants::time_function;
use crate::{error, info, usage};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Write};

/// 处理 time 命令: j time countdown <duration>
/// duration 支持: 30s（秒）、5m（分钟）、1h（小时），不带单位默认为分钟
pub fn handle_time(function: &str, arg: &str) {
    if function != time_function::COUNTDOWN {
        error!("❌ 未知的功能: {}，目前仅支持 countdown", function);
        usage!("j time countdown <duration>");
        info!("  duration 格式: 30s(秒), 5m(分钟), 1h(小时), 不带单位默认为分钟");
        return;
    }

    let duration_secs = parse_duration(arg);
    if duration_secs <= 0 {
        error!("❌ 无效的时长: {}", arg);
        return;
    }

    info!(
        "⏳ 倒计时开始：{}",
        format_duration_display(duration_secs as u64)
    );
    run_countdown(duration_secs as u64);
}

/// 格式化时长为可读的中文显示
fn format_duration_display(secs: u64) -> String {
    if secs >= 3600 {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        if m > 0 {
            format!("{}小时{}分钟", h, m)
        } else {
            format!("{}小时", h)
        }
    } else if secs >= 60 {
        let m = secs / 60;
        let s = secs % 60;
        if s > 0 {
            format!("{}分{}秒", m, s)
        } else {
            format!("{}分钟", m)
        }
    } else {
        format!("{}秒", secs)
    }
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

/// 格式化剩余时间为 HH:MM:SS 或 MM:SS
fn format_remaining(secs: u64) -> String {
    if secs >= 3600 {
        format!(
            "{:02}:{:02}:{:02}",
            secs / 3600,
            (secs % 3600) / 60,
            secs % 60
        )
    } else {
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }
}

/// 运行倒计时（带进度条和动画）
fn run_countdown(total_secs: u64) {
    let pb = ProgressBar::new(total_secs);

    // 设置进度条样式
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  {spinner:.cyan} ⏱️  {msg}  {wide_bar:.cyan/dark_gray}  {percent}%")
            .unwrap()
            .progress_chars("━╸─"),
    );

    pb.set_message(format_remaining(total_secs));

    let start = std::time::Instant::now();

    for elapsed in 1..=total_secs {
        // 精确校准每秒
        let next_tick = start + std::time::Duration::from_secs(elapsed);
        let now = std::time::Instant::now();
        if next_tick > now {
            std::thread::sleep(next_tick - now);
        }

        let remaining = total_secs - elapsed;
        pb.set_position(elapsed);
        pb.set_message(format_remaining(remaining));
    }

    pb.finish_and_clear();

    println!("  🎉 Time's up! 倒计时结束！");
    println!();

    // 结束动画
    display_celebration();
}

/// 结束庆祝动画
fn display_celebration() {
    let frames = [
        "  🔔 Ding Ding! Time's Up! 🔔",
        "  💢😤💢 Stop! Stop! Stop! 💢😤💢",
        "  🔥😠🔥 How dare you don't stop! 🔥😠🔥",
    ];

    // 先播放系统提示音（macOS，非阻塞）
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("afplay")
            .arg("/System/Library/Sounds/Glass.aiff")
            .spawn();
    }

    for i in 0..6 {
        // 用空格覆盖上一帧的残余字符
        print!("\r{:<60}", frames[i % frames.len()]);
        let _ = io::stdout().flush();
        std::thread::sleep(std::time::Duration::from_millis(600));
    }
    println!();
}
