use crossterm::{
    event::{self, Event},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::fmt;
use std::io;
use tui_textarea::{CursorMove, Input, Key, TextArea};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

// ========== Vim 模式定义 ==========

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    Normal,
    Insert,
    Visual,
    Operator(char),
    Command(String), // 命令行模式，存储输入的命令字符串
    Search(String),  // 搜索模式，存储输入的搜索词
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Visual => write!(f, "VISUAL"),
            Self::Operator(c) => write!(f, "OPERATOR({})", c),
            Self::Command(_) => write!(f, "COMMAND"),
            Self::Search(_) => write!(f, "SEARCH"),
        }
    }
}

impl Mode {
    fn cursor_style(&self) -> Style {
        let color = match self {
            Self::Normal => Color::Reset,
            Self::Insert => Color::LightBlue,
            Self::Visual => Color::LightYellow,
            Self::Operator(_) => Color::LightGreen,
            Self::Command(_) => Color::Reset,
            Self::Search(_) => Color::Reset,
        };
        Style::default().fg(color).add_modifier(Modifier::REVERSED)
    }

    fn border_color(&self) -> Color {
        match self {
            Self::Normal => Color::DarkGray,
            Self::Insert => Color::Cyan,
            Self::Visual => Color::LightYellow,
            Self::Operator(_) => Color::LightGreen,
            Self::Command(_) => Color::DarkGray,
            Self::Search(_) => Color::Magenta,
        }
    }
}

// ========== 搜索状态 ==========

/// 搜索匹配结果
#[derive(Debug, Clone)]
struct SearchMatch {
    line: usize,
    start: usize,
    #[allow(dead_code)]
    end: usize, // 保留用于将来可能的高亮增强
}

/// 搜索状态管理
#[derive(Debug, Clone, Default)]
struct SearchState {
    pattern: String,
    matches: Vec<SearchMatch>,
    current_index: usize,
}

impl SearchState {
    fn new() -> Self {
        Self::default()
    }

    /// 执行搜索，返回匹配数量
    fn search(&mut self, pattern: &str, lines: &[String]) -> usize {
        self.pattern = pattern.to_string();
        self.matches.clear();
        self.current_index = 0;

        if pattern.is_empty() {
            return 0;
        }

        for (line_idx, line) in lines.iter().enumerate() {
            let mut start = 0;
            while let Some(pos) = line[start..].find(pattern) {
                let abs_start = start + pos;
                self.matches.push(SearchMatch {
                    line: line_idx,
                    start: abs_start,
                    end: abs_start + pattern.len(),
                });
                start = abs_start + pattern.len();
                if start >= line.len() {
                    break;
                }
            }
        }

        self.matches.len()
    }

    /// 跳转到下一个匹配，返回 (line, column)
    fn next_match(&mut self) -> Option<(usize, usize)> {
        if self.matches.is_empty() {
            return None;
        }
        let m = &self.matches[self.current_index];
        let result = Some((m.line, m.start));
        self.current_index = (self.current_index + 1) % self.matches.len();
        result
    }

    /// 跳转到上一个匹配，返回 (line, column)
    fn prev_match(&mut self) -> Option<(usize, usize)> {
        if self.matches.is_empty() {
            return None;
        }
        if self.current_index == 0 {
            self.current_index = self.matches.len() - 1;
        } else {
            self.current_index -= 1;
        }
        let m = &self.matches[self.current_index];
        Some((m.line, m.start))
    }

    /// 获取当前匹配信息（用于状态栏显示）
    fn current_info(&self) -> (usize, usize) {
        if self.matches.is_empty() {
            (0, 0)
        } else {
            let display_idx = if self.current_index == 0 {
                self.matches.len()
            } else {
                self.current_index
            };
            (display_idx, self.matches.len())
        }
    }
}

// ========== 搜索高亮函数 ==========

