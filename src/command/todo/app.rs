use crate::config::YamlConfig;
use crate::error;
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ========== 数据结构 ==========

/// 单条待办事项
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TodoItem {
    /// 待办内容
    pub content: String,
    /// 是否已完成
    pub done: bool,
    /// 创建时间
    pub created_at: String,
    /// 完成时间（可选）
    pub done_at: Option<String>,
}

/// 待办列表（序列化到 JSON）
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
}

// ========== 文件路径 ==========

/// 获取 todo 数据目录: ~/.jdata/todo/
pub fn todo_dir() -> PathBuf {
    let dir = YamlConfig::data_dir().join("todo");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 获取 todo 数据文件路径: ~/.jdata/todo/todo.json
pub fn todo_file_path() -> PathBuf {
    todo_dir().join("todo.json")
}

// ========== 数据读写 ==========

/// 从文件加载待办列表
pub fn load_todo_list() -> TodoList {
    let path = todo_file_path();
    if !path.exists() {
        return TodoList::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            error!("❌ 解析 todo.json 失败: {}", e);
            TodoList::default()
        }),
        Err(e) => {
            error!("❌ 读取 todo.json 失败: {}", e);
            TodoList::default()
        }
    }
}

/// 保存待办列表到文件
pub fn save_todo_list(list: &TodoList) -> bool {
    let path = todo_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(list) {
        Ok(json) => match fs::write(&path, json) {
            Ok(_) => true,
            Err(e) => {
                error!("❌ 保存 todo.json 失败: {}", e);
                false
            }
        },
        Err(e) => {
            error!("❌ 序列化 todo 列表失败: {}", e);
            false
        }
    }
}

// ========== TUI 应用状态 ==========

/// TUI 应用状态
pub struct TodoApp {
    /// 待办列表数据
    pub list: TodoList,
    /// 加载时的快照（用于对比是否真正有修改）
    pub snapshot: TodoList,
    /// 列表选中状态
    pub state: ListState,
    /// 当前模式
    pub mode: AppMode,
    /// 输入缓冲区（添加/编辑模式使用）
    pub input: String,
    /// 编辑时记录的原始索引
    pub edit_index: Option<usize>,
    /// 状态栏消息
    pub message: Option<String>,
    /// 过滤模式: 0=全部, 1=未完成, 2=已完成
    pub filter: usize,
    /// 强制退出输入缓冲（用于 q! 退出）
    pub quit_input: String,
    /// 输入模式下的光标位置（字符索引）
    pub cursor_pos: usize,
    /// 预览区滚动偏移
    pub preview_scroll: u16,
}

#[derive(PartialEq)]
pub enum AppMode {
    /// 正常浏览模式
    Normal,
    /// 输入添加模式
    Adding,
    /// 编辑模式
    Editing,
    /// 确认删除
    ConfirmDelete,
    /// 显示帮助
    Help,
}

impl TodoApp {
    pub fn new() -> Self {
        let list = load_todo_list();
        let snapshot = list.clone();
        let mut state = ListState::default();
        if !list.items.is_empty() {
            state.select(Some(0));
        }
        Self {
            list,
            snapshot,
            state,
            mode: AppMode::Normal,
            input: String::new(),
            edit_index: None,
            message: None,
            filter: 0,
            quit_input: String::new(),
            cursor_pos: 0,
            preview_scroll: 0,
        }
    }

    /// 通过对比快照判断是否有未保存的修改
    pub fn is_dirty(&self) -> bool {
        self.list != self.snapshot
    }

