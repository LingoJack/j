use crate::command::chat::app::{AskAnswer, ChatApp, ChatMode};
use crossterm::event::{KeyCode, KeyEvent};

/// 统一交互区域按键处理：选项式（↑↓ 选择，Enter 确认，Esc 拒绝/退出）
pub fn handle_tool_confirm_mode(app: &mut ChatApp, key: KeyEvent) {
    let is_ask = app.tool_ask_mode;

    // ask 模式使用新的结构化问答处理
    if is_ask {
        handle_ask_mode(app, key);
        app.msg_lines_cache = None;
        return;
    }

    if app.tool_interact_typing {
        // 输入模式（工具确认）
        match key.code {
            KeyCode::Esc => {
                app.tool_interact_typing = false;
            }
            KeyCode::Enter => {
                let input_text = app.tool_interact_input.trim().to_string();
                app.reject_pending_tool(&input_text);
                app.tool_interact_input.clear();
                app.tool_interact_cursor = 0;
                app.tool_interact_typing = false;
            }
            KeyCode::Backspace => {
                if app.tool_interact_cursor > 0 {
                    let start = app
                        .tool_interact_input
                        .char_indices()
                        .nth(app.tool_interact_cursor - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end = app
                        .tool_interact_input
                        .char_indices()
                        .nth(app.tool_interact_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(app.tool_interact_input.len());
                    app.tool_interact_input.drain(start..end);
                    app.tool_interact_cursor -= 1;
                }
            }
            KeyCode::Left => {
                if app.tool_interact_cursor > 0 {
                    app.tool_interact_cursor -= 1;
                }
            }
            KeyCode::Right => {
                let char_count = app.tool_interact_input.chars().count();
                if app.tool_interact_cursor < char_count {
                    app.tool_interact_cursor += 1;
                }
            }
            KeyCode::Char(c) => {
                let byte_idx = app
                    .tool_interact_input
                    .char_indices()
                    .nth(app.tool_interact_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.tool_interact_input.len());
                app.tool_interact_input.insert(byte_idx, c);
                app.tool_interact_cursor += 1;
            }
            _ => {}
        }
        app.msg_lines_cache = None;
        return;
    }

    // 工具确认选项模式
    match key.code {
        KeyCode::Up => {
            if app.tool_interact_selected > 0 {
                app.tool_interact_selected -= 1;
            }
        }
        KeyCode::Down => {
            if app.tool_interact_selected < 2 {
                app.tool_interact_selected += 1;
            }
        }
        KeyCode::Enter => match app.tool_interact_selected {
            0 => app.execute_pending_tool(),
            1 => app.reject_pending_tool(""),
            2 => {
                app.tool_interact_typing = true;
                app.tool_interact_input.clear();
                app.tool_interact_cursor = 0;
            }
            _ => {}
        },
        KeyCode::Esc => {
            app.reject_pending_tool("");
        }
        _ => {}
    }
    app.msg_lines_cache = None;
}

/// Ask 模式的结构化问答交互处理
fn handle_ask_mode(app: &mut ChatApp, key: KeyEvent) {
    let total_questions = app.tool_ask_questions.len();
    if total_questions == 0 {
        return;
    }

    // 自由输入模式
    if app.tool_interact_typing {
        match key.code {
            KeyCode::Esc => {
                // 退出输入模式，回到选项
                app.tool_interact_typing = false;
            }
            KeyCode::Enter => {
                // 提交自由输入作为当前题答案
                let input_text = app.tool_interact_input.trim().to_string();
                let answer = if input_text.is_empty() {
                    AskAnswer::FreeText("（空）".to_string())
                } else {
                    AskAnswer::FreeText(input_text)
                };
                ask_submit_answer(app, answer);
                app.tool_interact_input.clear();
                app.tool_interact_cursor = 0;
                app.tool_interact_typing = false;
            }
            KeyCode::Backspace => {
                if app.tool_interact_cursor > 0 {
                    let start = app
                        .tool_interact_input
                        .char_indices()
                        .nth(app.tool_interact_cursor - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let end = app
                        .tool_interact_input
                        .char_indices()
                        .nth(app.tool_interact_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(app.tool_interact_input.len());
                    app.tool_interact_input.drain(start..end);
                    app.tool_interact_cursor -= 1;
                }
            }
            KeyCode::Left => {
                if app.tool_interact_cursor > 0 {
                    app.tool_interact_cursor -= 1;
                }
            }
            KeyCode::Right => {
                let char_count = app.tool_interact_input.chars().count();
                if app.tool_interact_cursor < char_count {
                    app.tool_interact_cursor += 1;
                }
            }
            KeyCode::Char(c) => {
                let byte_idx = app
                    .tool_interact_input
                    .char_indices()
                    .nth(app.tool_interact_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.tool_interact_input.len());
                app.tool_interact_input.insert(byte_idx, c);
                app.tool_interact_cursor += 1;
            }
            _ => {}
        }
        return;
    }

    let cur_q = &app.tool_ask_questions[app.tool_ask_current_idx];
    let option_count = cur_q.options.len() + 1; // +1 for free input
    let is_multi = cur_q.multi_select;

    match key.code {
        KeyCode::Up => {
            if app.tool_ask_cursor > 0 {
                app.tool_ask_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.tool_ask_cursor < option_count - 1 {
                app.tool_ask_cursor += 1;
            }
        }
        KeyCode::Char(' ') if is_multi => {
            // 多选 toggle（不对"自由输入"选项 toggle）
            if app.tool_ask_cursor < cur_q.options.len() {
                let idx = app.tool_ask_cursor;
                if idx < app.tool_ask_selections.len() {
                    app.tool_ask_selections[idx] = !app.tool_ask_selections[idx];
                }
            }
        }
        KeyCode::Enter => {
            let cursor = app.tool_ask_cursor;
            if cursor == cur_q.options.len() {
                // "自由输入"选项：进入输入模式
                app.tool_interact_typing = true;
                app.tool_interact_input.clear();
                app.tool_interact_cursor = 0;
            } else if is_multi {
                // 多选：收集所有选中的选项
                let selected: Vec<usize> = app
                    .tool_ask_selections
                    .iter()
                    .enumerate()
                    .filter(|(i, sel)| **sel && *i < cur_q.options.len())
                    .map(|(i, _)| i)
                    .collect();
                if selected.is_empty() {
                    // 没有勾选任何项，就以当前光标所在项为选择
                    ask_submit_answer(app, AskAnswer::Selected(vec![cursor]));
                } else {
                    ask_submit_answer(app, AskAnswer::Selected(selected));
                }
            } else {
                // 单选：直接选中当前项
                ask_submit_answer(app, AskAnswer::Selected(vec![cursor]));
            }
        }
        // 回退到上一题
        KeyCode::Left | KeyCode::BackTab => {
            if app.tool_ask_current_idx > 0 {
                app.tool_ask_current_idx -= 1;
                // 恢复上一题的状态
                if app.tool_ask_answers.len() > app.tool_ask_current_idx {
                    app.tool_ask_answers.truncate(app.tool_ask_current_idx);
                }
                app.init_ask_question_state();
            }
        }
        // 前进（仅当已回答过时才能快速前进）
        KeyCode::Right | KeyCode::Tab => {
            if app.tool_ask_current_idx < total_questions - 1
                && app.tool_ask_current_idx < app.tool_ask_answers.len()
            {
                app.tool_ask_current_idx += 1;
                app.init_ask_question_state();
            }
        }
        KeyCode::Esc => {
            // 取消整个问答
            if let Some(tx) = app.ask_response_tx.take() {
                let _ = tx.send("用户取消了问答".to_string());
            }
            app.tool_ask_mode = false;
            app.tool_ask_questions.clear();
            app.tool_ask_current_idx = 0;
            app.tool_ask_answers.clear();
            app.tool_ask_selections.clear();
            app.tool_ask_cursor = 0;
            app.mode = ChatMode::Chat;
        }
        // PageUp/PageDown 滚动消息区（查看长问题内容）
        KeyCode::PageUp => {
            for _ in 0..10 {
                app.scroll_up();
            }
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                app.scroll_down();
            }
        }
        _ => {}
    }
}

/// 提交当前问题的答案，前进到下一题或完成全部
fn ask_submit_answer(app: &mut ChatApp, answer: AskAnswer) {
    let total = app.tool_ask_questions.len();

    // 存储答案
    if app.tool_ask_current_idx < app.tool_ask_answers.len() {
        app.tool_ask_answers[app.tool_ask_current_idx] = answer;
    } else {
        app.tool_ask_answers.push(answer);
    }

    if app.tool_ask_current_idx + 1 < total {
        // 下一题
        app.tool_ask_current_idx += 1;
        app.init_ask_question_state();
    } else {
        // 全部完成，构建 JSON 响应
        let mut answers_map = serde_json::Map::new();
        for (i, q) in app.tool_ask_questions.iter().enumerate() {
            if let Some(ans) = app.tool_ask_answers.get(i) {
                let val = match ans {
                    AskAnswer::Selected(indices) => {
                        let labels: Vec<&str> = indices
                            .iter()
                            .filter_map(|&idx| q.options.get(idx).map(|o| o.label.as_str()))
                            .collect();
                        labels.join(", ")
                    }
                    AskAnswer::FreeText(text) => text.clone(),
                };
                answers_map.insert(q.question.clone(), serde_json::Value::String(val));
            }
        }

        let response = serde_json::json!({ "answers": answers_map }).to_string();
        if let Some(tx) = app.ask_response_tx.take() {
            let _ = tx.send(response);
        }

        // 清理状态
        app.tool_ask_mode = false;
        app.tool_ask_questions.clear();
        app.tool_ask_current_idx = 0;
        app.tool_ask_answers.clear();
        app.tool_ask_selections.clear();
        app.tool_ask_cursor = 0;
        app.mode = ChatMode::Chat;
    }
}