/// 应用搜索高亮到 buffer（直接修改 buffer 样式）
fn apply_search_highlight(buf: &mut ratatui::buffer::Buffer, area: Rect, search: &SearchState) {
    if search.pattern.is_empty() || search.matches.is_empty() {
        return;
    }

    let pattern = &search.pattern;

    // 遍历 buffer 查找 pattern
    for row in area.top()..area.bottom() {
        let content_start = area.left() + 3; // 跳过行号
        let mut chars_with_pos: Vec<(char, u16)> = Vec::new();

        for col in content_start..area.right() {
            if let Some(cell) = buf.cell((col, row)) {
                let symbol = cell.symbol();
                for c in symbol.chars() {
                    chars_with_pos.push((c, col));
                }
            }
        }

        let pattern_chars: Vec<char> = pattern.chars().collect();
        if pattern_chars.is_empty() {
            continue;
        }

        let mut i = 0;
        while i + pattern_chars.len() <= chars_with_pos.len() {
            let is_match = pattern_chars.iter().enumerate().all(|(j, pc)| {
                chars_with_pos
                    .get(i + j)
                    .map(|(c, _)| c == pc)
                    .unwrap_or(false)
            });

            if is_match {
                // 匹配文字用红色显示
                let style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);

                for j in 0..pattern_chars.len() {
                    if let Some((_, col)) = chars_with_pos.get(i + j) {
                        buf[(*col, row)].set_style(style);
                    }
                }

                i += pattern_chars.len();
            } else {
                i += 1;
            }
        }
    }
}

/// 跳转到指定行和列
fn jump_to_match(textarea: &mut TextArea, line: usize, col: usize) {
    textarea.move_cursor(CursorMove::Jump(
        line.try_into().unwrap_or(0),
        col.try_into().unwrap_or(0),
    ));
}

// ========== 编辑器状态转换 ==========

enum Transition {
    Nop,
    Mode(Mode),
    Pending(Input),
    Submit,         // 提交内容
    Quit,           // 强制取消退出（:q! / Ctrl+Q）
    TryQuit,        // 尝试退出，若有改动则拒绝（:q）
    Search(String), // 执行搜索
    NextMatch,      // 跳转到下一个匹配
    PrevMatch,      // 跳转到上一个匹配
}

/// Vim 状态机
struct Vim {
    mode: Mode,
    pending: Input,
    search: SearchState,
}