    /// 获取当前过滤后的索引列表（映射到 list.items 的真实索引）
    pub fn filtered_indices(&self) -> Vec<usize> {
        self.list
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| match self.filter {
                1 => !item.done,
                2 => item.done,
                _ => true,
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// 获取当前选中项在原始列表中的真实索引
    pub fn selected_real_index(&self) -> Option<usize> {
        let indices = self.filtered_indices();
        self.state
            .selected()
            .and_then(|sel| indices.get(sel).copied())
    }

    /// 向下移动
    pub fn move_down(&mut self) {
        let count = self.filtered_indices().len();
        if count == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// 向上移动
    pub fn move_up(&mut self) {
        let count = self.filtered_indices().len();
        if count == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// 切换当前选中项的完成状态
    pub fn toggle_done(&mut self) {
        if let Some(real_idx) = self.selected_real_index() {
            let item = &mut self.list.items[real_idx];
            item.done = !item.done;
            if item.done {
                item.done_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                self.message = Some("✅ 已标记为完成".to_string());
            } else {
                item.done_at = None;
                self.message = Some("⬜ 已标记为未完成".to_string());
            }
        }
    }

    /// 添加新待办
    pub fn add_item(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            self.message = Some("⚠️ 内容为空，已取消".to_string());
            self.mode = AppMode::Normal;
            self.input.clear();
            return;
        }
        self.list.items.push(TodoItem {
            content: text,
            done: false,
            created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            done_at: None,
        });
        self.input.clear();
        self.mode = AppMode::Normal;
        let count = self.filtered_indices().len();
        if count > 0 {
            self.state.select(Some(count - 1));
        }
        self.message = Some("✅ 已添加新待办".to_string());
    }

    /// 确认编辑
    pub fn confirm_edit(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            self.message = Some("⚠️ 内容为空，已取消编辑".to_string());
            self.mode = AppMode::Normal;
            self.input.clear();
            self.edit_index = None;
            return;
        }
        if let Some(idx) = self.edit_index {
            if idx < self.list.items.len() {
                self.list.items[idx].content = text;
                self.message = Some("✅ 已更新待办内容".to_string());
            }
        }
        self.input.clear();
        self.edit_index = None;
        self.mode = AppMode::Normal;
    }

    /// 删除当前选中项
    pub fn delete_selected(&mut self) {
        if let Some(real_idx) = self.selected_real_index() {
            let removed = self.list.items.remove(real_idx);
            self.message = Some(format!("🗑️ 已删除: {}", removed.content));
            let count = self.filtered_indices().len();
            if count == 0 {
                self.state.select(None);
            } else if let Some(sel) = self.state.selected() {
                if sel >= count {
                    self.state.select(Some(count - 1));
                }
            }
        }
        self.mode = AppMode::Normal;
    }

    /// 移动选中项向上（调整顺序）
    pub fn move_item_up(&mut self) {
        if let Some(real_idx) = self.selected_real_index() {
            if real_idx > 0 {
                self.list.items.swap(real_idx, real_idx - 1);
                self.move_up();
            }
        }
    }

    /// 移动选中项向下（调整顺序）
    pub fn move_item_down(&mut self) {
        if let Some(real_idx) = self.selected_real_index() {
            if real_idx < self.list.items.len() - 1 {
                self.list.items.swap(real_idx, real_idx + 1);
                self.move_down();
            }
        }
    }

    /// 切换过滤模式
    pub fn toggle_filter(&mut self) {
        self.filter = (self.filter + 1) % 3;
        let count = self.filtered_indices().len();
        if count > 0 {
            self.state.select(Some(0));
        } else {
            self.state.select(None);
        }
        let label = match self.filter {
            1 => "未完成",
            2 => "已完成",
            _ => "全部",
        };
        self.message = Some(format!("🔍 过滤: {}", label));
    }

    /// 保存数据
    pub fn save(&mut self) {
        if self.is_dirty() {
            if save_todo_list(&self.list) {
                self.snapshot = self.list.clone();
                self.message = Some("💾 已保存".to_string());
            }
        } else {
            self.message = Some("📋 无需保存，没有修改".to_string());
        }
    }
}

// ========== 按键处理 ==========

/// 正常模式按键处理，返回 true 表示退出
pub fn handle_normal_mode(app: &mut TodoApp, key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }

    match key.code {
        KeyCode::Char('q') => {
            if app.is_dirty() {
                app.message = Some(
                    "⚠️ 有未保存的修改！请先 s 保存，或输入 q! 强制退出（丢弃修改）".to_string(),
                );
                app.quit_input = "q".to_string();
                return false;
            }
            return true;
        }
        KeyCode::Esc => {
            if app.is_dirty() {
                app.message = Some(
                    "⚠️ 有未保存的修改！请先 s 保存，或输入 q! 强制退出（丢弃修改）".to_string(),
                );
                return false;
            }
            return true;
        }
        KeyCode::Char('!') => {
            if app.quit_input == "q" {
                return true;
            }
            app.quit_input.clear();
        }
        KeyCode::Char('n') | KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Char('N') | KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Char(' ') | KeyCode::Enter => app.toggle_done(),
        KeyCode::Char('a') => {
            app.mode = AppMode::Adding;
            app.input.clear();
            app.cursor_pos = 0;
            app.message = None;
        }
        KeyCode::Char('e') => {
            if let Some(real_idx) = app.selected_real_index() {
                app.input = app.list.items[real_idx].content.clone();
                app.cursor_pos = app.input.chars().count();
                app.edit_index = Some(real_idx);
                app.mode = AppMode::Editing;
                app.message = None;
            }
        }
        KeyCode::Char('y') => {
            if let Some(real_idx) = app.selected_real_index() {
                let content = app.list.items[real_idx].content.clone();
                if copy_to_clipboard(&content) {
                    app.message = Some(format!("📋 已复制到剪切板: {}", content));
                } else {
                    app.message = Some("❌ 复制到剪切板失败".to_string());
                }
            }
        }
        KeyCode::Char('d') => {
            if app.selected_real_index().is_some() {
                app.mode = AppMode::ConfirmDelete;
            }
        }
        KeyCode::Char('f') => app.toggle_filter(),
        KeyCode::Char('s') => app.save(),
        KeyCode::Char('K') => app.move_item_up(),
        KeyCode::Char('J') => app.move_item_down(),
        KeyCode::Char('?') => {
            app.mode = AppMode::Help;
        }
        _ => {}
    }

    if key.code != KeyCode::Char('q') && key.code != KeyCode::Char('!') {
        app.quit_input.clear();
    }

    false
}

/// 输入模式按键处理（添加/编辑通用，支持光标移动和行内编辑）
pub fn handle_input_mode(app: &mut TodoApp, key: KeyEvent) {
    let char_count = app.input.chars().count();

    match key.code {
        KeyCode::Enter => {
            if app.mode == AppMode::Adding {
                app.add_item();
            } else {
                app.confirm_edit();
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.input.clear();
            app.cursor_pos = 0;
            app.edit_index = None;
            app.message = Some("已取消".to_string());
        }
        KeyCode::Left => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
            }
        }
        KeyCode::Right => {
            if app.cursor_pos < char_count {
                app.cursor_pos += 1;
            }
        }
        KeyCode::Home => {
            app.cursor_pos = 0;
        }
        KeyCode::End => {
            app.cursor_pos = char_count;
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                let start = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let end = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
                app.input.drain(start..end);
                app.cursor_pos -= 1;
            }
        }
        KeyCode::Delete => {
            if app.cursor_pos < char_count {
                let start = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
                let end = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos + 1)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
                app.input.drain(start..end);
            }
        }
        KeyCode::Char(c) => {
            let byte_idx = app
                .input
                .char_indices()
                .nth(app.cursor_pos)
                .map(|(i, _)| i)
                .unwrap_or(app.input.len());
            app.input.insert_str(byte_idx, &c.to_string());
            app.cursor_pos += 1;
        }
        _ => {}
    }
}

