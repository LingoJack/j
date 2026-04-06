//! Markdown 编辑器 - 行级渲染切换
//!
//! 实现类似 Typora 的编辑体验：
//! - 当前编辑行显示原始 Markdown 源码
//! - 其他行显示渲染后的效果

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::fmt;
use std::io;
use tui_textarea::{CursorMove, Input, Key, TextArea};

use crate::command::chat::markdown::highlight::highlight_code_line;
use crate::command::chat::theme::Theme;
use crate::util::text::display_width;

// ========== Vim 模式定义 ==========

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    Normal,
    Insert,
    Visual,
    Operator(char),
    Command(String),
    Search(String),
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

#[derive(Debug, Clone)]
struct SearchMatch {
    line: usize,
    start: usize,
    end: usize,
}

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

    fn current_match(&self) -> Option<&SearchMatch> {
        self.matches.get(self.current_index)
    }

    fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_index = (self.current_index + 1) % self.matches.len();
        }
    }

    fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_index = if self.current_index == 0 {
                self.matches.len() - 1
            } else {
                self.current_index - 1
            };
        }
    }

    fn highlight_line(&self, line_idx: usize, line: &str, theme: &Theme) -> Vec<Span<'static>> {
        let line_matches: Vec<_> = self.matches.iter().filter(|m| m.line == line_idx).collect();

        if line_matches.is_empty() || self.pattern.is_empty() {
            return vec![Span::styled(
                line.to_string(),
                Style::default().fg(theme.text_normal),
            )];
        }

        let mut spans = Vec::new();
        let mut last_end = 0;
        let chars: Vec<char> = line.chars().collect();

        for m in line_matches {
            if m.start > last_end {
                let text: String = chars[last_end..m.start].iter().collect();
                spans.push(Span::styled(text, Style::default().fg(theme.text_normal)));
            }
            let match_text: String = chars[m.start..m.end].iter().collect();
            spans.push(Span::styled(
                match_text,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
            last_end = m.end;
        }

        if last_end < chars.len() {
            let text: String = chars[last_end..].iter().collect();
            spans.push(Span::styled(text, Style::default().fg(theme.text_normal)));
        }

        spans
    }
}

// ========== Vim 状态机 ==========

#[derive(Debug)]
enum Transition {
    Nop,
    Mode(Mode),
    Submit,
    Cancel,
}

struct Vim {
    #[allow(dead_code)]
    mode: Mode,
    yank_register: String,
    visual_start: (usize, usize),
    undo_stack: Vec<Vec<String>>,
    undo_cursor: usize,
}

impl Vim {
    fn new(initial_mode: Mode) -> Self {
        Self {
            mode: initial_mode,
            yank_register: String::new(),
            visual_start: (0, 0),
            undo_stack: Vec::new(),
            undo_cursor: 0,
        }
    }

    fn push_undo(&mut self, lines: &[String]) {
        self.undo_stack.truncate(self.undo_cursor);
        self.undo_stack.push(lines.to_vec());
        self.undo_cursor = self.undo_stack.len();
    }

    fn undo(&mut self, lines: &mut Vec<String>) -> bool {
        if self.undo_cursor > 1 {
            self.undo_cursor -= 1;
            *lines = self.undo_stack[self.undo_cursor].clone();
            true
        } else {
            false
        }
    }

    fn redo(&mut self, lines: &mut Vec<String>) -> bool {
        if self.undo_cursor < self.undo_stack.len() {
            self.undo_cursor += 1;
            *lines = self.undo_stack[self.undo_cursor - 1].clone();
            true
        } else {
            false
        }
    }
}

/// 处理 Vim 按键，返回转换动作
fn handle_vim_input(
    mode: &Mode,
    input: &Input,
    textarea: &mut TextArea<'_>,
    vim: &mut Vim,
) -> Transition {
    // 全局快捷键
    if input.ctrl && input.key == Key::Char('s') {
        return Transition::Submit;
    }
    if input.ctrl && input.key == Key::Char('q') {
        return Transition::Cancel;
    }

    match mode {
        Mode::Insert => handle_insert_mode(input, textarea),
        Mode::Normal => handle_normal_mode(input, textarea, vim),
        Mode::Command(cmd) => handle_command_mode(input, cmd),
        Mode::Search(pattern) => handle_search_mode(input, pattern),
        Mode::Visual => handle_visual_mode(input, textarea, vim),
        Mode::Operator(c) => handle_operator_mode(input, *c, textarea, vim),
    }
}

fn handle_insert_mode(input: &Input, textarea: &mut TextArea<'_>) -> Transition {
    match input.key {
        Key::Esc => Transition::Mode(Mode::Normal),
        _ => {
            textarea.input(input.clone());
            Transition::Nop
        }
    }
}

fn handle_normal_mode(input: &Input, textarea: &mut TextArea<'_>, vim: &mut Vim) -> Transition {
    match input.key {
        Key::Char('i') => Transition::Mode(Mode::Insert),
        Key::Char('a') => {
            textarea.move_cursor(CursorMove::Forward);
            Transition::Mode(Mode::Insert)
        }
        Key::Char('A') => {
            textarea.move_cursor(CursorMove::End);
            Transition::Mode(Mode::Insert)
        }
        Key::Char('I') => {
            textarea.move_cursor(CursorMove::Head);
            Transition::Mode(Mode::Insert)
        }
        Key::Char('o') => {
            textarea.move_cursor(CursorMove::End);
            textarea.insert_newline();
            Transition::Mode(Mode::Insert)
        }
        Key::Char('O') => {
            textarea.move_cursor(CursorMove::Head);
            textarea.insert_newline();
            textarea.move_cursor(CursorMove::Up);
            Transition::Mode(Mode::Insert)
        }
        Key::Char('h') | Key::Left => {
            textarea.move_cursor(CursorMove::Back);
            Transition::Nop
        }
        Key::Char('j') | Key::Down => {
            textarea.move_cursor(CursorMove::Down);
            Transition::Nop
        }
        Key::Char('k') | Key::Up => {
            textarea.move_cursor(CursorMove::Up);
            Transition::Nop
        }
        Key::Char('l') | Key::Right => {
            textarea.move_cursor(CursorMove::Forward);
            Transition::Nop
        }
        Key::Char('w') => {
            textarea.move_cursor(CursorMove::WordForward);
            Transition::Nop
        }
        Key::Char('b') => {
            textarea.move_cursor(CursorMove::WordBack);
            Transition::Nop
        }
        Key::Char('e') => {
            textarea.move_cursor(CursorMove::WordEnd);
            Transition::Nop
        }
        Key::Char('0') => {
            textarea.move_cursor(CursorMove::Head);
            Transition::Nop
        }
        Key::Char('$') => {
            textarea.move_cursor(CursorMove::End);
            Transition::Nop
        }
        Key::Char('g') => {
            textarea.move_cursor(CursorMove::Top);
            Transition::Nop
        }
        Key::Char('G') => {
            textarea.move_cursor(CursorMove::Bottom);
            Transition::Nop
        }
        Key::Char('x') => {
            textarea.delete_char();
            Transition::Nop
        }
        Key::Char('X') => {
            textarea.move_cursor(CursorMove::Back);
            textarea.delete_char();
            Transition::Nop
        }
        Key::Char('d') => Transition::Mode(Mode::Operator('d')),
        Key::Char('c') => Transition::Mode(Mode::Operator('c')),
        Key::Char('y') => Transition::Mode(Mode::Operator('y')),
        Key::Char('p') => {
            if !vim.yank_register.is_empty() {
                textarea.move_cursor(CursorMove::End);
                textarea.insert_newline();
                for line in vim.yank_register.lines() {
                    textarea.insert_str(line);
                    textarea.insert_newline();
                }
                textarea.move_cursor(CursorMove::Up);
            }
            Transition::Nop
        }
        Key::Char('u') => Transition::Nop, // undo handled externally
        Key::Char('r') if input.ctrl => Transition::Nop, // redo handled externally
        Key::Char(':') => Transition::Mode(Mode::Command(String::new())),
        Key::Char('/') => Transition::Mode(Mode::Search(String::new())),
        Key::Char('n') => Transition::Nop, // next search handled externally
        Key::Char('N') => Transition::Nop, // prev search handled externally
        Key::Char('v') => {
            vim.visual_start = textarea.cursor();
            Transition::Mode(Mode::Visual)
        }
        Key::PageDown => {
            for _ in 0..10 {
                textarea.move_cursor(CursorMove::Down);
            }
            Transition::Nop
        }
        Key::PageUp => {
            for _ in 0..10 {
                textarea.move_cursor(CursorMove::Up);
            }
            Transition::Nop
        }
        _ => Transition::Nop,
    }
}

