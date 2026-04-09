//! 文本缓冲区
//!
//! 独立于任何 UI 库的文本存储和编辑操作。
//! 所有编辑操作使用 `char_indices` 定位字节偏移，避免 `Vec<char>` 分配。

/// 光标位置 (行号, 列号)
pub type Cursor = (usize, usize);

/// 文本缓冲区
#[derive(Debug, Clone)]
pub struct TextBuffer {
    /// 文本行
    lines: Vec<String>,
    /// 光标位置 (行, 列)
    cursor: Cursor,
    /// 是否已修改
    modified: bool,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBuffer {
    /// 创建空的文本缓冲区
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: (0, 0),
            modified: false,
        }
    }

    /// 从文本内容创建缓冲区
    pub fn from_content(content: &str) -> Self {
        let lines = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(|l| l.to_string()).collect()
        };

        Self {
            lines,
            cursor: (0, 0),
            modified: false,
        }
    }

    /// 获取所有行
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// 获取指定行
    pub fn line(&self, row: usize) -> Option<&String> {
        self.lines.get(row)
    }

    /// 获取行数
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// 获取光标位置
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// 设置光标位置
    pub fn set_cursor(&mut self, row: usize, col: usize) {
        let row = row.min(self.lines.len().saturating_sub(1));
        let col = if row < self.lines.len() {
            col.min(self.lines[row].chars().count())
        } else {
            0
        };
        self.cursor = (row, col);
    }

    /// 获取当前行的字符数
    pub fn current_line_len(&self) -> usize {
        self.lines
            .get(self.cursor.0)
            .map(|l| l.chars().count())
            .unwrap_or(0)
    }

    // ========== 辅助方法 ==========

    /// 获取指定行中第 col 个字符的字节偏移（UTF-8 安全）
    ///
    /// 如果 col 超出行尾，返回行长度。
    fn byte_offset_of(line: &str, col: usize) -> usize {
        line.char_indices()
            .nth(col)
            .map(|(i, _)| i)
            .unwrap_or(line.len())
    }

    // ========== 光标移动 ==========

    /// 移动光标到行首
    pub fn move_cursor_head(&mut self) {
        self.cursor.1 = 0;
    }

    /// 移动光标到行尾
    pub fn move_cursor_end(&mut self) {
        self.cursor.1 = self.current_line_len();
    }

    /// 向左移动光标
    pub fn move_cursor_back(&mut self) {
        if self.cursor.1 > 0 {
            self.cursor.1 -= 1;
        } else if self.cursor.0 > 0 {
            // 移动到上一行末尾
            self.cursor.0 -= 1;
            self.cursor.1 = self.current_line_len();
        }
    }

    /// 向右移动光标
    pub fn move_cursor_forward(&mut self) {
        let line_len = self.current_line_len();
        if self.cursor.1 < line_len {
            self.cursor.1 += 1;
        } else if self.cursor.0 < self.lines.len() - 1 {
            // 移动到下一行开头
            self.cursor.0 += 1;
            self.cursor.1 = 0;
        }
    }

    /// 向上移动光标
    pub fn move_cursor_up(&mut self) {
        if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
            // 确保列位置不超出新行的长度
            let new_line_len = self.current_line_len();
            self.cursor.1 = self.cursor.1.min(new_line_len);
        }
    }

    /// 向下移动光标
    pub fn move_cursor_down(&mut self) {
        if self.cursor.0 < self.lines.len() - 1 {
            self.cursor.0 += 1;
            // 确保列位置不超出新行的长度
            let new_line_len = self.current_line_len();
            self.cursor.1 = self.cursor.1.min(new_line_len);
        }
    }

    /// 移动光标到文件开头
    pub fn move_cursor_top(&mut self) {
        self.cursor = (0, 0);
    }

    /// 移动光标到文件末尾
    pub fn move_cursor_bottom(&mut self) {
        self.cursor.0 = self.lines.len().saturating_sub(1);
        self.cursor.1 = self.current_line_len();
    }

    /// 移动光标到单词开头（向前）
    pub fn move_cursor_word_forward(&mut self) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get(row) {
            let total = line.chars().count();
            let mut new_col = col;

            // 跳过当前单词的非空白字符
            for (i, ch) in line.chars().enumerate().skip(col) {
                if ch.is_whitespace() {
                    new_col = i;
                    break;
                }
                new_col = i + 1;
            }

            // 跳过空白
            for (i, ch) in line.chars().enumerate().skip(new_col) {
                if !ch.is_whitespace() {
                    new_col = i;
                    break;
                }
                new_col = i + 1;
            }

            if new_col < total {
                self.cursor.1 = new_col;
            } else if row < self.lines.len() - 1 {
                // 移动到下一行
                self.cursor.0 += 1;
                self.cursor.1 = 0;
                // 如果下一行是空白开头，继续查找
                if let Some(next_line) = self.lines.get(self.cursor.0) {
                    for (i, ch) in next_line.chars().enumerate() {
                        if !ch.is_whitespace() {
                            self.cursor.1 = i;
                            break;
                        }
                    }
                }
            } else {
                self.cursor.1 = total;
            }
        }
    }

    /// 移动光标到单词开头（向后）
    pub fn move_cursor_word_back(&mut self) {
        let (row, col) = self.cursor;
        if col == 0 {
            if row > 0 {
                // 移动到上一行
                self.cursor.0 -= 1;
                self.cursor.1 = self
                    .lines
                    .get(self.cursor.0)
                    .map(|l| l.chars().count())
                    .unwrap_or(0);
            }
            return;
        }

        if let Some(line) = self.lines.get(row) {
            // 需要逆序遍历，使用 Vec<char> 是合理的（光标移动不是高频热路径）
            let chars: Vec<char> = line.chars().collect();
            let mut col = col;

            // 如果在空白处，先跳过空白
            while col > 0
                && chars
                    .get(col - 1)
                    .map(|c| c.is_whitespace())
                    .unwrap_or(false)
            {
                col -= 1;
            }
            // 跳过单词字符
            while col > 0
                && chars
                    .get(col - 1)
                    .map(|c| !c.is_whitespace())
                    .unwrap_or(false)
            {
                col -= 1;
            }

            self.cursor.1 = col;
        }
    }

    /// 移动光标到单词末尾
    pub fn move_cursor_word_end(&mut self) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get(row) {
            let total = line.chars().count();
            let mut col = col;

            // 如果在单词内，先移动到单词末尾
            let mut chars = line.chars().enumerate().skip(col);
            if let Some((_, ch)) = chars.next()
                && !ch.is_whitespace()
            {
                col += 1;
            }

            // 跳过空白
            for (i, ch) in line.chars().enumerate().skip(col) {
                if !ch.is_whitespace() {
                    break;
                }
                col = i + 1;
            }

            // 移动到单词末尾
            for (i, ch) in line.chars().enumerate().skip(col) {
                if ch.is_whitespace() {
                    break;
                }
                col = i + 1;
            }

            col = col.saturating_sub(1);

            self.cursor.1 = col.min(total);
        }
    }

    // ========== 文本编辑 ==========

    /// 在当前光标位置插入字符
    pub fn insert_char(&mut self, ch: char) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get_mut(row) {
            let byte_offset = Self::byte_offset_of(line, col);
            line.insert(byte_offset, ch);
            self.cursor.1 = col + 1;
            self.modified = true;
        }
    }

    /// 在当前光标位置插入字符串
    pub fn insert_str(&mut self, s: &str) {
        if !s.contains('\n') {
            let (row, col) = self.cursor;
            if let Some(line) = self.lines.get_mut(row) {
                let byte_offset = Self::byte_offset_of(line, col);
                line.insert_str(byte_offset, s);
                self.cursor.1 = col + s.chars().count();
                self.modified = true;
            }
        } else {
            for ch in s.chars() {
                if ch == '\n' {
                    self.insert_newline();
                } else {
                    self.insert_char(ch);
                }
            }
        }
    }

    /// 在当前光标位置插入换行
    pub fn insert_newline(&mut self) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get(row) {
            let byte_offset = Self::byte_offset_of(line, col);
            let before = line[..byte_offset].to_string();
            let after = line[byte_offset..].to_string();

            self.lines[row] = before;
            self.lines.insert(row + 1, after);
            self.cursor = (row + 1, 0);
            self.modified = true;
        }
    }

    /// 删除光标位置的字符
    pub fn delete_char(&mut self) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get_mut(row) {
            let chars_count = line.chars().count();
            if col < chars_count {
                let byte_start = Self::byte_offset_of(line, col);
                let byte_end = Self::byte_offset_of(line, col + 1);
                line.drain(byte_start..byte_end);
                self.modified = true;
            } else if row < self.lines.len() - 1 {
                // 合并下一行
                let next_line = self.lines.remove(row + 1);
                self.lines[row].push_str(&next_line);
                self.modified = true;
            }
        }
    }

    /// 删除光标前的字符（退格）
    pub fn backspace(&mut self) {
        if self.cursor.1 > 0 {
            self.cursor.1 -= 1;
            self.delete_char();
        } else if self.cursor.0 > 0 {
            // 合并到上一行
            let current_line = self.lines.remove(self.cursor.0);
            self.cursor.0 -= 1;
            let prev_line_len = self.lines[self.cursor.0].chars().count();
            self.lines[self.cursor.0].push_str(&current_line);
            self.cursor.1 = prev_line_len;
            self.modified = true;
        }
    }

    /// 删除当前行
    pub fn delete_line(&mut self) {
        if self.lines.len() > 1 {
            self.lines.remove(self.cursor.0);
            if self.cursor.0 >= self.lines.len() {
                self.cursor.0 = self.lines.len() - 1;
            }
            self.cursor.1 = self.cursor.1.min(self.current_line_len());
        } else {
            self.lines[0].clear();
            self.cursor.1 = 0;
        }
        self.modified = true;
    }

    /// 删除从光标到行尾的内容
    pub fn delete_line_by_end(&mut self) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get_mut(row) {
            let byte_offset = Self::byte_offset_of(line, col);
            line.truncate(byte_offset);
            self.modified = true;
        }
    }

    /// 删除当前单词
    pub fn delete_word(&mut self) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get(row) {
            let mut end = col;

            // 跳过空白
            for ch in line.chars().skip(col) {
                if !ch.is_whitespace() {
                    break;
                }
                end += 1;
            }
            // 跳过单词字符
            for ch in line.chars().skip(end) {
                if ch.is_whitespace() {
                    break;
                }
                end += 1;
            }

            if end > col
                && let Some(line) = self.lines.get_mut(row)
            {
                let byte_start = Self::byte_offset_of(line, col);
                let byte_end = Self::byte_offset_of(line, end);
                line.drain(byte_start..byte_end);
                self.modified = true;
            }
        }
    }

    /// 在当前行下方插入新行
    pub fn insert_line_below(&mut self) {
        let row = self.cursor.0;
        self.lines.insert(row + 1, String::new());
        self.cursor = (row + 1, 0);
        self.modified = true;
    }

    /// 在当前行上方插入新行
    pub fn insert_line_above(&mut self) {
        let row = self.cursor.0;
        self.lines.insert(row, String::new());
        self.cursor = (row, 0);
        self.modified = true;
    }

    // ========== 批量操作 ==========

    /// 替换所有行（用于撤销/重做）
    pub fn replace_lines(&mut self, lines: Vec<String>) {
        self.lines = lines;
        // 确保至少有一行
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        // 确保光标位置有效
        self.cursor.0 = self.cursor.0.min(self.lines.len() - 1);
        self.cursor.1 = self.cursor.1.min(self.current_line_len());
        self.modified = true;
    }

    /// 获取快照（用于撤销）
    pub fn snapshot(&self) -> Vec<String> {
        self.lines.clone()
    }
}

