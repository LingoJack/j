//! 命令面板命令执行

use super::{EditorAction, MarkdownEditor};
use crate::tui::editor_core::vim::{Input, Key, Mode, parse_command};

impl MarkdownEditor {
    /// 执行命令面板命令
    pub(super) fn execute_command(&mut self, cmd: &str) -> EditorAction {
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
                self.themes.popup_selected = self.themes.current_index;
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

    /// 执行 Insert 模式命令面板里的命令
    ///
    /// 命令面板从输入 `/` 触发时，`/` 字符与后续 filter 文本已经被写入 buffer。
    /// 这里需要先把这部分回退掉（共 `1 + filter` 个字符），再插入命令对应的内容。
    pub(super) fn execute_insert_command(&mut self, name: &str, filter: &str) -> EditorAction {
        // 删除已写入的 `/` + filter（共 1 + filter.chars().count() 个字符）
        let to_delete = 1 + filter.chars().count();
        for _ in 0..to_delete {
            self.buffer.backspace();
        }

        self.vim.set_mode(Mode::Insert);
        self.insert_panel_anchor = None;
        match name {
            "image" => {
                self.buffer.insert_str("![]()");
                // 光标移动到 () 之间
                let (row, col) = self.buffer.cursor();
                self.buffer.set_cursor(row, col.saturating_sub(1));
            }
            "/" => {
                self.buffer.insert_char('/');
            }
            _ => {}
        }
        self.rebuild_wrap_cache();
        EditorAction::Continue
    }

    /// 处理帮助弹窗模式按键（任意键关闭）
    pub(super) fn handle_help_popup(&mut self, _input: &Input) -> EditorAction {
        self.vim.set_mode(Mode::Normal);
        EditorAction::Continue
    }

    /// 处理主题选择模式按键
    pub(super) fn handle_theme_select(&mut self, input: &Input) -> EditorAction {
        let count = self.themes.gallery.len();
        match input.key {
            Key::Esc => {
                self.vim.set_mode(Mode::Normal);
            }
            Key::Up => {
                if self.themes.popup_selected > 0 {
                    self.themes.popup_selected -= 1;
                } else {
                    self.themes.popup_selected = count - 1;
                }
            }
            Key::Down => {
                if self.themes.popup_selected < count - 1 {
                    self.themes.popup_selected += 1;
                } else {
                    self.themes.popup_selected = 0;
                }
            }
            Key::Enter => {
                let idx = self.themes.popup_selected;
                if idx < count {
                    self.themes.current_index = idx;
                    let (name, theme_id, new_theme) = &self.themes.gallery[idx];
                    self.theme = new_theme.clone();
                    self.renderer.set_theme(new_theme.clone());
                    self.themes.selected_id = Some(theme_id);
                    self.status_message = Some(format!("主题: {}", name));
                }
                self.vim.set_mode(Mode::Normal);
            }
            _ => {}
        }
        EditorAction::Continue
    }
}
