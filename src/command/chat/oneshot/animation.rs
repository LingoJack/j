//! oneshot 思考动画：启动/停止思考指示器线程

use crate::command::chat::oneshot::display::thinking_pulse_color;
use crate::command::chat::storage::config::ThinkingStyle;
use colored::Colorize;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// 退出动画 tick 间隔（毫秒）。
const ONESHOT_EXIT_TICK_MS: u64 = 100;
/// 退出动画结束后等待时间（毫秒）。
const ONESHOT_EXIT_SETTLE_MS: u64 = 50;

/// 启动思考动画线程，返回停止标志
pub(crate) fn start_thinking_animation(thinking_style: ThinkingStyle) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);

    std::thread::spawn(move || {
        use crossterm::{cursor, execute, terminal};

        let mut tick: u64 = 0;
        let mut stdout = io::stdout();

        while !stop_clone.load(Ordering::Relaxed) {
            let pulse_color = thinking_pulse_color();
            let frame = thinking_style.frame(tick);
            let line = format!("  {} 思考中...", frame);
            let colored_line = crate::util::color_adapt::apply_fg(&line, pulse_color)
                .bold()
                .to_string();

            let _ = execute!(
                stdout,
                cursor::MoveToColumn(0),
                terminal::Clear(terminal::ClearType::CurrentLine),
                crossterm::style::Print(&colored_line),
            );
            let _ = stdout.flush();

            tick += 1;
            std::thread::sleep(Duration::from_millis(ONESHOT_EXIT_TICK_MS));
        }

        // 清除动画行
        let _ = execute!(
            stdout,
            cursor::MoveToColumn(0),
            terminal::Clear(terminal::ClearType::CurrentLine),
        );
        let _ = stdout.flush();
    });

    stop
}

/// 停止思考动画并清除行
pub(crate) fn stop_thinking_animation(stop_flag: &Arc<AtomicBool>) {
    stop_flag.store(true, Ordering::Relaxed);
    // 给动画线程时间清除行
    std::thread::sleep(Duration::from_millis(ONESHOT_EXIT_SETTLE_MS));
}
