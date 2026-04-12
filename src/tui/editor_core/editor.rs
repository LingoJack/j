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

use crossterm::{
    event::{self, Event},
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

/// 主题画廊项（名称 + 主题）
pub type ThemeGalleryItem = (&'static str, EditorTheme);

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
}

impl MarkdownEditor {
    /// 创建新的编辑器
    pub fn new(
        title: &str,
        content: &str,
        theme: EditorTheme,
        highlight_fn: HighlightFn,
        theme_gallery: Vec<ThemeGalleryItem>,
    ) -> Self {
        let buffer = TextBuffer::from_content(content);
        let initial_mode = if content.is_empty() {
            Mode::Insert
        } else {
            Mode::Normal
        };

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
            .position(|(_, t)| *t == theme)
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
        }
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
        // 清除状态消息
        self.status_message = None;

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
        }

        // 命令面板模式：拦截上下键移动选中项
        if let Mode::CommandPanel(filter) = self.vim.mode() {
            let filtered = filter_commands(filter);
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
                _ => {}
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
                self.vim.set_mode(new_mode);
                self.rebuild_wrap_cache();
            }
            Transition::Submit => {
                return EditorAction::Submit(self.buffer.to_string());
            }
            Transition::Cancel => {
                return EditorAction::Cancel;
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
                    _ => {}
                }
                self.vim.set_mode(Mode::Command(cmd));
            }
            Mode::Search(pattern) => {
                let mut pattern = pattern.clone();
                match &input.key {
                    Key::Char(c) => {
                        pattern.push(*c);
                        self.search.search(&pattern, self.buffer.lines());
                    }
                    Key::Backspace => {
                        pattern.pop();
                        self.search.search(&pattern, self.buffer.lines());
                    }
                    _ => {}
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
                    _ => {}
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
            _ => {
                self.vim.set_mode(Mode::Normal);
                EditorAction::Continue
            }
        }
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
                    let (name, new_theme) = &self.theme_gallery[idx];
                    self.theme = new_theme.clone();
                    self.renderer.set_theme(new_theme.clone());
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

    // ========== 渲染 ==========

    /// 渲染编辑器
    pub fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
        // 计算可用内容区域
        let content_height = area.height.saturating_sub(3) as usize; // 边框 + 状态栏
        let content_width = area.width.saturating_sub(2) as usize; // 左右边框

        self.viewport_height = content_height;
        self.viewport_width = content_width;
        let wrap_width = content_width.saturating_sub(6); // 6 = 行号宽度
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

        // 基于视觉位置更新滚动偏移
        self.update_scroll_from_visual(cursor_visual_pos, content_height);

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

        for logical_line in render_start..render_end {
            let is_cursor_line = logical_line == cursor_row;
            let cached = self.wrap.get_cached_lines(logical_line);

            for vl in cached {
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
                );
                all_visual_lines.extend(rendered);
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
        match self.vim.mode() {
            Mode::Command(cmd) => Line::from(vec![
                Span::styled(":", Style::default().fg(self.theme.text_normal)),
                Span::styled(cmd.clone(), Style::default().fg(self.theme.text_normal)),
                Span::styled(" ", Style::default().fg(self.theme.text_normal)),
            ]),
            Mode::Search(pattern) => Line::from(vec![
                Span::styled("/", Style::default().fg(Color::Magenta)),
                Span::styled(pattern.clone(), Style::default().fg(self.theme.text_normal)),
                Span::styled(" ", Style::default().fg(self.theme.text_normal)),
            ]),
            Mode::CommandPanel(filter) => Line::from(vec![
                Span::styled("/", Style::default().fg(Color::Magenta)),
                Span::styled(filter.clone(), Style::default().fg(self.theme.text_normal)),
                Span::styled(" ", Style::default().fg(self.theme.text_normal)),
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

        // 计算宽度
        let max_label_width = items
            .iter()
            .map(|cmd| {
                2 + unicode_width::UnicodeWidthStr::width(cmd.name)
                    + 3
                    + unicode_width::UnicodeWidthStr::width(cmd.desc)
            })
            .max()
            .unwrap_or(16)
            .max(16);
        let popup_width = (max_label_width as u16 + 2).min(area.width.saturating_sub(4));

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
        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                let is_selected = i == self.cmd_popup_selected;
                let name_style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let desc_style = if is_selected {
                    Style::default().fg(Color::Gray)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let pointer = if is_selected { "❯ " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::styled(pointer.to_string(), name_style),
                    Span::styled(format!("{:<10}", cmd.name), name_style),
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
                    .border_style(Style::default().fg(Color::Magenta))
                    .title(Span::styled(
                        title,
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .style(Style::default().bg(Color::Black)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Magenta)
                    .fg(Color::White)
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
        let list_items: Vec<ListItem> = self
            .theme_gallery
            .iter()
            .enumerate()
            .map(|(i, (name, _))| {
                let is_selected = i == self.theme_popup_selected;
                let is_current = i == self.theme_index;
                let pointer = if is_selected { "❯ " } else { "  " };
                let check = if is_current { " ●" } else { "" };
                let name_style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else if is_current {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
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
                    .border_style(Style::default().fg(Color::Magenta))
                    .title(Span::styled(
                        " 选择主题 ",
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .style(Style::default().bg(Color::Black)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Magenta)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(Clear, popup_area);
        f.render_stateful_widget(list, popup_area, &mut list_state);
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

/// 打开 Markdown 编辑器（在已有终端上）
pub fn open_markdown_editor_on_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    title: &str,
    content: &str,
    theme: &EditorTheme,
    highlight_fn: HighlightFn,
    theme_gallery: Vec<ThemeGalleryItem>,
) -> io::Result<Option<String>> {
    let mut editor =
        MarkdownEditor::new(title, content, theme.clone(), highlight_fn, theme_gallery);

    loop {
        let size = terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);

        terminal.draw(|f| {
            editor.render(f, area);
        })?;

        if event::poll(std::time::Duration::from_millis(16))? {
            let evt = event::read()?;

            if let Event::Key(key) = evt {
                let input = Input::from_keycode(key.code, key.modifiers);

                match editor.handle_input(&input) {
                    EditorAction::Submit(content) => return Ok(Some(content)),
                    EditorAction::Cancel => return Ok(None),
                    EditorAction::Continue => {}
                }
            }
        }
    }
}

/// 打开 Markdown 编辑器（独立终端）
pub fn open_markdown_editor(
    title: &str,
    content: &str,
    theme: &EditorTheme,
    highlight_fn: HighlightFn,
    theme_gallery: Vec<ThemeGalleryItem>,
) -> io::Result<Option<String>> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = open_markdown_editor_on_terminal(
        &mut terminal,
        title,
        content,
        theme,
        highlight_fn,
        theme_gallery,
    );

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

/// 打开 Markdown 编辑器（带预填充内容，NORMAL 模式）
pub fn open_markdown_editor_with_content(
    title: &str,
    initial_lines: &[String],
    theme: &EditorTheme,
    highlight_fn: HighlightFn,
    theme_gallery: Vec<ThemeGalleryItem>,
) -> io::Result<Option<String>> {
    let content = initial_lines.join("\n");
    open_markdown_editor(title, &content, theme, highlight_fn, theme_gallery)
}
