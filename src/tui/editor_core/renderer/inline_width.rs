//! 计算每行 Markdown 在渲染后的"逐字符可见显示宽度"。
//!
//! 用于 [`crate::tui::editor_core::wrap_engine::WrapEngine`] 的折行：
//! - 普通文本字符：`char_width(ch)`
//! - Markdown 标记符号（`**`/`*`/`_`/`~~`/`` ` ``、链接的 `[`、`]`、`(url)`）：0
//!
//! 复用 [`super::inline::parse_inline_text`] 解析 inline 元素，避免在折行逻辑里
//! 再写一套行内标记剥离器（违反 "复用既有解析" 的约定）。
//!
//! ## 范围
//!
//! 第一版只处理 **inline 范围内**的标记符号；**行前缀**（heading `#`、list `-`/`*`、
//! ordered `1.`、blockquote `>`）按源码字符宽计算，不做渲染宽度补偿。原因：
//! - 不同前缀渲染产物宽度差异不一（`# →◆ ` 同宽、`## →◇ ` 少 1 列、
//!   `### →〈 〉` 多 2 列、`> →  | ` 多 2 列…），与 [`super::line`] 实际渲染的
//!   占位字符紧耦合；后续在那一处统一抽象后再做也来得及。
//! - 用户报的主要偏差源是 inline 标记（`**bold**`、`[text](url)`、`` `code` ``），
//!   行前缀至多影响 1~4 列，远小于一个 `[text](url)` 链接被吃掉的列数。

use crate::markdown::ir::Inline;
use crate::util::text::char_width;

use super::inline::parse_inline_text;

/// 为一行 Markdown 源码计算每个**源码 char** 的渲染后显示宽度。
///
/// 返回 `Vec<u8>`，长度等于 `line.chars().count()`：
/// - Markdown 标记符号位置 → `0`
/// - 可见正文位置 → `char_width(ch) as u8`
///
/// 折行时累加这个数组（而不是 `char_width(ch)`）就能让折行点对齐到
/// 实际渲染宽度。`start_col` / `end_col` 仍是源码 char 索引——光标 / 选区 /
/// 鼠标定位的契约不变。
///
/// 实现要点：
/// 1. 默认每个 char 给 `char_width(ch) as u8`；
/// 2. 调 `parse_inline_text` 拿到 `Vec<Inline>`，递归遍历收集每个 inline
///    元素的"可见文本"。对每个可见文本片段，用一个游标在源码里"匹配"——
///    源码顺序与渲染顺序一致，且可见文本一定是源码的子序列：标记符号在
///    源码中的位置就是"游标跳过的字符"。把跳过的位置全部置 0；
/// 3. 行前缀 / 未识别字符保留 char_width。
///
/// 这个朴素游标对齐**对绝大多数 inline case 准确**：
/// - `**bold**`：可见 = `bold`，跳过 `**` 和 `**` 共 4 char → 4 个 0；
/// - `[text](url)`：可见 = `text`，跳过 `[`、`](url)` 共 `2 + url.len() + 1` char；
/// - `` `code` ``：可见 = `code`，跳过两个 backtick；
/// - `~~strike~~`：可见 = `strike`，跳过 `~~` 两端共 4 char。
///
/// 边界情形（HTML 实体、转义符 `\*`、嵌套强调）少数会偏 1~2 列，但不会
/// 越界 / panic（游标永远不超过源码长度）。第一版接受这种精度。
pub fn compute_visible_widths(line: &str) -> Vec<u8> {
    let src_chars: Vec<char> = line.chars().collect();
    let n = src_chars.len();
    if n == 0 {
        return Vec::new();
    }

    // 默认按源码 char_width，处理 emoji 表现序列（base + U+FE0F → base 宽度提升为 2）
    let mut widths: Vec<u8> = Vec::with_capacity(n);
    for (i, c) in src_chars.iter().enumerate() {
        let w = char_width(*c);
        // 基础字符宽度为 1 且下一个字符是 U+FE0F → emoji 表现样式，占 2 列
        if w == 1 && i + 1 < n && src_chars[i + 1] == '\u{FE0F}' {
            widths.push(2);
        } else {
            widths.push(w.min(u8::MAX as usize) as u8);
        }
    }

    let inlines = parse_inline_text(line);
    if inlines.is_empty() {
        return widths;
    }

    // 收集所有可见文本片段（按出现顺序）
    let mut visible_segments: Vec<String> = Vec::new();
    for inl in &inlines {
        collect_visible_text(inl, &mut visible_segments);
    }

    // 用游标在源码里依次匹配每个可见片段；跳过的源码 char 位置置 0
    // 注意：可见文本中相邻片段可能在源码中相隔多个标记字符（`*` `[` 等），
    // 这正是要置 0 的部分。
    let mut cursor = 0usize; // 源码 char 下标
    for seg in &visible_segments {
        let seg_chars: Vec<char> = seg.chars().collect();
        if seg_chars.is_empty() {
            continue;
        }
        // 在 src_chars[cursor..] 找 seg_chars 子串（朴素扫描，行内 char 数有限）
        if let Some(rel) = find_subsequence(&src_chars[cursor..], &seg_chars) {
            // 把 [cursor, cursor+rel) 范围内的 char 视为标记符号 → 置 0
            for w in widths.iter_mut().take(cursor + rel).skip(cursor) {
                *w = 0;
            }
            // 跳过这段可见文本（保留它们的 char_width）
            cursor += rel + seg_chars.len();
        }
        // 找不到（罕见：HTML 实体被 pulldown-cmark 解码）→ 不动 widths，cursor 保持
    }

    // 末尾如有未消费的源码 char（通常是闭合标记 `**` / `` ` `` 或链接 `](url)`），
    // 这些都是不可见标记 → 置 0
    if cursor < n {
        for w in widths.iter_mut().take(n).skip(cursor) {
            *w = 0;
        }
    }

    widths
}

