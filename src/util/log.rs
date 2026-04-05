use crate::constants::{AGENT_DIR, AGENT_LOG_DIR, AGENT_LOG_ERROR, AGENT_LOG_INFO, DATA_DIR};
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// 打印普通信息
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        println!($($arg)*)
    }};
}

/// 打印错误信息
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        use colored::Colorize;
        eprint!("{}", "[ERROR] ".red());
        eprintln!($($arg)*)
    }};
}

/// 打印 usage 提示
#[macro_export]
macro_rules! usage {
    ($($arg:tt)*) => {{
        use colored::Colorize;
        print!("{}", "💡 Usage: ".green());
        println!($($arg)*)
    }};
}

/// 打印 debug 日志（仅 verbose 模式下输出）
#[macro_export]
macro_rules! debug_log {
    ($config:expr, $($arg:tt)*) => {{
        if $config.is_verbose() {
            println!($($arg)*)
        }
    }};
}

/// 首字母大写
pub fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// 写入信息日志到文件
/// 日志文件位置：~/.jdata/agent/logs/info.log
pub fn write_info_log(context: &str, content: &str) {
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(DATA_DIR)
        .join(AGENT_DIR)
        .join(AGENT_LOG_DIR);

    if let Err(e) = fs::create_dir_all(&log_dir) {
        eprintln!("无法创建日志目录: {}", e);
        return;
    }

    let log_file = log_dir.join(AGENT_LOG_INFO);

    match OpenOptions::new().create(true).append(true).open(&log_file) {
        Ok(mut file) => {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let log_entry = format!(
                "\n========================================\n[{}] {}\n{}\n",
                timestamp, context, content
            );
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                eprintln!("写入信息日志失败: {}", e);
            }
        }
        Err(e) => {
            eprintln!("无法打开信息日志文件: {}", e);
        }
    }
}

/// 写入错误日志到文件
/// 日志文件位置：~/.jdata/agent/logs/error.log
pub fn write_error_log(context: &str, error: &str) {
    let log_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(DATA_DIR)
        .join(AGENT_DIR)
        .join(AGENT_LOG_DIR);

    // 创建日志目录
    if let Err(e) = fs::create_dir_all(&log_dir) {
        eprintln!("无法创建日志目录: {}", e);
        return;
    }

    let log_file = log_dir.join(AGENT_LOG_ERROR);

    // 写入日志
    match OpenOptions::new().create(true).append(true).open(&log_file) {
        Ok(mut file) => {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let log_entry = format!(
                "\n========================================\n[{}] {}\n错误详情:\n{}\n",
                timestamp, context, error
            );
            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                eprintln!("写入错误日志失败: {}", e);
            }
        }
        Err(e) => {
            eprintln!("无法打开错误日志文件: {}", e);
        }
    }
}