impl Vim {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            pending: Input::default(),
            search: SearchState::new(),
        }
    }

    fn with_pending(self, pending: Input) -> Self {
        Self {
            mode: self.mode,
            pending,
            search: self.search,
        }
    }

    /// 处理 Normal/Visual/Operator 模式的按键
    fn transition(&self, input: Input, textarea: &mut TextArea<'_>) -> Transition {
        if input.key == Key::Null {
            return Transition::Nop;
        }

        // 任何模式下 Ctrl+S → 提交
        if input.ctrl && input.key == Key::Char('s') {
            return Transition::Submit;
        }

        // 任何模式下 Ctrl+Q → 强制取消退出
        if input.ctrl && input.key == Key::Char('q') {
            return Transition::Quit;
        }

        match &self.mode {
            Mode::Command(cmd) => self.handle_command_mode(input, cmd),
            Mode::Search(pattern) => self.handle_search_mode(input, pattern),
            Mode::Insert => self.handle_insert_mode(input, textarea),
            Mode::Normal | Mode::Visual | Mode::Operator(_) => {
                self.handle_normal_visual_operator(input, textarea)
            }
        }
    }

    /// Insert 模式：Esc 回到 Normal，其他交给 textarea 默认处理
    fn handle_insert_mode(&self, input: Input, textarea: &mut TextArea<'_>) -> Transition {
        match input {
            Input { key: Key::Esc, .. }
            | Input {
                key: Key::Char('c'),
                ctrl: true,
                ..
            } => Transition::Mode(Mode::Normal),
            input => {
                textarea.input(input);
                Transition::Mode(Mode::Insert)
            }
        }
    }

    /// Command 模式：处理 :wq, :w, :q, :q! 等命令
    fn handle_command_mode(&self, input: Input, cmd: &str) -> Transition {
        match input.key {
            Key::Esc => Transition::Mode(Mode::Normal),
            Key::Enter => {
                // 执行命令
                let cmd = cmd.trim();
                match cmd {
                    "wq" | "x" => Transition::Submit,
                    "w" => Transition::Submit,
                    "q" => Transition::TryQuit, // 有改动时拒绝退出
                    "q!" => Transition::Quit,   // 强制退出
                    _ => Transition::Mode(Mode::Normal), // 未知命令，回到 Normal
                }
            }
            Key::Backspace => {
                if cmd.is_empty() {
                    Transition::Mode(Mode::Normal)
                } else {
                    let mut new_cmd = cmd.to_string();
                    new_cmd.pop();
                    Transition::Mode(Mode::Command(new_cmd))
                }
            }
            Key::Char(c) => {
                let mut new_cmd = cmd.to_string();
                new_cmd.push(c);
                Transition::Mode(Mode::Command(new_cmd))
            }
            _ => Transition::Nop,
        }
    }

    /// Search 模式：处理 /pattern 输入
    fn handle_search_mode(&self, input: Input, pattern: &str) -> Transition {
        match input.key {
            Key::Esc => Transition::Mode(Mode::Normal),
            Key::Enter => {
                // 执行搜索
                Transition::Search(pattern.to_string())
            }
            Key::Backspace => {
                if pattern.is_empty() {
                    Transition::Mode(Mode::Normal)
                } else {
                    let mut new_pattern = pattern.to_string();
                    new_pattern.pop();
                    Transition::Mode(Mode::Search(new_pattern))
                }
            }
            Key::Char(c) => {
                let mut new_pattern = pattern.to_string();
                new_pattern.push(c);
                Transition::Mode(Mode::Search(new_pattern))
            }
            _ => Transition::Nop,
        }
    }

    /// Normal / Visual / Operator 模式的 vim 按键处理
    fn handle_normal_visual_operator(
        &self,
        input: Input,
        textarea: &mut TextArea<'_>,
    ) -> Transition {
        match input {
            // : 进入命令模式
            Input {
                key: Key::Char(':'),
                ctrl: false,
                ..
            } if self.mode == Mode::Normal => {
                return Transition::Mode(Mode::Command(String::new()));
            }
            // / 进入搜索模式
            Input {
                key: Key::Char('/'),
                ctrl: false,
                ..
            } if self.mode == Mode::Normal => {
                return Transition::Mode(Mode::Search(String::new()));
            }
            // n 跳转到下一个匹配
            Input {
                key: Key::Char('n'),
                ctrl: false,
                ..
            } if self.mode == Mode::Normal => {
                return Transition::NextMatch;
            }
            // N 跳转到上一个匹配
            Input {
                key: Key::Char('N'),
                ctrl: false,
                ..
            } if self.mode == Mode::Normal => {
                return Transition::PrevMatch;
            }
            // 移动
            Input {
                key: Key::Char('h'),
                ..
            } => textarea.move_cursor(CursorMove::Back),
            Input {
                key: Key::Char('j'),
                ..
            } => textarea.move_cursor(CursorMove::Down),
            Input {
                key: Key::Char('k'),
                ..
            } => textarea.move_cursor(CursorMove::Up),
            Input {
                key: Key::Char('l'),
                ..
            } => textarea.move_cursor(CursorMove::Forward),
            Input {
                key: Key::Char('w'),
                ..
            } => textarea.move_cursor(CursorMove::WordForward),
            Input {
                key: Key::Char('e'),
                ctrl: false,
                ..
            } => {
                textarea.move_cursor(CursorMove::WordEnd);
                if matches!(self.mode, Mode::Operator(_)) {
                    textarea.move_cursor(CursorMove::Forward);
                }
            }
            Input {
                key: Key::Char('b'),
                ctrl: false,
                ..
            } => textarea.move_cursor(CursorMove::WordBack),
            Input {
                key: Key::Char('^' | '0'),
                ..
            } => textarea.move_cursor(CursorMove::Head),
            Input {
                key: Key::Char('$'),
                ..
            } => textarea.move_cursor(CursorMove::End),
            // 删除 / 修改
            Input {
                key: Key::Char('D'),
                ..
            } => {
                textarea.delete_line_by_end();
                return Transition::Mode(Mode::Normal);
            }
            Input {
                key: Key::Char('C'),
                ..
            } => {
                textarea.delete_line_by_end();
                textarea.cancel_selection();
                return Transition::Mode(Mode::Insert);
            }
            Input {
                key: Key::Char('p'),
                ..
            } => {
                textarea.paste();
                return Transition::Mode(Mode::Normal);
            }
            Input {
                key: Key::Char('u'),
                ctrl: false,
                ..
            } => {
                textarea.undo();
                return Transition::Mode(Mode::Normal);
            }
            Input {
                key: Key::Char('r'),
                ctrl: true,
                ..
            } => {
                textarea.redo();
                return Transition::Mode(Mode::Normal);
            }
            Input {
                key: Key::Char('x'),
                ..
            } => {
                textarea.delete_next_char();
                return Transition::Mode(Mode::Normal);
            }
            // 进入 Insert 模式
            Input {
                key: Key::Char('i'),
                ..
            } => {
                textarea.cancel_selection();
                return Transition::Mode(Mode::Insert);
            }
            Input {
                key: Key::Char('a'),
                ctrl: false,
                ..
            } => {
                textarea.cancel_selection();
                textarea.move_cursor(CursorMove::Forward);
                return Transition::Mode(Mode::Insert);
            }
            Input {
                key: Key::Char('A'),
                ..
            } => {
                textarea.cancel_selection();
                textarea.move_cursor(CursorMove::End);
                return Transition::Mode(Mode::Insert);
            }
            Input {
                key: Key::Char('o'),
                ..
            } => {
                textarea.move_cursor(CursorMove::End);
                textarea.insert_newline();
                return Transition::Mode(Mode::Insert);
            }
            Input {
                key: Key::Char('O'),
                ..
            } => {
                textarea.move_cursor(CursorMove::Head);
                textarea.insert_newline();
                textarea.move_cursor(CursorMove::Up);
                return Transition::Mode(Mode::Insert);
            }
            Input {
                key: Key::Char('I'),
                ..
            } => {
                textarea.cancel_selection();
                textarea.move_cursor(CursorMove::Head);
                return Transition::Mode(Mode::Insert);
            }
            // 滚动
            Input {
                key: Key::Char('e'),
                ctrl: true,
                ..
            } => textarea.scroll((1, 0)),
            Input {
                key: Key::Char('y'),
                ctrl: true,
                ..
            } => textarea.scroll((-1, 0)),
            Input {
                key: Key::Char('d'),
                ctrl: true,
                ..
            } => textarea.scroll(tui_textarea::Scrolling::HalfPageDown),
            Input {
                key: Key::Char('u'),
                ctrl: true,
                ..
            } => textarea.scroll(tui_textarea::Scrolling::HalfPageUp),
            Input {
                key: Key::Char('f'),
                ctrl: true,
                ..
            } => textarea.scroll(tui_textarea::Scrolling::PageDown),
            Input {
                key: Key::Char('b'),
                ctrl: true,
                ..
            } => textarea.scroll(tui_textarea::Scrolling::PageUp),
            // Visual 模式
            Input {
                key: Key::Char('v'),
                ctrl: false,
                ..
            } if self.mode == Mode::Normal => {
                textarea.start_selection();
                return Transition::Mode(Mode::Visual);
            }
            Input {
                key: Key::Char('V'),
                ctrl: false,
                ..
            } if self.mode == Mode::Normal => {
                textarea.move_cursor(CursorMove::Head);
                textarea.start_selection();
                textarea.move_cursor(CursorMove::End);
                return Transition::Mode(Mode::Visual);
            }
            Input { key: Key::Esc, .. }
            | Input {
                key: Key::Char('v'),
                ctrl: false,
                ..
            } if self.mode == Mode::Visual => {
                textarea.cancel_selection();
                return Transition::Mode(Mode::Normal);
            }
            // gg → 跳到开头
            Input {
                key: Key::Char('g'),
                ctrl: false,
                ..
            } if matches!(
                self.pending,
                Input {
                    key: Key::Char('g'),
                    ctrl: false,
                    ..
                }
            ) =>
            {
                textarea.move_cursor(CursorMove::Top);
            }
            // G → 跳到结尾
            Input {
                key: Key::Char('G'),
                ctrl: false,
                ..
            } => textarea.move_cursor(CursorMove::Bottom),
            // Operator 重复（yy, dd, cc）
            Input {
                key: Key::Char(c),
                ctrl: false,
                ..
            } if self.mode == Mode::Operator(c) => {
                textarea.move_cursor(CursorMove::Head);
                textarea.start_selection();
                let cursor = textarea.cursor();
                textarea.move_cursor(CursorMove::Down);
                if cursor == textarea.cursor() {
                    textarea.move_cursor(CursorMove::End);
                }
            }
            // 进入 Operator 模式（y/d/c）
            Input {
                key: Key::Char(op @ ('y' | 'd' | 'c')),
                ctrl: false,
                ..
            } if self.mode == Mode::Normal => {
                textarea.start_selection();
                return Transition::Mode(Mode::Operator(op));
            }
            // Visual 模式下的 y/d/c
            Input {
                key: Key::Char('y'),
                ctrl: false,
                ..
            } if self.mode == Mode::Visual => {
                textarea.move_cursor(CursorMove::Forward);
                textarea.copy();
                return Transition::Mode(Mode::Normal);
            }
            Input {
                key: Key::Char('d'),
                ctrl: false,
                ..
            } if self.mode == Mode::Visual => {
                textarea.move_cursor(CursorMove::Forward);
                textarea.cut();
                return Transition::Mode(Mode::Normal);
            }
            Input {
                key: Key::Char('c'),
                ctrl: false,
                ..
            } if self.mode == Mode::Visual => {
                textarea.move_cursor(CursorMove::Forward);
                textarea.cut();
                return Transition::Mode(Mode::Insert);
            }
            // Esc 在 Normal 模式下不退出（vim 标准行为）
            Input { key: Key::Esc, .. } if self.mode == Mode::Normal => {
                return Transition::Nop;
            }
            // Esc 在 Operator 模式取消
            Input { key: Key::Esc, .. } => {
                textarea.cancel_selection();
                return Transition::Mode(Mode::Normal);
            }
            // 其他未匹配的按键 → pending（用于 gg 等序列）
            input => return Transition::Pending(input),
        }

        // 处理 pending operator
        match self.mode {
            Mode::Operator('y') => {
                textarea.copy();
                Transition::Mode(Mode::Normal)
            }
            Mode::Operator('d') => {
                textarea.cut();
                Transition::Mode(Mode::Normal)
            }
            Mode::Operator('c') => {
                textarea.cut();
                Transition::Mode(Mode::Insert)
            }
            _ => Transition::Nop,
        }
    }
}

