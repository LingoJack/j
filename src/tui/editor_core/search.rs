//! 搜索功能
//!
//! 实现文本搜索和高亮。

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use super::theme::EditorTheme;

/// 搜索匹配
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// 匹配所在的行
    pub line: usize,
    /// 匹配起始列
    pub start: usize,
    /// 匹配结束列
    pub end: usize,
}

/// 搜索状态
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    /// 搜索模式
    pattern: String,
    /// 所有匹配
    matches: Vec<SearchMatch>,
    /// 按行分组的索引：line_idx -> (matches 中的起始位置, 数量)
    line_index: std::collections::HashMap<usize, (usize, usize)>,
    /// 当前匹配索引
    current_index: usize,
}

impl SearchState {
    /// 创建新的搜索状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 是否正在搜索
    pub fn is_searching(&self) -> bool {
        !self.pattern.is_empty()
    }

    /// 执行搜索
    pub fn search(&mut self, pattern: &str, lines: &[String]) -> usize {
        self.pattern = pattern.to_string();
        self.matches.clear();
        self.line_index.clear();
        self.current_index = 0;

        if pattern.is_empty() {
            return 0;
        }

        let pattern_char_len = pattern.chars().count();

        for (line_idx, line) in lines.iter().enumerate() {
            let match_start_idx = self.matches.len();
            let mut byte_start = 0;
            while let Some(byte_pos) = line[byte_start..].find(pattern) {
                let abs_byte = byte_start + byte_pos;
                // 将字节偏移转换为字符偏移
                let char_start = line[..abs_byte].chars().count();
                self.matches.push(SearchMatch {
                    line: line_idx,
                    start: char_start,
                    end: char_start + pattern_char_len,
                });
                byte_start = abs_byte + pattern.len();
                if byte_start >= line.len() {
                    break;
                }
            }
            let count = self.matches.len() - match_start_idx;
            if count > 0 {
                self.line_index.insert(line_idx, (match_start_idx, count));
            }
        }
        self.matches.len()
    }

