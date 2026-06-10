// ========== 工具函数 ==========

/// 计算字符串的显示宽度（使用 unicode-width，与 wrap_text 保持一致）
pub fn display_width(s: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    UnicodeWidthStr::width(s)
}

/// 将字符串截断到指定的显示宽度，超出部分用 ".." 替代
pub fn truncate_to_width(s: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    if max_width == 0 {
        return String::new();
    }
    let total_width = display_width(s);
    if total_width <= max_width {
        return s.to_string();
    }
    let ellipsis = "..";
    let ellipsis_width = 2;
    let content_budget = max_width.saturating_sub(ellipsis_width);
    let mut width = 0;
    let mut result = String::new();
    for ch in s.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > content_budget {
            break;
        }
        width += ch_width;
        result.push(ch);
    }
    result.push_str(ellipsis);
    result
}

/// 复制内容到系统剪切板（使用 arboard 库，支持 macOS/Linux/Windows）
pub fn copy_to_clipboard(content: &str) -> bool {
    use arboard::Clipboard;

    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(_) => return false,
    };
    clipboard.set_text(content).is_ok()
}
