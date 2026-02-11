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

/// 在终端中渲染 Markdown 文本
/// 优先通过管道调用外部 `ask -c render` 渲染（效果更佳），
/// 如果 ask 不可用则 fallback 到 termimad
#[macro_export]
macro_rules! md {
    ($($arg:tt)*) => {{
        let text = format!($($arg)*);
        $crate::util::log::render_markdown(&text);
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

/// 打印分隔线
#[allow(dead_code)]
pub fn print_line() {
    println!("- - - - - - - - - - - - - - - - - - - - - - -");
}

/// 首字母大写
pub fn capitalize_first_letter(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// 渲染 Markdown 文本到终端
/// 优先通过管道调用外部 `ask -c render`（效果更佳），
/// 如果 ask 不可用则 fallback 到 termimad
pub fn render_markdown(text: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};

    // 尝试调用外部 ask -c render
    let result = Command::new("ask")
        .args(["-c", "render"])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn();

    match result {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
                // 关闭 stdin 触发 ask 处理
                drop(stdin);
            }
            let _ = child.wait();
        }
        Err(_) => {
            // ask 不可用，fallback 到 termimad
            termimad::print_text(text);
        }
    }
}
