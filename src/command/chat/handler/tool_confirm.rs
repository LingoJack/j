use crate::command::chat::app::{Action, AskAnswer, ChatApp, ChatMode, CursorDirection};
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

/// 权限请求确认模式：Y/Enter 批准，N/Esc 拒绝
pub fn handle_agent_perm_confirm_mode(app: &mut ChatApp, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            if let Some(req) = app.ui.pending_agent_perm.take() {
                req.resolve(true);
            }
            app.ui.mode = ChatMode::Chat;
            app.ui.msg_lines_cache = None;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            if let Some(req) = app.ui.pending_agent_perm.take() {
                req.resolve(false);
            }
            app.ui.mode = ChatMode::Chat;
            app.ui.msg_lines_cache = None;
        }
        _ => {}
    }
}

/// Teammate Plan 审批确认模式：Y/Enter 批准，C 批准并清空上下文，N/Esc 拒绝
pub fn handle_plan_approval_confirm_mode(app: &mut ChatApp, key: KeyEvent) {
    use crate::command::chat::app::types::PlanDecision;
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            if let Some(req) = app.ui.pending_plan_approval.take() {
                req.resolve(PlanDecision::Approve);
            }
            app.ui.mode = ChatMode::Chat;
            app.ui.msg_lines_cache = None;
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            if let Some(req) = app.ui.pending_plan_approval.take() {
                req.resolve(PlanDecision::ApproveAndClearContext);
            }
            app.ui.mode = ChatMode::Chat;
            app.ui.msg_lines_cache = None;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            if let Some(req) = app.ui.pending_plan_approval.take() {
                req.resolve(PlanDecision::Reject);
            }
            app.ui.mode = ChatMode::Chat;
            app.ui.msg_lines_cache = None;
        }
        _ => {}
    }
}

/// Ask 模式的结构化问答交互处理
fn handle_ask_mode(app: &mut ChatApp, key: KeyEvent) {
    let total_questions = app.ui.tool_ask_questions.len();
    if total_questions == 0 {
        return;
    }

    let cur_q = match app.ui.tool_ask_questions.get(app.ui.tool_ask_current_idx) {
        Some(q) => q,
        None => return,
    };
    let free_input_idx = cur_q.options.len();

    // 自由输入模式
    if app.ui.tool_interact_typing {
        let action = match key.code {
            KeyCode::Esc => {
                // 保存草稿，退出输入模式但不清空内容
                save_draft(app);
                app.ui.tool_interact_typing = false;
                return;
            }
            KeyCode::Up => {
                // 光标在行首且上方有选项时，保存草稿退回选项列表
                if app.ui.tool_interact_cursor == 0 && app.ui.tool_ask_cursor > 0 {
                    save_draft(app);
                    app.ui.tool_interact_typing = false;
                    app.ui.tool_ask_cursor -= 1;
                    app.ui.tool_interact_input.clear();
                    app.ui.tool_interact_cursor = 0;
                }
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

    let is_multi = cur_q.multi_select;

    let action = match key.code {
        KeyCode::Up => Action::AskOptionNavigate(CursorDirection::Up),
        KeyCode::Down => Action::AskOptionNavigate(CursorDirection::Down),
        KeyCode::Char(' ') if is_multi => Action::AskToggleMultiSelect,
        KeyCode::Enter => {
            let cursor = app.ui.tool_ask_cursor;
            if cursor == free_input_idx {
                // "自由输入"选项：恢复草稿进入输入模式
                app.ui.tool_interact_typing = true;
                restore_draft(app);
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
            // 无论光标在哪个选项上，直接输入字符时自动跳到自由输入行
            app.ui.tool_ask_cursor = free_input_idx;
            app.ui.tool_interact_typing = true;
            // 恢复已有草稿后追加字符
            restore_draft(app);
            app.update(Action::AskInputChar(c));
            return;
        }
        _ => return,
    };
    app.update(action);
}

/// 保存当前自由输入内容到草稿缓存
fn save_draft(app: &mut ChatApp) {
    if let Some(draft) = app.ui.tool_ask_drafts.get_mut(app.ui.tool_ask_current_idx) {
        *draft = app.ui.tool_interact_input.clone();
    }
}

/// 从草稿缓存恢复自由输入内容
fn restore_draft(app: &mut ChatApp) {
    if let Some(draft) = app.ui.tool_ask_drafts.get(app.ui.tool_ask_current_idx) {
        app.ui.tool_interact_input = draft.clone();
    } else {
        app.ui.tool_interact_input.clear();
    }
    app.ui.tool_interact_cursor = app.ui.tool_interact_input.chars().count();
}
