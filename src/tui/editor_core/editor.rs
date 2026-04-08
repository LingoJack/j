//! 自研 Markdown 编辑器
//!
//! 完全摆脱 tui-textarea 依赖，支持自动折行、Vim 模式等。

use super::{
    history::Snapshot,
    renderer::MarkdownRenderer,
    search::SearchState,
    text_buffer::TextBuffer,
    vim::{Input, Key, Mode, Transition, Vim},
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
    widgets::{Block, Borders, Paragraph},
};
use std::io;

use crate::command::chat::theme::Theme;

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
    theme: Theme,
    /// 标题
    title: String,
    /// 视口垂直滚动偏移（视觉行级别）
    scroll_offset: usize,
    /// 视口高度
    viewport_height: usize,
    /// 视口宽度
    viewport_width: usize,
}

impl MarkdownEditor {
    /// 创建新的编辑器
    pub fn new(title: &str, content: &str, theme: Theme) -> Self {
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

        let renderer = MarkdownRenderer::new(theme.clone());

        let viewport_width: usize = 80; // 默认值，会在渲染时更新
        wrap.set_width(viewport_width.saturating_sub(6));

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
        }
    }

    /// 创建带指定初始模式的编辑器
    pub fn with_mode(title: &str, content: &str, theme: Theme, initial_mode: Mode) -> Self {
        let mut editor = Self::new(title, content, theme);
        editor.vim.set_mode(initial_mode);
        editor
    }

    /// 获取当前模式
    pub fn mode(&self) -> &Mode {
        self.vim.mode()
    }

    /// 获取光标位置（逻辑）
    pub fn cursor(&self) -> (usize, usize) {
        self.buffer.cursor()
    }

    /// 获取光标所在行
    pub fn cursor_line(&self) -> usize {
        self.buffer.cursor().0
    }

    /// 获取光标所在列
    pub fn cursor_col(&self) -> usize {
        self.buffer.cursor().1
    }

    /// 获取所有行
    pub fn lines(&self) -> &[String] {
        self.buffer.lines()
    }

    /// 获取可变缓冲区引用
    pub fn buffer_mut(&mut self) -> &mut TextBuffer {
        &mut self.buffer
    }

    /// 获取缓冲区引用
    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
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

    /// 获取视觉行总数
    pub fn visual_line_count(&self) -> usize {
        self.wrap.visual_line_count()
    }

    /// 设置折行宽度
    pub fn set_wrap_width(&mut self, width: usize) {
        self.wrap.set_width(width);
    }

    /// 设置折行开关
    pub fn set_wrap_enabled(&mut self, enabled: bool) {
        self.wrap.set_enabled(enabled);
    }

    /// 刷新折行缓存
    pub fn refresh_wrap(&mut self) {
        self.wrap.rebuild_if_needed(self.buffer.lines());
    }

    /// 获取文本内容
    pub fn content(&self) -> String {
        self.buffer.to_string()
    }

    /// 是否已修改
    pub fn is_modified(&self) -> bool {
        self.buffer.is_modified()
    }

    // ========== 输入处理 ==========

    /// 处理输入
    pub fn handle_input(&mut self, input: &Input) -> EditorAction {
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
        if self.vim.mode() == &Mode::Normal && !self.search.pattern.is_empty() {
            if input.key == Key::Char('n') && !input.ctrl {
                self.search_next();
                return EditorAction::Continue;
            }
            if input.key == Key::Char('N') && !input.ctrl {
                self.search_prev();
                return EditorAction::Continue;
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

        let (cursor_row, cursor_col) = self.buffer.cursor();
        let line_count = self.buffer.line_count();

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
                let line = self.renderer.render_visual_line(
                    vl,
                    is_cursor_line,
                    if is_cursor_line {
                        Some(cursor_col)
                    } else {
                        None
                    },
                    self.vim.mode(),
                    &self.search,
                    &self.buffer,
                    wrap_width,
                );
                all_visual_lines.push(line);
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
        if matches!(self.vim.mode(), Mode::Command(_) | Mode::Search(_)) {
            let cmd_bar = self.render_command_bar();
            let cmd_area = Rect::new(0, area.height - 2, area.width, 1);
            let cmd_block = Block::default().style(Style::default().bg(self.theme.bg_primary));
            f.render_widget(Paragraph::new(cmd_bar).block(cmd_block), cmd_area);
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
        let hints = " Ctrl+S 保存 | Ctrl+Q 取消 | :wq 提交 ";

        let used_width = mode_str.len() + pos_str.len() + wrap_str.len() + hints.len();
        let separator = " ".repeat(width.saturating_sub(used_width));

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
            Span::styled(hints, Style::default().fg(self.theme.text_dim)),
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
            _ => Line::default(),
        }
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
    theme: &Theme,
) -> io::Result<Option<String>> {
    let mut editor = MarkdownEditor::new(title, content, theme.clone());

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
    theme: &Theme,
) -> io::Result<Option<String>> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = open_markdown_editor_on_terminal(&mut terminal, title, content, theme);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

/// 打开 Markdown 编辑器（带预填充内容，NORMAL 模式）
pub fn open_markdown_editor_with_content(
    title: &str,
    initial_lines: &[String],
    theme: &Theme,
) -> io::Result<Option<String>> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let content = initial_lines.join("\n");
    let result = open_markdown_editor_on_terminal_internal(
        &mut terminal,
        title,
        &content,
        theme,
        Mode::Normal,
    );

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

/// 内部函数：支持指定初始模式
fn open_markdown_editor_on_terminal_internal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    title: &str,
    content: &str,
    theme: &Theme,
    initial_mode: Mode,
) -> io::Result<Option<String>> {
    let mut editor = MarkdownEditor::with_mode(title, content, theme.clone(), initial_mode);

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
