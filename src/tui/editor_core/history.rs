//! 撤销/重做历史管理
//!
//! 实现简单的撤销/重做栈。

/// 历史记录快照
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// 文本行
    pub lines: Vec<String>,
    /// 光标位置 (行, 列)
    pub cursor: (usize, usize),
}

impl Snapshot {
    /// 创建新快照
    pub fn new(lines: Vec<String>) -> Self {
        Self { lines, cursor: (0, 0) }
    }

    /// 创建带光标位置的快照
    pub fn with_cursor(lines: Vec<String>, cursor: (usize, usize)) -> Self {
        Self { lines, cursor }
    }
}

/// 撤销/重做历史管理器
#[derive(Debug, Clone)]
pub struct History {
    /// 历史栈
    stack: Vec<Snapshot>,
    /// 当前位置指针
    cursor: usize,
    /// 最大历史记录数
    max_size: usize,
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl History {
    /// 创建新的历史管理器
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            cursor: 0,
            max_size: 100,
        }
    }

    /// 创建指定最大容量的历史管理器
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            stack: Vec::new(),
            cursor: 0,
            max_size,
        }
    }

    /// 历史记录数量
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// 是否可以撤销
    pub fn can_undo(&self) -> bool {
        self.cursor > 1
    }

    /// 是否可以重做
    pub fn can_redo(&self) -> bool {
        self.cursor < self.stack.len()
    }

    /// 推入新快照
    pub fn push(&mut self, snapshot: Snapshot) {
        // 如果当前不在栈顶，丢弃之后的历史
        if self.cursor < self.stack.len() {
            self.stack.truncate(self.cursor);
        }

        // 推入新快照
        self.stack.push(snapshot);
        self.cursor = self.stack.len();

        // 如果超出最大容量，移除最旧的记录
        if self.stack.len() > self.max_size {
            self.stack.remove(0);
            self.cursor = self.cursor.saturating_sub(1);
        }
    }

    /// 撤销
    /// 返回撤销后的快照，如果没有则返回 None
    pub fn undo(&mut self) -> Option<&Snapshot> {
        if self.cursor > 1 {
            self.cursor -= 1;
            self.stack.get(self.cursor - 1)
        } else {
            None
        }
    }

    /// 重做
    /// 返回重做后的快照，如果没有则返回 None
    pub fn redo(&mut self) -> Option<&Snapshot> {
        if self.cursor < self.stack.len() {
            self.cursor += 1;
            self.stack.get(self.cursor - 1)
        } else {
            None
        }
    }

    /// 获取当前快照
    pub fn current(&self) -> Option<&Snapshot> {
        if self.cursor > 0 {
            self.stack.get(self.cursor - 1)
        } else {
            None
        }
    }

    /// 获取上一个快照（用于对比）
    pub fn previous(&self) -> Option<&Snapshot> {
        if self.cursor > 1 {
            self.stack.get(self.cursor - 2)
        } else {
            None
        }
    }

    /// 清空历史
    pub fn clear(&mut self) {
        self.stack.clear();
        self.cursor = 0;
    }

    /// 设置最大容量
    pub fn set_max_size(&mut self, max_size: usize) {
        self.max_size = max_size;
        // 如果当前超出新容量，裁剪
        if self.stack.len() > self.max_size {
            let excess = self.stack.len() - self.max_size;
            self.stack.drain(0..excess);
            self.cursor = self.cursor.saturating_sub(excess);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(text: &str) -> Snapshot {
        Snapshot::with_cursor(
            text.lines().map(|l| l.to_string()).collect(),
            (0, 0),
        )
    }

    #[test]
    fn test_push_and_undo() {
        let mut history = History::new();
        
        history.push(make_snapshot("first"));
        history.push(make_snapshot("second"));
        history.push(make_snapshot("third"));
        
        assert_eq!(history.len(), 3);
        assert!(history.can_undo());
        
        let snap = history.undo().unwrap();
        assert_eq!(snap.lines[0], "second");
        
        let snap = history.undo().unwrap();
        assert_eq!(snap.lines[0], "first");
        
        assert!(!history.can_undo());
    }

    #[test]
    fn test_redo() {
        let mut history = History::new();
        
        history.push(make_snapshot("first"));
        history.push(make_snapshot("second"));
        
        history.undo();
        assert!(history.can_redo());
        
        let snap = history.redo().unwrap();
        assert_eq!(snap.lines[0], "second");
        
        assert!(!history.can_redo());
    }

    #[test]
    fn test_push_clears_redo() {
        let mut history = History::new();
        
        history.push(make_snapshot("first"));
        history.push(make_snapshot("second"));
        history.push(make_snapshot("third"));
        
        history.undo(); // -> second
        history.undo(); // -> first
        
        // 现在推入新快照，应该清除 third
        history.push(make_snapshot("new"));
        
        assert_eq!(history.len(), 2);
        assert!(!history.can_redo());
        
        let snap = history.undo().unwrap();
        assert_eq!(snap.lines[0], "first");
    }

    #[test]
    fn test_max_size() {
        let mut history = History::with_max_size(3);
        
        history.push(make_snapshot("1"));
        history.push(make_snapshot("2"));
        history.push(make_snapshot("3"));
        history.push(make_snapshot("4"));
        
        assert_eq!(history.len(), 3);
        
        // 应该丢弃 "1"
        let snap = history.current().unwrap();
        assert_eq!(snap.lines[0], "4");
    }

    #[test]
    fn test_clear() {
        let mut history = History::new();
        
        history.push(make_snapshot("first"));
        history.push(make_snapshot("second"));
        
        history.clear();
        
        assert!(history.is_empty());
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }
}
