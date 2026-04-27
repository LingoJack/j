// ========== TUI 应用状态 ==========

use super::io::{load_todo_list, save_todo_list};
use super::types::{AppMode, CMD_POPUP_ITEMS, TodoItem, TodoList};
use crate::command::chat::storage::load_agent_config;
use crate::constants::todo_filter;
use crate::theme::Theme;
use chrono::Local;
use ratatui::widgets::ListState;

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
    /// 待写入日报的内容（toggle done 后暂存）
    pub report_pending_content: Option<String>,
    /// 命令面板筛选文本
    pub cmd_popup_filter: String,
    /// 命令面板选中索引
    pub cmd_popup_selected: usize,
    /// 当前主题
    pub theme: Theme,
}

impl Default for TodoApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TodoApp {
    /// 创建 TodoApp 实例，加载待办列表和主题配置
    pub fn new() -> Self {
        let list = load_todo_list();
        let snapshot = list.clone();
        let mut state = ListState::default();
        if !list.items.is_empty() {
            state.select(Some(0));
        }
        let agent_config = load_agent_config();
        let theme = Theme::from_name(&agent_config.theme);
        Self {
            list,
            snapshot,
            state,
            mode: AppMode::Normal,
            input: String::new(),
            edit_index: None,
            message: None,
            filter: todo_filter::DEFAULT,
            quit_input: String::new(),
            cursor_pos: 0,
            report_pending_content: None,
            cmd_popup_filter: String::new(),
            cmd_popup_selected: 0,
            theme,
        }
    }

    /// 模糊筛选命令面板选项，返回 (原索引, key, 标签)
    pub fn filtered_cmd_items(&self) -> Vec<(usize, &'static str, &'static str)> {
        let filter = self.cmd_popup_filter.to_lowercase();
        CMD_POPUP_ITEMS
            .iter()
            .enumerate()
            .filter(|(_, (key, label))| {
                filter.is_empty()
                    || key.contains(filter.as_str())
                    || label.contains(filter.as_str())
            })
            .map(|(i, (key, label))| (i, *key, *label))
            .collect()
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
                todo_filter::UNDONE => !item.done,
                todo_filter::DONE => item.done,
                todo_filter::ALL => true,
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
            Some(i) => (i + 1) % count,
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
            Some(i) => (i + count - 1) % count,
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
                // 暂存内容，进入确认写入日报模式
                self.report_pending_content = Some(item.content.clone());
                self.mode = AppMode::ConfirmReport;
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
        // 自动保存到文件
        if save_todo_list(&self.list) {
            self.snapshot = self.list.clone();
            self.message = Some("☑️ 已添加并保存".to_string());
        } else {
            self.message = Some("☑️ 已添加（保存失败）".to_string());
        }
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
        if let Some(idx) = self.edit_index
            && idx < self.list.items.len()
        {
            self.list.items[idx].content = text;
            // 自动保存到文件
            if save_todo_list(&self.list) {
                self.snapshot = self.list.clone();
                self.message = Some("☑️ 已更新并保存".to_string());
            } else {
                self.message = Some("☑️ 已更新（保存失败）".to_string());
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
            } else if let Some(sel) = self.state.selected()
                && sel >= count
            {
                self.state.select(Some(count - 1));
            }
        }
        self.mode = AppMode::Normal;
    }

    /// 移动选中项向上（调整顺序）
    pub fn move_item_up(&mut self) {
        if let Some(real_idx) = self.selected_real_index()
            && real_idx > 0
        {
            self.list.items.swap(real_idx, real_idx - 1);
            self.move_up();
        }
    }

    /// 移动选中项向下（调整顺序）
    pub fn move_item_down(&mut self) {
        if let Some(real_idx) = self.selected_real_index()
            && real_idx < self.list.items.len() - 1
        {
            self.list.items.swap(real_idx, real_idx + 1);
            self.move_down();
        }
    }

    /// 切换过滤模式
    pub fn toggle_filter(&mut self) {
        self.filter = (self.filter + 1) % todo_filter::COUNT;
        let count = self.filtered_indices().len();
        if count > 0 {
            self.state.select(Some(0));
        } else {
            self.state.select(None);
        }
        let label = todo_filter::label(self.filter);
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