    /// 获取当前匹配
    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.matches.get(self.current_index)
    }

    /// 下一个匹配
    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_index = (self.current_index + 1) % self.matches.len();
        }
    }

    /// 上一个匹配
    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_index = if self.current_index == 0 {
                self.matches.len() - 1
            } else {
                self.current_index - 1
            };
        }
    }

    /// 匹配数量
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// 高亮行中的搜索匹配
    ///
    /// `line` 可能是视觉行片段（折行后的子串），`char_offset` 是该片段
    /// 在逻辑行中的字符起始偏移，用于将匹配坐标映射到片段内坐标。
    pub fn highlight_line(
        &self,
        line_idx: usize,
        line: &str,
        theme: &EditorTheme,
        char_offset: usize,
    ) -> Vec<Span<'static>> {
        let normal_style = Style::default().fg(theme.text_normal);
        let highlight_style = Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD);

        let line_matches = if let Some(&(start, count)) = self.line_index.get(&line_idx) {
            &self.matches[start..start + count]
        } else {
            return vec![Span::styled(line.to_string(), normal_style)];
        };

        if line_matches.is_empty() || self.pattern.is_empty() {
            return vec![Span::styled(line.to_string(), normal_style)];
        }

        let chars: Vec<char> = line.chars().collect();
        let char_end = char_offset + chars.len();

        // 筛选与当前片段重叠的匹配，并裁剪到片段范围内
        let mut spans = Vec::new();
        let mut last_local = 0;

        for m in line_matches {
            // 跳过不与片段重叠的匹配
            if m.end <= char_offset || m.start >= char_end {
                continue;
            }
            // 裁剪到片段内的局部坐标
            let local_start = m.start.saturating_sub(char_offset).min(chars.len());
            let local_end = m.end.saturating_sub(char_offset).min(chars.len());

            if local_start > last_local {
                let text: String = chars[last_local..local_start].iter().collect();
                spans.push(Span::styled(text, normal_style));
            }
            if local_start < local_end {
                let match_text: String = chars[local_start..local_end].iter().collect();
                spans.push(Span::styled(match_text, highlight_style));
            }
            last_local = local_end;
        }

        if last_local < chars.len() {
            let text: String = chars[last_local..].iter().collect();
            spans.push(Span::styled(text, normal_style));
        }

        if spans.is_empty() {
            return vec![Span::styled(line.to_string(), normal_style)];
        }

        spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    /// 测试用主题
    fn test_theme() -> EditorTheme {
        EditorTheme {
            bg_primary: Color::Reset,
            bg_input: Color::Reset,
            code_bg: Color::DarkGray,
            cursor_fg: Color::Black,
            cursor_bg: Color::Cyan,
            text_normal: Color::White,
            text_dim: Color::DarkGray,
            text_bold: Color::White,
            md_h1: Color::Cyan,
            md_h2: Color::Green,
            md_h3: Color::Yellow,
            md_h4: Color::Magenta,
            md_link: Color::Blue,
            md_list_bullet: Color::Yellow,
            md_blockquote_bar: Color::Cyan,
            md_blockquote_bg: Color::DarkGray,
            md_blockquote_text: Color::Gray,
            md_inline_code_fg: Color::Magenta,
            md_inline_code_bg: Color::DarkGray,
            code_default: Color::White,
            code_keyword: Color::Magenta,
            code_string: Color::Green,
            code_comment: Color::DarkGray,
            code_number: Color::Yellow,
            code_type: Color::Yellow,
            code_primitive: Color::Cyan,
            code_macro: Color::LightCyan,
            code_lifetime: Color::LightMagenta,
            code_attribute: Color::LightBlue,
            code_shell_var: Color::LightCyan,
            label_ai: Color::Green,
        }
    }

    #[test]
    fn test_search() {
        let mut search = SearchState::new();
        let lines = vec!["hello world".to_string(), "hello universe".to_string()];

        let count = search.search("hello", &lines);
        assert_eq!(count, 2);
        assert_eq!(search.match_count(), 2);
    }

    #[test]
    fn test_navigation() {
        let mut search = SearchState::new();
        let lines = vec!["aaa bbb aaa".to_string()];

        search.search("aaa", &lines);

        let m = search.current_match().unwrap();
        assert_eq!(m.start, 0);

        search.next_match();
        let m = search.current_match().unwrap();
        assert_eq!(m.start, 8);

        search.prev_match();
        let m = search.current_match().unwrap();
        assert_eq!(m.start, 0);
    }

    #[test]
    fn test_search_chinese() {
        let mut search = SearchState::new();
        let lines = vec!["你好世界，你好宇宙".to_string()];

        let count = search.search("你好", &lines);
        assert_eq!(count, 2);

        let m = search.current_match().unwrap();
        assert_eq!(m.start, 0);
        assert_eq!(m.end, 2);

        search.next_match();
        let m = search.current_match().unwrap();
        assert_eq!(m.start, 5);
        assert_eq!(m.end, 7);
    }

    #[test]
    fn test_highlight_line_with_offset() {
        let mut search = SearchState::new();
        // 逻辑行 "hello world hello" 被折行成两个视觉行
        let lines = vec!["hello world hello".to_string()];
        search.search("hello", &lines);
        assert_eq!(search.match_count(), 2);

        let theme = test_theme();

        // 模拟第二个视觉行片段 "d hello"，char_offset = 10
        let spans = search.highlight_line(0, "d hello", &theme, 10);
        // 应该高亮 "hello" (local 2..7)
        assert!(spans.len() >= 2);
    }

    #[test]
    fn test_highlight_chinese_no_panic() {
        let mut search = SearchState::new();
        let lines = vec!["测试中文搜索功能".to_string()];
        search.search("中文", &lines);

        let theme = test_theme();
        // 传入完整行，不应 panic
        let spans = search.highlight_line(0, "测试中文搜索功能", &theme, 0);
        assert!(!spans.is_empty());

        // 传入片段（模拟折行），也不应 panic
        let spans = search.highlight_line(0, "搜索功能", &theme, 4);
        assert!(!spans.is_empty());
    }
}
