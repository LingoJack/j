//! Vim 模式引擎
//!
//! 实现 Vim 风格的编辑模式。

use super::history::{History, Snapshot};
use super::text_buffer::TextBuffer;
use std::fmt;

/// 命令面板项
#[derive(Debug, Clone)]
pub struct CmdItem {
    pub name: &'static str,
    pub desc: &'static str,
}

/// 命令面板所有可用命令
pub const COMMANDS: &[CmdItem] = &[
    CmdItem {
        name: "save",
        desc: "保存/提交",
    },
    CmdItem {
        name: "quit",
        desc: "取消退出",
    },
    CmdItem {
        name: "search",
        desc: "搜索",
    },
    CmdItem {
        name: "wrap",
        desc: "开启折行",
    },
    CmdItem {
        name: "nowrap",
        desc: "关闭折行",
    },
    CmdItem {
        name: "jump",
        desc: "跳转到指定行 (如 /jump 10)",
    },
    CmdItem {
        name: "undo",
        desc: "撤销",
    },
    CmdItem {
        name: "redo",
        desc: "重做",
    },
    CmdItem {
        name: "tohead",
        desc: "跳到文件开头",
    },
    CmdItem {
        name: "toend",
        desc: "跳到文件末尾",
    },
    CmdItem {
        name: "theme",
        desc: "切换主题",
    },
    CmdItem {
        name: "line-number",
        desc: "显示行号",
    },
    CmdItem {
        name: "no-line-number",
        desc: "隐藏行号",
    },
];

/// Vim 模式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    Operator(char),
    Command(String),
    Search(String),
    CommandPanel(String),
    /// 主题选择弹窗
    ThemeSelect,
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
            Self::CommandPanel(_) => write!(f, "CMD"),
            Self::ThemeSelect => write!(f, "THEME"),
        }
    }
}

impl Mode {
    /// 获取模式对应的边框颜色
    pub fn border_color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Self::Normal => Color::DarkGray,
            Self::Insert => Color::Cyan,
            Self::Visual => Color::LightYellow,
            Self::Operator(_) => Color::LightGreen,
            Self::Command(_) => Color::DarkGray,
            Self::Search(_) => Color::Magenta,
            Self::CommandPanel(_) => Color::Magenta,
            Self::ThemeSelect => Color::Magenta,
        }
    }
}

/// 按键
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Backspace,
    Esc,
    Left,
    Right,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Tab,
    Delete,
    F(u8),
    Null,
}

/// 输入事件
#[derive(Debug, Clone)]
pub struct Input {
    pub key: Key,
    pub ctrl: bool,
}

impl Input {
    /// 从 crossterm 的 KeyCode 创建 Input
    pub fn from_keycode(
        code: crossterm::event::KeyCode,
        modifiers: crossterm::event::KeyModifiers,
    ) -> Self {
        use crossterm::event::{KeyCode, KeyModifiers};

        let key = match code {
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
            KeyCode::F(n) => Key::F(n),
            _ => Key::Null,
        };

        Self {
            key,
            ctrl: modifiers.contains(KeyModifiers::CONTROL),
        }
    }
}

/// 状态转换
#[derive(Debug)]
pub enum Transition {
    /// 无操作
    Nop,
    /// 切换模式
    Mode(Mode),
    /// 提交
    Submit,
    /// 取消
    Cancel,
    /// 需要重建折行缓存
    NeedRebuild,
    /// 切换折行
    ToggleWrap(bool),
    /// 执行命令面板命令（交给 editor 层处理）
    ExecuteCommand(String),
    /// 取消搜索，恢复光标到搜索前位置
    SearchAbort,
    /// Visual 模式：复制选区到系统剪贴板
    ClipboardCopy,
}

/// Vim 引擎
#[derive(Debug)]
pub struct Vim {
    mode: Mode,
    yank_register: String,
    visual_start: (usize, usize),
    history: History,
}

impl Vim {
    /// 创建新的 Vim 引擎
    pub fn new(initial_mode: Mode) -> Self {
        Self {
            mode: initial_mode,
            yank_register: String::new(),
            visual_start: (0, 0),
            history: History::new(),
        }
    }

    /// 获取当前模式
    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    /// 设置模式
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// 设置 Visual 模式的选区起点
    pub fn set_visual_start(&mut self, pos: (usize, usize)) {
        self.visual_start = pos;
    }

