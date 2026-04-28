//! Notebook 模块的核心数据类型定义。

use ratatui::text::Line;
use ratatui::widgets::ListState;
use std::collections::{BTreeSet, HashSet};

use crate::theme::Theme;

use super::io::load_notes;

// ========== 笔记条目 ==========

/// 单条笔记信息（支持子目录路径）
#[derive(Debug, Clone)]
pub struct NoteItem {
    /// 笔记相对路径（不含 .md 后缀），如 "ideas/project" 或 "meeting-notes"
    pub path: String,
    /// 修改时间
    pub mtime: std::time::SystemTime,
}

impl NoteItem {
    /// 获取显示名称（路径最后一部分）
    pub fn display_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// 获取所在目录（相对路径），如 "ideas"，根目录返回 None
    pub fn parent_dir(&self) -> Option<&str> {
        self.path.rsplit_once('/').map(|(dir, _)| dir)
    }
}

// ========== 目录展开状态 ==========

/// 目录展开状态（持久化到 YamlConfig setting section）
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExpandedDirs(pub HashSet<String>);

impl ExpandedDirs {
    /// 创建空的展开目录集合
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    /// 判断指定目录是否已展开
    pub fn is_expanded(&self, dir_path: &str) -> bool {
        self.0.contains(dir_path)
    }

    /// 切换指定目录的展开/折叠状态
    pub fn toggle(&mut self, dir_path: &str) {
        if self.0.contains(dir_path) {
            self.0.remove(dir_path);
        } else {
            self.0.insert(dir_path.to_string());
        }
    }
}

impl Default for ExpandedDirs {
    fn default() -> Self {
        Self::new()
    }
}

// ========== 扁平化条目 ==========

/// 扁平化列表项（用于 TUI 渲染和选择）
#[derive(Debug, Clone)]
pub struct FlatEntry {
    /// 条目类型
    pub kind: FlatEntryKind,
    /// 树形缩进，如 "    " 表示两层深度
    pub guide: String,
}

/// 扁平化条目类型枚举，用于渲染时的行类型区分
#[derive(Debug, Clone)]
pub enum FlatEntryKind {
    /// 文件条目，引用 notes 列表中的索引
    File { note_index: usize },
    /// 目录条目
    Dir {
        /// 目录相对路径，如 "ideas"
        dir_path: String,
        /// 目录显示名
        name: String,
        /// 目录下文件数量（包含子目录中的文件）
        file_count: usize,
    },
}

// ========== 应用模式 ==========

/// Notebook 应用模式枚举
#[derive(PartialEq, Clone)]
pub enum AppMode {
    /// 正常浏览模式
    Normal,
    /// 全屏预览模式
    Preview,
    /// 新建笔记（输入标题）
    Adding,
    /// 重命名笔记（输入新标题）
    Renaming,
    /// 搜索模式（输入关键词）
    Search,
    /// 确认删除
    ConfirmDelete,
    /// 帮助页
    Help,
    /// 命令面板（/ 弹窗）
    CommandPopup,
    /// 比例输入模式（如 20:80）
    RatioInput,
    /// 新建目录（输入目录名）
    Mkdir,
    /// 移动笔记（输入目标路径）
    Mv,
}

// ========== 命令面板选项 ==========

/// 命令面板选项列表 (key, 中文标签)
pub const CMD_POPUP_ITEMS: &[(&str, &str)] = &[
    ("search", "搜索"),
    ("rename", "重命名"),
    ("delete", "删除"),
    ("mkdir", "新建目录"),
    ("mv", "移动"),
    ("open", "打开目录"),
    ("ratio", "调整比例"),
    ("help", "帮助"),
];

// ========== TUI 应用状态 ==========

