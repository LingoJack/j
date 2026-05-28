const TAB_REPLACEMENT: &str = "    ";

/// 快速判断字符串是否包含需要为终端/TUI 渲染做清洗的字符。
///
/// 覆盖：
/// - ANSI/OSC 转义起始字节 `ESC`
/// - `\t` / `\r`
/// - 其他会扰乱终端渲染的控制字符（保留 `\n`）
pub fn needs_terminal_sanitization(s: &str) -> bool {
    s.chars()
        .any(|c| matches!(c, '\t' | '\r' | '\x1b') || (c.is_control() && c != '\n'))
}

/// 将终端不会稳定按单列显示的控制字符归一化为可见文本。
///
/// 这里处理过一个很隐蔽的 TUI 渲染问题：聊天记录里如果混入原始 `\t` / `\r`
/// （典型来源是 shell / make 输出），窄屏滚动时右边界会残留上一帧的 `/` 等字符。
/// 根因不是滚动逻辑本身，而是 ratatui 会跳过 control char，而我们若仍按“有宽度”去参与
/// wrap / bubble 宽度计算，就会让预计算宽度与实际写入 buffer 的宽度失配。
///
/// - `\t` 展开为 4 个空格，确保宽度计算与最终渲染一致
/// - `\r` / 其他控制字符（BEL、BS、ESC、DEL 等）全部移除
/// - `\n` 保留，由调用方按行切分
pub fn normalize_terminal_text(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\t' => result.push_str(TAB_REPLACEMENT),
            '\n' => result.push('\n'),
            c if c.is_control() => { /* 跳过 BEL/BS/CR/ESC/DEL 等控制字符 */ }
            c => result.push(c),
        }
    }
    result
}

/// 清理终端/TUI 展示文本：剥离 ANSI 码，再归一化控制字符。
///
/// 与 `normalize_terminal_text()` 的区别是这里会先移除完整 ANSI/OSC 转义序列，
/// 避免只删除 `ESC` 后留下裸露的 `[31m` / `]0;...` 等残片。
pub fn sanitize_terminal_text(s: &str) -> String {
    let stripped = strip_ansi_codes(s);
    normalize_terminal_text(&stripped)
}

/// 清理单行终端展示文本：剥离 ANSI/控制字符，并将换行压平为空格。
pub fn sanitize_single_line_text(s: &str) -> String {
    sanitize_terminal_text(s)
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect()
}