    /// 获取 Visual 模式的选区起点
    pub fn visual_start(&self) -> (usize, usize) {
        self.visual_start
    }

    /// 设置 yank 寄存器内容
    pub fn set_yank_register(&mut self, text: &str) {
        self.yank_register = text.to_string();
    }

    /// 提取 Visual 选区文本（用于复制到剪贴板）
    pub fn get_selection_text(&self, buffer: &TextBuffer) -> Option<String> {
        let (start_row, start_col) = self.visual_start;
        let (end_row, end_col) = buffer.cursor();

        // 确保 start <= end
        let (sr, sc, er, ec) =
            if start_row > end_row || (start_row == end_row && start_col > end_col) {
                (end_row, end_col, start_row, start_col)
            } else {
                (start_row, start_col, end_row, end_col)
            };

        // 检查是否有实际选区（起点和终点不同）
        if sr == er && sc == ec {
            return None;
        }

        let lines = buffer.lines();
        if sr == er {
            // 单行选区
            lines.get(sr).map(|line| {
                let chars: Vec<char> = line.chars().collect();
                chars[sc..ec].iter().collect()
            })
        } else {
            // 多行选区
            let mut yanked = String::new();
            for (i, line) in lines.iter().enumerate() {
                let chars: Vec<char> = line.chars().collect();
                if i == sr {
                    yanked.push_str(&chars[sc..].iter().collect::<String>());
                    yanked.push('\n');
                } else if i == er {
                    yanked.push_str(&chars[..ec].iter().collect::<String>());
                } else if i > sr && i < er {
                    yanked.push_str(line);
                    yanked.push('\n');
                }
            }
            Some(yanked)
        }
    }

    /// 处理输入
    pub fn handle_input(&mut self, input: &Input, buffer: &mut TextBuffer) -> Transition {
        // 先克隆模式以避免借用冲突
        let mode = self.mode.clone();
        match &mode {
            Mode::Insert => self.handle_insert_mode(input, buffer),
            Mode::Normal => self.handle_normal_mode(input, buffer),
            Mode::Command(cmd) => self.handle_command_mode(input, cmd.clone()),
            Mode::Search(pattern) => self.handle_search_mode(input, pattern.clone()),
            Mode::CommandPanel(filter) => self.handle_command_panel_mode(input, filter.clone()),
            Mode::Visual => self.handle_visual_mode(input, buffer),
            Mode::Operator(c) => self.handle_operator_mode(input, *c, buffer),
            Mode::ThemeSelect => Transition::Nop, // handled by MarkdownEditor
        }
    }

    fn handle_insert_mode(&mut self, input: &Input, buffer: &mut TextBuffer) -> Transition {
        match input.key {
            Key::Esc => Transition::Mode(Mode::Normal),
            Key::Enter => {
                buffer.insert_newline();
                Transition::NeedRebuild
            }
            Key::Backspace => {
                buffer.backspace();
                Transition::NeedRebuild
            }
            Key::Delete => {
                buffer.delete_char();
                Transition::NeedRebuild
            }
            Key::Left => {
                buffer.move_cursor_back();
                Transition::Nop
            }
            Key::Right => {
                buffer.move_cursor_forward();
                Transition::Nop
            }
            Key::Up => {
                buffer.move_cursor_up();
                Transition::Nop
            }
            Key::Down => {
                buffer.move_cursor_down();
                Transition::Nop
            }
            Key::Char(c) => {
                buffer.insert_char(c);
                Transition::NeedRebuild
            }
            Key::Tab => {
                buffer.insert_str("    ");
                Transition::NeedRebuild
            }
            _ => Transition::Nop,
        }
    }