/// 确认删除按键处理
pub fn handle_confirm_delete(app: &mut TodoApp, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.delete_selected();
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.message = Some("已取消删除".to_string());
        }
        _ => {}
    }
}

/// 帮助模式按键处理（按任意键返回）
pub fn handle_help_mode(app: &mut TodoApp, _key: KeyEvent) {
    app.mode = AppMode::Normal;
    app.message = None;
}

// ========== 工具函数 ==========

/// 将输入字符串按光标位置分割为三部分：光标前、光标处字符、光标后
pub fn split_input_at_cursor(input: &str, cursor_pos: usize) -> (String, String, String) {
    let chars: Vec<char> = input.chars().collect();
    let before: String = chars[..cursor_pos].iter().collect();
    let cursor_ch = if cursor_pos < chars.len() {
        chars[cursor_pos].to_string()
    } else {
        " ".to_string()
    };
    let after: String = if cursor_pos < chars.len() {
        chars[cursor_pos + 1..].iter().collect()
    } else {
        String::new()
    };
    (before, cursor_ch, after)
}

/// 计算字符串的显示宽度（中文/全角字符占 2 列，ASCII 占 1 列）
pub fn display_width(s: &str) -> usize {
    s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum()
}

/// 计算字符串在指定列宽下换行后的行数
pub fn count_wrapped_lines(s: &str, col_width: usize) -> usize {
    if col_width == 0 || s.is_empty() {
        return 1;
    }
    let mut lines = 1usize;
    let mut current_width = 0usize;
    for c in s.chars() {
        let char_width = if c.is_ascii() { 1 } else { 2 };
        if current_width + char_width > col_width {
            lines += 1;
            current_width = char_width;
        } else {
            current_width += char_width;
        }
    }
    lines
}