fn handle_command_mode(input: &Input, cmd: &str) -> Transition {
    match input.key {
        Key::Esc => Transition::Mode(Mode::Normal),
        Key::Enter => {
            let trimmed = cmd.trim();
            match trimmed {
                "w" | "wq" | "x" => Transition::Submit,
                "q" | "q!" => Transition::Cancel,
                _ => Transition::Mode(Mode::Normal),
            }
        }
        _ => Transition::Nop, // 字符输入在主循环中处理
    }
}

fn handle_search_mode(input: &Input, _pattern: &str) -> Transition {
    match input.key {
        Key::Esc => Transition::Mode(Mode::Normal),
        Key::Enter => Transition::Mode(Mode::Normal),
        _ => Transition::Nop, // 字符输入在主循环中处理
    }
}

fn handle_visual_mode(input: &Input, textarea: &mut TextArea<'_>, vim: &mut Vim) -> Transition {
    match input.key {
        Key::Esc => Transition::Mode(Mode::Normal),
        Key::Char('y') => {
            let (start_row, start_col) = vim.visual_start;
            let (end_row, end_col) = textarea.cursor();
            let (start_row, start_col, end_row, end_col) =
                if start_row > end_row || (start_row == end_row && start_col > end_col) {
                    (end_row, end_col, start_row, start_col)
                } else {
                    (start_row, start_col, end_row, end_col)
                };
            let lines = textarea.lines();
            if start_row == end_row {
                if let Some(line) = lines.get(start_row) {
                    vim.yank_register = line[start_col..end_col].to_string();
                }
            } else {
                let mut yanked = String::new();
                for (i, line) in lines.iter().enumerate() {
                    if i == start_row {
                        yanked.push_str(&line[start_col..]);
                        yanked.push('\n');
                    } else if i == end_row {
                        yanked.push_str(&line[..end_col]);
                    } else if i > start_row && i < end_row {
                        yanked.push_str(line);
                        yanked.push('\n');
                    }
                }
                vim.yank_register = yanked;
            }
            Transition::Mode(Mode::Normal)
        }
        Key::Char('h') | Key::Left => {
            textarea.move_cursor(CursorMove::Back);
            Transition::Nop
        }
        Key::Char('j') | Key::Down => {
            textarea.move_cursor(CursorMove::Down);
            Transition::Nop
        }
        Key::Char('k') | Key::Up => {
            textarea.move_cursor(CursorMove::Up);
            Transition::Nop
        }
        Key::Char('l') | Key::Right => {
            textarea.move_cursor(CursorMove::Forward);
            Transition::Nop
        }
        _ => Transition::Nop,
    }
}

fn handle_operator_mode(
    input: &Input,
    op: char,
    textarea: &mut TextArea<'_>,
    vim: &mut Vim,
) -> Transition {
    match input.key {
        Key::Esc => Transition::Mode(Mode::Normal),
        Key::Char('d') if op == 'd' => {
            let (row, _) = textarea.cursor();
            vim.yank_register = textarea.lines().get(row).cloned().unwrap_or_default();
            textarea.delete_line_by_end();
            Transition::Mode(Mode::Normal)
        }
        Key::Char('w') => match op {
            'd' => {
                textarea.delete_word();
                Transition::Mode(Mode::Normal)
            }
            'c' => {
                textarea.delete_word();
                Transition::Mode(Mode::Insert)
            }
            _ => Transition::Mode(Mode::Normal),
        },
        Key::Char('$') => match op {
            'd' => {
                let (row, col) = textarea.cursor();
                if let Some(line) = textarea.lines().get(row) {
                    vim.yank_register = line[col..].to_string();
                }
                textarea.delete_line_by_end();
                Transition::Mode(Mode::Normal)
            }
            'c' => {
                let (row, col) = textarea.cursor();
                if let Some(line) = textarea.lines().get(row) {
                    vim.yank_register = line[col..].to_string();
                }
                textarea.delete_line_by_end();
                Transition::Mode(Mode::Insert)
            }
            _ => Transition::Mode(Mode::Normal),
        },
        Key::Char('c') if op == 'c' => {
            let (row, _) = textarea.cursor();
            vim.yank_register = textarea.lines().get(row).cloned().unwrap_or_default();
            textarea.delete_line_by_end();
            textarea.move_cursor(CursorMove::Head);
            Transition::Mode(Mode::Insert)
        }
        _ => Transition::Mode(Mode::Normal),
    }
}

// ========== Markdown 编辑器状态 ==========

/// 表格对齐方式
#[derive(Debug, Clone, Copy, PartialEq)]
enum TableAlign {
    Left,
    Center,
    Right,
}

/// 表格上下文
struct TableContext {
    start_idx: usize,
    end_idx: usize,
    col_widths: Vec<usize>,
    alignments: Vec<TableAlign>,
}

struct MarkdownEditorState<'a> {
    textarea: TextArea<'a>,
    mode: Mode,
    vim: Vim,
    search: SearchState,
    theme: Theme,
    #[allow(dead_code)]
    title: String,
    scroll_offset: usize,
    horizontal_scroll: usize,
    viewport_height: usize,
    viewport_width: usize,
}

impl<'a> MarkdownEditorState<'a> {
    fn new(title: &str, content: &str, theme: Theme) -> Self {
        let lines: Vec<String> = if content.is_empty() {
            vec!["".to_string()]
        } else {
            content.lines().map(|l| l.to_string()).collect()
        };

        let initial_mode = if content.is_empty() {
            Mode::Insert
        } else {
            Mode::Normal
        };

        let mut textarea = TextArea::new(lines);

        if !content.is_empty() {
            textarea.move_cursor(CursorMove::Bottom);
            textarea.move_cursor(CursorMove::End);
        }

        let vim = Vim::new(initial_mode.clone());

        Self {
            textarea,
            mode: initial_mode,
            vim,
            search: SearchState::new(),
            theme,
            title: title.to_string(),
            scroll_offset: 0,
            horizontal_scroll: 0,
            viewport_height: 20,
            viewport_width: 80,
        }
    }

    /// 创建带指定初始模式的状态
    fn with_mode(title: &str, content: &str, theme: Theme, initial_mode: Mode) -> Self {
        let lines: Vec<String> = if content.is_empty() {
            vec!["".to_string()]
        } else {
            content.lines().map(|l| l.to_string()).collect()
        };

        let mut textarea = TextArea::new(lines);

        if !content.is_empty() {
            textarea.move_cursor(CursorMove::Bottom);
            textarea.move_cursor(CursorMove::End);
        }

        let vim = Vim::new(initial_mode.clone());

        Self {
            textarea,
            mode: initial_mode,
            vim,
            search: SearchState::new(),
            theme,
            title: title.to_string(),
            scroll_offset: 0,
            horizontal_scroll: 0,
            viewport_height: 20,
            viewport_width: 80,
        }
    }

    fn cursor_line(&self) -> usize {
        self.textarea.cursor().0
    }

    fn cursor_col(&self) -> usize {
        self.textarea.cursor().1
    }

    /// 判断某行是否是代码块围栏 (```)
    fn is_code_fence_line(line: &str) -> bool {
        line.trim_start().starts_with("```")
    }