    fn handle_normal_mode(&mut self, input: &Input, buffer: &mut TextBuffer) -> Transition {
        match input.key {
            Key::Char('i') => Transition::Mode(Mode::Insert),
            Key::Char('a') => {
                buffer.move_cursor_forward();
                Transition::Mode(Mode::Insert)
            }
            Key::Char('A') => {
                buffer.move_cursor_end();
                Transition::Mode(Mode::Insert)
            }
            Key::Char('I') => {
                buffer.move_cursor_head();
                Transition::Mode(Mode::Insert)
            }
            Key::Char('o') => {
                buffer.insert_line_below();
                Transition::Mode(Mode::Insert)
            }
            Key::Char('O') => {
                buffer.insert_line_above();
                Transition::Mode(Mode::Insert)
            }
            Key::Char('h') | Key::Left => {
                buffer.move_cursor_back();
                Transition::Nop
            }
            Key::Char('j') | Key::Down => {
                buffer.move_cursor_down();
                Transition::Nop
            }
            Key::Char('k') | Key::Up => {
                buffer.move_cursor_up();
                Transition::Nop
            }
            Key::Char('l') | Key::Right => {
                buffer.move_cursor_forward();
                Transition::Nop
            }
            Key::Char('w') => {
                buffer.move_cursor_word_forward();
                Transition::Nop
            }
            Key::Char('b') => {
                buffer.move_cursor_word_back();
                Transition::Nop
            }
            Key::Char('e') => {
                buffer.move_cursor_word_end();
                Transition::Nop
            }
            Key::Char('0') => {
                buffer.move_cursor_head();
                Transition::Nop
            }
            Key::Char('$') => {
                buffer.move_cursor_end();
                Transition::Nop
            }
            Key::Char('g') => {
                buffer.move_cursor_top();
                Transition::Nop
            }
            Key::Char('G') => {
                buffer.move_cursor_bottom();
                Transition::Nop
            }
            Key::Char('x') => {
                buffer.delete_char();
                Transition::NeedRebuild
            }
            Key::Char('X') => {
                buffer.move_cursor_back();
                buffer.delete_char();
                Transition::NeedRebuild
            }
            Key::Char('d') => Transition::Mode(Mode::Operator('d')),
            Key::Char('c') => Transition::Mode(Mode::Operator('c')),
            Key::Char('y') => Transition::Mode(Mode::Operator('y')),
            Key::Char('p') => {
                if !self.yank_register.is_empty() {
                    buffer.move_cursor_end();
                    buffer.insert_newline();
                    buffer.insert_str(&self.yank_register);
                }
                Transition::NeedRebuild
            }
            Key::Char('v') => {
                self.visual_start = buffer.cursor();
                Transition::Mode(Mode::Visual)
            }
            Key::Char(':') | Key::Char('：') => Transition::Mode(Mode::Command(String::new())),
            Key::Char('/') => Transition::Mode(Mode::CommandPanel(String::new())),
            Key::PageDown => {
                for _ in 0..10 {
                    buffer.move_cursor_down();
                }
                Transition::Nop
            }
            Key::PageUp => {
                for _ in 0..10 {
                    buffer.move_cursor_up();
                }
                Transition::Nop
            }
            _ => Transition::Nop,
        }
    }

    fn handle_command_mode(&mut self, input: &Input, cmd: String) -> Transition {
        match input.key {
            Key::Esc => Transition::Mode(Mode::Normal),
            Key::Enter => {
                let trimmed = cmd.trim();
                match trimmed {
                    "w" | "wq" | "x" => Transition::Submit,
                    "q" | "q!" => Transition::Cancel,
                    "set wrap" => Transition::ToggleWrap(true),
                    "set nowrap" => Transition::ToggleWrap(false),
                    _ => Transition::Mode(Mode::Normal),
                }
            }
            _ => Transition::Nop,
        }
    }

    fn handle_search_mode(&mut self, input: &Input, _pattern: String) -> Transition {
        match input.key {
            Key::Esc => Transition::SearchAbort,
            Key::Enter => Transition::Mode(Mode::Normal),
            _ => Transition::Nop,
        }
    }

    fn handle_command_panel_mode(&mut self, input: &Input, filter: String) -> Transition {
        match input.key {
            Key::Esc => Transition::Mode(Mode::Normal),
            Key::Enter => {
                // 查找匹配的命令
                let matched = filter_commands(&filter);
                if let Some(cmd) = matched.first() {
                    let cmd_name = cmd.name.to_string();
                    // 需要带参数的命令（如 jump）保留 filter 中的参数部分
                    let full_cmd = if cmd_name == "jump" {
                        // filter 可能是 "jump 10" 或 "ju 10"
                        filter.clone()
                    } else {
                        cmd_name
                    };
                    Transition::ExecuteCommand(full_cmd)
                } else {
                    Transition::Mode(Mode::Normal)
                }
            }
            _ => Transition::Nop,
        }
    }

