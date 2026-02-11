use crossterm::{
    event::{self, Event},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use tui_textarea::{CursorMove, Input, Key, TextArea};
use std::io;
use std::fmt;

// ========== Vim 模式定义 ==========

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    Normal,
    Insert,
    Visual,
    Operator(char),
    Command(String), // 命令行模式，存储输入的命令字符串
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Visual => write!(f, "VISUAL"),
            Self::Operator(c) => write!(f, "OPERATOR({})", c),
            Self::Command(_) => write!(f, "COMMAND"),
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
        }
    }
}

// ========== 编辑器状态转换 ==========

enum Transition {
    Nop,
    Mode(Mode),
    Pending(Input),
    Submit,  // 提交内容
    Quit,    // 取消退出
}

/// Vim 状态机
struct Vim {
    mode: Mode,
    pending: Input,
}

impl Vim {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            pending: Input::default(),
        }
    }

    fn with_pending(self, pending: Input) -> Self {
        Self {
            mode: self.mode,
            pending,
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

        // 任何模式下 Ctrl+Q → 取消退出
        if input.ctrl && input.key == Key::Char('q') {
            return Transition::Quit;
        }

        match &self.mode {
            Mode::Command(cmd) => self.handle_command_mode(input, cmd),
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
                    "q" | "q!" => Transition::Quit,
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
pub fn open_multiline_editor_with_content(title: &str, initial_lines: &[String]) -> io::Result<Option<String>> {
    open_editor_internal(title, initial_lines, Mode::Normal)
}

/// 内部统一入口：初始化终端 + 编辑区 + 主循环
fn open_editor_internal(title: &str, initial_lines: &[String], initial_mode: Mode) -> io::Result<Option<String>> {
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

    let mut vim = Vim::new(initial_mode);
    let result = run_editor_loop(&mut terminal, &mut textarea, &mut vim, title);

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

/// 编辑器主循环
fn run_editor_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    textarea: &mut TextArea,
    vim: &mut Vim,
    title: &str,
) -> io::Result<Option<String>> {
    loop {
        let mode = &vim.mode;

        // 绘制界面
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),   // 编辑区
                    Constraint::Length(2), // 状态栏
                ])
                .split(frame.area());

            // 渲染编辑区
            frame.render_widget(&*textarea, chunks[0]);

            // 渲染状态栏
            let status_bar = build_status_bar(mode, textarea.lines().len());
            frame.render_widget(status_bar, chunks[1]);
        })?;

        // 处理输入事件
        if let Event::Key(key_event) = event::read()? {
            let input = Input::from(key_event);
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
                    let text = textarea.lines().join("\n").trim().to_string();
                    if text.is_empty() {
                        return Ok(None);
                    }
                    return Ok(Some(text));
                }
                Transition::Quit => {
                    return Ok(None);
                }
            }
        }
    }
}

/// 构建底部状态栏
fn build_status_bar(mode: &Mode, line_count: usize) -> Paragraph<'static> {
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
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
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
                " :q ",
                Style::default().fg(Color::Black).bg(Color::Red),
            ));
            spans.push(Span::raw(" 退出  "));
            spans.push(Span::styled(
                " Ctrl+Q ",
                Style::default().fg(Color::Black).bg(Color::Red),
            ));
            spans.push(Span::raw(" 取消  "));
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

    Paragraph::new(Line::from(spans))
}