/// 按显示宽度对文本进行自动换行，续行使用指定的前缀字符串。
///
/// 与 `wrap_text` 不同：当一行被截断为多行时，续行会以 `continuation_prefix` 开头，
/// 用于保持带行号前缀文本的视觉对齐。例如原始行 `59│     let x = ...`，
/// 续行前缀为 `  │     `，则续行显示为 `  │     ...`。
pub fn wrap_text_with_prefix(
    text: &str,
    max_width: usize,
    continuation_prefix: &str,
) -> Vec<String> {
    let normalized;
    let text = if needs_terminal_sanitization(text) {
        normalized = sanitize_terminal_text(text);
        normalized.as_str()
    } else {
        text
    };
    let max_width = max_width.max(2);
    let prefix_width = display_width(continuation_prefix);
    // 前缀不能超过最大宽度（至少留 2 给内容字符）
    let prefix_width = prefix_width.min(max_width.saturating_sub(2));
    let prefix_display: String = continuation_prefix
        .chars()
        .fold((String::new(), 0), |(mut s, w), ch| {
            let cw = char_width(ch);
            if w + cw > prefix_width {
                (s, w)
            } else {
                s.push(ch);
                (s, w + cw)
            }
        })
        .0;

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;
    let mut is_first_line = true;

    for ch in text.chars() {
        if ch == '\n' {
            result.push(current_line.clone());
            current_line.clear();
            current_width = 0;
            is_first_line = true;
            continue;
        }
        let ch_width = char_width(ch);
        if current_width + ch_width > max_width && !current_line.is_empty() {
            result.push(current_line.clone());
            current_line.clear();
            current_width = 0;
            is_first_line = false;
        }
        // 续行以前缀开头
        if current_line.is_empty() && !is_first_line && !prefix_display.is_empty() {
            current_line.push_str(&prefix_display);
            current_width = prefix_width;
            if current_width + ch_width > max_width {
                result.push(current_line.clone());
                current_line = prefix_display.clone();
                current_width = prefix_width;
            }
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

/// 检测行号前缀宽度。
///
/// 如果行以 `数字│` 格式开头（如 `123│` 或 `  5│`），返回前缀的显示宽度（含 `│` 后的空格）。
/// 否则返回 None。
pub fn line_number_prefix_width(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    // 查找开头的连续数字
    let digits_end = trimmed.find(|c: char| !c.is_ascii_digit())?;
    if digits_end == 0 {
        return None;
    }
    let rest = &trimmed[digits_end..];
    // 检查紧跟的是 │ (U+2502) 或 | (ASCII pipe)
    if let Some(after_pipe) = rest.strip_prefix('\u{2502}') {
        let prefix_byte_len = line.len() - rest.len() + '\u{2502}'.len_utf8();
        let space_count = after_pipe.chars().take_while(|c| *c == ' ').count();
        return Some(display_width(&line[..prefix_byte_len + space_count]));
    }
    if let Some(after_pipe) = rest.strip_prefix('|') {
        let prefix_byte_len = line.len() - rest.len() + 1;
        let space_count = after_pipe.chars().take_while(|c| *c == ' ').count();
        return Some(display_width(&line[..prefix_byte_len + space_count]));
    }
    None
}

/// 检测行号前缀并生成续行前缀字符串。
///
/// 如果行以 `数字│` 格式开头（如 `59│     `），返回续行前缀：
/// 将数字部分用等宽空格替换，保留 `│` 符号及其后的空格。
/// 例如 `59│     ` → `  │     `（`59` 替换为 2 个空格）。
///
/// 返回 `(prefix_width, continuation_prefix)`：
/// - `prefix_width`: 原始前缀的显示宽度
/// - `continuation_prefix`: 续行时使用的前缀字符串（数字替换为空格）
pub fn line_number_continuation_prefix(line: &str) -> Option<(usize, String)> {
    let leading_spaces = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();
    // 查找开头的连续数字
    let digits_end = trimmed.find(|c: char| !c.is_ascii_digit())?;
    if digits_end == 0 {
        return None;
    }
    let digits = &trimmed[..digits_end];
    let digits_width = display_width(digits);
    let rest = &trimmed[digits_end..];

    // 检查紧跟的是 │ (U+2502) 或 | (ASCII pipe)
    let (pipe_char, pipe_len) = if rest.starts_with('\u{2502}') {
        ('\u{2502}', '\u{2502}'.len_utf8())
    } else if rest.starts_with('|') {
        ('|', 1)
    } else {
        return None;
    };

    let after_pipe = &rest[pipe_len..];
    let trailing_space_count = after_pipe.chars().take_while(|c| *c == ' ').count();

    // 构建续行前缀：前导空格 + 数字宽度对应的空格 + │ + 后续空格
    let continuation = format!(
        "{}{}{}{}",
        " ".repeat(leading_spaces),
        " ".repeat(digits_width),
        pipe_char,
        " ".repeat(trailing_space_count)
    );
    let prefix_width =
        display_width(&line[..leading_spaces + digits_end + pipe_len + trailing_space_count]);

    Some((prefix_width, continuation))
}

/// 按显示宽度对文本进行自动换行
/// `\n` 字符会在该处断行（产生新的 wrapped line），`\n` 本身不出现在返回的行中
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let normalized;
    let text = if needs_terminal_sanitization(text) {
        normalized = sanitize_terminal_text(text);
        normalized.as_str()
    } else {
        text
    };
    // 最小宽度保证至少能放下一个字符（中文字符宽度2），避免无限循环或不截断
    let max_width = max_width.max(2);
    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;

    for ch in text.chars() {
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
///
/// 特别处理 emoji 表现序列：当字符后跟 U+FE0F (Variation Selector-16) 时，
/// 该字符以 emoji 样式渲染，在终端中占 2 列（而非 unicode-width 返回的 1 列）。
/// 例如 "⚠️" (U+26A0 + U+FE0F) 实际占 2 列，但逐码点计算只得 1 列。
pub fn display_width(s: &str) -> usize {
    let mut chars = s.chars().peekable();
    let mut total = 0usize;
    while let Some(c) = chars.next() {
        let w = char_width(c);
        // 基础字符宽度为 1 且紧跟 U+FE0F → emoji 表现样式，占 2 列
        if w == 1 && chars.peek() == Some(&'\u{FE0F}') {
            chars.next(); // 消费 U+FE0F
            total += 2;
        } else {
            total += w;
        }
    }
    total
}

/// 计算单个字符的显示宽度（使用 unicode-width crate）
/// 约定：tab 视为 4 列，其他控制字符视为 0 列，与终端归一化策略保持一致
///
/// 注意：此函数无法感知 emoji 表现序列（如 U+26A0 + U+FE0F），
/// 若需处理 emoji 序列请使用 `chars_with_display_width` 或 `display_width`。
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

/// 带宽度的字符迭代器，正确处理 emoji 表现序列（base + U+FE0F）。
///
/// 返回 `(字符, 显示宽度)` 的迭代结果，其中 U+FE0F 本身产出 `(FE0F, 0)`，
/// 但它前一个字符的宽度会被提升为 2（emoji 表现样式）。
///
/// 典型用法：
/// ```ignore
/// for (ch, w) in chars_with_display_width(text) {
///     cur_w += w;
///     if cur_w > max_width { break; }
///     buf.push(ch);
/// }
/// ```
pub fn chars_with_display_width(s: &str) -> CharsWithWidth<'_> {
    CharsWithWidth {
        chars: s.chars().peekable(),
        pending_fe0f: false,
    }
}

/// `chars_with_display_width` 的迭代器
pub struct CharsWithWidth<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    pending_fe0f: bool,
}

impl<'a> Iterator for CharsWithWidth<'a> {
    type Item = (char, usize);

    fn next(&mut self) -> Option<Self::Item> {
        // 先输出被延迟的 U+FE0F（宽度 0），同时消费迭代器中的实际字符
        if self.pending_fe0f {
            self.pending_fe0f = false;
            self.chars.next(); // 消费迭代器中的 U+FE0F，避免重复产出
            return Some(('\u{FE0F}', 0));
        }
        let c = self.chars.next()?;
        let w = char_width(c);
        // 基础字符宽度为 1 且紧跟 U+FE0F → emoji 表现样式，基础字符占 2 列
        if w == 1 && self.chars.peek() == Some(&'\u{FE0F}') {
            self.pending_fe0f = true;
            Some((c, 2))
        } else {
            Some((c, w))
        }
    }
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
    sanitize_terminal_text(s)
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
    use super::{
        needs_terminal_sanitization, normalize_terminal_text, sanitize_single_line_text,
        sanitize_terminal_text, sanitize_tool_output, wrap_text,
    };

    #[test]
    fn needs_terminal_sanitization_detects_ansi_and_control_chars() {
        assert!(needs_terminal_sanitization("a\tb"));
        assert!(needs_terminal_sanitization("a\r\nb"));
        assert!(needs_terminal_sanitization("\x1b[31mred\x1b[0m"));
        assert!(!needs_terminal_sanitization("plain\ntext"));
    }

    #[test]
    fn normalize_terminal_text_expands_tabs_and_removes_cr() {
        assert_eq!(normalize_terminal_text("a\tb\r\nc"), "a    b\nc");
    }

    #[test]
    fn normalize_terminal_text_strips_control_chars() {
        // BEL (\x07), BS (\x08), ESC (\x1b), DEL (\x7f) 等控制字符应被移除
        assert_eq!(
            normalize_terminal_text("hello\x07world\x1b[0m"),
            "helloworld[0m"
        );
        assert_eq!(normalize_terminal_text("\x00\x01\x02"), "");
        assert_eq!(normalize_terminal_text("a\x7fb"), "ab");
    }

    #[test]
    fn normalize_terminal_text_preserves_newline() {
        // \n 应被保留
        assert_eq!(normalize_terminal_text("line1\nline2"), "line1\nline2");
    }

    #[test]
    fn normalize_terminal_text_preserves_tab_expansion() {
        // \t 应展开为 4 个空格
        assert_eq!(normalize_terminal_text("\titem"), "    item");
    }

    #[test]
    fn sanitize_tool_output_strips_ansi_and_controls() {
        // ANSI 码 + 控制字符应同时被清理
        assert_eq!(sanitize_tool_output("\x1b[32mok\x1b[0m\x07"), "ok");
    }

    #[test]
    fn sanitize_terminal_text_strips_full_ansi_sequences() {
        assert_eq!(sanitize_terminal_text("a\x1b[31mred\x1b[0m\x07b"), "aredb");
    }

    #[test]
    fn sanitize_single_line_text_flattens_newlines() {
        assert_eq!(
            sanitize_single_line_text("a\x1b[31mred\x1b[0m\nb"),
            "ared b"
        );
    }

    #[test]
    fn wrap_text_outputs_spaces_instead_of_tabs() {
        let wrapped = wrap_text("ab\tcd", 4);
        assert_eq!(wrapped, vec!["ab  ".to_string(), "  cd".to_string()]);
        assert!(wrapped.iter().all(|line| !line.contains('\t')));
    }

    #[test]
    fn wrap_text_strips_ansi_sequences_instead_of_leaking_fragments() {
        let wrapped = wrap_text("x\x1b[31mred\x1b[0my", 80);
        assert_eq!(wrapped, vec!["xredy".to_string()]);
    }
}

#[cfg(test)]
mod emoji_width_tests {
    use super::*;

    #[test]
    fn test_emoji_presentation_sequence() {
        // ⚠️ = U+26A0 + U+FE0F, emoji presentation → 2 columns
        assert_eq!(display_width("⚠️"), 2);
        // Bare ⚠ without VS16 → 1 column
        assert_eq!(display_width("⚠"), 1);
    }

    #[test]
    fn test_chars_with_display_width_emoji() {
        let pairs: Vec<(char, usize)> = chars_with_display_width("⚠️").collect();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, '⚠');
        assert_eq!(pairs[0].1, 2); // base char promoted to 2
        assert_eq!(pairs[1].0, '\u{FE0F}');
        assert_eq!(pairs[1].1, 0); // VS16 = 0
    }

    #[test]
    fn test_cjk_unaffected() {
        assert_eq!(display_width("你好"), 4); // still 2 per CJK char
    }

    #[test]
    fn test_mixed_emoji_and_cjk() {
        // ⚠️(2) + 警(2) + 告(2) = 6
        assert_eq!(display_width("⚠️警告"), 6);
    }
}
