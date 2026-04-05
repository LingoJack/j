use crate::command::chat::app::{Action, AskAnswer, ChatApp, CursorDirection};
use crossterm::event::{KeyCode, KeyEvent};

/// 统一交互区域按键处理：选项式（↑↓ 选择，Enter 确认，Esc 拒绝/退出）
pub fn handle_tool_confirm_mode(app: &mut ChatApp, key: KeyEvent) {
    let is_ask = app.ui.tool_ask_mode;

    // ask 模式使用新的结构化问答处理
    if is_ask {
        handle_ask_mode(app, key);
        app.ui.msg_lines_cache = None;
        return;
    }

    if app.ui.tool_interact_typing {
        // 输入模式（工具确认拒绝原因）
        let action = match key.code {
            KeyCode::Esc => {
                app.ui.tool_interact_typing = false;
                app.ui.msg_lines_cache = None;
                return;
            }
            KeyCode::Enter => {
                let input_text = app.ui.tool_interact_input.trim().to_string();
                app.update(Action::RejectPendingToolWithReason(input_text));
                app.ui.tool_interact_input.clear();
                app.ui.tool_interact_cursor = 0;
                app.ui.tool_interact_typing = false;
                app.ui.msg_lines_cache = None;
                return;
            }
            KeyCode::Backspace => Action::ToolInteractDeleteChar,
            KeyCode::Left => {
                if app.ui.tool_interact_cursor > 0 {
                    app.ui.tool_interact_cursor -= 1;
                }
                app.ui.msg_lines_cache = None;
                return;
            }
            KeyCode::Right => {
                let char_count = app.ui.tool_interact_input.chars().count();
                if app.ui.tool_interact_cursor < char_count {
                    app.ui.tool_interact_cursor += 1;
                }
                app.ui.msg_lines_cache = None;
                return;
            }
            KeyCode::Char(c) => Action::ToolInteractInputChar(c),
            _ => {
                app.ui.msg_lines_cache = None;
                return;
            }
        };
        app.update(action);
        app.ui.msg_lines_cache = None;
        return;
    }

    // 工具确认选项模式
    let action = match key.code {
        KeyCode::Up => Action::ToolInteractNavigate(CursorDirection::Up),
        KeyCode::Down => Action::ToolInteractNavigate(CursorDirection::Down),
        KeyCode::Enter => Action::ToolInteractConfirm,
        KeyCode::Esc => Action::RejectPendingTool,
        _ => {
            app.ui.msg_lines_cache = None;
            return;
        }
    };
    app.update(action);
    app.ui.msg_lines_cache = None;
}

/// Ask 模式的结构化问答交互处理
fn handle_ask_mode(app: &mut ChatApp, key: KeyEvent) {
    let total_questions = app.ui.tool_ask_questions.len();
    if total_questions == 0 {
        return;
    }

    // 自由输入模式
    if app.ui.tool_interact_typing {
        let action = match key.code {
            KeyCode::Esc => {
                // 如果输入为空，退出输入模式返回选项列表
                // 如果有输入内容，也退出输入模式（清空输入）
                app.ui.tool_interact_typing = false;
                app.ui.tool_interact_input.clear();
                app.ui.tool_interact_cursor = 0;
                return;
            }
            KeyCode::Enter => {
                // 空输入时无效，不提交
                if app.ui.tool_interact_input.trim().is_empty() {
                    return;
                }
                Action::AskSubmitAnswer
            }
            KeyCode::Backspace => Action::AskDeleteChar,
            KeyCode::Left => {
                if app.ui.tool_interact_cursor > 0 {
                    app.ui.tool_interact_cursor -= 1;
                }
                return;
            }
            KeyCode::Right => {
                let char_count = app.ui.tool_interact_input.chars().count();
                if app.ui.tool_interact_cursor < char_count {
                    app.ui.tool_interact_cursor += 1;
                }
                return;
            }
            KeyCode::Char(c) => Action::AskInputChar(c),
            _ => return,
        };
        app.update(action);
        return;
    }

    let cur_q = match app.ui.tool_ask_questions.get(app.ui.tool_ask_current_idx) {
        Some(q) => q,
        None => return,
    };
    let is_multi = cur_q.multi_select;

    let action = match key.code {
        KeyCode::Up => Action::AskOptionNavigate(CursorDirection::Up),
        KeyCode::Down => Action::AskOptionNavigate(CursorDirection::Down),
        KeyCode::Char(' ') if is_multi => Action::AskToggleMultiSelect,
        KeyCode::Enter => {
            let cursor = app.ui.tool_ask_cursor;
            if cursor == cur_q.options.len() {
                // "自由输入"选项：由于选中时已自动进入输入模式，
                // 到这里说明用户之前按 Esc 退出了输入模式但光标还在自由输入选项上
                // 此时按 Enter 无效，用户应该用 ↓ 键移动到其他选项或直接输入
                return;
            } else if is_multi {
                // 多选：收集所有选中的选项
                let selected: Vec<usize> = app
                    .ui
                    .tool_ask_selections
                    .iter()
                    .enumerate()
                    .filter(|(i, sel)| **sel && *i < cur_q.options.len())
                    .map(|(i, _)| i)
                    .collect();
                if selected.is_empty() {
                    app.ask_submit_answer(AskAnswer::Selected(vec![cursor]));
                } else {
                    app.ask_submit_answer(AskAnswer::Selected(selected));
                }
                return;
            } else {
                // 单选：直接选中当前项
                app.ask_submit_answer(AskAnswer::Selected(vec![cursor]));
                return;
            }
        }
        KeyCode::Left | KeyCode::BackTab => Action::AskNavigate(CursorDirection::Up),
        KeyCode::Right | KeyCode::Tab => Action::AskNavigate(CursorDirection::Down),
        KeyCode::Esc => Action::AskCancel,
        KeyCode::PageUp => Action::PageScroll(CursorDirection::Up),
        KeyCode::PageDown => Action::PageScroll(CursorDirection::Down),
        KeyCode::Char(c) => {
            // 如果光标在自由输入选项上，直接输入字符
            let cursor = app.ui.tool_ask_cursor;
            if cursor == cur_q.options.len() {
                app.ui.tool_interact_typing = true;
                app.ui.tool_interact_input.clear();
                app.ui.tool_interact_cursor = 0;
                // 处理输入的字符
                app.update(Action::AskInputChar(c));
                return;
            }
            return;
        }
        _ => return,
    };
    app.update(action);
}
