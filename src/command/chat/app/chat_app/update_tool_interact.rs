use super::ChatApp;
use crate::command::chat::app::action::CursorDirection;
use crate::command::chat::app::types::AskAnswer;
use crate::command::chat::app::ui_state::ChatMode;
use crate::command::chat::constants::TOOL_INTERACT_MAX_OPTIONS;

impl ChatApp {
    // ========== Ask 工具交互 ==========

    pub(super) fn update_ask_navigate(&mut self, dir: CursorDirection) {
        let total = self.ui.tool_ask_questions.len();
        match dir {
            CursorDirection::Up => {
                if self.ui.tool_ask_current_idx > 0 {
                    self.ui.tool_ask_current_idx -= 1;
                    if self.ui.tool_ask_answers.len() > self.ui.tool_ask_current_idx {
                        self.ui
                            .tool_ask_answers
                            .truncate(self.ui.tool_ask_current_idx);
                    }
                    self.init_ask_question_state();
                }
            }
            CursorDirection::Down => {
                if self.ui.tool_ask_current_idx < total - 1
                    && self.ui.tool_ask_current_idx < self.ui.tool_ask_answers.len()
                {
                    self.ui.tool_ask_current_idx += 1;
                    self.init_ask_question_state();
                }
            }
        }
    }

    pub(super) fn update_ask_option_navigate(&mut self, dir: CursorDirection) {
        if let Some(q) = self.ui.tool_ask_questions.get(self.ui.tool_ask_current_idx) {
            let option_count = q.options.len() + 1; // +1 for free input
            let free_input_idx = q.options.len();
            let prev_cursor = self.ui.tool_ask_cursor;

            let new_cursor = match dir {
                CursorDirection::Up => {
                    if self.ui.tool_ask_cursor > 0 {
                        self.ui.tool_ask_cursor - 1
                    } else {
                        self.ui.tool_ask_cursor
                    }
                }
                CursorDirection::Down => {
                    if self.ui.tool_ask_cursor < option_count - 1 {
                        self.ui.tool_ask_cursor + 1
                    } else {
                        self.ui.tool_ask_cursor
                    }
                }
            };

            if new_cursor != prev_cursor {
                // 离开自由输入行：保存草稿
                if prev_cursor == free_input_idx
                    && let Some(draft) = self
                        .ui
                        .tool_ask_drafts
                        .get_mut(self.ui.tool_ask_current_idx)
                {
                    *draft = self.ui.tool_interact_input.clone();
                }

                self.ui.tool_ask_cursor = new_cursor;

                if new_cursor == free_input_idx {
                    // 进入自由输入行：恢复草稿，直接可输入
                    self.ui.tool_interact_typing = true;
                    if let Some(draft) = self.ui.tool_ask_drafts.get(self.ui.tool_ask_current_idx) {
                        self.ui.tool_interact_input = draft.clone();
                    } else {
                        self.ui.tool_interact_input.clear();
                    }
                    self.ui.tool_interact_cursor = self.ui.tool_interact_input.chars().count();
                } else {
                    // 进入选项行
                    self.ui.tool_interact_typing = false;
                    self.ui.tool_interact_input.clear();
                    self.ui.tool_interact_cursor = 0;
                }
            }
        }
    }

    pub(super) fn update_ask_single_select(&mut self) {
        if let Some(q) = self
            .ui
            .tool_ask_questions
            .get(self.ui.tool_ask_current_idx)
            .cloned()
        {
            let cursor = self.ui.tool_ask_cursor;
            if cursor == q.options.len() {
                // "自由输入"选项：进入输入模式，恢复草稿
                self.ui.tool_interact_typing = true;
                if let Some(draft) = self.ui.tool_ask_drafts.get(self.ui.tool_ask_current_idx) {
                    self.ui.tool_interact_input = draft.clone();
                } else {
                    self.ui.tool_interact_input.clear();
                }
                self.ui.tool_interact_cursor = self.ui.tool_interact_input.chars().count();
            } else {
                self.ask_submit_answer(AskAnswer::Selected(vec![cursor]));
            }
        }
    }

    pub(super) fn update_ask_toggle_multi_select(&mut self) {
        if let Some(q) = self.ui.tool_ask_questions.get(self.ui.tool_ask_current_idx)
            && self.ui.tool_ask_cursor < q.options.len()
        {
            let idx = self.ui.tool_ask_cursor;
            if idx < self.ui.tool_ask_selections.len() {
                self.ui.tool_ask_selections[idx] = !self.ui.tool_ask_selections[idx];
            }
        }
    }

