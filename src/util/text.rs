const TAB_REPLACEMENT: &str = "    ";

/// 将终端不会稳定按单列显示的控制字符归一化为可见文本。
///
/// - `\t` 展开为 4 个空格，避免 ratatui 跳过 tab 后造成宽度计算与实际渲染不一致
/// - `\r` 移除，避免覆盖式输出在 TUI 中留下脏字符
pub fn normalize_terminal_text(s: &str) -> String {
    s.replace('\t', TAB_REPLACEMENT).replace('\r', "")
}

/// 按显示宽度对文本进行自动换行
/// `\n` 字符会在该处断行（产生新的 wrapped line），`\n` 本身不出现在返回的行中
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let normalized = normalize_terminal_text(text);
    // 最小宽度保证至少能放下一个字符（中文字符宽度2），避免无限循环或不截断
    let max_width = max_width.max(2);
    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for ch in normalized.chars() {
        // 遇到 \n 时断行：push 当前行，开始新行
        if ch == '\n' {
            result.push(current_line.clone());
            current_line.clear();
            current_width = 0;
            continue;
        }
        let ch_width = char_width(ch);
        if current_width + ch_width > max_width && !current_line.is_empty() {
            result.push(current_line.clone());
            current_line.clear();
            current_width = 0;
        }
        current_line.push(ch);
        current_width += ch_width;
    }
    if !current_line.is_empty() {
        result.push(current_line);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

/// 计算字符串的显示宽度（使用 unicode-width crate，比手动范围匹配更准确）
/// 约定：tab 视为 4 列，其他控制字符视为 0 列，与终端归一化策略保持一致
pub fn display_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

/// 计算单个字符的显示宽度（使用 unicode-width crate）
/// 约定：tab 视为 4 列，其他控制字符视为 0 列，与终端归一化策略保持一致
pub fn char_width(c: char) -> usize {
    if c == '\t' {
        return TAB_REPLACEMENT.len();
    }
    if c.is_control() {
        return 0;
    }
    use unicode_width::UnicodeWidthChar;
    UnicodeWidthChar::width(c).unwrap_or(0)
}

/// 剥离 ANSI 转义序列（颜色、粗体等控制码），返回纯文本
/// 例如 "\x1b[1;34mhello\x1b[0m" → "hello"
pub fn strip_ansi_codes(s: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        // CSI 序列: ESC[ (参数字节 0x30-0x3F: 0-9;:?>=!等) (中间字节 0x20-0x2F) 终止字节
        // 覆盖 ESC[?1049h (alternate screen), ESC[38;2;...m (RGB color) 等
        // OSC 序列: ESC]...BEL 或 ESC]...ST
        // 其他 ESC 序列: ESC + 单字节
        Regex::new(r"\x1b\[[\x20-\x3f]*[\x40-\x7e]|\x1b\][^\x07]*(?:\x07|\x1b\\)|\x1b[^\[\]()]")
            .expect("正则表达式编译失败，这是一个静态模式，应该总是有效的")
    });
    re.replace_all(s, "").into_owned()
}

/// 清理工具输出文本：剥离 ANSI 码 + 将 tab 替换为空格 + 移除 \r
pub fn sanitize_tool_output(s: &str) -> String {
    let stripped = strip_ansi_codes(s);
    normalize_terminal_text(&stripped)
}

/// 去除字符串两端的引号（单引号或双引号）
pub fn remove_quotes(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2
        && ((s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')))
    {
        return s[1..s.len() - 1].to_string();
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::{normalize_terminal_text, wrap_text};

    #[test]
    fn normalize_terminal_text_expands_tabs_and_removes_cr() {
        assert_eq!(normalize_terminal_text("a\tb\r\nc"), "a    b\nc");
    }

    #[test]
    fn wrap_text_outputs_spaces_instead_of_tabs() {
        let wrapped = wrap_text("ab\tcd", 4);
        assert_eq!(wrapped, vec!["ab  ".to_string(), "  cd".to_string()]);
        assert!(wrapped.iter().all(|line| !line.contains('\t')));
    }
}