    /// 检测指定围栏行是否有配对的围栏
    /// 返回 Some((start_idx, end_idx)) 如果代码块完整
    /// 返回 None 如果代码块不完整（没有配对）
    fn find_complete_code_block(
        &self,
        fence_line: usize,
        lines: &[String],
    ) -> Option<(usize, usize)> {
        let mut in_block = false;
        let mut block_start = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                if !in_block {
                    // 开始新的代码块
                    in_block = true;
                    block_start = i;
                } else {
                    // 结束当前代码块
                    if block_start == fence_line || i == fence_line {
                        // 当前围栏行是这个完整代码块的一部分
                        return Some((block_start, i));
                    }
                    in_block = false;
                }
            }
        }
        // 没有找到配对
        None
    }

    /// 判断围栏行是否属于完整的代码块
    fn is_fence_line_paired(&self, fence_line: usize, lines: &[String]) -> bool {
        self.find_complete_code_block(fence_line, lines).is_some()
    }

    /// 判断某行是否在完整的代码块内（不包括围栏行本身，且代码块必须有配对）
    fn is_line_in_complete_code_block(&self, line_idx: usize, lines: &[String]) -> bool {
        let mut in_block = false;
        let mut block_start = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                if !in_block {
                    // 开始新的代码块
                    in_block = true;
                    block_start = i;
                } else {
                    // 结束当前代码块 - 这是一个完整的代码块
                    if block_start < line_idx && line_idx < i {
                        return true;
                    }
                    in_block = false;
                }
            }
        }
        false
    }

    /// 获取代码块的语言标识
    fn get_code_block_language(&self, line_idx: usize, lines: &[String]) -> Option<String> {
        let mut in_block = false;
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                if !in_block {
                    // 开始新的代码块，提取语言标识
                    let lang = trimmed[3..].trim();
                    if i <= line_idx && line_idx < lines.len() {
                        in_block = true;
                        // 检查当前行是否在这个代码块内
                        for j in (i + 1)..lines.len() {
                            if Self::is_code_fence_line(&lines[j]) {
                                // 找到结束围栏
                                if line_idx < j {
                                    return Some(lang.to_string());
                                }
                                break;
                            }
                        }
                    }
                } else {
                    in_block = false;
                }
            }
        }
        None
    }

    /// 查找包含指定行的代码块范围 (开始行, 结束行)
    /// 返回 (start_fence_idx, end_fence_idx)，如果不在此代码块内则返回 None
    fn find_code_block_range(&self, line_idx: usize, lines: &[String]) -> Option<(usize, usize)> {
        let mut in_block = false;
        let mut block_start = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                if !in_block {
                    // 开始新的代码块
                    in_block = true;
                    block_start = i;
                } else {
                    // 结束当前代码块
                    if block_start < line_idx && line_idx < i {
                        return Some((block_start, i));
                    }
                    in_block = false;
                }
            }
        }
        None
    }

    /// 计算代码块内容的最大显示宽度
    fn calculate_code_block_max_width(
        &self,
        start_idx: usize,
        end_idx: usize,
        lines: &[String],
    ) -> usize {
        let mut max_width = 0;
        for i in (start_idx + 1)..end_idx {
            if let Some(line) = lines.get(i) {
                // 应用水平滚动来计算可见部分的宽度
                let chars: Vec<char> = line.chars().collect();
                let visible_chars: Vec<char> =
                    chars.iter().skip(self.horizontal_scroll).copied().collect();
                let visible_line: String = visible_chars.iter().collect();
                let width = display_width(&visible_line);
                max_width = max_width.max(width);
            }
        }
        // 至少保证有一个最小宽度
        max_width.max(10)
    }

    /// 渲染单行（源码或渲染效果）
    fn render_line(&self, line_idx: usize, max_width: usize) -> Line<'static> {
        let lines = self.textarea.lines();
        let Some(line_content) = lines.get(line_idx) else {
            return Line::default();
        };

        // 检查是否是代码块围栏行
        if Self::is_code_fence_line(line_content) {
            // 只有成对的围栏才渲染围框样式
            if self.is_fence_line_paired(line_idx, lines) {
                return self.render_code_fence_line(line_content, line_idx, lines);
            }
            // 不成对的围栏，渲染为普通文本
            return self.render_source_line(line_content, line_idx, false);
        }

        // 检查是否在完整的代码块内
        let is_in_code_block = self.is_line_in_complete_code_block(line_idx, lines);

        // 当前编辑行 - 显示源码
        if line_idx == self.cursor_line() {
            return self.render_source_line(line_content, line_idx, is_in_code_block);
        }

        // 代码块内的行 - 渲染代码块样式
        if is_in_code_block {
            return self.render_code_block_line(line_content, line_idx, lines);
        }

        // 检查是否在表格内（表格行不显示源码，直接渲染）
        if let Some(table_ctx) = self.find_table_context(line_idx, lines) {
            return self.render_table_line(line_content, line_idx, &table_ctx, lines);
        }

        // 其他行 - 尝试渲染（带行号）
        self.render_single_line_with_number(line_content, line_idx, max_width)
    }

    // ========== 表格支持 ==========

    /// 判断一行是否是表格分隔行（如 |---|---|）
    fn is_table_separator_line(line: &str) -> bool {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            return false;
        }
        // 检查是否全是 -、|、: 和空格
        let inner = trimmed.trim_matches('|');
        inner.split('|').all(|cell| {
            let cell = cell.trim();
            cell.chars().all(|c| c == '-' || c == ':' || c == ' ')
        })
    }

    /// 判断一行是否是表格行
    fn is_table_row(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.contains('|')
    }

    /// 查找包含指定行的表格上下文
    fn find_table_context(&self, line_idx: usize, lines: &[String]) -> Option<TableContext> {
        let line = lines.get(line_idx)?;
        if !Self::is_table_row(line) {
            return None;
        }

        // 向上查找表格开始
        let mut start_idx = line_idx;
        while start_idx > 0 {
            if let Some(prev) = lines.get(start_idx - 1) {
                if Self::is_table_row(prev) {
                    start_idx -= 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // 向下查找表格结束
        let mut end_idx = line_idx;
        while end_idx < lines.len() - 1 {
            if let Some(next) = lines.get(end_idx + 1) {
                if Self::is_table_row(next) {
                    end_idx += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // 必须至少有表头和分隔行
        if end_idx - start_idx < 1 {
            return None;
        }

        // 解析对齐方式（从分隔行获取）
        let alignments = if let Some(sep_line) = lines.get(start_idx + 1) {
            Self::parse_table_alignments(sep_line)
        } else {
            return None;
        };

        // 计算每列最大宽度
        let num_cols = alignments.len();
        let mut col_widths = vec![1; num_cols];

        for row_idx in start_idx..=end_idx {
            let row_line = lines.get(row_idx)?;
            let cells = Self::parse_table_cells(row_line);
            for (i, cell) in cells.iter().enumerate() {
                if i < num_cols {
                    col_widths[i] = col_widths[i].max(display_width(cell));
                }
            }
        }

        Some(TableContext {
            start_idx,
            end_idx,
            col_widths,
            alignments,
        })
    }

    /// 解析表格对齐方式
    fn parse_table_alignments(line: &str) -> Vec<TableAlign> {
        let trimmed = line.trim();
        let inner = trimmed.trim_matches('|');
        inner
            .split('|')
            .map(|cell| {
                let cell = cell.trim();
                let left = cell.starts_with(':');
                let right = cell.ends_with(':');
                if left && right {
                    TableAlign::Center
                } else if right {
                    TableAlign::Right
                } else {
                    TableAlign::Left
                }
            })
            .collect()
    }

    /// 解析表格单元格
    fn parse_table_cells(line: &str) -> Vec<String> {
        let trimmed = line.trim();
        let inner = trimmed.trim_matches('|');
        inner.split('|').map(|s| s.trim().to_string()).collect()
    }

    /// 渲染表格行
    fn render_table_line(
        &self,
        line: &str,
        line_idx: usize,
        ctx: &TableContext,
        _lines: &[String],
    ) -> Line<'static> {
        let line_num = format!("{:4} ", line_idx + 1);
        let cells = Self::parse_table_cells(line);

        // 判断是否是分隔行
        if Self::is_table_separator_line(line) {
            // 渲染分隔线
            let mut spans = vec![Span::styled(line_num, Style::default().fg(Color::DarkGray))];

            // 表头行的分隔线样式：├─┼─┤
            let border_style = Style::default().fg(self.theme.text_dim);

            spans.push(Span::styled("├", border_style));
            for (i, cw) in ctx.col_widths.iter().enumerate() {
                spans.push(Span::styled("─".repeat(*cw + 2), border_style));
                if i < ctx.col_widths.len() - 1 {
                    spans.push(Span::styled("┼", border_style));
                }
            }
            spans.push(Span::styled("┤", border_style));

            return Line::from(spans);
        }

        // 判断是否是表头行（表格第一行）
        let is_header = line_idx == ctx.start_idx;

        // 渲染表格内容行
        let mut spans = vec![Span::styled(line_num, Style::default().fg(Color::DarkGray))];
        let border_style = Style::default().fg(self.theme.text_dim);
        let content_style = if is_header {
            Style::default()
                .fg(self.theme.text_bold)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.text_normal)
        };

        // 左边框
        spans.push(Span::styled("│", border_style));

        for (i, cw) in ctx.col_widths.iter().enumerate() {
            let cell_text = cells.get(i).map(|s: &String| s.as_str()).unwrap_or("");
            let cell_width = display_width(cell_text);
            let fill = cw.saturating_sub(cell_width);

            let align = ctx.alignments.get(i).copied().unwrap_or(TableAlign::Left);
            let formatted = match align {
                TableAlign::Center => {
                    let left = fill / 2;
                    let right = fill - left;
                    format!(" {}{}{} ", " ".repeat(left), cell_text, " ".repeat(right))
                }
                TableAlign::Right => {
                    format!(" {}{} ", " ".repeat(fill), cell_text)
                }
                TableAlign::Left => {
                    format!(" {}{} ", cell_text, " ".repeat(fill))
                }
            };

            spans.push(Span::styled(formatted, content_style));
            spans.push(Span::styled("│", border_style));
        }

        Line::from(spans)
    }

    /// 渲染代码块围栏行 (```)
    fn render_code_fence_line(
        &self,
        line: &str,
        line_idx: usize,
        lines: &[String],
    ) -> Line<'static> {
        let line_num = format!("{:4} ", line_idx + 1);
        let trimmed = line.trim_start();

        // 判断是开始围栏还是结束围栏
        // 开始围栏：在代码块外部（之前的代码块都已闭合）
        // 结束围栏：在代码块内部（之前有一个未闭合的开始围栏）
        let is_start = {
            let mut in_block = false;
            for (i, l) in lines.iter().enumerate() {
                if i >= line_idx {
                    // 到达当前行，不切换状态，直接判断
                    break;
                }
                if Self::is_code_fence_line(l) {
                    in_block = !in_block;
                }
            }
            // 如果不在代码块内，当前围栏就是开始围栏
            !in_block
        };

        // 计算代码块内容的最大宽度
        let content_max_width = self
            .find_code_block_range_for_fence(line_idx, lines)
            .map(|(start, end)| self.calculate_code_block_max_width(start, end, lines))
            .unwrap_or(10);

        // 目标格式：
        // 开始围栏：┌─ lang ──────┐
        // 内容行：  │ code        │
        // 结束围栏：└─────────────┘
        //
        // 宽度计算：
        // - 内容区域宽度 = content_max_width
        // - 总宽度 = 1(左│) + content_max_width + 1(右│) = content_max_width + 2

        let total_width = content_max_width + 2; // 内容宽度 + 两侧边框

        if is_start {
            // 开始围栏：┌─ lang ──────┐
            let lang = trimmed[3..].trim();

            // 构建左侧部分：┌─ lang ─ (如果有lang) 或 ┌─ (如果没有)
            let (left_part, left_width) = if lang.is_empty() {
                ("┌─".to_string(), 2)
            } else {
                let s = format!("┌─ {} ─", lang);
                let w = display_width(&s);
                (s, w)
            };

            // 破折号数量 = 总宽度 - 左侧宽度 - 右侧┐(1字符)
            let dash_count = total_width.saturating_sub(left_width + 1).max(1);

            Line::from(vec![
                Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                Span::styled(
                    left_part,
                    Style::default()
                        .fg(self.theme.text_dim)
                        .bg(self.theme.code_bg),
                ),
                Span::styled(
                    "─".repeat(dash_count),
                    Style::default()
                        .fg(self.theme.text_dim)
                        .bg(self.theme.code_bg),
                ),
                Span::styled(
                    "┐",
                    Style::default()
                        .fg(self.theme.text_dim)
                        .bg(self.theme.code_bg),
                ),
            ])
        } else {
            // 结束围栏：└─────────────┘
            // 破折号数量 = 总宽度 - 2 (└ 和 ┘)
            let dash_count = total_width.saturating_sub(2).max(1);

            Line::from(vec![
                Span::styled(line_num, Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "└",
                    Style::default()
                        .fg(self.theme.text_dim)
                        .bg(self.theme.code_bg),
                ),
                Span::styled(
                    "─".repeat(dash_count),
                    Style::default()
                        .fg(self.theme.text_dim)
                        .bg(self.theme.code_bg),
                ),
                Span::styled(
                    "┘",
                    Style::default()
                        .fg(self.theme.text_dim)
                        .bg(self.theme.code_bg),
                ),
            ])
        }
    }

    /// 查找围栏行对应的代码块范围
    /// 对于开始围栏，返回 (当前行, 结束围栏行)
    /// 对于结束围栏，返回 (开始围栏行, 当前行)
    fn find_code_block_range_for_fence(
        &self,
        fence_line: usize,
        lines: &[String],
    ) -> Option<(usize, usize)> {
        let mut in_block = false;
        let mut block_start = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                if !in_block {
                    // 开始新的代码块
                    in_block = true;
                    block_start = i;
                    if i == fence_line {
                        // 当前围栏是开始围栏，查找结束围栏
                        for j in (i + 1)..lines.len() {
                            let t = lines[j].trim_start();
                            if t.starts_with("```") {
                                return Some((block_start, j));
                            }
                        }
                    }
                } else {
                    // 结束当前代码块
                    if i == fence_line {
                        // 当前围栏是结束围栏
                        return Some((block_start, i));
                    }
                    in_block = false;
                }
            }
        }
        None
    }

    /// 渲染代码块内容行
    fn render_code_block_line(
        &self,
        line: &str,
        line_idx: usize,
        lines: &[String],
    ) -> Line<'static> {
        let line_num = format!("{:4} ", line_idx + 1);

        // 应用水平滚动
        let chars: Vec<char> = line.chars().collect();
        let visible_chars: Vec<char> = chars.iter().skip(self.horizontal_scroll).copied().collect();
        let visible_line: String = visible_chars.iter().collect();

        // 获取代码块语言
        let lang = self
            .get_code_block_language(line_idx, lines)
            .unwrap_or_default();

        // 应用语法高亮
        let highlighted_spans = highlight_code_line(&visible_line, &lang, &self.theme);

        // 计算当前行的显示宽度
        let content_width = display_width(&visible_line);

        // 查找代码块范围并计算最大宽度
        let max_width = self
            .find_code_block_range(line_idx, lines)
            .map(|(start, end)| self.calculate_code_block_max_width(start, end, lines))
            .unwrap_or(content_width);

        // 计算需要填充的空格数
        let fill_width = max_width.saturating_sub(content_width);

        // 格式：│ code │
        // 宽度 = 1 + max_width + 1
        let mut spans = vec![
            Span::styled(
                line_num,
                Style::default().fg(Color::DarkGray).bg(self.theme.code_bg),
            ),
            Span::styled(
                "│",
                Style::default()
                    .fg(self.theme.text_dim)
                    .bg(self.theme.code_bg),
            ),
        ];

        // 添加高亮代码
        for span in highlighted_spans {
            spans.push(Span::styled(
                span.content,
                span.style.bg(self.theme.code_bg),
            ));
        }

        // 添加填充空格和右侧竖线
        spans.push(Span::styled(
            " ".repeat(fill_width),
            Style::default().bg(self.theme.code_bg),
        ));
        spans.push(Span::styled(
            "│",
            Style::default()
                .fg(self.theme.text_dim)
                .bg(self.theme.code_bg),
        ));

        Line::from(spans)
    }

    /// 创建带背景色的 Style
    #[inline]
    fn style(&self, fg: Color) -> Style {
        Style::default().fg(fg).bg(self.theme.bg_primary)
    }

    /// 创建带背景色和加粗的 Style
    #[inline]
    fn style_bold(&self, fg: Color) -> Style {
        Style::default()
            .fg(fg)
            .bg(self.theme.bg_primary)
            .add_modifier(Modifier::BOLD)
    }

    /// 创建带背景色和斜体的 Style
    #[inline]
    fn style_italic(&self, fg: Color) -> Style {
        Style::default()
            .fg(fg)
            .bg(self.theme.bg_primary)
            .add_modifier(Modifier::ITALIC)
    }

    /// 渲染源码行（带行号和光标）
    fn render_source_line(
        &self,
        line: &str,
        line_idx: usize,
        _in_code_block: bool,
    ) -> Line<'static> {
        let line_num = format!("{:4} ", line_idx + 1);
        let is_cursor_line = line_idx == self.cursor_line();
        let cursor_col = self.cursor_col();

        // 应用水平滚动
        let chars: Vec<char> = line.chars().collect();
        let visible_chars: Vec<char> = chars.iter().skip(self.horizontal_scroll).copied().collect();
        let visible_line: String = visible_chars.iter().collect();
        let visible_cursor_col = cursor_col.saturating_sub(self.horizontal_scroll);

        if !self.search.pattern.is_empty() {
            let mut spans = vec![Span::styled(line_num, self.style(Color::DarkGray))];
            spans.extend(
                self.search
                    .highlight_line(line_idx, &visible_line, &self.theme),
            );
            return Line::from(spans);
        }

        // 如果是光标所在行，需要显示光标
        if is_cursor_line && self.mode == Mode::Insert {
            let visible_chars: Vec<char> = visible_line.chars().collect();
            let mut spans = vec![Span::styled(line_num, self.style(Color::DarkGray))];

            // 光标前的内容
            if visible_cursor_col > 0 {
                let before: String = visible_chars.iter().take(visible_cursor_col).collect();
                spans.push(Span::styled(before, self.style(self.theme.text_normal)));
            }

            // 光标位置
            if visible_cursor_col < visible_chars.len() {
                // 光标在字符上 - 反色显示
                let cursor_char = visible_chars[visible_cursor_col];
                spans.push(Span::styled(
                    cursor_char.to_string(),
                    Style::default()
                        .fg(self.theme.bg_primary)
                        .bg(self.theme.text_normal)
                        .add_modifier(Modifier::BOLD),
                ));
                // 光标后的内容
                if visible_cursor_col + 1 < visible_chars.len() {
                    let after: String = visible_chars.iter().skip(visible_cursor_col + 1).collect();
                    spans.push(Span::styled(after, self.style(self.theme.text_normal)));
                }
            } else {
                // 光标在行尾 - 显示块状光标
                spans.push(Span::styled(
                    " ",
                    Style::default()
                        .fg(self.theme.bg_primary)
                        .bg(self.theme.text_normal),
                ));
            }

            return Line::from(spans);
        }

        // Normal 模式下的光标行也显示光标指示
        if is_cursor_line && self.mode == Mode::Normal {
            let visible_chars: Vec<char> = visible_line.chars().collect();
            let mut spans = vec![Span::styled(line_num, self.style(Color::DarkGray))];

            // 整行用稍微不同的背景色表示当前行
            let line_style = Style::default()
                .fg(self.theme.text_normal)
                .bg(self.theme.bg_primary);

            // 光标前的内容
            if visible_cursor_col > 0 {
                let before: String = visible_chars.iter().take(visible_cursor_col).collect();
                spans.push(Span::styled(before, line_style));
            }

            // 光标位置 - 用下划线表示
            if visible_cursor_col < visible_chars.len() {
                let cursor_char = visible_chars[visible_cursor_col];
                spans.push(Span::styled(
                    cursor_char.to_string(),
                    Style::default()
                        .fg(self.theme.text_normal)
                        .bg(self.theme.cursor_bg)
                        .add_modifier(Modifier::UNDERLINED),
                ));
                // 光标后的内容
                if visible_cursor_col + 1 < visible_chars.len() {
                    let after: String = visible_chars.iter().skip(visible_cursor_col + 1).collect();
                    spans.push(Span::styled(after, line_style));
                }
            } else if visible_chars.is_empty() {
                // 空行显示光标
                spans.push(Span::styled(
                    " ",
                    Style::default()
                        .fg(self.theme.bg_primary)
                        .bg(self.theme.cursor_bg),
                ));
            } else {
                // 光标在行尾
                spans.push(Span::styled(
                    " ",
                    Style::default()
                        .fg(self.theme.bg_primary)
                        .bg(self.theme.cursor_bg),
                ));
            }

            return Line::from(spans);
        }

        Line::from(vec![
            Span::styled(line_num, self.style(Color::DarkGray)),
            Span::styled(visible_line, self.style(self.theme.text_normal)),
        ])
    }

    /// 渲染单行 Markdown（带行号）
    fn render_single_line_with_number(
        &self,
        line: &str,
        line_idx: usize,
        max_width: usize,
    ) -> Line<'static> {
        let line_num = format!("{:4} ", line_idx + 1);

        // 应用水平滚动
        let chars: Vec<char> = line.chars().collect();
        let visible_chars: Vec<char> = chars.iter().skip(self.horizontal_scroll).copied().collect();
        let visible_line: String = visible_chars.iter().collect();

        let trimmed = visible_line.trim_start();
        let indent_len = visible_line.len() - trimmed.len();
        let indent = " ".repeat(indent_len);

        // 标题
        if trimmed.starts_with("# ") {
            let text = trimmed[2..].trim();
            return Line::from(vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled(format!("◆ {}", text), self.style_bold(self.theme.md_h1)),
            ]);
        }
        if trimmed.starts_with("## ") {
            let text = trimmed[3..].trim();
            return Line::from(vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled(format!("◇ {}", text), self.style_bold(self.theme.md_h2)),
            ]);
        }
        if trimmed.starts_with("### ") {
            let text = trimmed[4..].trim();
            return Line::from(vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled(format!("〈 {} ", text), self.style_bold(self.theme.md_h3)),
            ]);
        }
        if trimmed.starts_with("#### ") {
            let text = trimmed[5..].trim();
            return Line::from(vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled(format!("› {} ", text), self.style_bold(self.theme.md_h4)),
            ]);
        }

        // 水平线
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            let width = max_width.saturating_sub(indent_len).min(40);
            return Line::from(vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("─".repeat(width), self.style(self.theme.text_dim)),
            ]);
        }

        // 无序列表
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let text = &trimmed[2..];
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("• ", self.style(self.theme.text_normal)),
            ];
            spans.extend(rendered);
            return Line::from(spans);
        }

        // 任务列表
        if trimmed.starts_with("- [ ]") {
            let text = trimmed[5..].trim();
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("○ ", self.style(self.theme.text_dim)),
            ];
            spans.extend(rendered);
            return Line::from(spans);
        }
        if trimmed.starts_with("- [x]") || trimmed.starts_with("- [X]") {
            let text = trimmed[5..].trim();
            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled("● ", self.style(self.theme.md_list_bullet)),
            ];
            spans.extend(rendered);
            return Line::from(spans);
        }

        // 有序列表
        if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_ascii_digit()) {
            if let Some(num_end) = rest.find(|c: char| c == '.' || c == ')') {
                if rest.get(num_end..num_end + 2) == Some(". ")
                    || rest.get(num_end..num_end + 2) == Some(") ")
                {
                    let num_str = &trimmed[..rest.len() - rest.len() + num_end + 1];
                    let text = &rest[num_end + 2..];
                    let rendered = self.render_inline(text);
                    let mut spans = vec![
                        Span::styled(line_num, self.style(Color::DarkGray)),
                        Span::styled(indent, self.style(self.theme.text_normal)),
                        Span::styled(format!("{} ", num_str), self.style(self.theme.text_normal)),
                    ];
                    spans.extend(rendered);
                    return Line::from(spans);
                }
            }
        }

        // 引用块 - 支持嵌套
        if trimmed.starts_with('>') {
            // 计算嵌套层级
            let mut level = 0;
            let mut rest = trimmed;
            while rest.starts_with('>') {
                level += 1;
                rest = rest[1..].trim_start();
            }
            let text = rest;

            // 根据层级选择不同的竖线样式
            let prefix: String = (0..level).map(|_| "│").collect::<Vec<_>>().join("");
            let prefix_style = if level == 1 {
                self.style(self.theme.text_dim)
            } else {
                Style::default()
                    .fg(self.theme.text_dim)
                    .bg(self.theme.bg_primary)
            };

            let rendered = self.render_inline(text);
            let mut spans = vec![
                Span::styled(line_num, self.style(Color::DarkGray)),
                Span::styled(indent, self.style(self.theme.text_normal)),
                Span::styled(format!("{} ", prefix), prefix_style),
            ];
            spans.extend(rendered);
            return Line::from(spans);
        }

        // 普通文本
        let rendered = self.render_inline(trimmed);
        let mut spans = vec![
            Span::styled(line_num, self.style(Color::DarkGray)),
            Span::styled(indent, self.style(self.theme.text_normal)),
        ];
        spans.extend(rendered);
        Line::from(spans)
    }

    /// 渲染行内元素
    fn render_inline(&self, text: &str) -> Vec<Span<'static>> {
        let mut spans = Vec::new();
        let mut remaining = text;

        while !remaining.is_empty() {
            // 行内代码
            if let Some(start) = remaining.find('`') {
                if start > 0 {
                    spans.push(Span::styled(
                        remaining[..start].to_string(),
                        self.style(self.theme.text_normal),
                    ));
                }
                remaining = &remaining[start + 1..];
                if let Some(end) = remaining.find('`') {
                    let code = &remaining[..end];
                    // 行内代码使用特殊背景色
                    spans.push(Span::styled(
                        code.to_string(),
                        Style::default()
                            .fg(self.theme.md_inline_code_fg)
                            .bg(self.theme.md_inline_code_bg),
                    ));
                    remaining = &remaining[end + 1..];
                } else {
                    spans.push(Span::styled(
                        format!("`{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 图片 ![alt](url)
            else if let Some(pos) = remaining.find("![") {
                if pos > 0 {
                    spans.push(Span::styled(
                        remaining[..pos].to_string(),
                        self.style(self.theme.text_normal),
                    ));
                }
                remaining = &remaining[pos + 2..];
                if let Some(alt_end) = remaining.find("](") {
                    let alt = &remaining[..alt_end];
                    remaining = &remaining[alt_end + 2..];
                    if let Some(url_end) = remaining.find(')') {
                        // 渲染图片：🖼 alt
                        spans.push(Span::styled(
                            format!("🖼 {}", alt),
                            Style::default()
                                .fg(self.theme.text_dim)
                                .add_modifier(Modifier::ITALIC),
                        ));
                        remaining = &remaining[url_end + 1..];
                    } else {
                        spans.push(Span::styled(
                            format!("![{}{}", alt, remaining),
                            self.style(self.theme.text_normal),
                        ));
                        break;
                    }
                } else {
                    spans.push(Span::styled(
                        format!("![{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 链接 [text](url)
            else if let Some(pos) = remaining.find('[') {
                if pos > 0 {
                    spans.push(Span::styled(
                        remaining[..pos].to_string(),
                        self.style(self.theme.text_normal),
                    ));
                }
                remaining = &remaining[pos + 1..];
                if let Some(text_end) = remaining.find("](") {
                    let link_text = &remaining[..text_end];
                    remaining = &remaining[text_end + 2..];
                    if let Some(url_end) = remaining.find(')') {
                        // 渲染链接：text ↗ 或 text (url)
                        spans.push(Span::styled(
                            link_text.to_string(),
                            self.style(self.theme.md_link)
                                .add_modifier(Modifier::UNDERLINED),
                        ));
                        spans.push(Span::styled(
                            " ↗".to_string(),
                            self.style(self.theme.text_dim),
                        ));
                        remaining = &remaining[url_end + 1..];
                    } else {
                        spans.push(Span::styled(
                            format!("[{}{}", link_text, remaining),
                            self.style(self.theme.text_normal),
                        ));
                        break;
                    }
                } else {
                    spans.push(Span::styled(
                        format!("[{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 删除线 ~~text~~
            else if let Some(pos) = remaining.find("~~") {
                if pos > 0 {
                    spans.push(Span::styled(
                        remaining[..pos].to_string(),
                        self.style(self.theme.text_normal),
                    ));
                }
                remaining = &remaining[pos + 2..];
                if let Some(end) = remaining.find("~~") {
                    let struck_text = &remaining[..end];
                    spans.push(Span::styled(
                        struck_text.to_string(),
                        self.style(self.theme.text_dim)
                            .add_modifier(Modifier::CROSSED_OUT),
                    ));
                    remaining = &remaining[end + 2..];
                } else {
                    spans.push(Span::styled(
                        format!("~~{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 粗体
            else if let Some(pos) = remaining.find("**") {
                if pos > 0 {
                    spans.push(Span::styled(
                        remaining[..pos].to_string(),
                        self.style(self.theme.text_normal),
                    ));
                }
                remaining = &remaining[pos + 2..];
                if let Some(end) = remaining.find("**") {
                    let bold_text = &remaining[..end];
                    spans.push(Span::styled(
                        bold_text.to_string(),
                        self.style_bold(self.theme.text_normal),
                    ));
                    remaining = &remaining[end + 2..];
                } else {
                    spans.push(Span::styled(
                        format!("**{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            }
            // 斜体
            else if let Some(pos) = remaining.find('*') {
                if pos > 0 {
                    spans.push(Span::styled(
                        remaining[..pos].to_string(),
                        self.style(self.theme.text_normal),
                    ));
                }
                remaining = &remaining[pos + 1..];
                if let Some(end) = remaining.find('*') {
                    let italic_text = &remaining[..end];
                    spans.push(Span::styled(
                        italic_text.to_string(),
                        self.style_italic(self.theme.text_normal),
                    ));
                    remaining = &remaining[end + 1..];
                } else {
                    spans.push(Span::styled(
                        format!("*{}", remaining),
                        self.style(self.theme.text_normal),
                    ));
                    break;
                }
            } else {
                spans.push(Span::styled(
                    remaining.to_string(),
                    self.style(self.theme.text_normal),
                ));
                break;
            }
        }

        spans
    }

    /// 生成状态栏
    fn render_status_bar(&self, width: usize) -> Line<'static> {
        let mode_str = format!(" {} ", self.mode);
        let pos_str = format!(" {}:{} ", self.cursor_line() + 1, self.cursor_col() + 1);
        let hints = " Ctrl+S 保存 | Ctrl+Q 取消 | :wq 提交 ";

        let used_width = mode_str.len() + pos_str.len() + hints.len();
        let separator = " ".repeat(width.saturating_sub(used_width));

        Line::from(vec![
            Span::styled(
                mode_str,
                Style::default()
                    .fg(Color::Black)
                    .bg(self.mode.border_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(pos_str, self.style(self.theme.text_dim)),
            Span::styled(separator, self.style(self.theme.text_normal)),
            Span::styled(hints, self.style(self.theme.text_dim)),
        ])
    }

    /// 生成命令/搜索栏
    fn render_command_bar(&self) -> Line<'static> {
        match &self.mode {
            Mode::Command(cmd) => Line::from(vec![
                Span::styled(":", self.style(self.theme.text_normal)),
                Span::styled(cmd.clone(), self.style(self.theme.text_normal)),
                Span::styled(" ", self.style(self.theme.text_normal)),
            ]),
            Mode::Search(pattern) => Line::from(vec![
                Span::styled("/", self.style(Color::Magenta)),
                Span::styled(pattern.clone(), self.style(self.theme.text_normal)),
                Span::styled(" ", self.style(self.theme.text_normal)),
            ]),
            _ => Line::default(),
        }
    }
}

// ========== 公共 API ==========

/// 打开 Markdown 编辑器（在已有终端上）
pub fn open_markdown_editor_on_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    title: &str,
    content: &str,
    theme: &Theme,
) -> io::Result<Option<String>> {
    let mut state = MarkdownEditorState::new(title, content, theme.clone());
    let initial_lines: Vec<String> = state
        .textarea
        .lines()
        .iter()
        .map(|l| l.to_string())
        .collect();
    state.vim.push_undo(&initial_lines);

    loop {
        // 获取终端尺寸
        let size = terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);

        // 渲染
        terminal.draw(|f| {
            let mut lines_to_render = Vec::new();

            // 计算可用内容区域
            // 边框占用：上下各1行，状态栏1行
            let content_height = area.height.saturating_sub(3) as usize; // 边框(2) + 状态栏(1)
            let content_width = area.width.saturating_sub(2) as usize; // 左右边框

            state.viewport_height = content_height;
            state.viewport_width = content_width;

            // 更新垂直滚动偏移
            let cursor_line = state.cursor_line();
            if cursor_line < state.scroll_offset {
                state.scroll_offset = cursor_line;
            } else if cursor_line >= state.scroll_offset + content_height {
                state.scroll_offset = cursor_line - content_height + 1;
            }

            // 更新水平滚动偏移
            let cursor_col = state.cursor_col();
            // 行号占用5个字符（4位数字+空格）
            let line_num_width = 5;
            let effective_width = content_width.saturating_sub(line_num_width);

            if cursor_col < state.horizontal_scroll {
                state.horizontal_scroll = cursor_col;
            } else if cursor_col >= state.horizontal_scroll + effective_width {
                state.horizontal_scroll = cursor_col - effective_width + 1;
            }

            // 渲染每一行
            let all_lines = state.textarea.lines();
            let total_lines = all_lines.len();
            let end_line = (state.scroll_offset + content_height).min(total_lines);

            for line_idx in state.scroll_offset..end_line {
                let line = state.render_line(line_idx, area.width as usize);
                lines_to_render.push(line);
            }

            // 填充空行
            for _ in lines_to_render.len()..content_height {
                lines_to_render.push(Line::from(Span::styled(
                    "~",
                    Style::default()
                        .fg(Color::DarkGray)
                        .bg(state.theme.bg_primary),
                )));
            }

            // 主内容区（设置背景色）
            let block = Block::default()
                .title(format!(" {} ", title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(state.mode.border_color()))
                .style(Style::default().bg(state.theme.bg_primary));

            let paragraph = Paragraph::new(lines_to_render).block(block);
            f.render_widget(paragraph, area);

            // 状态栏（设置背景色）
            let status_bar = state.render_status_bar(area.width as usize);
            let status_area = Rect::new(0, area.height - 1, area.width, 1);
            let status_block = Block::default().style(Style::default().bg(state.theme.bg_primary));
            f.render_widget(Paragraph::new(status_bar).block(status_block), status_area);

            // 命令/搜索栏（设置背景色）
            if matches!(state.mode, Mode::Command(_) | Mode::Search(_)) {
                let cmd_bar = state.render_command_bar();
                let cmd_area = Rect::new(0, area.height - 2, area.width, 1);
                let cmd_block = Block::default().style(Style::default().bg(state.theme.bg_primary));
                f.render_widget(Paragraph::new(cmd_bar).block(cmd_block), cmd_area);
            }
        })?;

        // 处理输入
        if event::poll(std::time::Duration::from_millis(16))? {
            let evt = event::read()?;

            if let Event::Key(key) = evt {
                let tui_input = Input {
                    key: match key.code {
                        KeyCode::Char(c) => Key::Char(c),
                        KeyCode::Enter => Key::Enter,
                        KeyCode::Backspace => Key::Backspace,
                        KeyCode::Esc => Key::Esc,
                        KeyCode::Left => Key::Left,
                        KeyCode::Right => Key::Right,
                        KeyCode::Up => Key::Up,
                        KeyCode::Down => Key::Down,
                        KeyCode::PageUp => Key::PageUp,
                        KeyCode::PageDown => Key::PageDown,
                        KeyCode::Home => Key::Home,
                        KeyCode::End => Key::End,
                        KeyCode::Tab => Key::Tab,
                        KeyCode::Delete => Key::Delete,
                        _ => Key::Null,
                    },
                    ctrl: key.modifiers.contains(KeyModifiers::CONTROL),
                    alt: key.modifiers.contains(KeyModifiers::ALT),
                    shift: key.modifiers.contains(KeyModifiers::SHIFT),
                };

                // 处理撤销
                if state.mode == Mode::Normal && tui_input.key == Key::Char('u') && !tui_input.ctrl
                {
                    let mut lines: Vec<String> = state
                        .textarea
                        .lines()
                        .iter()
                        .map(|l| l.to_string())
                        .collect();
                    if state.vim.undo(&mut lines) {
                        state.textarea = TextArea::new(lines);
                        state.textarea.move_cursor(CursorMove::Bottom);
                    }
                    continue;
                }

                // 处理重做
                if state.mode == Mode::Normal && tui_input.key == Key::Char('r') && tui_input.ctrl {
                    let mut lines: Vec<String> = state
                        .textarea
                        .lines()
                        .iter()
                        .map(|l| l.to_string())
                        .collect();
                    if state.vim.undo(&mut lines) {
                        state.textarea = TextArea::new(lines);
                        state.textarea.move_cursor(CursorMove::Bottom);
                    }
                    continue;
                }

                // 处理重做
                if state.mode == Mode::Normal && tui_input.key == Key::Char('r') && tui_input.ctrl {
                    let mut lines: Vec<String> = state
                        .textarea
                        .lines()
                        .iter()
                        .map(|l| l.to_string())
                        .collect();
                    if state.vim.redo(&mut lines) {
                        state.textarea = TextArea::new(lines);
                        state.textarea.move_cursor(CursorMove::Bottom);
                    }
                    continue;
                }

                // 处理搜索跳转
                if state.mode == Mode::Normal && !state.search.pattern.is_empty() {
                    if tui_input.key == Key::Char('n') && !tui_input.ctrl {
                        state.search.next_match();
                        if let Some(m) = state.search.current_match() {
                            state
                                .textarea
                                .move_cursor(CursorMove::Jump(m.line as u16, m.start as u16));
                        }
                        continue;
                    }
                    if tui_input.key == Key::Char('N') && !tui_input.ctrl {
                        state.search.prev_match();
                        if let Some(m) = state.search.current_match() {
                            state
                                .textarea
                                .move_cursor(CursorMove::Jump(m.line as u16, m.start as u16));
                        }
                        continue;
                    }
                }

                // Vim 状态机处理
                let old_mode = state.mode.clone();
                let transition =
                    handle_vim_input(&state.mode, &tui_input, &mut state.textarea, &mut state.vim);

                match transition {
                    Transition::Mode(new_mode) => {
                        // 如果从 Insert 模式退出，保存 undo 点
                        if old_mode == Mode::Insert && new_mode != Mode::Insert {
                            let lines: Vec<String> = state
                                .textarea
                                .lines()
                                .iter()
                                .map(|l| l.to_string())
                                .collect();
                            state.vim.push_undo(&lines);
                        }
                        state.mode = new_mode;
                    }
                    Transition::Submit => {
                        let content = state.textarea.lines().join("\n");
                        return Ok(Some(content));
                    }
                    Transition::Cancel => {
                        return Ok(None);
                    }
                    Transition::Nop => {
                        // 处理 Command/Search 模式的字符输入
                        match &mut state.mode {
                            Mode::Command(cmd) => match tui_input.key {
                                Key::Char(c) => {
                                    cmd.push(c);
                                }
                                Key::Backspace => {
                                    cmd.pop();
                                }
                                _ => {}
                            },
                            Mode::Search(pattern) => {
                                match tui_input.key {
                                    Key::Char(c) => {
                                        pattern.push(c);
                                        // 实时搜索
                                        let lines: Vec<String> = state
                                            .textarea
                                            .lines()
                                            .iter()
                                            .map(|l| l.to_string())
                                            .collect();
                                        state.search.search(pattern, &lines);
                                    }
                                    Key::Backspace => {
                                        pattern.pop();
                                        // 更新搜索
                                        let lines: Vec<String> = state
                                            .textarea
                                            .lines()
                                            .iter()
                                            .map(|l| l.to_string())
                                            .collect();
                                        state.search.search(pattern, &lines);
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

/// 打开 Markdown 编辑器（独立终端）
#[allow(dead_code)]
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

/// 打开 Markdown 编辑器（独立终端，带预填充内容，NORMAL 模式）
///
/// 适用于日报编辑场景：
/// - `initial_lines`: 预填充到编辑区的行（如历史日报 + 日期前缀）
/// - 以 NORMAL 模式进入（方便用户浏览历史内容）
///
/// 返回 Some(text) 表示提交，None 表示取消
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

    // 将 initial_lines 转换为 content 字符串
    let content = initial_lines.join("\n");

    let result = open_markdown_editor_on_terminal_internal(
        &mut terminal,
        title,
        &content,
        theme,
        Mode::Normal, // 以 NORMAL 模式进入
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
    let mut state =
        MarkdownEditorState::with_mode(title, content, theme.clone(), initial_mode.clone());
    let initial_lines: Vec<String> = state
        .textarea
        .lines()
        .iter()
        .map(|l| l.to_string())
        .collect();
    state.vim.push_undo(&initial_lines);
    state.mode = initial_mode;

    loop {
        // 获取终端尺寸
        let size = terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);

        // 渲染
        terminal.draw(|f| {
            let mut lines_to_render = Vec::new();

            let content_height = area.height.saturating_sub(2) as usize;
            state.viewport_height = content_height;

            // 更新滚动偏移
            let cursor_line = state.cursor_line();
            if cursor_line < state.scroll_offset {
                state.scroll_offset = cursor_line;
            } else if cursor_line >= state.scroll_offset + content_height {
                state.scroll_offset = cursor_line - content_height + 1;
            }

            // 渲染每一行
            let all_lines = state.textarea.lines();
            let total_lines = all_lines.len();
            let end_line = (state.scroll_offset + content_height).min(total_lines);

            for line_idx in state.scroll_offset..end_line {
                let line = state.render_line(line_idx, area.width as usize);
                lines_to_render.push(line);
            }

            // 填充空行
            for _ in lines_to_render.len()..content_height {
                lines_to_render.push(Line::from(Span::styled(
                    "~",
                    Style::default()
                        .fg(Color::DarkGray)
                        .bg(state.theme.bg_primary),
                )));
            }

            // 主内容区（设置背景色）
            let block = Block::default()
                .title(format!(" {} ", title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(state.mode.border_color()))
                .style(Style::default().bg(state.theme.bg_primary));

            let paragraph = Paragraph::new(lines_to_render).block(block);
            f.render_widget(paragraph, area);

            // 状态栏（设置背景色）
            let status_bar = state.render_status_bar(area.width as usize);
            let status_area = Rect::new(0, area.height - 1, area.width, 1);
            let status_block = Block::default().style(Style::default().bg(state.theme.bg_primary));
            f.render_widget(Paragraph::new(status_bar).block(status_block), status_area);

            // 命令/搜索栏（设置背景色）
            if matches!(state.mode, Mode::Command(_) | Mode::Search(_)) {
                let cmd_bar = state.render_command_bar();
                let cmd_area = Rect::new(0, area.height - 2, area.width, 1);
                let cmd_block = Block::default().style(Style::default().bg(state.theme.bg_primary));
                f.render_widget(Paragraph::new(cmd_bar).block(cmd_block), cmd_area);
            }
        })?;

        // 处理输入
        if event::poll(std::time::Duration::from_millis(16))? {
            let evt = event::read()?;

            if let Event::Key(key) = evt {
                let tui_input = Input {
                    key: match key.code {
                        KeyCode::Char(c) => Key::Char(c),
                        KeyCode::Enter => Key::Enter,
                        KeyCode::Backspace => Key::Backspace,
                        KeyCode::Esc => Key::Esc,
                        KeyCode::Left => Key::Left,
                        KeyCode::Right => Key::Right,
                        KeyCode::Up => Key::Up,
                        KeyCode::Down => Key::Down,
                        KeyCode::PageUp => Key::PageUp,
                        KeyCode::PageDown => Key::PageDown,
                        KeyCode::Home => Key::Home,
                        KeyCode::End => Key::End,
                        KeyCode::Tab => Key::Tab,
                        KeyCode::Delete => Key::Delete,
                        _ => Key::Null,
                    },
                    ctrl: key.modifiers.contains(KeyModifiers::CONTROL),
                    alt: key.modifiers.contains(KeyModifiers::ALT),
                    shift: key.modifiers.contains(KeyModifiers::SHIFT),
                };

                // 处理撤销
                if state.mode == Mode::Normal && tui_input.key == Key::Char('u') && !tui_input.ctrl
                {
                    let mut lines: Vec<String> = state
                        .textarea
                        .lines()
                        .iter()
                        .map(|l| l.to_string())
                        .collect();
                    if state.vim.undo(&mut lines) {
                        state.textarea = TextArea::new(lines);
                        state.textarea.move_cursor(CursorMove::Bottom);
                    }
                    continue;
                }

                // 处理重做
                if state.mode == Mode::Normal && tui_input.key == Key::Char('r') && tui_input.ctrl {
                    let mut lines: Vec<String> = state
                        .textarea
                        .lines()
                        .iter()
                        .map(|l| l.to_string())
                        .collect();
                    if state.vim.redo(&mut lines) {
                        state.textarea = TextArea::new(lines);
                        state.textarea.move_cursor(CursorMove::Bottom);
                    }
                    continue;
                }

                // 处理搜索跳转
                if state.mode == Mode::Normal && !state.search.pattern.is_empty() {
                    if tui_input.key == Key::Char('n') && !tui_input.ctrl {
                        state.search.next_match();
                        if let Some(m) = state.search.current_match() {
                            state
                                .textarea
                                .move_cursor(CursorMove::Jump(m.line as u16, m.start as u16));
                        }
                        continue;
                    }
                    if tui_input.key == Key::Char('N') && !tui_input.ctrl {
                        state.search.prev_match();
                        if let Some(m) = state.search.current_match() {
                            state
                                .textarea
                                .move_cursor(CursorMove::Jump(m.line as u16, m.start as u16));
                        }
                        continue;
                    }
                }

                // Vim 状态机处理
                let old_mode = state.mode.clone();
                let transition =
                    handle_vim_input(&state.mode, &tui_input, &mut state.textarea, &mut state.vim);

                match transition {
                    Transition::Mode(new_mode) => {
                        // 如果从 Insert 模式退出，保存 undo 点
                        if old_mode == Mode::Insert && new_mode != Mode::Insert {
                            let lines: Vec<String> = state
                                .textarea
                                .lines()
                                .iter()
                                .map(|l| l.to_string())
                                .collect();
                            state.vim.push_undo(&lines);
                        }
                        state.mode = new_mode;
                    }
                    Transition::Submit => {
                        let content = state.textarea.lines().join("\n");
                        return Ok(Some(content));
                    }
                    Transition::Cancel => {
                        return Ok(None);
                    }
                    Transition::Nop => {
                        // 处理 Command/Search 模式的字符输入
                        match &mut state.mode {
                            Mode::Command(cmd) => match tui_input.key {
                                Key::Char(c) => {
                                    cmd.push(c);
                                }
                                Key::Backspace => {
                                    cmd.pop();
                                }
                                _ => {}
                            },
                            Mode::Search(pattern) => {
                                match tui_input.key {
                                    Key::Char(c) => {
                                        pattern.push(c);
                                        // 实时搜索
                                        let lines: Vec<String> = state
                                            .textarea
                                            .lines()
                                            .iter()
                                            .map(|l| l.to_string())
                                            .collect();
                                        state.search.search(pattern, &lines);
                                    }
                                    Key::Backspace => {
                                        pattern.pop();
                                        // 更新搜索
                                        let lines: Vec<String> = state
                                            .textarea
                                            .lines()
                                            .iter()
                                            .map(|l| l.to_string())
                                            .collect();
                                        state.search.search(pattern, &lines);
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