    pub(super) fn update_ask_input_char(&mut self, c: char) {
        let byte_idx = self
            .ui
            .tool_interact_input
            .char_indices()
            .nth(self.ui.tool_interact_cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.ui.tool_interact_input.len());
        self.ui.tool_interact_input.insert(byte_idx, c);
        self.ui.tool_interact_cursor += 1;
    }

    pub(super) fn update_ask_delete_char(&mut self) {
        if self.ui.tool_interact_cursor > 0 {
            let start = self
                .ui
                .tool_interact_input
                .char_indices()
                .nth(self.ui.tool_interact_cursor - 1)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let end = self
                .ui
                .tool_interact_input
                .char_indices()
                .nth(self.ui.tool_interact_cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.ui.tool_interact_input.len());
            self.ui.tool_interact_input.drain(start..end);
            self.ui.tool_interact_cursor -= 1;
        }
    }

    pub(super) fn update_ask_submit_answer(&mut self) {
        let input_text = self.ui.tool_interact_input.trim().to_string();
        let answer = if input_text.is_empty() {
            AskAnswer::FreeText("（空）".to_string())
        } else {
            AskAnswer::FreeText(input_text)
        };
        // 清空当前问题的草稿
        if let Some(draft) = self
            .ui
            .tool_ask_drafts
            .get_mut(self.ui.tool_ask_current_idx)
        {
            draft.clear();
        }
        self.ask_submit_answer(answer);
        self.ui.tool_interact_input.clear();
        self.ui.tool_interact_cursor = 0;
        self.ui.tool_interact_typing = false;
    }

    pub(super) fn update_ask_cancel(&mut self) {
        if let Some(tx) = self.ask_response_tx.take() {
            let _ = tx.send("用户取消了问答".to_string());
        }
        self.ui.tool_ask_mode = false;
        self.ui.tool_ask_questions.clear();
        self.ui.tool_ask_current_idx = 0;
        self.ui.tool_ask_answers.clear();
        self.ui.tool_ask_selections.clear();
        self.ui.tool_ask_cursor = 0;
        self.ui.tool_ask_drafts.clear();
        if !self.tool_executor.has_pending_confirm() {
            self.ui.mode = ChatMode::Chat;
        }
    }

    // ========== 工具交互区 ==========

    pub(super) fn update_tool_interact_navigate(&mut self, dir: CursorDirection) {
        let prev = self.ui.tool_interact_selected;
        match dir {
            CursorDirection::Up => {
                if self.ui.tool_interact_selected > 0 {
                    self.ui.tool_interact_selected -= 1;
                }
            }
            CursorDirection::Down => {
                if self.ui.tool_interact_selected < TOOL_INTERACT_MAX_OPTIONS {
                    self.ui.tool_interact_selected += 1;
                }
            }
        }
        let cur = self.ui.tool_interact_selected;
        if cur != prev {
            if cur == 3 {
                // 进入 "type something..." 行：自动切换到输入模式
                self.ui.tool_interact_typing = true;
                self.ui.tool_interact_cursor = self.ui.tool_interact_input.chars().count();
            } else {
                // 离开输入行：退出输入模式，保留已输入内容
                self.ui.tool_interact_typing = false;
            }
        }
    }

    pub(super) fn update_tool_interact_input_char(&mut self, c: char) {
        let byte_idx = self
            .ui
            .tool_interact_input
            .char_indices()
            .nth(self.ui.tool_interact_cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.ui.tool_interact_input.len());
        self.ui.tool_interact_input.insert(byte_idx, c);
        self.ui.tool_interact_cursor += 1;
    }

    pub(super) fn update_tool_interact_delete_char(&mut self) {
        if self.ui.tool_interact_cursor > 0 {
            let start = self
                .ui
                .tool_interact_input
                .char_indices()
                .nth(self.ui.tool_interact_cursor - 1)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let end = self
                .ui
                .tool_interact_input
                .char_indices()
                .nth(self.ui.tool_interact_cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.ui.tool_interact_input.len());
            self.ui.tool_interact_input.drain(start..end);
            self.ui.tool_interact_cursor -= 1;
        }
    }

    pub(super) fn update_tool_interact_confirm(&mut self) {
        match self.ui.tool_interact_selected {
            0 => self.execute_pending_tool(),
            1 => self.allow_and_execute_pending_tool(),
            2 => self.reject_pending_tool(""),
            3 => {
                self.ui.tool_interact_typing = true;
                self.ui.tool_interact_input.clear();
                self.ui.tool_interact_cursor = 0;
            }
            _ => {}
        }
    }
}
