//! 自研 Markdown 编辑器
//!
//! 完全摆脱 tui-textarea 依赖，支持自动折行、Vim 模式等。

use super::{
    history::Snapshot,
    renderer::MarkdownRenderer,
    search::SearchState,
    text_buffer::TextBuffer,
    theme::{EditorTheme, HighlightFn},
    vim::{Input, Key, Mode, Transition, Vim, filter_commands, parse_command},
    wrap_engine::WrapEngine,
};

use crate::tui::components::selection::rebuild_spans_with_selection;
use crossterm::{
    event::{self, Event, MouseButton, MouseEvent, MouseEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use std::io;

/// 编辑器事件轮询间隔（约 60fps）。
const EDITOR_POLL_MS: u64 = 16;

/// 主题画廊项（显示名称 + 主题ID + 主题）
pub type ThemeGalleryItem = (&'static str, &'static str, EditorTheme);

/// 编辑器初始光标策略
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CursorPolicy {
    /// 光标在文件开头（默认，向后兼容）
    #[default]
    StartOfFile,
    /// 光标在文件末尾
    EndOfFile,
}

/// 编辑器主结构
pub struct MarkdownEditor {
    /// 文本缓冲区
    buffer: TextBuffer,
    /// 折行引擎
    wrap: WrapEngine,
    /// Vim 引擎
    vim: Vim,
    /// 搜索状态
    search: SearchState,
    /// 渲染器
    renderer: MarkdownRenderer,
    /// 主题
    theme: EditorTheme,
    /// 标题
    title: String,
    /// 视口垂直滚动偏移（视觉行级别）
    scroll_offset: usize,
    /// 视口高度
    viewport_height: usize,
    /// 视口宽度
    viewport_width: usize,
    /// 命令面板选中项索引
    cmd_popup_selected: usize,
    /// 主题画廊（名称 + 主题列表）
    theme_gallery: Vec<ThemeGalleryItem>,
    /// 当前主题在画廊中的索引
    theme_index: usize,
    /// 主题选择弹窗选中项索引
    theme_popup_selected: usize,
    /// 状态消息（短暂显示，下次按键清除）
    status_message: Option<String>,
    /// 用户在主题画廊中选择的主题ID（退出时返回）
    selected_theme_id: Option<&'static str>,
    /// 进入搜索前的光标位置，用于 Esc 恢复
    cursor_before_search: Option<(usize, usize)>,
    /// 鼠标拖拽锚点（左键按下时的逻辑位置）
    mouse_anchor: Option<(usize, usize)>,
    /// 滚轮滚动锁定：防止 render() 自动将视口拉回到光标位置
    scroll_locked: bool,
}

impl MarkdownEditor {
    /// 创建新的编辑器
    pub fn new(
        title: &str,
        content: &str,
        theme: EditorTheme,
        highlight_fn: HighlightFn,
        theme_gallery: Vec<ThemeGalleryItem>,
        cursor_policy: CursorPolicy,
    ) -> Self {
        let mut buffer = TextBuffer::from_content(content);
        let initial_mode = if content.is_empty() {
            Mode::Insert
        } else {
            Mode::Normal
        };

        // 根据策略移动光标
        if cursor_policy == CursorPolicy::EndOfFile {
            buffer.move_cursor_bottom();
        }

        let mut vim = Vim::new(initial_mode.clone());
        vim.push_snapshot(Snapshot::new(buffer.snapshot()), buffer.cursor());

        let mut wrap = WrapEngine::new();
        wrap.rebuild_cache(buffer.lines());

        let renderer = MarkdownRenderer::new(theme.clone(), highlight_fn);

        let viewport_width: usize = 80; // 默认值，会在渲染时更新
        wrap.set_width(viewport_width.saturating_sub(6));

        // 查找当前主题在画廊中的索引
        let theme_index = theme_gallery
            .iter()
            .position(|(_, _, t)| *t == theme)
            .unwrap_or(0);

        Self {
            buffer,
            wrap,
            vim,
            search: SearchState::new(),
            renderer,
            theme,
            title: title.to_string(),
            scroll_offset: 0,
            viewport_height: 20,
            viewport_width,
            cmd_popup_selected: 0,
            theme_gallery,
            theme_index,
            theme_popup_selected: theme_index,
            status_message: None,
            selected_theme_id: None,
            cursor_before_search: None,
            mouse_anchor: None,
            scroll_locked: false,
        }
    }

    /// 获取用户选择的主题ID（退出时读取）
    pub fn selected_theme_id(&self) -> Option<&'static str> {
        self.selected_theme_id
    }

    /// 获取光标所在的视觉行
    pub fn cursor_visual_line(&self) -> usize {
        let (row, col) = self.buffer.cursor();
        self.wrap.logical_to_visual(row, col)
    }

    /// 视觉行上移（折行感知）
    pub fn move_cursor_visual_up(&mut self) {
        let current_visual = self.cursor_visual_line();
        if current_visual == 0 {
            return;
        }
        let target_visual = current_visual - 1;
        let (_, current_col) = self.buffer.cursor();

        // 确保目标行的缓存已构建
        let (target_logical, _) = self.wrap.visual_to_logical(target_visual);
        self.wrap
            .build_range(self.buffer.lines(), target_logical, target_logical + 1);

        if let Some(target_vl) = self.wrap.get_visual_line(target_visual) {
            let logical_line = target_vl.logical_line;
            let end_col = target_vl.end_col;
            let start_col = target_vl.start_col;
            let new_col = current_col.min(end_col.saturating_sub(1)).max(start_col);
            self.buffer.set_cursor(logical_line, new_col);
        }
    }

    /// 视觉行下移（折行感知）
    pub fn move_cursor_visual_down(&mut self) {
        let current_visual = self.cursor_visual_line();
        let total_visual = self.wrap.visual_line_count();
        if current_visual >= total_visual.saturating_sub(1) {
            return;
        }
        let target_visual = current_visual + 1;
        let (_, current_col) = self.buffer.cursor();

        // 确保目标行的缓存已构建
        let (target_logical, _) = self.wrap.visual_to_logical(target_visual);
        self.wrap
            .build_range(self.buffer.lines(), target_logical, target_logical + 1);

        if let Some(target_vl) = self.wrap.get_visual_line(target_visual) {
            let logical_line = target_vl.logical_line;
            let end_col = target_vl.end_col;
            let start_col = target_vl.start_col;
            let new_col = current_col.min(end_col.saturating_sub(1)).max(start_col);
            self.buffer.set_cursor(logical_line, new_col);
        }
    }

    // ========== 输入处理 ==========

    /// 处理输入
    pub fn handle_input(&mut self, input: &Input) -> EditorAction {
        // 键盘输入解除滚动锁定
        self.scroll_locked = false;

        // 清除状态消息
        self.status_message = None;

        // 帮助弹窗模式：拦截所有按键
        if self.vim.mode() == &Mode::HelpPopup {
            return self.handle_help_popup(input);
        }

        // 主题选择模式：拦截所有按键
        if self.vim.mode() == &Mode::ThemeSelect {
            return self.handle_theme_select(input);
        }

        // 全局快捷键
        if input.ctrl && input.key == Key::Char('s') {
            return EditorAction::Submit(self.buffer.to_string());
        }
        if input.ctrl && input.key == Key::Char('q') {
            return EditorAction::Cancel;
        }

        // 处理撤销
        if self.vim.mode() == &Mode::Normal && input.key == Key::Char('u') && !input.ctrl {
            self.undo();
            return EditorAction::Continue;
        }

        // 处理重做
        if self.vim.mode() == &Mode::Normal && input.key == Key::Char('r') && input.ctrl {
            self.redo();
            return EditorAction::Continue;
        }

        // 处理搜索跳转
        if self.vim.mode() == &Mode::Normal && self.search.is_searching() {
            if input.key == Key::Char('n') && !input.ctrl {
                self.search_next();
                return EditorAction::Continue;
            }
            if input.key == Key::Char('N') && !input.ctrl {
                self.search_prev();
                return EditorAction::Continue;
            }
            // Enter 跳到下一个匹配（直观一致）
            if input.key == Key::Enter && !input.ctrl {
                self.search_next();
                return EditorAction::Continue;
            }
            // Esc 清除搜索高亮
            if input.key == Key::Esc && !input.ctrl {
                self.search.clear();
                return EditorAction::Continue;
            }
        }

        // 命令面板模式：拦截上下键和回车键
        // 先克隆 filter 以释放 self.vim 的借用，避免后续调用 execute_command 时的借用冲突
        {
            let filter_clone = match self.vim.mode() {
                Mode::CommandPanel(f) => Some(f.clone()),
                _ => None,
            };
            if let Some(filter) = filter_clone {
                let filtered = filter_commands(&filter);
                match input.key {
                    Key::Up => {
                        if !filtered.is_empty() {
                            if self.cmd_popup_selected > 0 {
                                self.cmd_popup_selected -= 1;
                            } else {
                                self.cmd_popup_selected = filtered.len() - 1;
                            }
                        }
                        return EditorAction::Continue;
                    }
                    Key::Down => {
                        if !filtered.is_empty() {
                            if self.cmd_popup_selected < filtered.len() - 1 {
                                self.cmd_popup_selected += 1;
                            } else {
                                self.cmd_popup_selected = 0;
                            }
                        }
                        return EditorAction::Continue;
                    }
                    Key::Enter => {
                        let selected = self
                            .cmd_popup_selected
                            .min(filtered.len().saturating_sub(1));
                        if let Some(cmd) = filtered.get(selected) {
                            let full_cmd = if cmd.name == "jump" {
                                filter
                            } else {
                                cmd.name.to_string()
                            };
                            return self.execute_command(&full_cmd);
                        }
                        self.vim.set_mode(Mode::Normal);
                        return EditorAction::Continue;
                    }
                    _ => {} // 忽略其他按键（如功能键、组合键等）
                }
            }
        }

        // 折行感知的上下移动
        // j/k 只在 Normal 模式拦截，方向键在所有模式拦截
        if self.wrap.is_enabled() {
            let is_normal = self.vim.mode() == &Mode::Normal;
            let is_down = matches!(input.key, Key::Down)
                || (is_normal && matches!(input.key, Key::Char('j')));
            let is_up =
                matches!(input.key, Key::Up) || (is_normal && matches!(input.key, Key::Char('k')));

            if is_down && !input.ctrl {
                self.move_cursor_visual_down();
                return EditorAction::Continue;
            }
            if is_up && !input.ctrl {
                self.move_cursor_visual_up();
                return EditorAction::Continue;
            }
        }

        // Vim 状态机处理
        let old_mode = self.vim.mode().clone();
        let transition = self.vim.handle_input(input, &mut self.buffer);

        match transition {
            Transition::Mode(new_mode) => {
                // 如果从 Insert 模式退出，保存 undo 点
                if old_mode == Mode::Insert && new_mode != Mode::Insert {
                    self.vim
                        .push_snapshot(Snapshot::new(self.buffer.snapshot()), self.buffer.cursor());
                }
                // 从 Search 模式退出时跳转到当前匹配结果
                if matches!(old_mode, Mode::Search(_))
                    && new_mode == Mode::Normal
                    && let Some(m) = self.search.current_match()
                {
                    self.buffer.set_cursor(m.line, m.start);
                }
                if matches!(old_mode, Mode::Search(_)) {
                    self.cursor_before_search = None;
                }
                self.vim.set_mode(new_mode);
                self.rebuild_wrap_cache();
            }
            Transition::Submit => {
                return EditorAction::Submit(self.buffer.to_string());
            }
            Transition::Cancel => {
                return EditorAction::Cancel;
            }
            Transition::SearchAbort => {
                // Esc 取消搜索：恢复光标到搜索前位置，清除搜索高亮
                if let Some(pos) = self.cursor_before_search.take() {
                    self.buffer.set_cursor(pos.0, pos.1);
                }
                self.search.clear();
                self.vim.set_mode(Mode::Normal);
            }
            Transition::Nop => {
                // 处理 Command/Search 模式的字符输入
                self.handle_mode_input(input);
            }
            Transition::NeedRebuild => {
                // Normal 模式下的破坏性操作（dd/x/dw/d$）需要 undo 点
                if old_mode == Mode::Normal {
                    self.vim
                        .push_snapshot(Snapshot::new(self.buffer.snapshot()), self.buffer.cursor());
                }
                self.rebuild_wrap_cache();
            }
            Transition::ToggleWrap(enabled) => {
                self.wrap.set_enabled(enabled);
                self.rebuild_wrap_cache();
            }
            Transition::ExecuteCommand(cmd) => {
                return self.execute_command(&cmd);
            }
            Transition::ClipboardCopy => {
                if let Some(text) = self.vim.get_selection_text(&self.buffer) {
                    self.vim.set_yank_register(&text);
                    let _ = self.copy_to_clipboard(&text);
                }
                self.vim.set_mode(Mode::Normal);
                self.rebuild_wrap_cache();
            }
        }

        EditorAction::Continue
    }

    /// 处理模式特定的输入
    fn handle_mode_input(&mut self, input: &Input) {
        match self.vim.mode() {
            Mode::Command(cmd) => {
                let mut cmd = cmd.clone();
                match &input.key {
                    Key::Char(c) => cmd.push(*c),
                    Key::Backspace => {
                        cmd.pop();
                    }
                    _ => {} // 忽略其他按键（如功能键、组合键等）
                }
                self.vim.set_mode(Mode::Command(cmd));
            }
            Mode::Search(pattern) => {
                let mut pattern = pattern.clone();
                match &input.key {
                    Key::Char(c)
                        // 过滤控制字符（如 Esc 产生的 \x1b）
                        if !c.is_control() => {
                            pattern.push(*c);
                            self.search.search(&pattern, self.buffer.lines());
                        }
                    Key::Backspace => {
                        pattern.pop();
                        self.search.search(&pattern, self.buffer.lines());
                    }
                    _ => {} // 忽略其他按键（如功能键、组合键等）
                }
                self.vim.set_mode(Mode::Search(pattern));
            }
            Mode::CommandPanel(filter) => {
                let mut filter = filter.clone();
                match &input.key {
                    Key::Char(c) => {
                        filter.push(*c);
                        self.cmd_popup_selected = 0;
                    }
                    Key::Backspace => {
                        if !filter.is_empty() {
                            filter.pop();
                            self.cmd_popup_selected = 0;
                        } else {
                            self.vim.set_mode(Mode::Normal);
                            return;
                        }
                    }
                    _ => {} // 忽略其他按键（如功能键、组合键等）
                }
                self.vim.set_mode(Mode::CommandPanel(filter));
            }
            _ => {}
        }
    }

    /// 撤销
    pub fn undo(&mut self) {
        if let Some(snap) = self.vim.undo() {
            self.buffer.replace_lines(snap.lines.clone());
            self.buffer.set_cursor(snap.cursor.0, snap.cursor.1);
            self.rebuild_wrap_cache();
        }
    }

    /// 重做
    pub fn redo(&mut self) {
        if let Some(snap) = self.vim.redo() {
            self.buffer.replace_lines(snap.lines.clone());
            self.buffer.set_cursor(snap.cursor.0, snap.cursor.1);
            self.rebuild_wrap_cache();
        }
    }

    /// 搜索下一个匹配
    pub fn search_next(&mut self) {
        self.search.next_match();
        if let Some(m) = self.search.current_match() {
            self.buffer.set_cursor(m.line, m.start);
        }
    }

    /// 搜索上一个匹配
    pub fn search_prev(&mut self) {
        self.search.prev_match();
        if let Some(m) = self.search.current_match() {
            self.buffer.set_cursor(m.line, m.start);
        }
    }

    /// 执行命令面板命令
    fn execute_command(&mut self, cmd: &str) -> EditorAction {
        let (name, arg) = parse_command(cmd);
        match name {
            "save" | "w" | "wq" | "x" => EditorAction::Submit(self.buffer.to_string()),
            "quit" | "q" => EditorAction::Cancel,
            "search" => {
                self.cursor_before_search = Some(self.buffer.cursor());
                self.vim.set_mode(Mode::Search(String::new()));
                EditorAction::Continue
            }
            "wrap" => {
                self.wrap.set_enabled(true);
                self.rebuild_wrap_cache();
                self.vim.set_mode(Mode::Normal);
                EditorAction::Continue
            }
            "nowrap" => {
                self.wrap.set_enabled(false);
                self.rebuild_wrap_cache();
                self.vim.set_mode(Mode::Normal);
                EditorAction::Continue
            }
            "jump" => {
                if let Ok(line_num) = arg.parse::<usize>()
                    && line_num > 0
                {
                    self.buffer.set_cursor(line_num - 1, 0);
                }
                self.rebuild_wrap_cache();
                self.vim.set_mode(Mode::Normal);
                EditorAction::Continue
            }
            "undo" => {
                self.undo();
                self.vim.set_mode(Mode::Normal);
                EditorAction::Continue
            }
            "redo" => {
                self.redo();
                self.vim.set_mode(Mode::Normal);
                EditorAction::Continue
            }
            "tohead" => {
                self.buffer.move_cursor_top();
                self.rebuild_wrap_cache();
                self.vim.set_mode(Mode::Normal);
                EditorAction::Continue
            }
            "toend" => {
                self.buffer.move_cursor_bottom();
                self.rebuild_wrap_cache();
                self.vim.set_mode(Mode::Normal);
                EditorAction::Continue
            }
            "theme" => {
                self.theme_popup_selected = self.theme_index;
                self.vim.set_mode(Mode::ThemeSelect);
                EditorAction::Continue
            }
            "line-number" => {
                self.renderer.set_show_line_numbers(true);
                self.vim.set_mode(Mode::Normal);
                EditorAction::Continue
            }
            "no-line-number" => {
                self.renderer.set_show_line_numbers(false);
                self.vim.set_mode(Mode::Normal);
                EditorAction::Continue
            }
            "help" => {
                self.vim.set_mode(Mode::HelpPopup);
                EditorAction::Continue
            }
            _ => {
                self.vim.set_mode(Mode::Normal);
                EditorAction::Continue
            }
        }
    }

    /// 处理帮助弹窗模式按键（任意键关闭）
    fn handle_help_popup(&mut self, _input: &Input) -> EditorAction {
        self.vim.set_mode(Mode::Normal);
        EditorAction::Continue
    }

    /// 处理主题选择模式按键
    fn handle_theme_select(&mut self, input: &Input) -> EditorAction {
        let count = self.theme_gallery.len();
        match input.key {
            Key::Esc => {
                self.vim.set_mode(Mode::Normal);
            }
            Key::Up => {
                if self.theme_popup_selected > 0 {
                    self.theme_popup_selected -= 1;
                } else {
                    self.theme_popup_selected = count - 1;
                }
            }
            Key::Down => {
                if self.theme_popup_selected < count - 1 {
                    self.theme_popup_selected += 1;
                } else {
                    self.theme_popup_selected = 0;
                }
            }
            Key::Enter => {
                let idx = self.theme_popup_selected;
                if idx < count {
                    self.theme_index = idx;
                    let (name, theme_id, new_theme) = &self.theme_gallery[idx];
                    self.theme = new_theme.clone();
                    self.renderer.set_theme(new_theme.clone());
                    self.selected_theme_id = Some(theme_id);
                    self.status_message = Some(format!("主题: {}", name));
                }
                self.vim.set_mode(Mode::Normal);
            }
            _ => {}
        }
        EditorAction::Continue
    }

    /// 重建折行缓存
    fn rebuild_wrap_cache(&mut self) {
        self.wrap.rebuild_cache(self.buffer.lines());
        // 同时使渲染器缓存失效
        self.renderer.invalidate_cache();
    }

    /// 更新滚动偏移（基于视觉位置）
    fn update_scroll_from_visual(&mut self, visual_pos: usize, viewport_height: usize) {
        if visual_pos < self.scroll_offset {
            self.scroll_offset = visual_pos;
        } else if visual_pos >= self.scroll_offset + viewport_height {
            self.scroll_offset = visual_pos - viewport_height + 1;
        }
    }

    // ========== 鼠标操作 ==========

    /// 将屏幕坐标转换为逻辑位置 (logical_row, logical_col)。
    ///
    /// 返回 `None` 表示点击在内容区域之外（边框、状态栏等）。
    fn screen_to_logical(
        &self,
        screen_x: u16,
        screen_y: u16,
        area: Rect,
    ) -> Option<(usize, usize)> {
        // 减去边框偏移，得到内容区域内的坐标
        let content_x = screen_x.saturating_sub(area.x + 1) as usize; // 左边框 1 列
        let content_y = screen_y.saturating_sub(area.y + 1) as usize; // 上边框 1 行

        let content_height = area.height.saturating_sub(3) as usize; // 上边框 + 下边框 + 状态栏
        let line_num_width = if self.renderer.is_show_line_numbers() {
            6
        } else {
            0
        };

        // 超出内容区域
        if content_y >= content_height {
            return None;
        }

        // 计算视觉行号
        let visual_row = content_y + self.scroll_offset;

        // 使用 wrap_engine 映射到逻辑行
        let (logical_row, _start_col) = self.wrap.visual_to_logical(visual_row);

        // 减去行号区域得到内容列
        let content_col = content_x.saturating_sub(line_num_width);

        // 获取该逻辑行的原始文本
        let line_text = self.buffer.line(logical_row)?;

        // 获取该逻辑行的所有视觉行，找到当前视觉行的子行偏移
        let vlines = self.wrap.get_cached_lines(logical_row);
        let line_visual_offset = self.wrap.visual_offset_of(logical_row);
        let sub_index = visual_row.saturating_sub(line_visual_offset);

        // 该视觉行的起始字符偏移
        let vl_start_col = vlines.get(sub_index).map(|vl| vl.start_col).unwrap_or(0);

        // 获取该视觉行实际渲染的文本段（从 start_col 开始的子串）
        let vl_text: String = line_text.chars().skip(vl_start_col).collect();

        // 将屏幕列转换为字符偏移（考虑宽字符）
        let logical_col = Self::screen_col_to_char_offset(&vl_text, content_col) + vl_start_col;

        // 限制到行尾
        let max_col = line_text.chars().count();
        let logical_col = logical_col.min(max_col);

        Some((logical_row, logical_col))
    }

    /// 将屏幕列号转换为字符偏移（考虑 CJK 等宽字符）。
    fn screen_col_to_char_offset(text: &str, screen_col: usize) -> usize {
        use crate::util::text::char_width;

        let mut acc_width = 0;
        for (i, ch) in text.chars().enumerate() {
            if acc_width >= screen_col {
                return i;
            }
            acc_width += char_width(ch);
        }
        text.chars().count()
    }

    /// 处理鼠标事件。
    pub fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some((row, col)) = self.screen_to_logical(mouse.column, mouse.row, area) {
                    // 点击有效区域：移动光标
                    self.vim.set_mode(Mode::Normal);
                    self.buffer.set_cursor(row, col);
                    self.mouse_anchor = Some((row, col));
                } else {
                    // 点击空白区域（边框、状态栏等）：取消选区
                    self.vim.set_mode(Mode::Normal);
                    self.mouse_anchor = None;
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some((row, col)) = self.screen_to_logical(mouse.column, mouse.row, area) {
                    if let Some(anchor) = self.mouse_anchor
                        && *self.vim.mode() != Mode::Visual
                    {
                        // 进入 Visual 模式，选区起点为按下位置
                        self.vim.set_mode(Mode::Visual);
                        self.vim.set_visual_start(anchor);
                    }
                    self.buffer.set_cursor(row, col);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.mouse_anchor = None;
            }
            MouseEventKind::ScrollUp => {
                let step = 3;
                self.scroll_offset = self.scroll_offset.saturating_sub(step);
                self.scroll_locked = true;
            }
            MouseEventKind::ScrollDown => {
                let step = 3;
                let content_height = area.height.saturating_sub(3) as usize;
                let total_visual = self.wrap.visual_line_count();
                let max_offset = total_visual.saturating_sub(content_height);
                self.scroll_offset = (self.scroll_offset + step).min(max_offset);
                self.scroll_locked = true;
            }
            _ => {}
        }
    }

    /// 复制文本到系统剪贴板
    fn copy_to_clipboard(&self, text: &str) -> Result<(), String> {
        use arboard::Clipboard;
        let mut clipboard = Clipboard::new().map_err(|e| format!("无法访问剪贴板: {e}"))?;
        clipboard
            .set_text(text)
            .map_err(|e| format!("复制到剪贴板失败: {e}"))?;
        Ok(())
    }

    // ========== 渲染 ==========

    /// 渲染编辑器
    pub fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
        // 计算可用内容区域
        let content_height = area.height.saturating_sub(3) as usize; // 边框 + 状态栏
        let content_width = area.width.saturating_sub(2) as usize; // 左右边框

        self.viewport_height = content_height;
        self.viewport_width = content_width;
        let line_num_width = if self.renderer.is_show_line_numbers() {
            6
        } else {
            0
        };
        let wrap_width = content_width.saturating_sub(line_num_width);
        self.wrap.set_width(wrap_width);

        // 重建折行元数据（视觉行计数 + 前缀和）
        if self.wrap.is_dirty() {
            self.rebuild_wrap_cache();
        }

        let (cursor_row, mut cursor_col) = self.buffer.cursor();
        let line_count = self.buffer.line_count();

        // Vim Normal 模式下光标不能在行尾（最后一个字符之后），
        // 需要限制到行内最后一个字符上，否则会渲染一个多余的空光标块
        if *self.vim.mode() == Mode::Normal {
            let line_len = self.buffer.current_line_len();
            if line_len > 0 {
                cursor_col = cursor_col.min(line_len - 1);
            }
        }

        // 确保代码块缓存有效（用于快速判断行是否在代码块内）
        self.renderer.ensure_cache_valid(self.buffer.lines());

        // 使用前缀和快速计算光标的视觉位置（O(1) 或 O(log n)）
        let cursor_visual_pos = self.wrap.logical_to_visual(cursor_row, cursor_col);

        // 基于视觉位置更新滚动偏移（滚轮滚动锁定时跳过）
        if !self.scroll_locked {
            self.update_scroll_from_visual(cursor_visual_pos, content_height);
        }

        // 计算视口范围内需要渲染的逻辑行（O(log n)）
        let first_visible_visual = self.scroll_offset;
        let last_visible_visual = self.scroll_offset + content_height;
        let (start_logical, _) = self.wrap.visual_to_logical(first_visible_visual);
        let (end_logical, _) = self.wrap.visual_to_logical(last_visible_visual);

        // 扩展范围以处理边界情况，确保光标行在范围内
        let render_start = start_logical.saturating_sub(2).min(cursor_row);
        let render_end = (end_logical + 3).min(line_count).max(cursor_row + 1);

        // 为视口范围构建详细视觉行缓存（只构建未缓存的行）
        self.wrap
            .build_range(self.buffer.lines(), render_start, render_end);

        // 使用前缀和获取渲染起始的视觉偏移（O(1)，替代旧的 O(n) 循环）
        let visual_offset = self.wrap.visual_offset_of(render_start);

        let mut all_visual_lines: Vec<Line<'static>> = Vec::new();
        let mut all_vl_meta: Vec<RenderedVL> = Vec::new();

        for logical_line in render_start..render_end {
            let is_cursor_line = logical_line == cursor_row;
            let cached = self.wrap.get_cached_lines(logical_line);

            for vl in cached {
                let is_insert_mode = *self.vim.mode() == Mode::Insert;
                let rendered = self.renderer.render_visual_line(
                    vl,
                    is_cursor_line,
                    if is_cursor_line {
                        Some(cursor_col)
                    } else {
                        None
                    },
                    &self.search,
                    &self.buffer,
                    wrap_width,
                    is_insert_mode,
                );
                let n = rendered.len();
                let meta_entry = RenderedVL {
                    logical_line,
                    start_col: vl.start_col,
                    end_col: vl.end_col,
                };
                for _ in 0..n {
                    all_vl_meta.push(meta_entry.clone());
                }
                all_visual_lines.extend(rendered);
            }
        }

        // Visual 模式：对选区范围内的行应用精确字符级高亮
        if *self.vim.mode() == Mode::Visual {
            let (vs_row, vs_col) = self.vim.visual_start();
            let (ve_row, ve_col) = (cursor_row, cursor_col);
            // 确保 start <= end
            let ((sr, sc), (er, ec)) = if vs_row < ve_row || (vs_row == ve_row && vs_col <= ve_col)
            {
                ((vs_row, vs_col), (ve_row, ve_col))
            } else {
                ((ve_row, ve_col), (vs_row, vs_col))
            };

            let sel_fg = self.theme.text_normal;
            let sel_bg = Color::DarkGray;
            let line_num_chars = if self.renderer.is_show_line_numbers() {
                6usize
            } else {
                0usize
            };

            for (idx, meta) in all_vl_meta.iter().enumerate() {
                // 计算该视觉行与选区 [sr,sc)-(er,ec) 的交集字符范围
                let (hl_start, hl_end) = visual_line_selection_range(meta, sr, sc, er, ec);
                if hl_start >= hl_end {
                    continue; // 无交集
                }

                // 转为视觉行内的局部字符偏移（相对于 vl.start_col）
                let local_start = hl_start.saturating_sub(meta.start_col);
                let local_end = hl_end.saturating_sub(meta.start_col);

                if let Some(line) = all_visual_lines.get_mut(idx) {
                    line.spans = rebuild_spans_with_selection(
                        &line.spans,
                        line_num_chars,
                        local_start,
                        local_end,
                        sel_fg,
                        sel_bg,
                    );
                }
            }
        }

        // 提取可见范围
        let scroll_in_rendered = self.scroll_offset.saturating_sub(visual_offset);
        let visible_start = scroll_in_rendered.min(all_visual_lines.len().saturating_sub(1));
        let visible_end = (scroll_in_rendered + content_height).min(all_visual_lines.len());

        let mut lines_to_render: Vec<Line<'static>> = if visible_start < all_visual_lines.len() {
            all_visual_lines[visible_start..visible_end].to_vec()
        } else {
            Vec::new()
        };

        // 填充空行
        for _ in lines_to_render.len()..content_height {
            lines_to_render.push(Line::from(Span::styled(
                "~",
                Style::default()
                    .fg(Color::DarkGray)
                    .bg(self.theme.bg_primary),
            )));
        }

        // 渲染主内容
        let border_color = self.vim.mode().border_color();
        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(self.theme.bg_primary));

        let paragraph = Paragraph::new(lines_to_render).block(block);
        f.render_widget(paragraph, area);

        // 渲染状态栏
        let status_bar = self.render_status_bar(area.width as usize);
        let status_area = Rect::new(0, area.height - 1, area.width, 1);
        let status_block = Block::default().style(Style::default().bg(self.theme.bg_primary));
        f.render_widget(Paragraph::new(status_bar).block(status_block), status_area);

        // 渲染命令/搜索栏
        if matches!(
            self.vim.mode(),
            Mode::Command(_) | Mode::Search(_) | Mode::CommandPanel(_)
        ) {
            let cmd_bar = self.render_command_bar();
            let cmd_area = Rect::new(0, area.height - 2, area.width, 1);
            let cmd_block = Block::default().style(Style::default().bg(self.theme.bg_primary));
            f.render_widget(Paragraph::new(cmd_bar).block(cmd_block), cmd_area);
        }

        // 渲染命令面板弹窗
        if let Mode::CommandPanel(filter) = self.vim.mode() {
            let filter = filter.clone();
            self.render_command_popup(f, &filter, area);
        }

        // 渲染主题选择弹窗
        if self.vim.mode() == &Mode::ThemeSelect {
            self.render_theme_popup(f, area);
        }

        // 渲染帮助弹窗
        if self.vim.mode() == &Mode::HelpPopup {
            self.render_help_popup(f, area);
        }
    }

    /// 渲染状态栏
    fn render_status_bar(&self, width: usize) -> Line<'static> {
        let mode_str = format!(" {} ", self.vim.mode());
        let (row, col) = self.buffer.cursor();
        let pos_str = format!(" {}:{} ", row + 1, col + 1);
        let wrap_str = if self.wrap.is_enabled() {
            " WRAP "
        } else {
            " NOWRAP "
        };
        let hints: String = if let Some(ref msg) = self.status_message {
            msg.clone()
        } else {
            " Ctrl+S 保存 | Ctrl+Q 取消 | / 命令面板 ".to_string()
        };

        let used_width = mode_str.len() + pos_str.len() + wrap_str.len() + hints.len();
        let separator = " ".repeat(width.saturating_sub(used_width));

        let hints_style = if self.status_message.is_some() {
            Style::default()
                .fg(self.theme.text_bold)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.text_dim)
        };

        Line::from(vec![
            Span::styled(
                mode_str,
                Style::default()
                    .fg(Color::Black)
                    .bg(self.vim.mode().border_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(pos_str, Style::default().fg(self.theme.text_dim)),
            Span::styled(wrap_str, Style::default().fg(self.theme.text_dim)),
            Span::styled(separator, Style::default().fg(self.theme.text_normal)),
            Span::styled(hints, hints_style),
        ])
    }

    /// 渲染命令栏
    fn render_command_bar(&self) -> Line<'static> {
        let cursor_style = Style::default()
            .fg(self.theme.cursor_fg)
            .bg(self.theme.cursor_bg)
            .add_modifier(Modifier::BOLD);
        let text_style = Style::default().fg(self.theme.text_normal);
        let hint_style = Style::default().fg(self.theme.text_dim);
        match self.vim.mode() {
            Mode::Command(cmd) => Line::from(vec![
                Span::styled(":", text_style),
                Span::styled(cmd.clone(), text_style),
                Span::styled(" ", cursor_style),
                Span::styled("  Esc:取消  Enter:执行", hint_style),
            ]),
            Mode::Search(pattern) => {
                let count = self.search.match_count();
                let hint = if pattern.is_empty() {
                    "  Esc:取消  Enter:跳到匹配".to_string()
                } else {
                    format!("  [{}匹配]  Esc:取消  Enter:跳转  n/N:上下条", count)
                };
                Line::from(vec![
                    Span::styled("/", Style::default().fg(Color::Magenta)),
                    Span::styled(pattern.clone(), text_style),
                    Span::styled(" ", cursor_style),
                    Span::styled(hint, hint_style),
                ])
            }
            Mode::CommandPanel(filter) => Line::from(vec![
                Span::styled("/", Style::default().fg(Color::Magenta)),
                Span::styled(filter.clone(), text_style),
                Span::styled(" ", cursor_style),
                Span::styled("  Esc:取消  Enter:执行", hint_style),
            ]),
            _ => Line::default(),
        }
    }

    /// 渲染命令面板弹窗
    fn render_command_popup(&mut self, f: &mut Frame<'_>, filter: &str, area: Rect) {
        let items = filter_commands(filter);
        if items.is_empty() {
            return;
        }

        let item_count = items.len();
        let popup_height = (item_count as u16 + 2).min(area.height.saturating_sub(4));

        // 宽度计算与渲染保持一致：
        //   pointer(2) + name(对齐到 max_name_w) + GAP(3) + desc(完整显示)
        // 旧实现的 bug：max_label_width 假设 name 后 3 空格间隔，但渲染却用
        // `format!("{:<10}", name)` 把 name 列硬编码为 10 字符——name 短于 10
        // 时弹窗宽度被低估、desc 被截；name 长于 10 时根本没有间隔，紧贴 desc。
        const POINTER_W: usize = 2;
        const GAP: usize = 3;
        let max_name_w = items
            .iter()
            .map(|cmd| unicode_width::UnicodeWidthStr::width(cmd.name))
            .max()
            .unwrap_or(0);
        let max_desc_w = items
            .iter()
            .map(|cmd| unicode_width::UnicodeWidthStr::width(cmd.desc))
            .max()
            .unwrap_or(0);
        let content_w = POINTER_W + max_name_w + GAP + max_desc_w;
        // +2 给左右边框；保底 16，避免空标题/极短列表时弹窗过窄
        let popup_width = ((content_w + 2) as u16)
            .max(16)
            .min(area.width.saturating_sub(4));

        // 位置：编辑区底部偏左
        let x = area.x + 2;
        let y = area
            .bottom()
            .saturating_sub(popup_height + 2) // 留出状态栏和命令栏
            .max(area.y + 2);
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        // 标题
        let title = if filter.is_empty() {
            " 命令面板 ".to_string()
        } else {
            format!(" 命令面板 [{}] ", filter)
        };

        // 确保选中项在范围内
        self.cmd_popup_selected = self.cmd_popup_selected.min(item_count.saturating_sub(1));

        // 构建列表项
        let accent = self.theme.md_h1;
        let popup_bg = self.theme.bg_primary;
        let dim_color = self.theme.text_dim;
        let label_ai = self.theme.label_ai;
        let gap_str = " ".repeat(GAP);
        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                let is_selected = i == self.cmd_popup_selected;
                let name_style = if is_selected {
                    Style::default().fg(label_ai).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(label_ai)
                };
                let desc_style = Style::default().fg(dim_color);
                let pointer = if is_selected { "❯ " } else { "  " };
                // name 列动态对齐到 max_name_w（命令名都是 ASCII，char 数等于显示宽度）。
                let name_padded = format!("{:<width$}", cmd.name, width = max_name_w);
                ListItem::new(Line::from(vec![
                    Span::styled(pointer.to_string(), name_style),
                    Span::styled(name_padded, name_style),
                    Span::raw(gap_str.clone()),
                    Span::styled(cmd.desc.to_string(), desc_style),
                ]))
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.cmd_popup_selected));

        let list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(accent))
                    .title(Span::styled(
                        title,
                        Style::default().fg(accent).add_modifier(Modifier::BOLD),
                    ))
                    .style(Style::default().bg(popup_bg)),
            )
            .highlight_style(
                Style::default()
                    .bg(accent)
                    .fg(popup_bg)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(Clear, popup_area);
        f.render_stateful_widget(list, popup_area, &mut list_state);
    }

    /// 渲染主题选择弹窗
    fn render_theme_popup(&mut self, f: &mut Frame<'_>, area: Rect) {
        let item_count = self.theme_gallery.len();
        if item_count == 0 {
            return;
        }

        let popup_height = (item_count as u16 + 2).min(area.height.saturating_sub(4));
        let popup_width = 28u16.min(area.width.saturating_sub(4));

        // 位置：编辑区底部偏左
        let x = area.x + 2;
        let y = area
            .bottom()
            .saturating_sub(popup_height + 2)
            .max(area.y + 2);
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        // 确保选中项在范围内
        self.theme_popup_selected = self.theme_popup_selected.min(item_count.saturating_sub(1));

        // 构建列表项
        let accent = self.theme.md_h1;
        let popup_bg = self.theme.bg_primary;
        let text_color = self.theme.text_normal;
        let current_color = self.theme.md_link;
        let list_items: Vec<ListItem> = self
            .theme_gallery
            .iter()
            .enumerate()
            .map(|(i, (name, _, _))| {
                let is_selected = i == self.theme_popup_selected;
                let is_current = i == self.theme_index;
                let pointer = if is_selected { "❯ " } else { "  " };
                let check = if is_current { " ●" } else { "" };
                let name_style = if is_selected {
                    Style::default().fg(text_color).add_modifier(Modifier::BOLD)
                } else if is_current {
                    Style::default().fg(current_color)
                } else {
                    Style::default().fg(text_color)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(pointer.to_string(), name_style),
                    Span::styled(format!("{}{}", name, check), name_style),
                ]))
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.theme_popup_selected));

        let list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(accent))
                    .title(Span::styled(
                        " 选择主题 ",
                        Style::default().fg(accent).add_modifier(Modifier::BOLD),
                    ))
                    .style(Style::default().bg(popup_bg)),
            )
            .highlight_style(
                Style::default()
                    .bg(accent)
                    .fg(popup_bg)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(Clear, popup_area);
        f.render_stateful_widget(list, popup_area, &mut list_state);
    }

    /// 渲染帮助页面（全屏覆盖编辑区域）
    fn render_help_popup(&mut self, f: &mut Frame<'_>, area: Rect) {
        let accent = self.theme.md_h1;
        let bg = self.theme.bg_primary;
        let text_color = self.theme.text_normal;
        let dim_color = self.theme.text_dim;

        // 辅助：快捷键行
        let key = |k: &str| -> Span<'static> {
            let padded = format!(" {:<10}", k);
            Span::styled(padded, Style::default().fg(accent).bg(bg))
        };
        let desc = |d: &str| -> Span<'static> {
            Span::styled(d.to_string(), Style::default().fg(text_color).bg(bg))
        };
        let section = |s: &str| -> Line<'static> {
            Line::from(Span::styled(
                format!("  ── {} ──", s),
                Style::default().fg(dim_color).bg(bg),
            ))
        };
        let blank = || -> Line<'static> { Line::from(Span::styled(" ", Style::default().bg(bg))) };

        let help_lines: Vec<Line<'static>> = vec![
            Line::from(Span::styled(
                "  Markdown 编辑器帮助指南",
                Style::default()
                    .fg(accent)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("  {}", "─".repeat(area.width.saturating_sub(4) as usize)),
                Style::default().fg(dim_color).bg(bg),
            )),
            blank(),
            section("模式切换"),
            Line::from(vec![key("i"), desc("进入 Insert 模式（编辑文本）")]),
            Line::from(vec![key("Esc"), desc("退出到 Normal 模式")]),
            Line::from(vec![key("v"), desc("进入 Visual 模式（选择文本）")]),
            blank(),
            section("光标移动"),
            Line::from(vec![key("h/j/k/l"), desc("左 / 下 / 上 / 右")]),
            Line::from(vec![key("w / b / e"), desc("下一个词 / 上一个词 / 词尾")]),
            Line::from(vec![key("0 / $"), desc("行首 / 行尾")]),
            Line::from(vec![key("gg / G"), desc("文档开头 / 结尾")]),
            Line::from(vec![key("Ctrl-D/U"), desc("下 / 上翻半页")]),
            blank(),
            section("编辑操作"),
            Line::from(vec![key("d"), desc("删除当前行")]),
            Line::from(vec![key("x"), desc("删除当前字符")]),
            Line::from(vec![key("p"), desc("粘贴（yank 寄存器）")]),
            Line::from(vec![key("u"), desc("撤销")]),
            Line::from(vec![key("Ctrl-r"), desc("重做")]),
            blank(),
            section("Visual 选区"),
            Line::from(vec![key("y"), desc("Yank 到内部寄存器")]),
            Line::from(vec![key("c"), desc("复制到系统剪贴板")]),
            blank(),
            section("鼠标操作"),
            Line::from(vec![key("左键点击"), desc("定位光标")]),
            Line::from(vec![key("左键拖拽"), desc("选择文本（进入 Visual）")]),
            Line::from(vec![key("滚轮"), desc("滚动视口")]),
            blank(),
            section("搜索"),
            Line::from(vec![key("/"), desc("开始搜索")]),
            Line::from(vec![key("n / N"), desc("下一个 / 上一个匹配")]),
            blank(),
            section("命令面板 (:)"),
            Line::from(vec![key("wrap"), desc("启用自动折行")]),
            Line::from(vec![key("nowrap"), desc("禁用折行")]),
            Line::from(vec![key("theme"), desc("切换主题")]),
            Line::from(vec![key("help"), desc("显示帮助")]),
            Line::from(vec![key("line-number"), desc("显示行号")]),
            Line::from(vec![key("no-line-number"), desc("隐藏行号")]),
            blank(),
            section("全局快捷键"),
            Line::from(vec![key("Ctrl-s"), desc("保存并退出")]),
            Line::from(vec![key("Ctrl-q"), desc("取消退出")]),
        ];

        // 渲染帮助内容（留出底部状态栏 1 行）
        let content_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
        let paragraph = Paragraph::new(help_lines).style(Style::default().bg(bg));

        f.render_widget(Clear, content_area);
        f.render_widget(paragraph, content_area);

        // 底部状态栏：提示按任意键返回
        let status_y = area.y + area.height.saturating_sub(1);
        let status_area = Rect::new(area.x, status_y, area.width, 1);
        let status_line = Line::from(vec![
            Span::styled(
                " HELP ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" 按任意键返回编辑器", Style::default().fg(dim_color).bg(bg)),
        ]);
        f.render_widget(Clear, status_area);
        f.render_widget(
            Paragraph::new(status_line).style(Style::default().bg(bg)),
            status_area,
        );
    }
}