impl std::fmt::Display for TextBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_insert() {
        let mut buf = TextBuffer::new();
        buf.insert_char('H');
        buf.insert_char('i');
        assert_eq!(buf.to_string(), "Hi");
        assert_eq!(buf.cursor(), (0, 2));
    }

    #[test]
    fn test_newline() {
        let mut buf = TextBuffer::new();
        buf.insert_str("Hello\nWorld");
        assert_eq!(buf.lines().len(), 2);
        assert_eq!(buf.lines()[0], "Hello");
        assert_eq!(buf.lines()[1], "World");
    }

    #[test]
    fn test_cursor_movement() {
        let mut buf = TextBuffer::from_content("Hello\nWorld");
        buf.move_cursor_end();
        assert_eq!(buf.cursor(), (0, 5));
        buf.move_cursor_down();
        assert_eq!(buf.cursor(), (1, 5));
        buf.move_cursor_head();
        assert_eq!(buf.cursor(), (1, 0));
    }

    #[test]
    fn test_delete() {
        let mut buf = TextBuffer::from_content("Hello");
        buf.set_cursor(0, 1);
        buf.delete_char();
        assert_eq!(buf.to_string(), "Hllo");
    }

    #[test]
    fn test_word_movement() {
        let mut buf = TextBuffer::from_content("hello world test");
        buf.move_cursor_word_forward();
        assert_eq!(buf.cursor(), (0, 6));
        buf.move_cursor_word_forward();
        assert_eq!(buf.cursor(), (0, 12));
        buf.move_cursor_word_back();
        assert_eq!(buf.cursor(), (0, 6));
    }

    #[test]
    fn test_chinese_insert() {
        let mut buf = TextBuffer::new();
        buf.insert_char('你');
        buf.insert_char('好');
        buf.insert_char('世');
        buf.insert_char('界');
        assert_eq!(buf.to_string(), "你好世界");
        assert_eq!(buf.cursor(), (0, 4));
    }

    #[test]
    fn test_chinese_delete() {
        let mut buf = TextBuffer::from_content("你好世界");
        buf.set_cursor(0, 2);
        buf.delete_char();
        assert_eq!(buf.to_string(), "你好界");
        assert_eq!(buf.cursor(), (0, 2));

        buf.backspace();
        assert_eq!(buf.to_string(), "你界");
        assert_eq!(buf.cursor(), (0, 1));
    }

    #[test]
    fn test_chinese_insert_mid() {
        let mut buf = TextBuffer::from_content("你好世界");
        buf.set_cursor(0, 2);
        buf.insert_char('的');
        assert_eq!(buf.to_string(), "你好的世界");
    }

    #[test]
    fn test_chinese_newline() {
        let mut buf = TextBuffer::from_content("你好世界");
        buf.set_cursor(0, 2);
        buf.insert_newline();
        assert_eq!(buf.lines().len(), 2);
        assert_eq!(buf.lines()[0], "你好");
        assert_eq!(buf.lines()[1], "世界");
        assert_eq!(buf.cursor(), (1, 0));
    }

    #[test]
    fn test_chinese_delete_line_by_end() {
        let mut buf = TextBuffer::from_content("你好世界");
        buf.set_cursor(0, 2);
        buf.delete_line_by_end();
        assert_eq!(buf.to_string(), "你好");
    }

    #[test]
    fn test_chinese_delete_word() {
        let mut buf = TextBuffer::from_content("你好 世界 测试");
        buf.set_cursor(0, 0);
        buf.delete_word();
        assert_eq!(buf.to_string(), " 世界 测试");
    }

    #[test]
    fn test_chinese_word_movement() {
        let mut buf = TextBuffer::from_content("你好 世界 测试");
        buf.move_cursor_word_forward();
        assert_eq!(buf.cursor(), (0, 3));
        buf.move_cursor_word_forward();
        assert_eq!(buf.cursor(), (0, 6));
        buf.move_cursor_word_back();
        assert_eq!(buf.cursor(), (0, 3));
    }
}
