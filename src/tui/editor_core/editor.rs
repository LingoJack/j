//! 自研 Markdown 编辑器
//!
//! 完全摆脱 tui-textarea 依赖，支持自动折行、Vim 模式等。

mod api;
mod commands;
mod render;
mod selection;

pub use api::{
    MarkdownEditorOpts, open_markdown_editor, open_markdown_editor_on_terminal,
    open_markdown_editor_with_content,
};
pub use selection::RenderedVL;

use super::{
    history::Snapshot,
    renderer::MarkdownRenderer,
    search::SearchState,
    text_buffer::TextBuffer,
    theme::{EditorTheme, HighlightFn},
    vim::{CmdItem, Input, Key, Mode, Transition, Vim, filter_commands, filter_insert_commands},
    wrap_engine::WrapEngine,
};

#[cfg(test)]
use super::theme::BorderStyle;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

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

/// 视口/滚动状态
struct ViewportState {
    /// 垂直滚动偏移（视觉行级别）
    scroll_offset: usize,
    /// 视口高度
    height: usize,
    /// 视口宽度
    width: usize,
    /// 滚轮滚动锁定：防止 render() 自动将视口拉回到光标位置
    scroll_locked: bool,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            scroll_offset: 0,
            height: 20,
            width: 80,
            scroll_locked: false,
        }
    }
}

/// 主题管理状态
struct ThemeState {
    /// 主题画廊（名称 + 主题列表）
    gallery: Vec<ThemeGalleryItem>,
    /// 当前主题在画廊中的索引
    current_index: usize,
    /// 主题选择弹窗选中项索引
    popup_selected: usize,
    /// 用户在主题画廊中选择的主题ID（退出时返回）
    selected_id: Option<&'static str>,
}

/// 渲染行元数据映射
#[derive(Default)]
struct RenderMeta {
    /// 每个屏幕行对应一个 RenderedVL（每次渲染时更新，用于鼠标点击定位）
    vl_map: Vec<RenderedVL>,
    /// 当前屏幕顶部对应的渲染行索引（在 vl_map 中的偏移）
    map_index: usize,
    /// 上一次渲染时的实际渲染行总数（用于鼠标滚动计算 max_offset）
    /// 注意：wrap_engine 的 visual_line_count() 是源码行的视觉行计数，
    /// 但表格等元素渲染后会产生更多行。此字段存储实际渲染输出行数，
    /// 确保鼠标滚动能到达表格底部。
    rendered_line_count: usize,
}

/// 编辑器主结构
pub struct MarkdownEditor {
    // ---- 核心引擎 ----
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

    // ---- 分组状态 ----
    /// 视口/滚动状态
    viewport: ViewportState,
    /// 主题管理状态
    themes: ThemeState,
    /// 渲染行元数据映射
    render_meta: RenderMeta,

