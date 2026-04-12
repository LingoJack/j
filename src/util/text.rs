/// 按显示宽度对文本进行自动换行
/// `\n` 字符会在该处断行（产生新的 wrapped line），`\n` 本身不出现在返回的行中
/// 对于 ASCII/Latin 内容，在溢出时优先回退到上一个空格处断行，保持单词完整；
/// 若当前行没有空格（如长中文串），则回退到字符边界切断。
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    // 最小宽度保证至少能放下一个字符（中文字符宽度2），避免无限循环或不截断
    let max_width = max_width.max(2);
    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;
    // 上一个空格在 current_line 中的字节偏移（用于单词边界断行）
    let mut last_space_byte: Option<usize> = None;

    for ch in text.chars() {
        // 遇到 \n 时断行：push 当前行，开始新行
        if ch == '\n' {
            result.push(std::mem::take(&mut current_line));
            current_width = 0;
            last_space_byte = None;
            continue;
        }
        let ch_width = char_width(ch);
        if current_width + ch_width > max_width && !current_line.is_empty() {
            if let Some(sp) = last_space_byte {
                // 回退到上一个空格处断行，保持 ASCII 单词完整
                let rest = current_line[sp..].trim_start_matches(' ').to_string();
                current_line.truncate(sp);
                result.push(std::mem::take(&mut current_line));
                current_line = rest;
                current_width = display_width(&current_line);
                last_space_byte = None;
            } else {
                // 无空格可回退，按字符边界切断
                result.push(std::mem::take(&mut current_line));
                current_width = 0;
                last_space_byte = None;
            }
        }
        if ch == ' ' {
            last_space_byte = Some(current_line.len());
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
/// 注意：unicode-width 将 tab 视为宽度 0，这里补充处理将其视为宽度 1
pub fn display_width(s: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    let base = UnicodeWidthStr::width(s);
    // 统计 tab 数量，每个 tab 补偿 1 的宽度（unicode-width 视为 0）
    let tab_count = s.chars().filter(|&c| c == '\t').count();
    base + tab_count
}

/// 计算单个字符的显示宽度（使用 unicode-width crate）
/// 注意：unicode-width 将 tab 视为宽度 0，这里补充处理将其视为宽度 1
pub fn char_width(c: char) -> usize {
    if c == '\t' {
        return 1;
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
    stripped.replace('\t', "    ").replace('\r', "")
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
