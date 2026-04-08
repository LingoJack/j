//! 搜索功能
//!
//! 实现文本搜索和高亮。

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use crate::command::chat::theme::Theme;

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
    pub pattern: String,
    /// 所有匹配
    matches: Vec<SearchMatch>,
    /// 当前匹配索引
    current_index: usize,
}

impl SearchState {
    /// 创建新的搜索状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 执行搜索
    pub fn search(&mut self, pattern: &str, lines: &[String]) -> usize {
        self.pattern = pattern.to_string();
        self.matches.clear();
        self.current_index = 0;

        if pattern.is_empty() {
            return 0;
        }

        for (line_idx, line) in lines.iter().enumerate() {
            let mut start = 0;
            while let Some(pos) = line[start..].find(pattern) {
                let abs_start = start + pos;
                self.matches.push(SearchMatch {
                    line: line_idx,
                    start: abs_start,
                    end: abs_start + pattern.len(),
                });
                start = abs_start + pattern.len();
                if start >= line.len() {
                    break;
                }
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

    /// 当前匹配索引（1-based）
    pub fn current_index_display(&self) -> usize {
        if self.matches.is_empty() {
            0
        } else {
            self.current_index + 1
        }
    }

    /// 高亮行中的搜索匹配
    pub fn highlight_line(&self, line_idx: usize, line: &str, theme: &Theme) -> Vec<Span<'static>> {
        let line_matches: Vec<_> = self.matches.iter().filter(|m| m.line == line_idx).collect();

        if line_matches.is_empty() || self.pattern.is_empty() {
            return vec![Span::styled(
                line.to_string(),
                Style::default().fg(theme.text_normal),
            )];
        }

        let mut spans = Vec::new();
        let mut last_end = 0;
        let chars: Vec<char> = line.chars().collect();

        for m in line_matches {
            if m.start > last_end {
                let text: String = chars[last_end..m.start].iter().collect();
                spans.push(Span::styled(text, Style::default().fg(theme.text_normal)));
            }
            let match_text: String = chars[m.start..m.end].iter().collect();
            spans.push(Span::styled(
                match_text,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
            last_end = m.end;
        }

        if last_end < chars.len() {
            let text: String = chars[last_end..].iter().collect();
            spans.push(Span::styled(text, Style::default().fg(theme.text_normal)));
        }

        spans
    }

    /// 清空搜索
    pub fn clear(&mut self) {
        self.pattern.clear();
        self.matches.clear();
        self.current_index = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search() {
        let mut search = SearchState::new();
        let lines = vec![
            "hello world".to_string(),
            "hello universe".to_string(),
        ];

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
}