/// 编辑器动作
#[derive(Debug)]
pub enum EditorAction {
    /// 继续编辑
    Continue,
    /// 提交内容
    Submit(String),
    /// 取消编辑
    Cancel,
}

// ========== 公共 API ==========

/// Markdown 编辑器共享配置参数（封装 title/theme/highlight_fn/theme_gallery）
pub struct MarkdownEditorOpts<'a> {
    pub title: &'a str,
    pub theme: EditorTheme,
    pub highlight_fn: HighlightFn,
    pub theme_gallery: Vec<ThemeGalleryItem>,
    /// 初始光标策略（默认 StartOfFile）
    pub cursor_policy: CursorPolicy,
}

/// 打开 Markdown 编辑器（在已有终端上）
pub fn open_markdown_editor_on_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    opts: &MarkdownEditorOpts,
    content: &str,
) -> io::Result<(Option<String>, Option<&'static str>)> {
    let mut editor = MarkdownEditor::new(
        opts.title,
        content,
        opts.theme.clone(),
        opts.highlight_fn,
        opts.theme_gallery.clone(),
        opts.cursor_policy.clone(),
    );

    loop {
        let size = terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);

        terminal.draw(|f| {
            editor.render(f, area);
        })?;

        if event::poll(std::time::Duration::from_millis(EDITOR_POLL_MS))? {
            let evt = event::read()?;

            if let Event::Key(key) = evt {
                let input = Input::from_keycode(key.code, key.modifiers);

                match editor.handle_input(&input) {
                    EditorAction::Submit(content) => {
                        return Ok((Some(content), editor.selected_theme_id()));
                    }
                    EditorAction::Cancel => return Ok((None, editor.selected_theme_id())),
                    EditorAction::Continue => {}
                }
            } else if let Event::Mouse(mouse) = evt {
                editor.handle_mouse(mouse, area);
            }
        }
    }
}