// ========== 公共 API ==========

/// 打开全屏多行编辑器（vim 模式），返回用户输入的文本内容
///
/// 操作方式：
/// - 默认进入 INSERT 模式，可直接输入
/// - Esc: 回到 NORMAL 模式
/// - `:wq` / `:w` / `:x`: 提交内容
/// - `:q` / `:q!`: 取消退出
/// - `/pattern`: 搜索 pattern
/// - `n` / `N`: 跳转到下一个/上一个匹配
/// - Ctrl+S: 任何模式下快速提交
///
/// 返回 Some(text) 表示提交，None 表示取消
#[allow(dead_code)]
pub fn open_multiline_editor(title: &str) -> io::Result<Option<String>> {
    open_editor_internal(title, &[], Mode::Insert)
}

/// 打开全屏多行编辑器，带有预填充内容，默认 NORMAL 模式
///
/// - `initial_lines`: 预填充到编辑区的行（如历史日报 + 日期前缀）
///
/// 返回 Some(text) 表示提交，None 表示取消
pub fn open_multiline_editor_with_content(
    title: &str,
    initial_lines: &[String],
) -> io::Result<Option<String>> {
    open_editor_internal(title, initial_lines, Mode::Normal)
}

/// 内部统一入口：初始化终端 + 编辑区 + 主循环
fn open_editor_internal(
    title: &str,
    initial_lines: &[String],
    initial_mode: Mode,
) -> io::Result<Option<String>> {
    // 进入终端原始模式
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 初始化文本编辑区
    let mut textarea = if initial_lines.is_empty() {
        TextArea::default()
    } else {
        TextArea::new(initial_lines.to_vec())
    };
    textarea.set_block(make_block(title, &initial_mode));
    textarea.set_cursor_style(initial_mode.cursor_style());
    textarea.set_cursor_line_style(Style::default().add_modifier(Modifier::UNDERLINED));
    textarea.set_line_number_style(Style::default().fg(Color::DarkGray));

    // 如果有预填内容，光标跳到最后一行末尾
    if !initial_lines.is_empty() {
        textarea.move_cursor(CursorMove::Bottom);
        textarea.move_cursor(CursorMove::End);
    }

    // 记录初始内容的快照，用于判断是否有实际改动
    let initial_snapshot: Vec<String> = textarea.lines().iter().map(|l| l.to_string()).collect();

    let mut vim = Vim::new(initial_mode);
    let result = run_editor_loop(
        &mut terminal,
        &mut textarea,
        &mut vim,
        title,
        &initial_snapshot,
    );

    // 恢复终端状态
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}

