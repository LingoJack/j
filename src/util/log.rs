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

/// 在终端中渲染 Markdown 文本（使用 termimad）
#[macro_export]
macro_rules! md {
    ($($arg:tt)*) => {{
        let text = format!($($arg)*);
        termimad::print_text(&text);
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

/// 使用自定义皮肤渲染 Markdown 文本
/// 用法: md_skin!(skin, "markdown text {}", arg)
#[macro_export]
macro_rules! md_skin {
    ($skin:expr, $($arg:tt)*) => {{
        let text = format!($($arg)*);
        $skin.print_text(&text);
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