/// 打开 Markdown 编辑器（独立终端）
pub fn open_markdown_editor(
    opts: &MarkdownEditorOpts,
    content: &str,
) -> io::Result<(Option<String>, Option<&'static str>)> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        event::EnableMouseCapture // 启用鼠标事件捕获
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = open_markdown_editor_on_terminal(&mut terminal, opts, content);

    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        event::DisableMouseCapture // 禁用鼠标事件捕获
    )?;

    result
}

/// 打开 Markdown 编辑器（带预填充内容，NORMAL 模式）
pub fn open_markdown_editor_with_content(
    opts: &MarkdownEditorOpts,
    initial_lines: &[String],
) -> io::Result<(Option<String>, Option<&'static str>)> {
    let content = initial_lines.join("\n");
    open_markdown_editor(opts, &content)
}

// ========== Visual 选区辅助函数 ==========

/// 渲染元数据（记录每个已渲染视觉行对应的逻辑行号和起止列）。
#[derive(Clone)]
struct RenderedVL {
    logical_line: usize,
    start_col: usize,
    end_col: usize,
}

/// 计算视觉行与选区 `[sr,sc)-(er,ec)` 的交集字符范围。
///
/// 返回 `(hl_start, hl_end)`——需要高亮的逻辑列范围（闭区间左、开区间右）。
/// 若无交集，返回 `(0, 0)`。
fn visual_line_selection_range(
    meta: &RenderedVL,
    sr: usize,
    sc: usize,
    er: usize,
    ec: usize,
) -> (usize, usize) {
    let ll = meta.logical_line;
    let vl_start = meta.start_col;
    let vl_end = meta.end_col;

    // 逻辑行完全在选区中间 → 整个视觉行都高亮
    if ll > sr && ll < er {
        return (vl_start, vl_end);
    }

    // 起始行 == 结束行：视觉行与 [sc, ec) 求交集
    if ll == sr && ll == er {
        let hl_start = vl_start.max(sc);
        let hl_end = vl_end.min(ec);
        return (hl_start, hl_end);
    }

    // 仅起始行：高亮 [sc, ∞) ∩ 视觉行范围
    if ll == sr {
        let hl_start = vl_start.max(sc);
        let hl_end = vl_end;
        if hl_start < vl_end {
            return (hl_start, hl_end);
        }
        return (0, 0);
    }

    // 仅结束行：高亮 [0, ec) ∩ 视觉行范围
    if ll == er {
        let hl_start = vl_start;
        let hl_end = vl_end.min(ec);
        if vl_start < hl_end {
            return (hl_start, hl_end);
        }
        return (0, 0);
    }

    (0, 0)
}