/// 构造编辑区的 Block（根据模式变色）
fn make_block<'a>(title: &str, mode: &Mode) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(Style::default().fg(mode.border_color()))
}

/// 计算字符串的显示宽度（使用 unicode_width，与 ratatui 内部一致）
fn display_width_of(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// 精确计算字符串在给定列宽下 wrap 后的行数（用于预览区滚动进度显示）
fn count_wrapped_lines_unicode(s: &str, col_width: usize) -> usize {
    if col_width == 0 || s.is_empty() {
        return 1;
    }
    let mut lines = 1usize;
    let mut current_width = 0usize;
    for c in s.chars() {
        let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
        if char_width == 0 {
            continue;
        }
        if current_width + char_width > col_width {
            lines += 1;
            current_width = char_width;
        } else {
            current_width += char_width;
        }
    }
    lines
}

/// 编辑器主循环
fn run_editor_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    textarea: &mut TextArea,
    vim: &mut Vim,
    title: &str,
    initial_snapshot: &[String],
) -> io::Result<Option<String>> {
    // 是否显示 "有未保存改动" 的提示（下次按键后清除）
    let mut unsaved_warning = false;
    // 预览区滚动偏移（向下滚动的行数）
    let mut preview_scroll: u16 = 0;
    // 上一次预览的行索引，切换行时重置滚动
    let mut last_preview_row: usize = usize::MAX;

    loop {
        let mode = &vim.mode.clone();

        // 获取当前光标所在行的内容（用于预览区）
        let cursor_row = textarea.cursor().0;
        let current_line_text: String = textarea
            .lines()
            .get(cursor_row)
            .map(|l| l.to_string())
            .unwrap_or_default();

        // 切换到新行时重置预览滚动
        if cursor_row != last_preview_row {
            preview_scroll = 0;
            last_preview_row = cursor_row;
        }

        // 判断当前行是否超过终端宽度，需要显示预览区
        // 用终端宽度粗略判断（不减行号宽度，保守估计）
        let display_width: usize = display_width_of(&current_line_text);

        // 绘制界面
        terminal.draw(|frame| {
            let area_width = frame.area().width as usize;
            let _area_height = frame.area().height;
            // 预留行号宽度（行号位数 + 2 个边距）+ 边框宽度 2
            let lnum_width = format!("{}", textarea.lines().len()).len() + 2 + 2;
            let effective_width = area_width.saturating_sub(lnum_width);
            let needs_preview = display_width > effective_width;

            let constraints = if needs_preview {
                vec![
                    // 编辑区占 55%，预览区占 40%，状态栏固定 2 行
                    Constraint::Percentage(55),
                    Constraint::Min(5),
                    Constraint::Length(2),
                ]
            } else {
                vec![
                    Constraint::Min(3),    // 编辑区
                    Constraint::Length(2), // 状态栏
                ]
            };

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(frame.area());

            // 渲染编辑区
            frame.render_widget(&*textarea, chunks[0]);

            // 在 TextArea 渲染后，应用搜索高亮
            if !vim.search.pattern.is_empty() {
                apply_search_highlight(frame.buffer_mut(), chunks[0], &vim.search);
            }

            if needs_preview {
                // 预览区内部可用高度（去掉上下边框各 1 行）
                let preview_inner_h = chunks[1].height.saturating_sub(2) as u16;
                // 预览区内部可用宽度（去掉左右边框各 1 列）
                let preview_inner_w = (chunks[1].width.saturating_sub(2)) as usize;

                // 计算总 wrap 行数（用于显示滚动进度）
                let total_wrapped =
                    count_wrapped_lines_unicode(&current_line_text, preview_inner_w) as u16;
                let max_scroll = total_wrapped.saturating_sub(preview_inner_h);
                // 钳制滚动偏移（防止越界）
                let clamped_scroll = preview_scroll.min(max_scroll);

                let scroll_hint = if total_wrapped > preview_inner_h {
                    format!(
                        " 📖 第 {} 行预览  [{}/{}行]  Alt+↓/↑滚动 ",
                        cursor_row + 1,
                        clamped_scroll + preview_inner_h,
                        total_wrapped
                    )
                } else {
                    format!(" 📖 第 {} 行预览 ", cursor_row + 1)
                };

                let preview_block = Block::default()
                    .borders(Borders::ALL)
                    .title(scroll_hint)
                    .title_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .border_style(Style::default().fg(Color::Cyan));
                let preview = Paragraph::new(current_line_text.clone())
                    .block(preview_block)
                    .style(Style::default().fg(Color::White))
                    .wrap(Wrap { trim: false })
                    .scroll((clamped_scroll, 0));
                frame.render_widget(preview, chunks[1]);

                // 渲染状态栏
                let status_bar = build_status_bar(mode, textarea.lines().len(), &vim.search);
                frame.render_widget(status_bar, chunks[2]);
            } else {
                // 渲染状态栏
                let status_bar = build_status_bar(mode, textarea.lines().len(), &vim.search);
                frame.render_widget(status_bar, chunks[1]);
            }
        })?;

        // 处理输入事件
        if let Event::Key(key_event) = event::read()? {
            // 清除上次的警告提示
            if unsaved_warning {
                unsaved_warning = false;
                textarea.set_block(make_block(title, &vim.mode));
            }

            let input = Input::from(key_event);

            // Alt+↓ / Alt+↑：预览区滚动（不影响编辑区）
            use crossterm::event::{KeyCode, KeyModifiers};
            if key_event.modifiers == KeyModifiers::ALT {
                match key_event.code {
                    KeyCode::Down => {
                        preview_scroll = preview_scroll.saturating_add(1);
                        continue;
                    }
                    KeyCode::Up => {
                        preview_scroll = preview_scroll.saturating_sub(1);
                        continue;
                    }
                    _ => {}
                }
            }

            match vim.transition(input, textarea) {
                Transition::Mode(new_mode) if vim.mode != new_mode => {
                    textarea.set_block(make_block(title, &new_mode));
                    textarea.set_cursor_style(new_mode.cursor_style());
                    *vim = Vim::new(new_mode);
                }
                Transition::Nop | Transition::Mode(_) => {}
                Transition::Pending(input) => {
                    let old = std::mem::replace(vim, Vim::new(Mode::Normal));
                    *vim = old.with_pending(input);
                }
                Transition::Submit => {
                    let lines = textarea.lines();
                    // 不使用 trim()，保留每行的原始缩进
                    let text = lines.join("\n");
                    if text.is_empty() {
                        return Ok(None);
                    }
                    return Ok(Some(text));
                }
                Transition::TryQuit => {
                    // :q — 检查是否有实际改动
                    let current_lines: Vec<String> =
                        textarea.lines().iter().map(|l| l.to_string()).collect();
                    if current_lines == initial_snapshot {
                        // 无改动，直接退出
                        return Ok(None);
                    } else {
                        // 有改动，拒绝退出并提示
                        unsaved_warning = true;
                        textarea.set_block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(" ⚠️ 有未保存的改动！使用 :q! 强制退出，或 :wq 保存退出 ")
                                .border_style(Style::default().fg(Color::LightRed)),
                        );
                        *vim = Vim::new(Mode::Normal);
                    }
                }
                Transition::Quit => {
                    return Ok(None);
                }
                Transition::Search(pattern) => {
                    // 执行搜索
                    let lines: Vec<String> =
                        textarea.lines().iter().map(|l| l.to_string()).collect();
                    let count = vim.search.search(&pattern, &lines);

                    // 跳转到第一个匹配
                    if count > 0 {
                        if let Some((line, col)) = vim.search.next_match() {
                            // 移动光标到匹配位置
                            jump_to_match(textarea, line, col);
                        }
                    }

                    *vim = Vim::new(Mode::Normal);
                    vim.search = SearchState::new();
                    vim.search.search(&pattern, &lines);
                }
                Transition::NextMatch => {
                    if let Some((line, col)) = vim.search.next_match() {
                        jump_to_match(textarea, line, col);
                    }
                }
                Transition::PrevMatch => {
                    if let Some((line, col)) = vim.search.prev_match() {
                        jump_to_match(textarea, line, col);
                    }
                }
            }
        }
    }
}

