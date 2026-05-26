//! Markdown 编辑器公共 API 入口

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect};
use std::io;

use super::super::theme::{EditorTheme, HighlightFn};
use super::{CursorPolicy, EDITOR_POLL_MS, EditorAction, MarkdownEditor, ThemeGalleryItem};
use crate::tui::editor_core::vim::Input;

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
                    EditorAction::Submit(content) | EditorAction::Save(content) => {
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
        EnableMouseCapture // 启用鼠标事件捕获
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = open_markdown_editor_on_terminal(&mut terminal, opts, content);

    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture // 禁用鼠标事件捕获
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
