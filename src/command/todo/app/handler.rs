// ========== 按键处理 ==========

use super::io::save_todo_list;
use super::state::TodoApp;
use super::types::AppMode;
use super::util::copy_to_clipboard;
use crate::command::report;
use crate::config::YamlConfig;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// 正常模式按键处理，返回 true 表示退出
pub fn handle_normal_mode(app: &mut TodoApp, key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }

    match key.code {
        KeyCode::Char('q') => {
            if app.is_dirty() {
                app.message = Some(
                    "⚠️ 有未保存的修改！请先 s 保存，或输入 q! 强制退出（丢弃修改）".to_string(),
                );
                app.quit_input = "q".to_string();
                return false;
            }
            return true;
        }
        KeyCode::Esc => {
            if app.is_dirty() {
                app.message = Some(
                    "⚠️ 有未保存的修改！请先 s 保存，或输入 q! 强制退出（丢弃修改）".to_string(),
                );
                return false;
            }
            return true;
        }
        KeyCode::Char('!') => {
            if app.quit_input == "q" {
                return true;
            }
            app.quit_input.clear();
        }
        KeyCode::Char('n') | KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Char('N') | KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Char(' ') | KeyCode::Enter => app.toggle_done(),
        KeyCode::Char('a') => {
            app.mode = AppMode::Adding;
            app.input.clear();
            app.cursor_pos = 0;
            app.message = None;
            // 选中新增行（列表末尾）
            let count = app.filtered_indices().len();
            app.state.select(Some(count));
        }
        KeyCode::Char('e') => {
            if let Some(real_idx) = app.selected_real_index() {
                app.input = app.list.items[real_idx].content.clone();
                app.cursor_pos = app.input.chars().count();
                app.edit_index = Some(real_idx);
                app.mode = AppMode::Editing;
                app.message = None;
            }
        }
        KeyCode::Char('y') => {
            if let Some(real_idx) = app.selected_real_index() {
                let content = app.list.items[real_idx].content.clone();
                if copy_to_clipboard(&content) {
                    app.message = Some(format!("📋 已复制到剪切板: {}", content));
                } else {
                    app.message = Some("✖️ 复制到剪切板失败".to_string());
                }
            }
        }
        KeyCode::Char('d') if app.selected_real_index().is_some() => {
            app.mode = AppMode::ConfirmDelete;
        }
        KeyCode::Char('f') => app.toggle_filter(),
        KeyCode::Char('s') => app.save(),
        KeyCode::Char('K') => app.move_item_up(),
        KeyCode::Char('J') => app.move_item_down(),
        KeyCode::Char('?') => {
            app.mode = AppMode::Help;
        }
        KeyCode::Char('/') => {
            app.mode = AppMode::CommandPopup;
            app.cmd_popup_filter.clear();
            app.cmd_popup_selected = 0;
        }
        _ => {}
    }

    if key.code != KeyCode::Char('q') && key.code != KeyCode::Char('!') {
        app.quit_input.clear();
    }

    false
}