/// 计算光标在指定列宽下 wrap 后所在的行号（0-based）
pub fn cursor_wrapped_line(s: &str, cursor_pos: usize, col_width: usize) -> u16 {
    if col_width == 0 {
        return 0;
    }
    let mut line: u16 = 0;
    let mut current_width: usize = 0;
    for (i, c) in s.chars().enumerate() {
        if i == cursor_pos {
            return line;
        }
        let char_width = if c.is_ascii() { 1 } else { 2 };
        if current_width + char_width > col_width {
            line += 1;
            current_width = char_width;
        } else {
            current_width += char_width;
        }
    }
    // cursor_pos == chars.len() (cursor at end)
    line
}

/// 将字符串截断到指定的显示宽度，超出部分用 ".." 替代
pub fn truncate_to_width(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let total_width = display_width(s);
    if total_width <= max_width {
        return s.to_string();
    }
    let ellipsis = "..";
    let ellipsis_width = 2;
    let content_budget = max_width.saturating_sub(ellipsis_width);
    let mut width = 0;
    let mut result = String::new();
    for ch in s.chars() {
        let ch_width = if ch.is_ascii() { 1 } else { 2 };
        if width + ch_width > content_budget {
            break;
        }
        width += ch_width;
        result.push(ch);
    }
    result.push_str(ellipsis);
    result
}

/// 复制内容到系统剪切板（macOS 使用 pbcopy，Linux 使用 xclip）
pub fn copy_to_clipboard(content: &str) -> bool {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let (cmd, args): (&str, Vec<&str>) = if cfg!(target_os = "macos") {
        ("pbcopy", vec![])
    } else if cfg!(target_os = "linux") {
        if Command::new("which")
            .arg("xclip")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            ("xclip", vec!["-selection", "clipboard"])
        } else {
            ("xsel", vec!["--clipboard", "--input"])
        }
    } else {
        return false;
    };

    let child = Command::new(cmd).args(&args).stdin(Stdio::piped()).spawn();

    match child {
        Ok(mut child) => {
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(content.as_bytes());
            }
            child.wait().map(|s| s.success()).unwrap_or(false)
        }
        Err(_) => false,
    }
}
