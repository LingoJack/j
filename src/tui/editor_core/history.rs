//! 撤销/重做历史管理
//!
//! 实现简单的撤销/重做栈。

use std::collections::VecDeque;

/// 编辑器历史记录最大条数
const EDITOR_HISTORY_MAX_SIZE: usize = 100;

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
        Self {
            lines,
            cursor: (0, 0),
        }
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
    stack: VecDeque<Snapshot>,
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
            stack: VecDeque::new(),
            cursor: 0,
            max_size: EDITOR_HISTORY_MAX_SIZE,
        }
    }

    /// 推入新快照
    pub fn push(&mut self, snapshot: Snapshot) {
        // 如果当前不在栈顶，丢弃之后的历史
        if self.cursor < self.stack.len() {
            self.stack.truncate(self.cursor);
        }

        // 推入新快照
        self.stack.push_back(snapshot);
        self.cursor = self.stack.len();

        // 如果超出最大容量，移除最旧的记录（O(1)）
        if self.stack.len() > self.max_size {
            self.stack.pop_front();
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
}