/// 输入模式按键处理（添加/编辑通用，支持光标移动和行内编辑）
pub fn handle_input_mode(app: &mut TodoApp, key: KeyEvent) {
    let char_count = app.input.chars().count();

    match key.code {
        KeyCode::Enter => {
            if app.mode == AppMode::Adding {
                app.add_item();
            } else {
                app.confirm_edit();
            }
        }
        KeyCode::Esc => {
            // 检查是否有内容变化，有变化则提示是否保存
            let has_changes = if app.mode == AppMode::Adding {
                !app.input.trim().is_empty()
            } else if app.mode == AppMode::Editing {
                // 编辑模式：比较当前输入和原始内容
                if let Some(idx) = app.edit_index {
                    if idx < app.list.items.len() {
                        app.input.trim() != app.list.items[idx].content.trim()
                    } else {
                        !app.input.trim().is_empty()
                    }
                } else {
                    !app.input.trim().is_empty()
                }
            } else {
                false
            };

            if has_changes {
                // 有修改，进入确认模式，询问是否保存
                app.mode = AppMode::ConfirmCancelInput;
                app.message = Some(
                    "⚠️ 有未保存的内容，是否保存？(Enter/y 保存 / n 放弃 / 其他键继续编辑)"
                        .to_string(),
                );
            } else {
                // 无修改，直接取消
                app.mode = AppMode::Normal;
                app.input.clear();
                app.cursor_pos = 0;
                app.edit_index = None;
                app.message = Some("已取消".to_string());
            }
        }
        KeyCode::Left if app.cursor_pos > 0 => {
            app.cursor_pos -= 1;
        }
        KeyCode::Right if app.cursor_pos < char_count => {
            app.cursor_pos += 1;
        }
        KeyCode::Home => {
            app.cursor_pos = 0;
        }
        KeyCode::End => {
            app.cursor_pos = char_count;
        }
        KeyCode::Backspace if app.cursor_pos > 0 => {
            let start = app
                .input
                .char_indices()
                .nth(app.cursor_pos - 1)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let end = app
                .input
                .char_indices()
                .nth(app.cursor_pos)
                .map(|(i, _)| i)
                .unwrap_or(app.input.len());
            app.input.drain(start..end);
            app.cursor_pos -= 1;
        }
        KeyCode::Delete if app.cursor_pos < char_count => {
            let start = app
                .input
                .char_indices()
                .nth(app.cursor_pos)
                .map(|(i, _)| i)
                .unwrap_or(app.input.len());
            let end = app
                .input
                .char_indices()
                .nth(app.cursor_pos + 1)
                .map(|(i, _)| i)
                .unwrap_or(app.input.len());
            app.input.drain(start..end);
        }
        KeyCode::Char(c) => {
            let byte_idx = app
                .input
                .char_indices()
                .nth(app.cursor_pos)
                .map(|(i, _)| i)
                .unwrap_or(app.input.len());
            app.input.insert(byte_idx, c);
            app.cursor_pos += 1;
        }
        _ => {}
    }
}

/// 确认删除按键处理
pub fn handle_confirm_delete(app: &mut TodoApp, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.delete_selected();
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.message = Some("已取消删除".to_string());
        }
        _ => {}
    }
}

/// 帮助模式按键处理（按任意键返回）
pub fn handle_help_mode(app: &mut TodoApp, _key: KeyEvent) {
    app.mode = AppMode::Normal;
    app.message = None;
}

/// 确认取消输入按键处理
pub fn handle_confirm_cancel_input(app: &mut TodoApp, key: KeyEvent, prev_mode: AppMode) {
    match key.code {
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            // Enter/y 确认保存
            if prev_mode == AppMode::Adding {
                app.add_item();
            } else {
                app.confirm_edit();
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            // n 放弃修改
            app.mode = AppMode::Normal;
            app.input.clear();
            app.cursor_pos = 0;
            app.edit_index = None;
            app.message = Some("已放弃".to_string());
        }
        KeyCode::Esc => {
            // Esc 放弃修改
            app.mode = AppMode::Normal;
            app.input.clear();
            app.cursor_pos = 0;
            app.edit_index = None;
            app.message = Some("已放弃".to_string());
        }
        _ => {
            // 其他键继续编辑
            app.mode = prev_mode;
            app.message = None;
        }
    }
}

