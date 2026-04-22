use crate::command::chat::markdown::markdown_to_lines;
use crate::command::chat::storage::load_agent_config;
use crate::config::YamlConfig;
use crate::constants::{config_key, section, shell};
use crate::error;
use crate::info;
use crate::theme::Theme;
use crate::util::fuzzy;
use chrono::{DateTime, Local};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::text::Line;
use ratatui::widgets::ListState;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

// ========== 数据结构 ==========

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

// ========== 文件路径 ==========

/// 获取 notebook 目录路径: ~/.jdata/notebook/
pub fn notebook_dir() -> PathBuf {
    crate::config::YamlConfig::notebook_dir()
}

/// 从 YamlConfig setting section 加载面板比例
fn load_panel_ratio() -> Option<u16> {
    YamlConfig::load()
        .get_property(section::SETTING, config_key::NOTEBOOK_PANEL_RATIO)
        .and_then(|v| v.parse().ok())
}

/// 保存面板比例到 YamlConfig setting section
fn save_panel_ratio(ratio: u16) {
    let mut config = YamlConfig::load();
    config.set_property(
        section::SETTING,
        config_key::NOTEBOOK_PANEL_RATIO,
        &ratio.to_string(),
    );
}

/// 从 YamlConfig setting section 加载展开目录列表
fn load_expanded_dirs() -> ExpandedDirs {
    YamlConfig::load()
        .get_property(section::SETTING, config_key::NOTEBOOK_EXPANDED_DIRS)
        .and_then(|v| serde_json::from_str(v).ok())
        .unwrap_or_default()
}

/// 保存展开目录列表到 YamlConfig setting section
fn save_expanded_dirs(dirs: &ExpandedDirs) {
    if let Ok(json) = serde_json::to_string(dirs) {
        let mut config = YamlConfig::load();
        config.set_property(section::SETTING, config_key::NOTEBOOK_EXPANDED_DIRS, &json);
    }
}

/// 目录展开状态（持久化到 YamlConfig setting section）
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExpandedDirs(pub std::collections::HashSet<String>);

impl ExpandedDirs {
    /// 创建空的展开目录集合
    pub fn new() -> Self {
        Self(std::collections::HashSet::new())
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

/// 扁平化列表项（用于 TUI 渲染和选择）
#[derive(Debug, Clone)]
pub struct FlatEntry {
    /// 条目类型
    pub kind: FlatEntryKind,
    /// 树形缩进，如 "    " 表示两层深度
    pub guide: String,
}

#[derive(Debug, Clone)]
/// 扁平化条目类型枚举，用于渲染时的行类型区分
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

/// 获取笔记文件路径
pub fn note_file_path(name: &str) -> PathBuf {
    notebook_dir().join(format!("{}.md", name))
}

// ========== 数据读写 ==========

/// 从磁盘加载笔记列表（递归子目录），按修改时间倒序
pub fn load_notes() -> Vec<NoteItem> {
    let dir = notebook_dir();
    let mut notes = Vec::new();
    walk_dir_for_notes(&dir, "", &mut notes);
    notes.sort_by(|a, b| b.mtime.cmp(&a.mtime));
    notes
}

fn walk_dir_for_notes(dir: &std::path::Path, prefix: &str, notes: &mut Vec<NoteItem>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // 跳过隐藏目录（以 . 开头）
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                if dir_name.starts_with('.') {
                    continue;
                }
                let sub_prefix = if prefix.is_empty() {
                    dir_name.to_string()
                } else {
                    format!("{}/{}", prefix, dir_name)
                };
                walk_dir_for_notes(&path, &sub_prefix, notes);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                let stem = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let note_path = if prefix.is_empty() {
                    stem
                } else {
                    format!("{}/{}", prefix, stem)
                };
                let mtime = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(std::time::UNIX_EPOCH);
                notes.push(NoteItem {
                    path: note_path,
                    mtime,
                });
            }
        }
    }
}

