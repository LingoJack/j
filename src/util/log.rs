use crate::constants::{AGENT_DIR, AGENT_LOG_DIR, AGENT_LOG_ERROR, AGENT_LOG_INFO, DATA_DIR};
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// 单个日志文件最大大小（10 MB），超过后触发轮转
const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;
/// 日志保留的历史备份数（info.log.1 ~ info.log.3）
const MAX_LOG_BACKUPS: u32 = 3;

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

/// 如果日志文件超过阈值，执行轮转。
/// 轮转策略：info.log → info.log.1 → info.log.2 → info.log.3（最旧的被丢弃）
fn rotate_log_if_needed(log_dir: &Path, file_name: &str) {
    let current = log_dir.join(file_name);
    let need_rotate = match fs::metadata(&current) {
        Ok(meta) => meta.len() >= MAX_LOG_SIZE,
        Err(_) => false,
    };
    if !need_rotate {
        return;
    }
    // 删除最旧的备份
    let oldest = log_dir.join(format!("{}.{}", file_name, MAX_LOG_BACKUPS));
    let _ = fs::remove_file(&oldest);
    // 从旧到新依次重命名: .2→.3, .1→.2
    for i in (1..MAX_LOG_BACKUPS).rev() {
        let src = log_dir.join(format!("{}.{}", file_name, i));
        let dst = log_dir.join(format!("{}.{}", file_name, i + 1));
        let _ = fs::rename(&src, &dst);
    }
    // original → .1
    let backup = log_dir.join(format!("{}.1", file_name));
    let _ = fs::rename(&current, &backup);
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

    rotate_log_if_needed(&log_dir, AGENT_LOG_INFO);

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

    rotate_log_if_needed(&log_dir, AGENT_LOG_ERROR);

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
