//! 搜索功能
//!
//! 实现文本搜索和高亮。

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

#[cfg(test)]
use super::theme::BorderStyle;
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

    /// 清除搜索状态
    pub fn clear(&mut self) {
        self.pattern.clear();
        self.matches.clear();
        self.line_index.clear();
        self.current_index = 0;
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
mod tests;