/// 列出 notebook 下的所有子目录（相对路径）
pub fn list_dirs() -> Vec<String> {
    let dir = notebook_dir();
    let mut dirs = Vec::new();
    walk_dir_for_dirs(&dir, "", &mut dirs);
    dirs.sort();
    dirs
}

fn walk_dir_for_dirs(dir: &std::path::Path, prefix: &str, dirs: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                if dir_name.starts_with('.') {
                    continue;
                }
                let dir_path = if prefix.is_empty() {
                    dir_name.to_string()
                } else {
                    format!("{}/{}", prefix, dir_name)
                };
                dirs.push(dir_path.clone());
                walk_dir_for_dirs(&path, &dir_path, dirs);
            }
        }
    }
}

/// 读取笔记内容
pub fn read_note_content(name: &str) -> Option<String> {
    let path = note_file_path(name);
    fs::read_to_string(path).ok()
}

/// 格式化 SystemTime 为可读字符串
pub fn format_time(time: std::time::SystemTime) -> String {
    let dt: DateTime<Local> = time.into();
    dt.format("%Y-%m-%d %H:%M").to_string()
}

/// 用 Markdown 编辑器编辑笔记（在已有 terminal 上），返回是否有内容变化
pub fn edit_note_on_terminal(
    title: &str,
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
) -> bool {
    let file_path = note_file_path(title);
    let (content, is_new) = if file_path.exists() {
        match fs::read_to_string(&file_path) {
            Ok(c) => (c, false),
            Err(e) => {
                error!("读取笔记失败: {}", e);
                return false;
            }
        }
    } else {
        (String::new(), true)
    };

    let editor_title = if is_new {
        format!("{} (新笔记)", title)
    } else {
        title.to_string()
    };

    let theme = Theme::from_name(&load_agent_config().theme);
    match crate::tui::editor_markdown::open_markdown_editor_on_terminal(
        terminal,
        &editor_title,
        &content,
        &theme,
    ) {
        Ok((Some(new_content), _)) => {
            if new_content != content {
                match fs::write(&file_path, &new_content) {
                    Ok(()) => {
                        info!("笔记已保存: {}", title);
                        return true;
                    }
                    Err(e) => error!("保存笔记失败: {}", e),
                }
            } else {
                info!("内容未变化，跳过保存");
            }
        }
        Ok((None, _)) => info!("已取消编辑"),
        Err(e) => error!("编辑器启动失败: {}", e),
    }
    false
}

/// 用 Markdown 编辑器编辑笔记（独立终端），返回是否有内容变化
pub fn edit_note_with_editor(title: &str) -> bool {
    let file_path = note_file_path(title);
    let (content, is_new) = if file_path.exists() {
        match fs::read_to_string(&file_path) {
            Ok(c) => (c, false),
            Err(e) => {
                error!("读取笔记失败: {}", e);
                return false;
            }
        }
    } else {
        (String::new(), true)
    };

    let editor_title = if is_new {
        format!("{} (新笔记)", title)
    } else {
        title.to_string()
    };

    let theme = Theme::from_name(&load_agent_config().theme);
    match crate::tui::editor_markdown::open_markdown_editor(&editor_title, &content, &theme) {
        Ok((Some(new_content), _)) => {
            if new_content != content {
                match fs::write(&file_path, &new_content) {
                    Ok(()) => {
                        info!("笔记已保存: {}", title);
                        return true;
                    }
                    Err(e) => error!("保存笔记失败: {}", e),
                }
            } else {
                info!("内容未变化，跳过保存");
            }
        }
        Ok((None, _)) => info!("已取消编辑"),
        Err(e) => error!("编辑器启动失败: {}", e),
    }
    false
}

/// 复制内容到系统剪切板
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

