//! 文本缓冲区
//!
//! 独立于任何 UI 库的文本存储和编辑操作。

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
        let line = self.lines.get(self.cursor.0).cloned().unwrap_or_default();
        let chars: Vec<char> = line.chars().collect();
        let mut col = self.cursor.1;

        // 跳过当前单词的非空白字符
        while col < chars.len() && !chars[col].is_whitespace() {
            col += 1;
        }
        // 跳过空白
        while col < chars.len() && chars[col].is_whitespace() {
            col += 1;
        }

        if col < chars.len() {
            self.cursor.1 = col;
        } else if self.cursor.0 < self.lines.len() - 1 {
            // 移动到下一行
            self.cursor.0 += 1;
            self.cursor.1 = 0;
            // 如果下一行是空白开头，继续查找
            let next_line = self.lines.get(self.cursor.0).cloned().unwrap_or_default();
            let next_chars: Vec<char> = next_line.chars().collect();
            while self.cursor.1 < next_chars.len() && next_chars[self.cursor.1].is_whitespace() {
                self.cursor.1 += 1;
            }
        } else {
            self.cursor.1 = chars.len();
        }
    }

    /// 移动光标到单词开头（向后）
    pub fn move_cursor_word_back(&mut self) {
        let line = self.lines.get(self.cursor.0).cloned().unwrap_or_default();
        let chars: Vec<char> = line.chars().collect();

        if self.cursor.1 == 0 {
            if self.cursor.0 > 0 {
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

        let mut col = self.cursor.1;

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

    /// 移动光标到单词末尾
    pub fn move_cursor_word_end(&mut self) {
        let line = self.lines.get(self.cursor.0).cloned().unwrap_or_default();
        let chars: Vec<char> = line.chars().collect();
        let mut col = self.cursor.1;

        // 如果在单词内，先移动到单词末尾
        if col < chars.len() && !chars[col].is_whitespace() {
            col += 1;
        }
        // 跳过空白
        while col < chars.len() && chars[col].is_whitespace() {
            col += 1;
        }
        // 移动到单词末尾
        while col < chars.len() && !chars[col].is_whitespace() {
            col += 1;
        }
        if col > 0 && col <= chars.len() {
            col -= 1;
        }

        self.cursor.1 = col;
    }

    // ========== 文本编辑 ==========

    /// 在当前光标位置插入字符
    pub fn insert_char(&mut self, ch: char) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get_mut(row) {
            let chars: Vec<char> = line.chars().collect();
            let mut new_chars = Vec::with_capacity(chars.len() + 1);
            new_chars.extend(chars.iter().take(col));
            new_chars.push(ch);
            new_chars.extend(chars.iter().skip(col));
            *line = new_chars.into_iter().collect();
            self.cursor.1 = col + 1;
            self.modified = true;
        }
    }

    /// 在当前光标位置插入字符串
    pub fn insert_str(&mut self, s: &str) {
        for ch in s.chars() {
            if ch == '\n' {
                self.insert_newline();
            } else {
                self.insert_char(ch);
            }
        }
    }

    /// 在当前光标位置插入换行
    pub fn insert_newline(&mut self) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get(row) {
            let chars: Vec<char> = line.chars().collect();
            let before: String = chars.iter().take(col).collect();
            let after: String = chars.iter().skip(col).collect();

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
            let chars: Vec<char> = line.chars().collect();
            if col < chars.len() {
                let mut new_chars: Vec<char> = Vec::with_capacity(chars.len() - 1);
                new_chars.extend(chars.iter().take(col));
                new_chars.extend(chars.iter().skip(col + 1));
                *line = new_chars.into_iter().collect();
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
            let chars: Vec<char> = line.chars().collect();
            *line = chars.iter().take(col).collect();
            self.modified = true;
        }
    }

    /// 删除当前单词
    pub fn delete_word(&mut self) {
        let (row, col) = self.cursor;
        if let Some(line) = self.lines.get(row) {
            let chars: Vec<char> = line.chars().collect();
            let mut end = col;

            // 跳过空白
            while end < chars.len() && chars[end].is_whitespace() {
                end += 1;
            }
            // 跳过单词字符
            while end < chars.len() && !chars[end].is_whitespace() {
                end += 1;
            }

            if end > col {
                let mut new_chars: Vec<char> = chars.iter().take(col).copied().collect();
                new_chars.extend(chars.iter().skip(end).copied());
                self.lines[row] = new_chars.into_iter().collect();
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
}