/// TUI 应用状态
pub struct NotebookApp {
    /// 笔记列表（全量，从磁盘加载）
    pub notes: Vec<NoteItem>,
    /// 列表选中状态
    pub state: ListState,
    /// 当前模式
    pub mode: AppMode,
    /// 输入缓冲区（新建/重命名/搜索/比例）
    pub input: String,
    /// 光标位置（字符索引）
    pub cursor_pos: usize,
    /// 状态栏消息
    pub message: Option<String>,
    /// 搜索关键词（过滤后保存，用于显示）
    pub search_filter: Option<String>,
    /// 重命名目标索引
    pub rename_index: Option<usize>,
    /// 预览区滚动偏移
    pub preview_scroll: u16,
    /// 当前预览内容缓存（原始 Markdown）
    pub preview_content: Option<String>,
    /// 预览渲染行缓存（Markdown 渲染后的 Lines）
    pub preview_lines: Vec<Line<'static>>,
    /// 预览区宽度缓存（用于判断是否需要重新渲染）
    pub preview_width: u16,
    /// 强制退出输入缓冲
    pub quit_input: String,
    /// 新建笔记确认后，待打开编辑器的标题（TUI loop 消费）
    pub pending_edit_title: Option<String>,
    /// 左侧面板比例 (15-60, 默认 30)
    pub panel_ratio: u16,
    /// 命令面板选中索引
    pub cmd_popup_selected: usize,
    /// 命令面板筛选文本
    pub cmd_popup_filter: String,
    /// 展开的目录集合
    pub expanded_dirs: ExpandedDirs,
    /// 扁平化条目列表（由 build_flat_entries() 生成）
    pub flat_entries: Vec<FlatEntry>,
    /// 当前主题
    pub theme: Theme,
}

impl Default for NotebookApp {
    fn default() -> Self {
        Self::new()
    }
}

impl NotebookApp {
    /// 创建 NotebookApp 实例，加载笔记列表和配置
    pub fn new() -> Self {
        let notes = load_notes();
        let expanded_dirs = super::io::load_expanded_dirs();
        let agent_config = crate::command::chat::storage::load_agent_config();
        let theme = Theme::from_name(&agent_config.theme);
        let mut app = Self {
            notes,
            state: ListState::default(),
            mode: AppMode::Normal,
            input: String::new(),
            cursor_pos: 0,
            message: None,
            search_filter: None,
            rename_index: None,
            preview_scroll: 0,
            preview_content: None,
            preview_lines: Vec::new(),
            preview_width: 0,
            quit_input: String::new(),
            pending_edit_title: None,
            panel_ratio: super::io::load_panel_ratio().unwrap_or(30),
            cmd_popup_selected: 0,
            cmd_popup_filter: String::new(),
            expanded_dirs,
            flat_entries: Vec::new(),
            theme,
        };
        app.build_flat_entries();
        if !app.flat_entries.is_empty() {
            app.state.select(Some(0));
        }
        app.update_preview();
        app
    }

    /// 从磁盘刷新笔记列表
    pub fn reload(&mut self) {
        self.notes = load_notes();
        self.build_flat_entries();
        let count = self.flat_entries.len();
        if count == 0 {
            self.state.select(None);
        } else if let Some(sel) = self.state.selected()
            && sel >= count
        {
            self.state.select(Some(count - 1));
        }
        self.update_preview();
        self.message = Some(format!("已刷新，共 {} 篇笔记", self.notes.len()));
    }