/// 在 Finder 中打开 notebook 目录
pub fn open_in_finder() {
    let dir = notebook_dir();
    let path = dir.to_string_lossy().to_string();
    let os = std::env::consts::OS;
    let result = if os == shell::MACOS_OS {
        Command::new("open").arg(&path).status()
    } else if os == shell::WINDOWS_OS {
        Command::new(shell::WINDOWS_CMD)
            .args([shell::WINDOWS_CMD_FLAG, "start", "", &path])
            .status()
    } else {
        Command::new("xdg-open").arg(&path).status()
    };

    if let Err(e) = result {
        error!("打开目录失败: {}", e);
    }
}

// ========== 命令面板选项 ==========

/// 命令面板选项列表 (key, 中文标签)
const CMD_POPUP_ITEMS: &[(&str, &str)] = &[
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

#[derive(PartialEq, Clone)]
/// Notebook 应用模式枚举
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

impl Default for NotebookApp {
    fn default() -> Self {
        Self::new()
    }
}

impl NotebookApp {
    /// 创建 NotebookApp 实例，加载笔记列表和配置
    pub fn new() -> Self {
        let notes = load_notes();
        let expanded_dirs = load_expanded_dirs();
        let agent_config = load_agent_config();
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
            panel_ratio: load_panel_ratio().unwrap_or(30),
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
                    if fuzzy::fuzzy_match(&item.path, keyword) {
                        return true;
                    }
                    // 也搜索笔记内容
                    if let Some(content) = read_note_content(&item.path) {
                        return fuzzy::fuzzy_match(&content, keyword);
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
        let filtered_set: std::collections::HashSet<usize> = filtered.iter().copied().collect();

        // 收集过滤后笔记涉及的所有目录
        let mut dir_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
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
        build_flat_entries_recursive(
            &self.notes,
            &filtered_set,
            &dir_set,
            &self.expanded_dirs,
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
                self.preview_content = read_note_content(&self.notes[*note_index].path);
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
            80 // 默认宽度
        };
        match &self.preview_content {
            Some(content) if !content.is_empty() => {
                self.preview_lines = markdown_to_lines(content, width, &self.theme.clone());
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
}

/// 递归构建扁平化条目列表
fn build_flat_entries_recursive(
    notes: &[NoteItem],
    filtered_set: &std::collections::HashSet<usize>,
    dir_set: &std::collections::BTreeSet<String>,
    expanded_dirs: &ExpandedDirs,
    prefix: &str,
    depth: usize,
    flat: &mut Vec<FlatEntry>,
) {
    // 1. 收集当前前缀下的直接子目录（已排序，BTreeSet 保证）
    let mut child_dirs: Vec<String> = Vec::new();
    for dir_path in dir_set.iter() {
        if prefix.is_empty() {
            if !dir_path.contains('/') {
                child_dirs.push(dir_path.clone());
            }
        } else if dir_path.starts_with(&format!("{}/", prefix)) {
            let rest = &dir_path[prefix.len() + 1..];
            if !rest.contains('/') {
                child_dirs.push(dir_path.clone());
            }
        }
    }

    // 2. 收集当前前缀下的直接笔记文件
    let mut child_files: Vec<usize> = Vec::new();
    for &idx in filtered_set {
        let note = &notes[idx];
        let parent = note.parent_dir().unwrap_or("");
        if parent == prefix {
            child_files.push(idx);
        }
    }

    // 3. 先渲染子目录，再渲染文件
    for dir_path in &child_dirs {
        let name = dir_path.rsplit('/').next().unwrap_or(dir_path);
        let expanded = expanded_dirs.is_expanded(dir_path);
        let file_count = filtered_set
            .iter()
            .filter(|&&idx| {
                notes[idx]
                    .parent_dir()
                    .is_some_and(|p| p == *dir_path || p.starts_with(&format!("{}/", dir_path)))
            })
            .count();

        let guide = "  ".repeat(depth);
        flat.push(FlatEntry {
            kind: FlatEntryKind::Dir {
                dir_path: dir_path.clone(),
                name: name.to_string(),
                file_count,
            },
            guide,
        });

        // 如果展开，递归
        if expanded {
            build_flat_entries_recursive(
                notes,
                filtered_set,
                dir_set,
                expanded_dirs,
                dir_path,
                depth + 1,
                flat,
            );
        }
    }

    for &idx in &child_files {
        let guide = "  ".repeat(depth);
        flat.push(FlatEntry {
            kind: FlatEntryKind::File { note_index: idx },
            guide,
        });
    }
}

// ========== 按键处理 ==========

/// 正常模式按键处理，返回 true 表示退出
pub fn handle_normal_mode(app: &mut NotebookApp, key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }

    match key.code {
        KeyCode::Char('q') => {
            app.quit_input = "q".to_string();
            return true;
        }
        KeyCode::Esc => {
            if app.search_filter.is_some() {
                app.clear_search();
            } else {
                return true;
            }
        }
        KeyCode::Char('n') | KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Char('N') | KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Enter | KeyCode::Char('e') => {
            if app.selected_name().is_some() {
                return false; // 编辑操作在 TUI loop 中处理（需暂停/恢复终端）
            }
        }
        KeyCode::Char('a') => {
            app.mode = AppMode::Adding;
            app.input.clear();
            app.cursor_pos = 0;
            app.message = None;
        }
        KeyCode::Char('d') => {
            if app.selected_real_index().is_some() {
                app.mode = AppMode::ConfirmDelete;
            }
        }
        KeyCode::Char('r') => {
            if let Some(idx) = app.selected_real_index() {
                app.input = app.notes[idx].path.clone();
                app.cursor_pos = app.input.chars().count();
                app.rename_index = Some(idx);
                app.mode = AppMode::Renaming;
                app.message = None;
            }
        }
        KeyCode::Tab => {
            if let Some(sel) = app.state.selected()
                && sel < app.flat_entries.len()
                && let FlatEntryKind::Dir { ref dir_path, .. } = app.flat_entries[sel].kind
            {
                app.expanded_dirs.toggle(dir_path);
                save_expanded_dirs(&app.expanded_dirs);
                app.build_flat_entries();
                if sel >= app.flat_entries.len() {
                    app.state
                        .select(Some(app.flat_entries.len().saturating_sub(1)));
                }
                app.update_preview();
            }
        }
        KeyCode::Char('p') => {
            if app.selected_real_index().is_some() {
                app.mode = AppMode::Preview;
                app.preview_scroll = 0;
            }
        }
        KeyCode::Char('/') => {
            app.mode = AppMode::CommandPopup;
            app.cmd_popup_filter.clear();
            app.cmd_popup_selected = 0;
            app.message = None;
        }
        KeyCode::Char('[') => {
            app.panel_ratio = app.panel_ratio.saturating_sub(5).max(15);
            app.preview_width = 0;
            app.message = Some(format!(
                "面板比例: {}:{}",
                app.panel_ratio,
                100 - app.panel_ratio
            ));
            save_panel_ratio(app.panel_ratio);
        }
        KeyCode::Char(']') => {
            app.panel_ratio = app.panel_ratio.saturating_add(5).min(60);
            app.preview_width = 0;
            app.message = Some(format!(
                "面板比例: {}:{}",
                app.panel_ratio,
                100 - app.panel_ratio
            ));
            save_panel_ratio(app.panel_ratio);
        }
        KeyCode::Char('y') => {
            if let Some(name) = app.selected_name() {
                if copy_to_clipboard(&name) {
                    app.message = Some(format!("已复制笔记名: {}", name));
                } else {
                    app.message = Some("复制到剪切板失败".to_string());
                }
            }
        }
        KeyCode::Char('o') => {
            open_in_finder();
        }
        KeyCode::Char('s') => {
            app.reload();
        }
        KeyCode::Char('?') => {
            app.mode = AppMode::Help;
        }
        _ => {}
    }

    if key.code != KeyCode::Char('q') {
        app.quit_input.clear();
    }

    false
}

/// 预览模式按键处理
pub fn handle_preview_mode(app: &mut NotebookApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('p') | KeyCode::Char('q') => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.preview_scroll = app.preview_scroll.saturating_add(1);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.preview_scroll = app.preview_scroll.saturating_sub(1);
        }
        KeyCode::Char('n') => {
            app.move_down();
        }
        KeyCode::Char('N') => {
            app.move_up();
        }
        _ => {}
    }
}

/// 输入模式按键处理（添加/重命名/搜索通用）
pub fn handle_input_mode(app: &mut NotebookApp, key: KeyEvent) {
    let char_count = app.input.chars().count();

    match key.code {
        KeyCode::Enter => {
            match app.mode {
                AppMode::Adding => {
                    let title = app.input.trim().to_string();
                    if title.is_empty() {
                        app.message = Some("标题为空，已取消".to_string());
                        app.mode = AppMode::Normal;
                        app.input.clear();
                        return;
                    }
                    // 设置待编辑标题，由 TUI loop 负责暂停终端并打开编辑器
                    app.pending_edit_title = Some(title);
                    app.input.clear();
                    app.mode = AppMode::Normal;
                }
                AppMode::Renaming => {
                    let new_name = app.input.trim().to_string();
                    if new_name.is_empty() {
                        app.message = Some("名称为空，已取消".to_string());
                        app.mode = AppMode::Normal;
                        app.input.clear();
                        app.rename_index = None;
                        return;
                    }
                    if let Some(idx) = app.rename_index
                        && idx < app.notes.len()
                    {
                        let old_name = &app.notes[idx].path;
                        if old_name == &new_name {
                            app.message = Some("名称未变化".to_string());
                            app.mode = AppMode::Normal;
                            app.input.clear();
                            app.rename_index = None;
                            return;
                        }
                        let old_path = note_file_path(old_name);
                        let new_path = note_file_path(&new_name);
                        if new_path.exists() {
                            app.message = Some(format!("目标笔记已存在: {}", new_name));
                            return;
                        }
                        // 确保目标目录存在
                        if let Some(parent) = new_path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        match fs::rename(&old_path, &new_path) {
                            Ok(()) => {
                                cleanup_empty_dirs();
                                app.message =
                                    Some(format!("已重命名: {} → {}", old_name, new_name));
                                app.reload();
                            }
                            Err(e) => {
                                app.message = Some(format!("重命名失败: {}", e));
                            }
                        }
                    }
                    app.mode = AppMode::Normal;
                    app.input.clear();
                    app.rename_index = None;
                }
                AppMode::Mkdir => {
                    let dir_name = app.input.trim().to_string();
                    if dir_name.is_empty() {
                        app.message = Some("目录名为空，已取消".to_string());
                        app.mode = AppMode::Normal;
                        app.input.clear();
                        return;
                    }
                    let dir_path = notebook_dir().join(&dir_name);
                    if dir_path.exists() {
                        app.message = Some(format!("目录已存在: {}", dir_name));
                        app.mode = AppMode::Normal;
                        app.input.clear();
                        return;
                    }
                    match fs::create_dir_all(&dir_path) {
                        Ok(()) => {
                            app.expanded_dirs.toggle(&dir_name);
                            save_expanded_dirs(&app.expanded_dirs);
                            app.message = Some(format!("已创建目录: {}", dir_name));
                            app.reload();
                        }
                        Err(e) => {
                            app.message = Some(format!("创建目录失败: {}", e));
                        }
                    }
                    app.mode = AppMode::Normal;
                    app.input.clear();
                }
                AppMode::Mv => {
                    let target = app.input.trim().to_string();
                    if target.is_empty() {
                        app.message = Some("目标路径为空，已取消".to_string());
                        app.mode = AppMode::Normal;
                        app.input.clear();
                        return;
                    }
                    let current_name = app.selected_name().unwrap_or_default();
                    if current_name.is_empty() {
                        app.message = Some("没有选中的笔记".to_string());
                        app.mode = AppMode::Normal;
                        app.input.clear();
                        return;
                    }
                    let old_path = note_file_path(&current_name);
                    let new_path = note_file_path(&target);
                    if !old_path.exists() {
                        app.message = Some(format!("源笔记不存在: {}", current_name));
                        app.mode = AppMode::Normal;
                        app.input.clear();
                        return;
                    }
                    if new_path.exists() {
                        app.message = Some(format!("目标笔记已存在: {}", target));
                        app.mode = AppMode::Normal;
                        app.input.clear();
                        return;
                    }
                    // 确保目标目录存在
                    if let Some(parent) = new_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    match fs::rename(&old_path, &new_path) {
                        Ok(()) => {
                            cleanup_empty_dirs();
                            app.message = Some(format!("已移动: {} → {}", current_name, target));
                            app.reload();
                        }
                        Err(e) => {
                            app.message = Some(format!("移动失败: {}", e));
                        }
                    }
                    app.mode = AppMode::Normal;
                    app.input.clear();
                }
                AppMode::Search => {
                    let keyword = app.input.trim().to_string();
                    if keyword.is_empty() {
                        app.clear_search();
                        app.mode = AppMode::Normal;
                    } else {
                        app.search_filter = Some(keyword);
                        let count = app.filtered_indices().len();
                        if count > 0 {
                            app.state.select(Some(0));
                        } else {
                            app.state.select(None);
                        }
                        app.update_preview();
                        app.message = Some(format!(
                            "搜索: {} (匹配 {} 条)",
                            app.search_filter.as_deref().unwrap_or(""),
                            count
                        ));
                        app.mode = AppMode::Normal;
                    }
                    app.input.clear();
                }
                _ => {}
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.input.clear();
            app.cursor_pos = 0;
            app.rename_index = None;
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
            app.input.insert(byte_idx, c);
            app.cursor_pos += 1;
        }
        _ => {}
    }
}

/// 确认删除按键处理
pub fn handle_confirm_delete(app: &mut NotebookApp, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(idx) = app.selected_real_index() {
                let name = &app.notes[idx].path;
                let path = note_file_path(name);
                match fs::remove_file(&path) {
                    Ok(()) => {
                        cleanup_empty_dirs();
                        app.message = Some(format!("已删除: {}", name));
                        app.reload();
                    }
                    Err(e) => {
                        app.message = Some(format!("删除失败: {}", e));
                    }
                }
            }
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.message = Some("已取消删除".to_string());
        }
        _ => {}
    }
}

/// 命令面板按键处理
pub fn handle_command_popup_mode(app: &mut NotebookApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.cmd_popup_filter.clear();
            app.message = None;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let count = app.filtered_cmd_items().len();
            if count > 0 {
                app.cmd_popup_selected = if app.cmd_popup_selected == 0 {
                    count - 1
                } else {
                    app.cmd_popup_selected - 1
                };
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let count = app.filtered_cmd_items().len();
            if count > 0 {
                app.cmd_popup_selected = (app.cmd_popup_selected + 1) % count;
            }
        }
        KeyCode::Enter => {
            let items = app.filtered_cmd_items();
            if let Some(&(_orig_idx, cmd_key, _label)) = items.get(app.cmd_popup_selected) {
                match cmd_key {
                    "search" => {
                        app.mode = AppMode::Search;
                        app.input.clear();
                        app.cursor_pos = 0;
                        app.message = None;
                    }
                    "rename" => {
                        if let Some(idx) = app.selected_real_index() {
                            app.input = app.notes[idx].path.clone();
                            app.cursor_pos = app.input.chars().count();
                            app.rename_index = Some(idx);
                            app.mode = AppMode::Renaming;
                            app.message = None;
                        } else {
                            app.mode = AppMode::Normal;
                            app.message = Some("没有选中的笔记".to_string());
                        }
                    }
                    "delete" => {
                        if app.selected_real_index().is_some() {
                            app.mode = AppMode::ConfirmDelete;
                        } else {
                            app.mode = AppMode::Normal;
                            app.message = Some("没有选中的笔记".to_string());
                        }
                    }
                    "mkdir" => {
                        app.mode = AppMode::Mkdir;
                        app.input.clear();
                        app.cursor_pos = 0;
                        app.message = None;
                    }
                    "mv" => {
                        if let Some(name) = app.selected_name() {
                            app.mode = AppMode::Mv;
                            app.input = name;
                            app.cursor_pos = app.input.chars().count();
                            app.message = None;
                        } else {
                            app.mode = AppMode::Normal;
                            app.message = Some("没有选中的笔记".to_string());
                        }
                    }
                    "open" => {
                        open_in_finder();
                        app.mode = AppMode::Normal;
                        app.message = Some("已打开目录".to_string());
                    }
                    "ratio" => {
                        app.mode = AppMode::RatioInput;
                        app.input = format!("{}:{}", app.panel_ratio, 100 - app.panel_ratio);
                        app.cursor_pos = app.input.chars().count();
                        app.message = None;
                    }
                    "help" => {
                        app.mode = AppMode::Help;
                    }
                    _ => {}
                }
            }
            app.cmd_popup_filter.clear();
            // 不重置 cmd_popup_selected，保留选项位置
        }
        KeyCode::Backspace => {
            if app.cmd_popup_filter.is_empty() {
                app.mode = AppMode::Normal;
                app.message = None;
            } else {
                app.cmd_popup_filter.pop();
                app.cmd_popup_selected = 0;
            }
        }
        KeyCode::Char(c) => {
            app.cmd_popup_filter.push(c);
            app.cmd_popup_selected = 0;
        }
        _ => {}
    }
}

