use crate::command::chat::markdown::markdown_to_lines;
use crate::command::chat::theme::{Theme, ThemeName};
use crate::config::YamlConfig;
use crate::constants::shell;
use crate::error;
use crate::info;
use crate::util::fuzzy;
use chrono::{DateTime, Local};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::text::Line;
use ratatui::widgets::ListState;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

// ========== 数据结构 ==========

/// 单条笔记信息
#[derive(Debug, Clone)]
pub struct NoteItem {
    /// 笔记名称（不含 .md 后缀）
    pub name: String,
    /// 修改时间
    pub mtime: std::time::SystemTime,
}

// ========== 文件路径 ==========

/// 获取 notebook 目录路径: ~/.jdata/notebook/
pub fn notebook_dir() -> PathBuf {
    YamlConfig::notebook_dir()
}

/// 获取笔记文件路径
pub fn note_file_path(name: &str) -> PathBuf {
    notebook_dir().join(format!("{}.md", name))
}

// ========== 数据读写 ==========

/// 从磁盘加载笔记列表，按修改时间倒序
pub fn load_notes() -> Vec<NoteItem> {
    let dir = notebook_dir();
    let mut notes = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let mtime = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(std::time::UNIX_EPOCH);
                notes.push(NoteItem { name, mtime });
            }
        }
    }
    notes.sort_by(|a, b| b.mtime.cmp(&a.mtime));
    notes
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

/// 用 Markdown 编辑器编辑笔记，返回是否有内容变化
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

    let theme = Theme::from_name(&ThemeName::default());
    match crate::tui::editor_markdown::open_markdown_editor(&editor_title, &content, &theme) {
        Ok(Some(new_content)) => {
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
        Ok(None) => info!("已取消编辑"),
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

// ========== TUI 应用状态 ==========

/// TUI 应用状态
pub struct NotebookApp {
    /// 笔记列表（全量，从磁盘加载）
    pub notes: Vec<NoteItem>,
    /// 列表选中状态
    pub state: ListState,
    /// 当前模式
    pub mode: AppMode,
    /// 输入缓冲区（新建/重命名/搜索）
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
}

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
}

impl Default for NotebookApp {
    fn default() -> Self {
        Self::new()
    }
}

impl NotebookApp {
    pub fn new() -> Self {
        let notes = load_notes();
        let mut state = ListState::default();
        if !notes.is_empty() {
            state.select(Some(0));
        }
        let mut app = Self {
            notes,
            state,
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
        };
        app.update_preview();
        app
    }

    /// 从磁盘刷新笔记列表
    pub fn reload(&mut self) {
        self.notes = load_notes();
        let count = self.filtered_indices().len();
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
                    if fuzzy::fuzzy_match(&item.name, keyword) {
                        return true;
                    }
                    // 也搜索笔记内容
                    if let Some(content) = read_note_content(&item.name) {
                        return fuzzy::fuzzy_match(&content, keyword);
                    }
                    false
                }
                None => true,
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

    /// 获取选中笔记名称
    pub fn selected_name(&self) -> Option<String> {
        self.selected_real_index()
            .map(|idx| self.notes[idx].name.clone())
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
        self.preview_scroll = 0;
        self.update_preview();
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
        self.preview_scroll = 0;
        self.update_preview();
    }

    /// 更新预览内容缓存
    pub fn update_preview(&mut self) {
        self.preview_content = self
            .selected_real_index()
            .and_then(|idx| read_note_content(&self.notes[idx].name));
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
                let theme = Theme::from_name(&ThemeName::default());
                self.preview_lines = markdown_to_lines(content, width, &theme);
            }
            _ => {
                self.preview_lines.clear();
            }
        }
    }

    /// 清除搜索过滤
    pub fn clear_search(&mut self) {
        self.search_filter = None;
        let count = self.filtered_indices().len();
        if count > 0 {
            self.state.select(Some(0));
        } else {
            self.state.select(None);
        }
        self.update_preview();
        self.message = Some("已清除搜索过滤".to_string());
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
                app.input = app.notes[idx].name.clone();
                app.cursor_pos = app.input.chars().count();
                app.rename_index = Some(idx);
                app.mode = AppMode::Renaming;
                app.message = None;
            }
        }
        KeyCode::Char('p') => {
            if app.selected_real_index().is_some() {
                app.mode = AppMode::Preview;
                app.preview_scroll = 0;
            }
        }
        KeyCode::Char('/') => {
            app.mode = AppMode::Search;
            app.input.clear();
            app.cursor_pos = 0;
            app.message = None;
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
                        let old_name = &app.notes[idx].name;
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
                        match fs::rename(&old_path, &new_path) {
                            Ok(()) => {
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
                let name = &app.notes[idx].name;
                let path = note_file_path(name);
                match fs::remove_file(&path) {
                    Ok(()) => {
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

/// 帮助模式按键处理（按任意键返回）
pub fn handle_help_mode(app: &mut NotebookApp, _key: KeyEvent) {
    app.mode = AppMode::Normal;
    app.message = None;
}