/// 命令面板按键处理
pub fn handle_command_popup_mode(app: &mut TodoApp, key: KeyEvent) {
    let items = app.filtered_cmd_items();
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Up | KeyCode::Char('k') if !items.is_empty() => {
            if app.cmd_popup_selected > 0 {
                app.cmd_popup_selected -= 1;
            } else {
                app.cmd_popup_selected = items.len() - 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') if !items.is_empty() => {
            if app.cmd_popup_selected < items.len() - 1 {
                app.cmd_popup_selected += 1;
            } else {
                app.cmd_popup_selected = 0;
            }
        }
        KeyCode::Backspace => {
            if app.cmd_popup_filter.pop().is_none() {
                app.mode = AppMode::Normal;
            } else {
                app.cmd_popup_selected = 0;
            }
        }
        KeyCode::Enter => {
            let selected = app.cmd_popup_selected.min(items.len().saturating_sub(1));
            if let Some((_, key, _)) = items.get(selected) {
                match *key {
                    "toggle" => {
                        app.toggle_done();
                        return; // toggle_done 会进入 ConfirmReport 模式
                    }
                    "edit" => {
                        if let Some(real_idx) = app.selected_real_index() {
                            app.input = app.list.items[real_idx].content.clone();
                            app.cursor_pos = app.input.chars().count();
                            app.edit_index = Some(real_idx);
                            app.mode = AppMode::Editing;
                            app.message = None;
                        }
                        return;
                    }
                    "add" => {
                        app.mode = AppMode::Adding;
                        app.input.clear();
                        app.cursor_pos = 0;
                        app.message = None;
                        let count = app.filtered_indices().len();
                        app.state.select(Some(count));
                        return;
                    }
                    "delete" => {
                        if app.selected_real_index().is_some() {
                            app.mode = AppMode::ConfirmDelete;
                        } else {
                            app.mode = AppMode::Normal;
                        }
                        return;
                    }
                    "copy" => {
                        if let Some(real_idx) = app.selected_real_index() {
                            let content = app.list.items[real_idx].content.clone();
                            if copy_to_clipboard(&content) {
                                app.message = Some(format!("📋 已复制到剪切板: {}", content));
                            } else {
                                app.message = Some("✖️ 复制到剪切板失败".to_string());
                            }
                        }
                    }
                    "filter" => {
                        app.toggle_filter();
                    }
                    "moveup" => {
                        app.move_item_up();
                    }
                    "movedown" => {
                        app.move_item_down();
                    }
                    "save" => {
                        app.save();
                    }
                    "quit" => {
                        if app.is_dirty() {
                            app.message = Some(
                                "⚠️ 有未保存的修改！请先 s 保存，或输入 q! 强制退出（丢弃修改）"
                                    .to_string(),
                            );
                            app.quit_input = "q".to_string();
                        } else {
                            // 返回 true 表示退出，但这里在子函数中无法直接返回 true
                            // 设置一个特殊状态让主循环处理
                            app.mode = AppMode::Normal;
                            app.message = Some("按 q 或 Esc 退出".to_string());
                        }
                        return;
                    }
                    "help" => {
                        app.mode = AppMode::Help;
                        return;
                    }
                    _ => {}
                }
            }
            app.mode = AppMode::Normal;
        }
        KeyCode::Char(c) => {
            app.cmd_popup_filter.push(c);
            app.cmd_popup_selected = 0;
        }
        _ => {}
    }
}

/// 确认写入日报按键处理
pub fn handle_confirm_report(app: &mut TodoApp, key: KeyEvent, config: &mut YamlConfig) {
    match key.code {
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(content) = app.report_pending_content.take() {
                let content_with_done = format!("{} [Done]", content);
                let write_ok = report::write_to_report(&content_with_done, config);
                // 先保存 todo（save 会覆盖 message）
                if app.is_dirty() && save_todo_list(&app.list) {
                    app.snapshot = app.list.clone();
                }
                // 保存后再设置最终 message
                if write_ok {
                    app.message = Some("☑️ 已标记为完成，已写入日报并保存".to_string());
                } else {
                    app.message = Some("☑️ 已标记为完成，但写入日报失败".to_string());
                }
            }
            app.mode = AppMode::Normal;
        }
        _ => {
            // 其他任意键跳过，不写入日报
            app.report_pending_content = None;
            app.message = Some("☑️ 已标记为完成".to_string());
            app.mode = AppMode::Normal;
        }
    }
}