/// 比例输入按键处理
pub fn handle_ratio_input_mode(app: &mut NotebookApp, key: KeyEvent) {
    let char_count = app.input.chars().count();

    match key.code {
        KeyCode::Enter => {
            match parse_ratio(&app.input) {
                Some(ratio) => {
                    app.panel_ratio = ratio;
                    app.preview_width = 0; // 强制重新渲染预览
                    app.message = Some(format!("面板比例已设为 {}:{}", ratio, 100 - ratio));
                    save_panel_ratio(ratio);
                }
                None => {
                    app.message = Some("格式错误，请输入如 20:80".to_string());
                }
            }
            app.mode = AppMode::Normal;
            app.input.clear();
            app.cursor_pos = 0;
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.input.clear();
            app.cursor_pos = 0;
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
            // 只允许数字和冒号
            if c.is_ascii_digit() || c == ':' {
                let byte_idx = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
                app.input.insert(byte_idx, c);
                app.cursor_pos += 1;
            }
        }
        _ => {}
    }
}

/// 解析比例字符串 (如 "20:80") 为左侧面板百分比
fn parse_ratio(input: &str) -> Option<u16> {
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let left: u16 = parts[0].parse().ok()?;
    let right: u16 = parts[1].parse().ok()?;
    if left == 0 || right == 0 {
        return None;
    }
    let pct = left * 100 / (left + right);
    Some(pct.clamp(15, 60))
}

/// 清理 notebook 下的空目录（递归，从深层到浅层）
pub fn cleanup_empty_dirs() {
    let dir = notebook_dir();
    let all_dirs = list_dirs();
    // 按路径深度倒序排列，先删除最深的空目录
    let mut sorted_dirs: Vec<String> = all_dirs;
    sorted_dirs.sort_by_key(|b| std::cmp::Reverse(b.matches('/').count()));
    for dir_path in &sorted_dirs {
        let full_path = dir.join(dir_path);
        if full_path.is_dir() {
            // 检查目录是否为空（只有子目录且子目录也已为空也算空）
            if let Ok(entries) = fs::read_dir(&full_path) {
                let has_content = entries
                    .flatten()
                    .any(|e| e.path().is_file() || e.path().is_dir());
                if !has_content {
                    let _ = fs::remove_dir(&full_path);
                }
            }
        }
    }
}

/// 帮助模式按键处理（按任意键返回）
pub fn handle_help_mode(app: &mut NotebookApp, _key: KeyEvent) {
    app.mode = AppMode::Normal;
    app.message = None;
}