    /// 获取过滤后的索引列表
    pub fn filtered_indices(&self) -> Vec<usize> {
        self.notes
            .iter()
            .enumerate()
            .filter(|(_, item)| match &self.search_filter {
                Some(keyword) => {
                    if crate::util::fuzzy::fuzzy_match(&item.path, keyword) {
                        return true;
                    }
                    // 也搜索笔记内容
                    if let Some(content) = super::io::read_note_content(&item.path) {
                        return crate::util::fuzzy::fuzzy_match(&content, keyword);
                    }
                    false
                }
                None => true,
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// 构建扁平化条目列表（供 TUI 渲染）
    pub fn build_flat_entries(&mut self) {
        let filtered: Vec<usize> = self.filtered_indices();
        let filtered_set: HashSet<usize> = filtered.iter().copied().collect();

        // 收集过滤后笔记涉及的所有目录
        let mut dir_set: BTreeSet<String> = BTreeSet::new();
        for &idx in &filtered {
            if let Some(parent) = self.notes[idx].parent_dir() {
                // 添加所有祖先目录
                let parts: Vec<&str> = parent.split('/').collect();
                let mut acc = String::new();
                for part in &parts {
                    if !acc.is_empty() {
                        acc.push('/');
                    }
                    acc.push_str(part);
                    dir_set.insert(acc.clone());
                }
            }
        }

        let mut flat = Vec::new();
        super::flat_entries::build_flat_entries_recursive(
            super::flat_entries::FlatEntriesContext::new(
                &self.notes,
                &filtered_set,
                &dir_set,
                &self.expanded_dirs,
            ),
            "",
            0,
            &mut flat,
        );
        self.flat_entries = flat;
    }

    /// 获取当前选中的 flat entry
    pub fn selected_entry(&self) -> Option<&FlatEntry> {
        self.state.selected().and_then(|i| self.flat_entries.get(i))
    }

    /// 获取当前选中项在原始列表中的真实索引（仅文件条目）
    pub fn selected_real_index(&self) -> Option<usize> {
        self.state.selected().and_then(|i| {
            self.flat_entries
                .get(i)
                .and_then(|entry| match &entry.kind {
                    FlatEntryKind::File { note_index } => Some(*note_index),
                    FlatEntryKind::Dir { .. } => None,
                })
        })
    }

    /// 获取选中笔记路径
    pub fn selected_name(&self) -> Option<String> {
        self.selected_real_index()
            .map(|idx| self.notes[idx].path.clone())
    }

    /// 向下移动（不循环）
    pub fn move_down(&mut self) {
        let count = self.flat_entries.len();
        if count == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => (i + 1).min(count - 1),
            None => 0,
        };
        self.state.select(Some(i));
        self.preview_scroll = 0;
        self.update_preview();
    }

    /// 向上移动（不循环）
    pub fn move_up(&mut self) {
        let count = self.flat_entries.len();
        if count == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.state.select(Some(i));
        self.preview_scroll = 0;
        self.update_preview();
    }

    /// 更新预览内容缓存
    pub fn update_preview(&mut self) {
        match self.selected_entry() {
            Some(FlatEntry {
                kind: FlatEntryKind::File { note_index },
                ..
            }) => {
                self.preview_content = super::io::read_note_content(&self.notes[*note_index].path);
            }
            Some(FlatEntry {
                kind:
                    FlatEntryKind::Dir {
                        dir_path,
                        file_count,
                        ..
                    },
                ..
            }) => {
                self.preview_content = Some(format!(
                    "目录: {}\n包含 {} 篇笔记\n\n按 Tab 展开/折叠",
                    dir_path, file_count
                ));
            }
            None => {
                self.preview_content = None;
            }
        }
        self.render_preview_lines();
    }

    /// 渲染 Markdown 预览行（带宽度参数，供 UI 层调用）
    pub fn render_preview_with_width(&mut self, width: u16) {
        if width != self.preview_width {
            self.preview_width = width;
            self.render_preview_lines();
        }
    }

    /// 内部渲染预览行
    fn render_preview_lines(&mut self) {
        let width = if self.preview_width > 0 {
            self.preview_width as usize
        } else {
            Self::DEFAULT_PREVIEW_WIDTH
        };
        match &self.preview_content {
            Some(content) if !content.is_empty() => {
                self.preview_lines =
                    crate::markdown::markdown_to_lines(content, width, &self.theme);
            }
            _ => {
                self.preview_lines.clear();
            }
        }
    }

    /// 清除搜索过滤
    pub fn clear_search(&mut self) {
        self.search_filter = None;
        self.build_flat_entries();
        let count = self.flat_entries.len();
        if count > 0 {
            self.state.select(Some(0));
        } else {
            self.state.select(None);
        }
        self.update_preview();
        self.message = Some("已清除搜索过滤".to_string());
    }

    /// 获取筛选后的命令面板选项
    pub fn filtered_cmd_items(&self) -> Vec<(usize, &'static str, &'static str)> {
        CMD_POPUP_ITEMS
            .iter()
            .enumerate()
            .filter(|(_, (key, label))| {
                if self.cmd_popup_filter.is_empty() {
                    return true;
                }
                let f = self.cmd_popup_filter.to_lowercase();
                key.to_lowercase().contains(&f) || label.contains(f.as_str())
            })
            .map(|(i, (k, l))| (i, *k, *l))
            .collect()
    }

    /// 默认预览宽度
    const DEFAULT_PREVIEW_WIDTH: usize = 80;
}