/// 构建底部状态栏
fn build_status_bar(mode: &Mode, line_count: usize, search: &SearchState) -> Paragraph<'static> {
    let mut spans = vec![];

    // 模式标签
    let (mode_text, mode_bg) = match mode {
        Mode::Insert => (" INSERT ", Color::LightBlue),
        Mode::Normal => (" NORMAL ", Color::DarkGray),
        Mode::Visual => (" VISUAL ", Color::LightYellow),
        Mode::Operator(c) => {
            // 这里需要 'static，用 leaked string
            let s: &'static str = match c {
                'y' => " YANK ",
                'd' => " DELETE ",
                'c' => " CHANGE ",
                _ => " OP ",
            };
            (s, Color::LightGreen)
        }
        Mode::Command(cmd) => {
            // 命令模式特殊处理：直接显示命令行
            let cmd_display = format!(":{}", cmd);
            return Paragraph::new(Line::from(vec![
                Span::styled(
                    " COMMAND ",
                    Style::default().fg(Color::Black).bg(Color::LightMagenta),
                ),
                Span::raw(" "),
                Span::styled(
                    cmd_display,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("█", Style::default().fg(Color::White)),
            ]));
        }
        Mode::Search(pattern) => {
            // 搜索模式特殊处理：直接显示搜索词
            let search_display = format!("/{}", pattern);
            return Paragraph::new(Line::from(vec![
                Span::styled(
                    " SEARCH ",
                    Style::default().fg(Color::Black).bg(Color::Magenta),
                ),
                Span::raw(" "),
                Span::styled(
                    search_display,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("█", Style::default().fg(Color::White)),
            ]));
        }
    };

    spans.push(Span::styled(
        mode_text,
        Style::default().fg(Color::Black).bg(mode_bg),
    ));
    spans.push(Span::raw("  "));

    // 快捷键提示
    match mode {
        Mode::Insert => {
            spans.push(Span::styled(
                " Ctrl+S ",
                Style::default().fg(Color::Black).bg(Color::Green),
            ));
            spans.push(Span::raw(" 提交  "));
            spans.push(Span::styled(
                " Ctrl+Q ",
                Style::default().fg(Color::Black).bg(Color::Red),
            ));
            spans.push(Span::raw(" 取消  "));
            spans.push(Span::styled(
                " Esc ",
                Style::default().fg(Color::Black).bg(Color::Yellow),
            ));
            spans.push(Span::raw(" Normal  "));
        }
        Mode::Normal => {
            spans.push(Span::styled(
                " :wq ",
                Style::default().fg(Color::Black).bg(Color::Green),
            ));
            spans.push(Span::raw(" 提交  "));
            spans.push(Span::styled(
                " / ",
                Style::default().fg(Color::Black).bg(Color::Magenta),
            ));
            spans.push(Span::raw(" 搜索  "));
            spans.push(Span::styled(
                " n/N ",
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ));
            spans.push(Span::raw(" 下/上  "));
            spans.push(Span::styled(
                " i ",
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ));
            spans.push(Span::raw(" 编辑  "));
        }
        Mode::Visual => {
            spans.push(Span::styled(
                " y ",
                Style::default().fg(Color::Black).bg(Color::Green),
            ));
            spans.push(Span::raw(" 复制  "));
            spans.push(Span::styled(
                " d ",
                Style::default().fg(Color::Black).bg(Color::Red),
            ));
            spans.push(Span::raw(" 删除  "));
            spans.push(Span::styled(
                " Esc ",
                Style::default().fg(Color::Black).bg(Color::Yellow),
            ));
            spans.push(Span::raw(" 取消  "));
        }
        _ => {}
    }

    // 行数
    spans.push(Span::styled(
        format!(" {} 行 ", line_count),
        Style::default().fg(Color::DarkGray),
    ));

    // 搜索匹配信息
    if !search.pattern.is_empty() {
        let (current, total) = search.current_info();
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!(" [{}: {}/{}] ", search.pattern, current, total),
            Style::default().fg(Color::Magenta),
        ));
    }

    Paragraph::new(Line::from(spans))
}