    // ---- UI 杂项 ----
    /// 标题
    title: String,
    /// 命令面板选中项索引
    cmd_popup_selected: usize,
    /// 状态消息（短暂显示，下次按键清除）
    status_message: Option<String>,
    /// 进入搜索前的光标位置，用于 Esc 恢复
    cursor_before_search: Option<(usize, usize)>,
    /// Insert 模式命令面板的锚点（触发的 `/` 字符在 buffer 中的逻辑位置）
    /// 用于把 popup 渲染到 `/` 下方，而不是固定在编辑区底部
    insert_panel_anchor: Option<(usize, usize)>,
    /// 鼠标拖拽锚点（左键按下时的逻辑位置）
    mouse_anchor: Option<(usize, usize)>,
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
            viewport: ViewportState::default(),
            themes: ThemeState {
                gallery: theme_gallery,
                current_index: theme_index,
                popup_selected: theme_index,
                selected_id: None,
            },
            render_meta: RenderMeta::default(),
            title: title.to_string(),
            cmd_popup_selected: 0,
            status_message: None,
            cursor_before_search: None,
            insert_panel_anchor: None,
            mouse_anchor: None,
        }
    }

    /// 获取用户选择的主题ID（退出时读取）
    pub fn selected_theme_id(&self) -> Option<&'static str> {
        self.themes.selected_id
    }

    /// 获取编辑器当前全部文本内容
    pub fn content(&self) -> String {
        self.buffer.lines().join("\n")
    }

    /// 判断编辑器是否处于"空闲 Normal 模式"（可安全拦截 Esc）
    ///
    /// 当 vim 处于 Normal 模式且无活跃搜索时返回 true。
    /// 此状态下 Esc 对编辑器无实际作用，外部可安全拦截用于焦点切换。
    pub fn is_idle_normal_mode(&self) -> bool {
        self.vim.mode() == &Mode::Normal && !self.search.is_searching()
    }

    /// 获取光标所在的视觉行
    pub fn cursor_visual_line(&self) -> usize {
        let (row, col) = self.buffer.cursor();
        self.wrap.logical_to_visual(row, col)
    }

    /// 视觉行上移（折行感知）
    pub fn move_cursor_visual_up(&mut self) {
        use crate::util::text::char_width;

        let current_visual = self.cursor_visual_line();
        if current_visual == 0 {
            return;
        }

        // 当前光标所在 logical 是否处于表格块续行（不该停留），先把光标拉回首行
        let (cursor_row, _) = self.buffer.cursor();
        if let Some((tbl_start, _tbl_end)) = self.wrap.table_block_for_line(cursor_row)
            && cursor_row != tbl_start
        {
            self.buffer.set_cursor(tbl_start, 0);
            // 直接以 tbl_start 为基准再触发一次上移
            return self.move_cursor_visual_up();
        }

        let target_visual = current_visual - 1;

        // 目标视觉行如果落在某个表格块的"膨胀区"（视觉行 N..N+height 之间），
        // 跳到表格首行的最后一个有效列；这是穿越表格上行的语义。
        if let Some((tbl_start, _)) = self.wrap.table_block_for_visual_row(target_visual) {
            let line = self.buffer.line(tbl_start).map_or("", |v| v);
            let end_col = line.chars().count();
            self.buffer.set_cursor(tbl_start, end_col);
            return;
        }

        let (current_row, current_col) = self.buffer.cursor();

        // 确保目标行的缓存已构建
        let (target_logical, _) = self.wrap.visual_to_logical(target_visual);
        self.wrap
            .build_range(self.buffer.lines(), target_logical, target_logical + 1);

        if let Some(target_vl) = self.wrap.get_visual_line(target_visual) {
            let logical_line = target_vl.logical_line;
            let end_col = target_vl.end_col;
            let start_col = target_vl.start_col;

            // 保持视觉列位置：计算当前光标在当前视觉行中的屏幕偏移
            let current_vl = self.wrap.get_visual_line(current_visual);
            let current_start_col = current_vl.map(|vl| vl.start_col).unwrap_or(0);
            let current_line_text = self.buffer.line(current_row).map_or("", |v| v);
            let visual_x: usize = current_line_text
                .chars()
                .skip(current_start_col)
                .take(current_col.saturating_sub(current_start_col))
                .map(char_width)
                .sum();

            // 在目标视觉行中找到最接近该视觉 X 的逻辑列
            let target_line_text = self.buffer.line(logical_line).map_or("", |v| v);
            let new_col = if target_line_text.is_empty() {
                0
            } else {
                let segment: String = target_line_text.chars().skip(start_col).collect();
                Self::screen_col_to_char_offset(&segment, visual_x) + start_col
            };
            let new_col = new_col.min(end_col);
            self.buffer.set_cursor(logical_line, new_col);
        }
    }

    /// 视觉行下移（折行感知）
    pub fn move_cursor_visual_down(&mut self) {
        use crate::util::text::char_width;

        let current_visual = self.cursor_visual_line();
        let total_visual = self.wrap.visual_line_count();

        // 当前光标所在 logical 是否处于表格块续行（不该停留），先把光标拉到首行
        let (cursor_row, _) = self.buffer.cursor();
        if let Some((tbl_start, _tbl_end)) = self.wrap.table_block_for_line(cursor_row)
            && cursor_row != tbl_start
        {
            self.buffer.set_cursor(tbl_start, 0);
            return self.move_cursor_visual_down();
        }

        // 当前光标在表格首行：穿越整张表，跳到表格末行 + 1（如果存在）
        if let Some((tbl_start, tbl_end)) = self.wrap.table_block_for_line(cursor_row)
            && cursor_row == tbl_start
        {
            let after = tbl_end + 1;
            if after < self.buffer.line_count() {
                self.buffer.set_cursor(after, 0);
                return;
            }
            // 表格在 EOF：cursor 不动，但视口往下推一格，让用户能看到表格底部。
            // `scroll_locked = true` 防止下一帧光标同步把视口拉回去。
            let max_offset = total_visual.saturating_sub(1);
            if self.viewport.scroll_offset < max_offset {
                self.viewport.scroll_offset += 1;
                self.viewport.scroll_locked = true;
            }
            return;
        }

        if current_visual >= total_visual.saturating_sub(1) {
            return;
        }
        let target_visual = current_visual + 1;

        // 目标视觉行如果落在某个表格块的"膨胀区"，把光标放到该表格末行 + 1
        // （或者 EOF 时停在表格首行，由下一次 j 触发上面的"表格首行"分支）。
        if let Some((tbl_start, tbl_end)) = self.wrap.table_block_for_visual_row(target_visual) {
            let after = tbl_end + 1;
            if after < self.buffer.line_count() {
                self.buffer.set_cursor(after, 0);
            } else {
                self.buffer.set_cursor(tbl_start, 0);
            }
            return;
        }

        let (current_row, current_col) = self.buffer.cursor();

        // 确保目标行的缓存已构建
        let (target_logical, _) = self.wrap.visual_to_logical(target_visual);
        self.wrap
            .build_range(self.buffer.lines(), target_logical, target_logical + 1);

        if let Some(target_vl) = self.wrap.get_visual_line(target_visual) {
            let logical_line = target_vl.logical_line;
            let end_col = target_vl.end_col;
            let start_col = target_vl.start_col;

            // 保持视觉列位置：计算当前光标在当前视觉行中的屏幕偏移
            let current_vl = self.wrap.get_visual_line(current_visual);
            let current_start_col = current_vl.map(|vl| vl.start_col).unwrap_or(0);
            let current_line_text = self.buffer.line(current_row).map_or("", |v| v);
            let visual_x: usize = current_line_text
                .chars()
                .skip(current_start_col)
                .take(current_col.saturating_sub(current_start_col))
                .map(char_width)
                .sum();

            // 在目标视觉行中找到最接近该视觉 X 的逻辑列
            let target_line_text = self.buffer.line(logical_line).map_or("", |v| v);
            let new_col = if target_line_text.is_empty() {
                0
            } else {
                let segment: String = target_line_text.chars().skip(start_col).collect();
                Self::screen_col_to_char_offset(&segment, visual_x) + start_col
            };
            let new_col = new_col.min(end_col);
            self.buffer.set_cursor(logical_line, new_col);
        }
    }

    // ========== 输入处理 ==========

    /// 处理输入
    pub fn handle_input(&mut self, input: &Input) -> EditorAction {
        // 键盘输入解除滚动锁定
        self.viewport.scroll_locked = false;

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
        //
        // 同时处理两种面板：
        //  - CommandPanel：Normal 模式触发，命令列表 = COMMANDS
        //  - InsertCommandPanel：Insert 模式触发，命令列表 = INSERT_COMMANDS
        {
            #[derive(Clone, Copy)]
            enum PanelKind {
                Normal,
                Insert,
            }
            let panel_state: Option<(PanelKind, String)> = match self.vim.mode() {
                Mode::CommandPanel(f) => Some((PanelKind::Normal, f.clone())),
                Mode::InsertCommandPanel(f) => Some((PanelKind::Insert, f.clone())),
                _ => None,
            };
            if let Some((kind, filter)) = panel_state {
                let filtered: Vec<&'static CmdItem> = match kind {
                    PanelKind::Normal => filter_commands(&filter),
                    PanelKind::Insert => filter_insert_commands(&filter),
                };
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
                            match kind {
                                PanelKind::Normal => {
                                    let full_cmd = if cmd.name == "jump" {
                                        filter
                                    } else {
                                        cmd.name.to_string()
                                    };
                                    return self.execute_command(&full_cmd);
                                }
                                PanelKind::Insert => {
                                    return self.execute_insert_command(cmd.name, &filter);
                                }
                            }
                        }
                        // 没有匹配项：恢复到对应的来源模式
                        match kind {
                            PanelKind::Normal => self.vim.set_mode(Mode::Normal),
                            PanelKind::Insert => {
                                self.vim.set_mode(Mode::Insert);
                                self.insert_panel_anchor = None;
                            }
                        }
                        return EditorAction::Continue;
                    }
                    Key::Esc => {
                        // Insert 面板：保留已插入的 / 与 filter 文本，回到 Insert
                        // Normal 面板：保持原有行为，由 vim 状态机处理（fallthrough）
                        if matches!(kind, PanelKind::Insert) {
                            self.vim.set_mode(Mode::Insert);
                            self.insert_panel_anchor = None;
                            return EditorAction::Continue;
                        }
                    }
                    _ => {} // 其他按键交由后续 handle_mode_input 处理
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
                // 进入 Insert 命令面板时记录触发的 `/` 字符位置（光标已前进 1 位）
                if old_mode == Mode::Insert && matches!(new_mode, Mode::InsertCommandPanel(_)) {
                    let (row, col) = self.buffer.cursor();
                    self.insert_panel_anchor = Some((row, col.saturating_sub(1)));
                }
                self.vim.set_mode(new_mode);
                self.rebuild_wrap_cache();
            }
            Transition::Submit => {
                return EditorAction::Submit(self.buffer.to_string());
            }
            Transition::Save => {
                return EditorAction::Save(self.buffer.to_string());
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
            Mode::InsertCommandPanel(filter) => {
                // Insert 模式专用面板：触发 `/` 已经写入 buffer。
                // 这里继续把字符同步插入 buffer + filter，让用户的真实输入和面板状态一致。
                let mut filter = filter.clone();
                match &input.key {
                    Key::Char(c) => {
                        // `/` 字符已经走 vim::handle_insert_mode 的分支被插入；
                        // 这里只处理后续字符。
                        self.buffer.insert_char(*c);
                        filter.push(*c);
                        self.cmd_popup_selected = 0;

                        // 如果新输入后没有任何匹配项，自动关闭面板回到 Insert
                        // （让用户能够正常打 `https://` 之类的真实文本）
                        if filter_insert_commands(&filter).is_empty() {
                            self.vim.set_mode(Mode::Insert);
                            self.insert_panel_anchor = None;
                            self.rebuild_wrap_cache();
                            return;
                        }
                        self.rebuild_wrap_cache();
                    }
                    Key::Backspace => {
                        if !filter.is_empty() {
                            // 同步从 buffer 删除最后一个 filter 字符
                            self.buffer.backspace();
                            filter.pop();
                            self.cmd_popup_selected = 0;
                            self.rebuild_wrap_cache();
                        } else {
                            // filter 为空 → 删除触发的 `/`，回到 Insert
                            self.buffer.backspace();
                            self.vim.set_mode(Mode::Insert);
                            self.insert_panel_anchor = None;
                            self.rebuild_wrap_cache();
                            return;
                        }
                    }
                    _ => {}
                }
                self.vim.set_mode(Mode::InsertCommandPanel(filter));
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

    /// 重建折行缓存
    fn rebuild_wrap_cache(&mut self) {
        // 先确保代码块缓存有效，获取代码块范围
        self.renderer.ensure_cache_valid(self.buffer.lines());
        let cb_ranges = self.renderer.code_block_content_ranges();

        // 计算表格渲染高度，灌进 wrap_engine。`wrap_width` 用上一帧 render() 缓存的
        // viewport.width 推算（首帧 viewport.width 为默认 80，差几个像素无关紧要——
        // 第二帧 set_width 不同会触发 `dirty`，自动重建）。
        let line_num_width = if self.renderer.is_show_line_numbers() {
            6
        } else {
            0
        };
        let wrap_width = self.viewport.width.saturating_sub(line_num_width).max(10);
        let table_blocks = self
            .renderer
            .compute_table_block_heights(self.buffer.lines(), wrap_width);

        self.wrap
            .rebuild_cache_with_blocks(self.buffer.lines(), &cb_ranges, &table_blocks);
        // 同时使渲染器缓存失效（语法高亮等）
        self.renderer.invalidate_cache();
    }

    /// 计算编辑区可用的内容行数。
    ///
    /// 必须与 `render()` 中的 `content_height` 计算保持一致：
    ///  - 顶部 block border 1 行
    ///  - 底部状态栏 1 行（直接覆盖 block 底边框）
    ///  - 命令栏可见时再多占 1 行
    ///
    /// 之前误写为 `area.height - 3`，导致内容区底部少了一行可显示位置，
    /// 表现为代码块/普通文本最末一行被替换成 `~` 占位符。
    fn viewport_content_height(&self, area: Rect) -> usize {
        let has_cmd_bar = matches!(
            self.vim.mode(),
            Mode::Command(_) | Mode::Search(_) | Mode::CommandPanel(_)
        );
        let reserved: u16 = if has_cmd_bar { 3 } else { 2 };
        area.height.saturating_sub(reserved) as usize
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

        let content_height = self.viewport_content_height(area);
        let line_num_width = if self.renderer.is_show_line_numbers() {
            6
        } else {
            0
        };

        // 超出内容区域
        if content_y >= content_height {
            return None;
        }

        // 使用渲染行元数据映射，将屏幕行号转换为渲染行索引
        let rendered_row = content_y + self.render_meta.map_index;

        // 如果点击位置超出实际渲染内容（落在空白填充 `~` 行），
        // 则把光标移动到最后一行的末尾（符合常见编辑器行为）
        let vl_meta = match self.render_meta.vl_map.get(rendered_row) {
            Some(meta) => meta,
            None => {
                // 没有内容可点击，定位到最后一个有效渲染行
                let last_meta = self.render_meta.vl_map.last()?;
                let line_text = self.buffer.line(last_meta.logical_line)?;
                let max_col = line_text.chars().count();
                return Some((last_meta.logical_line, max_col));
            }
        };

        let logical_row = vl_meta.logical_line;
        let vl_start_col = vl_meta.start_col;

        // 减去行号区域得到内容列
        let content_col_pre_deco = content_x.saturating_sub(line_num_width);

        // 减去渲染装饰列：代码块内容行渲染为 `│ <code> │`，左侧装饰占 2 列；
        // 不补这个偏移，鼠标点击映射到的源码列就比真实位置后移 2 个字符
        // （症状：选 `vscode` 复制成 `code .`）。
        let deco_left_cols = self.row_left_decoration_cols(logical_row);
        let content_col = content_col_pre_deco.saturating_sub(deco_left_cols);

        // 获取该逻辑行的原始文本
        let line_text = self.buffer.line(logical_row)?;

        // 获取该视觉行实际渲染的文本段（从 start_col 开始的子串）
        let vl_text: String = line_text.chars().skip(vl_start_col).collect();

        // 将屏幕列转换为字符偏移（考虑宽字符）
        let logical_col = Self::screen_col_to_char_offset(&vl_text, content_col) + vl_start_col;

        // 限制到行尾
        let max_col = line_text.chars().count();
        let logical_col = logical_col.min(max_col);

        Some((logical_row, logical_col))
    }

    /// 给定源码逻辑行号，返回其渲染时左侧装饰占用的屏幕列数（除行号外）。
    ///
    /// 当前覆盖：
    /// - 代码块内容行（含围栏行内部）：`│ ` 占 2 列
    /// - 其他行：0
    ///
    /// 用于把鼠标点击的"屏幕列"还原成"源码字符列"。围栏行视觉上是
    /// `╭───<lang>───╮`（顶/底边框），用户基本不会在围栏行点选文字；
    /// 这里保守返回 0，落点是行首（行尾），不会出现偏移导致的复制错误。
    fn row_left_decoration_cols(&self, logical_row: usize) -> usize {
        if self.wrap.is_code_block_line(logical_row) {
            2 // `│ `
        } else {
            0
        }
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
                    // 点击有效区域：移动光标，解除滚动锁定让视口跟随
                    self.viewport.scroll_locked = false;
                    self.vim.set_mode(Mode::Normal);
                    self.buffer.set_cursor(row, col);
                    self.mouse_anchor = Some((row, col));
                } else {
                    // 点击空白区域（边框、状态栏等）：取消选区
                    self.viewport.scroll_locked = false;
                    self.vim.set_mode(Mode::Normal);
                    self.mouse_anchor = None;
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some((row, col)) = self.screen_to_logical(mouse.column, mouse.row, area) {
                    // 拖拽时也解除滚动锁定，确保视口跟随光标移动
                    self.viewport.scroll_locked = false;
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
                // 鼠标滚轮按 step 视觉行驱动光标，再让 render 自动追到视口；
                // 这样无论文档是否满屏都能给出一致的"光标随滚动"反馈。
                self.viewport.scroll_locked = false;
                for _ in 0..3 {
                    self.move_cursor_visual_up();
                }
            }
            MouseEventKind::ScrollDown => {
                self.viewport.scroll_locked = false;
                for _ in 0..3 {
                    self.move_cursor_visual_down();
                }
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
}

/// 编辑器动作
#[derive(Debug)]
pub enum EditorAction {
    /// 继续编辑
    Continue,
    /// 提交内容（保存并退出）
    Submit(String),
    /// 保存内容但不退出
    Save(String),
    /// 取消编辑
    Cancel,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;
    use ratatui::text::Span;

    fn test_theme() -> EditorTheme {
        EditorTheme {
            bg_primary: Color::Reset,
            bg_input: Color::Reset,
            code_bg: Color::DarkGray,
            cursor_fg: Color::Black,
            cursor_bg: Color::Cyan,
            text_normal: Color::White,
            text_dim: Color::DarkGray,
            text_bold: Color::White,
            md_h1: Color::Cyan,
            md_h2: Color::Green,
            md_h3: Color::Yellow,
            md_h4: Color::Magenta,
            md_heading_sep: Color::DarkGray,
            md_link: Color::Blue,
            md_list_bullet: Color::Yellow,
            md_blockquote_bar: Color::Cyan,
            md_blockquote_bg: Color::DarkGray,
            md_blockquote_text: Color::Gray,
            md_inline_code_fg: Color::Magenta,
            md_inline_code_bg: Color::DarkGray,
            md_rule: Color::DarkGray,
            code_border: Color::DarkGray,
            code_border_style: BorderStyle::default(),
            table_header: Color::White,
            table_body: Color::White,
            code_default: Color::White,
            code_keyword: Color::Magenta,
            code_string: Color::Green,
            code_comment: Color::DarkGray,
            code_number: Color::Yellow,
            code_type: Color::Yellow,
            code_primitive: Color::Cyan,
            code_macro: Color::LightCyan,
            code_lifetime: Color::LightMagenta,
            code_attribute: Color::LightBlue,
            code_shell_var: Color::LightCyan,
            label_ai: Color::Green,
        }
    }

    fn noop_highlight(_: &str, _: &str, _: &EditorTheme) -> Vec<Span<'static>> {
        Vec::new()
    }

    #[test]
    fn scroll_down_moves_cursor_three_visual_lines() {
        let content = (0..20)
            .map(|idx| format!("line {idx}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut editor = MarkdownEditor::new(
            "test",
            &content,
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );
        let area = Rect::new(0, 0, 80, 10);

        editor.buffer.set_cursor(0, 0);
        editor.viewport.scroll_offset = 0;
        editor.viewport.scroll_locked = true;

        editor.handle_mouse(
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 0,
                row: 0,
                modifiers: crossterm::event::KeyModifiers::empty(),
            },
            area,
        );

        // 滚轮下滚 → 光标按 3 视觉行下移；render() 自动追视口，因此解锁。
        assert_eq!(editor.buffer.cursor().0, 3);
        assert!(!editor.viewport.scroll_locked);
    }

    #[test]
    fn move_cursor_visual_down_skips_table_block_to_line_after() {
        // 文档：3 行普通文字 + 4 行表格 + 1 行结尾
        let content = "para1\npara2\npara3\n| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\nepilogue";
        let mut editor = MarkdownEditor::new(
            "test",
            content,
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );
        // 触发一次重建，让 wrap_engine 灌入表格高度（构造函数里的 rebuild_cache 不带表格信息）
        editor.viewport.width = 78;
        editor.rebuild_wrap_cache();

        // 光标定位到表格首行（line 3）
        editor.buffer.set_cursor(3, 0);
        assert_eq!(editor.buffer.cursor().0, 3);

        // 按一次 j：从表格首行应跳到表格末行 + 1 = line 7（epilogue）
        editor.move_cursor_visual_down();
        assert_eq!(
            editor.buffer.cursor().0,
            7,
            "光标在表格首行按 j 应跳过整张表到 line 7 (epilogue)"
        );
    }

    #[test]
    fn move_cursor_visual_down_at_eof_table_pushes_viewport() {
        // 文档：仅 1 行普通文字 + 4 行表格（表格在 EOF）
        let content = "header\n| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |";
        let mut editor = MarkdownEditor::new(
            "test",
            content,
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );
        editor.viewport.width = 78;
        editor.viewport.height = 6;
        editor.rebuild_wrap_cache();

        // 光标定位到表格首行（line 1）
        editor.buffer.set_cursor(1, 0);
        let initial_offset = editor.viewport.scroll_offset;

        // 在 EOF 表格首行按 j：光标不动（无后继行），但视口 scroll_offset 应推进 1 行，
        // 并且 scroll_locked 设为 true（避免下一帧把视口拉回光标位置）。
        editor.move_cursor_visual_down();
        assert_eq!(
            editor.buffer.cursor().0,
            1,
            "EOF 表格无后继行，光标应留在表格首行"
        );
        assert!(
            editor.viewport.scroll_offset > initial_offset,
            "scroll_offset 应被推进；现在 = {}, 初始 = {}",
            editor.viewport.scroll_offset,
            initial_offset
        );
        assert!(
            editor.viewport.scroll_locked,
            "推视口后必须 scroll_locked=true，否则下一帧光标同步会把它拉回去"
        );
    }

    /// Visual 选区复制测试：character-wise visual mode 两端均闭合，
    /// 屏幕上看到的高亮 + 光标块 = 复制到剪贴板的内容。
    #[test]
    fn visual_selection_copy_includes_cursor_char_single_line() {
        let mut editor = MarkdownEditor::new(
            "test",
            "hello",
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );

        // 模拟：光标在 (0, 0)，按 v 后再按 l 一次：visual_start=(0,0), cursor=(0,1)
        editor.buffer.set_cursor(0, 0);
        editor.vim.set_visual_start((0, 0));
        editor.vim.set_mode(Mode::Visual);
        editor.buffer.set_cursor(0, 1);

        let copied = editor.vim.get_selection_text(&editor.buffer);
        assert_eq!(
            copied.as_deref(),
            Some("he"),
            "v + l 应选中 'he'（cursor 块字符也算选中）"
        );
    }

    #[test]
    fn visual_selection_copy_inclusive_at_line_end() {
        let mut editor = MarkdownEditor::new(
            "test",
            "hello",
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );

        // 光标在 (0, 0)，按 v 后跳到行末 cursor=(0, 4) on 'o'
        editor.buffer.set_cursor(0, 0);
        editor.vim.set_visual_start((0, 0));
        editor.vim.set_mode(Mode::Visual);
        editor.buffer.set_cursor(0, 4);

        let copied = editor.vim.get_selection_text(&editor.buffer);
        assert_eq!(
            copied.as_deref(),
            Some("hello"),
            "v + 跳到 'o' 应选中 'hello'"
        );
    }

    #[test]
    fn visual_selection_copy_past_line_end_no_overshoot() {
        let mut editor = MarkdownEditor::new(
            "test",
            "hello",
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );

        // 光标越过行末（cursor=(0, 5) = line_len），不应在 'o' 后再多 +1 越界
        editor.buffer.set_cursor(0, 0);
        editor.vim.set_visual_start((0, 0));
        editor.vim.set_mode(Mode::Visual);
        editor.buffer.set_cursor(0, 5);

        let copied = editor.vim.get_selection_text(&editor.buffer);
        assert_eq!(
            copied.as_deref(),
            Some("hello"),
            "光标已在 line_len 处不再 +1，避免越界 panic 或多取字符"
        );
    }

    #[test]
    fn visual_selection_copy_multi_line_inclusive_end() {
        let mut editor = MarkdownEditor::new(
            "test",
            "hello\nworld",
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );

        // 起点 (0,0)，cursor 在第二行第 2 个字符 (1, 2) on 'r'
        editor.buffer.set_cursor(0, 0);
        editor.vim.set_visual_start((0, 0));
        editor.vim.set_mode(Mode::Visual);
        editor.buffer.set_cursor(1, 2);

        let copied = editor.vim.get_selection_text(&editor.buffer);
        assert_eq!(
            copied.as_deref(),
            Some("hello\nwor"),
            "多行选区结尾包含 cursor 字符 'r'"
        );
    }

    #[test]
    fn visual_selection_copy_chinese_inclusive() {
        let mut editor = MarkdownEditor::new(
            "test",
            "你好世界",
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );

        // 起点 (0,0)，cursor 在 '世' (col 2)
        editor.buffer.set_cursor(0, 0);
        editor.vim.set_visual_start((0, 0));
        editor.vim.set_mode(Mode::Visual);
        editor.buffer.set_cursor(0, 2);

        let copied = editor.vim.get_selection_text(&editor.buffer);
        assert_eq!(
            copied.as_deref(),
            Some("你好世"),
            "中文按字符切片，cursor 块所在字也算选中"
        );
    }

    /// 鼠标点击代码块内字符时，screen_col → logical_col 必须扣掉左侧
    /// 装饰列（`│ ` 占 2 列）。否则点 `vscode` 的 'v' 实际落到 'c'（偏移 +2），
    /// 后续选区复制就会变成错位的内容（例如 `code .`）。
    #[test]
    fn screen_to_logical_subtracts_code_block_decoration() {
        // 文档：3 行围栏 + 1 行内容（在代码块内）
        let content = "```bash\nset vscode \"/Applications/Visual Studio Code.app\"\n```";
        let mut editor = MarkdownEditor::new(
            "test",
            content,
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );
        // 让 wrap_engine 重新加载 cb_ranges，code_block_lines 才能正确填充
        editor.viewport.width = 78;
        editor.rebuild_wrap_cache();

        // 行 1 是代码块内容行 → is_code_block_line == true
        assert!(
            editor.wrap.is_code_block_line(1),
            "行 1 应被识别为代码块内容行；当前 wrap.code_block_lines = {:?}",
            (0..3)
                .map(|i| editor.wrap.is_code_block_line(i))
                .collect::<Vec<_>>()
        );

        // 装饰列 = 2（`│ `）
        assert_eq!(editor.row_left_decoration_cols(1), 2);
        assert_eq!(editor.row_left_decoration_cols(0), 0); // 围栏行不计装饰
    }
}