/// 递归收集 inline 元素中所有可见的文本片段（按出现顺序）。
///
/// - `Text(s)` / `Code(s)`：整段加入；
/// - `Strong/Emphasis/Strikethrough`：递归其子元素；
/// - `Link { text, url }`：只递归 `text`，丢弃 `url`；
/// - `SoftBreak/HardBreak`：单行折行场景不出现，忽略。
fn collect_visible_text(inline: &Inline, out: &mut Vec<String>) {
    match inline {
        Inline::Text(s) => out.push(s.clone()),
        Inline::Code(s) => out.push(s.clone()),
        Inline::Strong(children) | Inline::Emphasis(children) | Inline::Strikethrough(children) => {
            for c in children {
                collect_visible_text(c, out);
            }
        }
        Inline::Link { text, .. } => {
            for c in text {
                collect_visible_text(c, out);
            }
        }
        Inline::Image { alt, url } => {
            // 与终端渲染的占位文本保持一致
            let placeholder = if alt.is_empty() {
                format!("[图片]({})", url)
            } else {
                format!("[图片: {}]({})", alt, url)
            };
            out.push(placeholder);
        }
        Inline::SoftBreak | Inline::HardBreak => {}
    }
}

/// 在 `haystack` 中找 `needle` 的首次出现位置（按 char 比较）。返回 char 偏移。
/// 朴素 O(n*m) 搜索；行内 char 数通常 < 200，开销可忽略。
fn find_subsequence(haystack: &[char], needle: &[char]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > haystack.len() {
        return None;
    }
    'outer: for start in 0..=haystack.len() - needle.len() {
        for (i, c) in needle.iter().enumerate() {
            if haystack[start + i] != *c {
                continue 'outer;
            }
        }
        return Some(start);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total_visible(line: &str) -> usize {
        compute_visible_widths(line)
            .iter()
            .map(|&w| w as usize)
            .sum()
    }

    #[test]
    fn bold_marks_are_zero_width() {
        let line = "**bold**";
        let ws = compute_visible_widths(line);
        // 8 个 char：* * b o l d * *
        assert_eq!(ws.len(), 8);
        assert_eq!(ws[0], 0);
        assert_eq!(ws[1], 0);
        assert_eq!(ws[2], 1); // b
        assert_eq!(ws[3], 1); // o
        assert_eq!(ws[4], 1); // l
        assert_eq!(ws[5], 1); // d
        assert_eq!(ws[6], 0);
        assert_eq!(ws[7], 0);
        assert_eq!(total_visible(line), 4);
    }

    #[test]
    fn link_url_is_zero_width() {
        let line = "[CodeBuddy](https://example.com)";
        let ws = compute_visible_widths(line);
        // 可见部分 = "CodeBuddy" = 9 列
        assert_eq!(total_visible(line), 9);
        // 第 0 位 '[' = 0
        assert_eq!(ws[0], 0);
        // '['=0 后跟 C, o, d, e, B, u, d, d, y 各 1 列
        for (i, expected) in [1, 1, 1, 1, 1, 1, 1, 1, 1].iter().enumerate() {
            assert_eq!(
                ws[1 + i],
                *expected as u8,
                "pos {} should be visible",
                1 + i
            );
        }
        // 后面 "](https://example.com)" 全部 0
        let total = line.chars().count();
        for w in ws.iter().take(total).skip(10) {
            assert_eq!(*w, 0);
        }
    }

    #[test]
    fn inline_code_marks_are_zero_width() {
        let line = "use `foo` here";
        let ws = compute_visible_widths(line);
        // u s e _ ` f o o ` _ h e r e   (14 chars)
        // backticks 应为 0
        let chars: Vec<char> = line.chars().collect();
        let bt1 = chars.iter().position(|c| *c == '`').unwrap();
        let bt2 = chars.iter().rposition(|c| *c == '`').unwrap();
        assert_eq!(ws[bt1], 0);
        assert_eq!(ws[bt2], 0);
        // foo 三个字符可见
        assert_eq!(ws[bt1 + 1], 1);
        assert_eq!(ws[bt1 + 2], 1);
        assert_eq!(ws[bt1 + 3], 1);
        // 可见总宽 = "use foo here" = 12
        assert_eq!(total_visible(line), 12);
    }

    #[test]
    fn strikethrough_marks_are_zero_width() {
        let line = "~~gone~~";
        let ws = compute_visible_widths(line);
        // ~ ~ g o n e ~ ~
        assert_eq!(ws[0], 0);
        assert_eq!(ws[1], 0);
        assert_eq!(ws[6], 0);
        assert_eq!(ws[7], 0);
        assert_eq!(total_visible(line), 4);
    }

    #[test]
    fn plain_text_unchanged() {
        let line = "just plain text";
        let ws = compute_visible_widths(line);
        let expected: Vec<u8> = line
            .chars()
            .map(|c| char_width(c).min(u8::MAX as usize) as u8)
            .collect();
        assert_eq!(ws, expected);
    }

    #[test]
    fn cjk_characters_count_two_columns() {
        // 中文：每个 char = 2 列
        let line = "你**好**世界";
        let ws = compute_visible_widths(line);
        // 你 ** 好 ** 世 界  → 共 8 char
        assert_eq!(ws.len(), 8);
        assert_eq!(ws[0], 2); // 你
        assert_eq!(ws[1], 0); // *
        assert_eq!(ws[2], 0); // *
        assert_eq!(ws[3], 2); // 好
        assert_eq!(ws[4], 0); // *
        assert_eq!(ws[5], 0); // *
        assert_eq!(ws[6], 2); // 世
        assert_eq!(ws[7], 2); // 界
        // 可见总宽 = (你 + 好 + 世 + 界) = 8
        assert_eq!(total_visible(line), 8);
    }

    #[test]
    fn empty_line_returns_empty() {
        let ws = compute_visible_widths("");
        assert!(ws.is_empty());
    }

    #[test]
    fn line_without_any_markdown_matches_char_width() {
        let line = "纯中文 with English 123";
        let expected: usize = line.chars().map(char_width).sum();
        assert_eq!(total_visible(line), expected);
    }
}