    fn handle_visual_mode(&mut self, input: &Input, buffer: &mut TextBuffer) -> Transition {
        match input.key {
            Key::Esc => Transition::Mode(Mode::Normal),
            Key::Char('y') => {
                if let Some(text) = self.get_selection_text(buffer) {
                    self.yank_register = text;
                }
                Transition::Mode(Mode::Normal)
            }
            // 复制选区到系统剪贴板
            Key::Char('c') => Transition::ClipboardCopy,
            Key::Char('h') | Key::Left => {
                buffer.move_cursor_back();
                Transition::Nop
            }
            Key::Char('j') | Key::Down => {
                buffer.move_cursor_down();
                Transition::Nop
            }
            Key::Char('k') | Key::Up => {
                buffer.move_cursor_up();
                Transition::Nop
            }
            Key::Char('l') | Key::Right => {
                buffer.move_cursor_forward();
                Transition::Nop
            }
            _ => Transition::Nop,
        }
    }

    fn handle_operator_mode(
        &mut self,
        input: &Input,
        op: char,
        buffer: &mut TextBuffer,
    ) -> Transition {
        match input.key {
            Key::Esc => Transition::Mode(Mode::Normal),
            Key::Char('d') if op == 'd' => {
                let (row, _) = buffer.cursor();
                self.yank_register = buffer.line(row).cloned().unwrap_or_default();
                buffer.delete_line();
                Transition::NeedRebuild
            }
            Key::Char('w') => match op {
                'd' => {
                    buffer.delete_word();
                    Transition::NeedRebuild
                }
                'c' => {
                    buffer.delete_word();
                    Transition::Mode(Mode::Insert)
                }
                _ => Transition::Mode(Mode::Normal),
            },
            Key::Char('$') => match op {
                'd' => {
                    let (row, col) = buffer.cursor();
                    if let Some(line) = buffer.line(row) {
                        let chars: Vec<char> = line.chars().collect();
                        self.yank_register = chars[col..].iter().collect();
                    }
                    buffer.delete_line_by_end();
                    Transition::NeedRebuild
                }
                'c' => {
                    let (row, col) = buffer.cursor();
                    if let Some(line) = buffer.line(row) {
                        let chars: Vec<char> = line.chars().collect();
                        self.yank_register = chars[col..].iter().collect();
                    }
                    buffer.delete_line_by_end();
                    Transition::Mode(Mode::Insert)
                }
                _ => Transition::Mode(Mode::Normal),
            },
            Key::Char('c') if op == 'c' => {
                let (row, _) = buffer.cursor();
                self.yank_register = buffer.line(row).cloned().unwrap_or_default();
                buffer.delete_line_by_end();
                buffer.move_cursor_head();
                Transition::Mode(Mode::Insert)
            }
            _ => Transition::Mode(Mode::Normal),
        }
    }
}

// ========== 撤销/重做支持 ==========

impl Vim {
    /// 推入快照
    pub fn push_snapshot(&mut self, snapshot: Snapshot, cursor: (usize, usize)) {
        let snap = Snapshot::with_cursor(snapshot.lines, cursor);
        self.history.push(snap);
    }

    /// 撤销
    pub fn undo(&mut self) -> Option<Snapshot> {
        self.history.undo().cloned()
    }

    /// 重做
    pub fn redo(&mut self) -> Option<Snapshot> {
        self.history.redo().cloned()
    }
}

// ========== 命令面板辅助 ==========

/// 根据筛选文本过滤命令列表
pub fn filter_commands(filter: &str) -> Vec<&'static CmdItem> {
    if filter.is_empty() {
        COMMANDS.iter().collect()
    } else {
        let filter_lower = filter.to_lowercase();
        COMMANDS
            .iter()
            .filter(|cmd| {
                cmd.name.contains(&filter_lower)
                    || cmd.name.starts_with(&filter_lower)
                    || cmd.desc.contains(&filter_lower)
            })
            .collect()
    }
}

/// 解析命令面板输入，提取命令名和参数
pub fn parse_command(input: &str) -> (&str, &str) {
    if let Some(space_pos) = input.find(' ') {
        (&input[..space_pos], input[space_pos + 1..].trim())
    } else {
        (input, "")
    }
}
