//! 按键处理逻辑。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;

use super::io::{
    cleanup_empty_dirs, copy_to_clipboard, note_file_path, notebook_dir, open_in_finder,
    parse_ratio, save_expanded_dirs, save_panel_ratio,
};
use super::types::{AppMode, FlatEntryKind, NotebookApp};

/// 正常模式按键处理，返回 true 表示退出
pub fn handle_normal_mode(app: &mut NotebookApp, key: KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }

    match key.code {
        KeyCode::Char('q') => {
            app.quit_input = "q".to_string();
            return true;
        }
        KeyCode::Esc => {
            if app.search_filter.is_some() {
                app.clear_search();
            } else {
                return true;
            }
        }
        KeyCode::Char('n') | KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Char('N') | KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Enter | KeyCode::Char('e') if app.selected_name().is_some() => {
            return false; // 编辑操作在 TUI loop 中处理（需暂停/恢复终端）
        }
        KeyCode::Char('a') => {
            app.mode = AppMode::Adding;
            app.input.clear();
            app.cursor_pos = 0;
            app.message = None;
        }
        KeyCode::Char('d') if app.selected_real_index().is_some() => {
            app.mode = AppMode::ConfirmDelete;
        }
        KeyCode::Char('r') => {
            if let Some(idx) = app.selected_real_index() {
                app.input = app.notes[idx].path.clone();
                app.cursor_pos = app.input.chars().count();
                app.rename_index = Some(idx);
                app.mode = AppMode::Renaming;
                app.message = None;
            }
        }
        KeyCode::Tab => {
            if let Some(sel) = app.state.selected()
                && sel < app.flat_entries.len()
                && let FlatEntryKind::Dir { ref dir_path, .. } = app.flat_entries[sel].kind
            {
                app.expanded_dirs.toggle(dir_path);
                save_expanded_dirs(&app.expanded_dirs);
                app.build_flat_entries();
                if sel >= app.flat_entries.len() {
                    app.state
                        .select(Some(app.flat_entries.len().saturating_sub(1)));
                }
                app.update_preview();
            }
        }
        KeyCode::Char('p') if app.selected_real_index().is_some() => {
            app.mode = AppMode::Preview;
            app.preview_scroll = 0;
        }
        KeyCode::Char('/') => {
            app.mode = AppMode::CommandPopup;
            app.cmd_popup_filter.clear();
            app.cmd_popup_selected = 0;
            app.message = None;
        }
        KeyCode::Char('[') => {
            app.panel_ratio = app.panel_ratio.saturating_sub(5).max(15);
            app.preview_width = 0;
            app.message = Some(format!(
                "面板比例: {}:{}",
                app.panel_ratio,
                100 - app.panel_ratio
            ));
            save_panel_ratio(app.panel_ratio);
        }
        KeyCode::Char(']') => {
            app.panel_ratio = app.panel_ratio.saturating_add(5).min(60);
            app.preview_width = 0;
            app.message = Some(format!(
                "面板比例: {}:{}",
                app.panel_ratio,
                100 - app.panel_ratio
            ));
            save_panel_ratio(app.panel_ratio);
        }
        KeyCode::Char('y') => {
            if let Some(name) = app.selected_name() {
                if copy_to_clipboard(&name) {
                    app.message = Some(format!("已复制笔记名: {}", name));
                } else {
                    app.message = Some("复制到剪切板失败".to_string());
                }
            }
        }
        KeyCode::Char('o') => {
            open_in_finder();
        }
        KeyCode::Char('s') => {
            app.reload();
        }
        KeyCode::Char('?') => {
            app.mode = AppMode::Help;
        }
        _ => {}
    }

    if key.code != KeyCode::Char('q') {
        app.quit_input.clear();
    }

    false
}

/// 预览模式按键处理
pub fn handle_preview_mode(app: &mut NotebookApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('p') | KeyCode::Char('q') => {
            app.mode = AppMode::Normal;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.preview_scroll = app.preview_scroll.saturating_add(1);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.preview_scroll = app.preview_scroll.saturating_sub(1);
        }
        KeyCode::Char('n') => {
            app.move_down();
        }
        KeyCode::Char('N') => {
            app.move_up();
        }
        _ => {}
    }
}

/// 输入模式按键处理（添加/重命名/搜索/目录/移动通用）
pub fn handle_input_mode(app: &mut NotebookApp, key: KeyEvent) {
    let char_count = app.input.chars().count();

    match key.code {
        KeyCode::Enter => {
            dispatch_enter(app);
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.input.clear();
            app.cursor_pos = 0;
            app.rename_index = None;
            app.message = Some("已取消".to_string());
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
            delete_char_before(app);
        }
        KeyCode::Delete if app.cursor_pos < char_count => {
            delete_char_at(app);
        }
        KeyCode::Char(c) => {
            insert_char(app, c);
        }
        _ => {}
    }
}

/// 帮助模式按键处理（按任意键返回）
pub fn handle_help_mode(app: &mut NotebookApp, _key: KeyEvent) {
    app.mode = AppMode::Normal;
    app.message = None;
}

/// 确认删除按键处理
pub fn handle_confirm_delete(app: &mut NotebookApp, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(idx) = app.selected_real_index() {
                let name = &app.notes[idx].path;
                let path = note_file_path(name);
                match fs::remove_file(&path) {
                    Ok(()) => {
                        cleanup_empty_dirs();
                        app.message = Some(format!("已删除: {}", name));
                        app.reload();
                    }
                    Err(e) => {
                        app.message = Some(format!("删除失败: {}", e));
                    }
                }
            }
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.message = Some("已取消删除".to_string());
        }
        _ => {}
    }
}

/// 命令面板按键处理
pub fn handle_command_popup_mode(app: &mut NotebookApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.cmd_popup_filter.clear();
            app.message = None;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let count = app.filtered_cmd_items().len();
            if count > 0 {
                app.cmd_popup_selected = if app.cmd_popup_selected == 0 {
                    count - 1
                } else {
                    app.cmd_popup_selected - 1
                };
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let count = app.filtered_cmd_items().len();
            if count > 0 {
                app.cmd_popup_selected = (app.cmd_popup_selected + 1) % count;
            }
        }
        KeyCode::Enter => {
            execute_cmd_popup_action(app);
            app.cmd_popup_filter.clear();
            // 不重置 cmd_popup_selected，保留选项位置
        }
        KeyCode::Backspace => {
            if app.cmd_popup_filter.is_empty() {
                app.mode = AppMode::Normal;
                app.message = None;
            } else {
                app.cmd_popup_filter.pop();
                app.cmd_popup_selected = 0;
            }
        }
        KeyCode::Char(c) => {
            app.cmd_popup_filter.push(c);
            app.cmd_popup_selected = 0;
        }
        _ => {}
    }
}

/// 比例输入按键处理
pub fn handle_ratio_input_mode(app: &mut NotebookApp, key: KeyEvent) {
    let char_count = app.input.chars().count();

    match key.code {
        KeyCode::Enter => {
            match parse_ratio(&app.input) {
                Some(ratio) => {
                    app.panel_ratio = ratio;
                    app.preview_width = 0; // 强制重新渲染预览
                    app.message = Some(format!("面板比例已设为 {}:{}", ratio, 100 - ratio));
                    save_panel_ratio(ratio);
                }
                None => {
                    app.message = Some("格式错误，请输入如 20:80".to_string());
                }
            }
            app.mode = AppMode::Normal;
            app.input.clear();
            app.cursor_pos = 0;
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.input.clear();
            app.cursor_pos = 0;
            app.message = Some("已取消".to_string());
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
            delete_char_before(app);
        }
        KeyCode::Delete if app.cursor_pos < char_count => {
            delete_char_at(app);
        }
        KeyCode::Char(c) if c.is_ascii_digit() || c == ':' => {
            insert_char(app, c);
        }
        _ => {}
    }
}

// ========== Enter 模式分发 ==========

/// 根据当前模式分发 Enter 键处理
fn dispatch_enter(app: &mut NotebookApp) {
    match app.mode {
        AppMode::Adding => enter_adding(app),
        AppMode::Renaming => enter_renaming(app),
        AppMode::Mkdir => enter_mkdir(app),
        AppMode::Mv => enter_mv(app),
        AppMode::Search => enter_search(app),
        _ => {}
    }
}

/// 新建笔记 Enter 处理
fn enter_adding(app: &mut NotebookApp) {
    let title = app.input.trim().to_string();
    if title.is_empty() {
        app.message = Some("标题为空，已取消".to_string());
        app.mode = AppMode::Normal;
        app.input.clear();
        return;
    }
    // 设置待编辑标题，由 TUI loop 负责暂停终端并打开编辑器
    app.pending_edit_title = Some(title);
    app.input.clear();
    app.mode = AppMode::Normal;
}

/// 重命名 Enter 处理
fn enter_renaming(app: &mut NotebookApp) {
    let new_name = app.input.trim().to_string();
    if new_name.is_empty() {
        app.message = Some("名称为空，已取消".to_string());
        app.mode = AppMode::Normal;
        app.input.clear();
        app.rename_index = None;
        return;
    }
    if let Some(idx) = app.rename_index
        && idx < app.notes.len()
    {
        let old_name = &app.notes[idx].path;
        if old_name == &new_name {
            app.message = Some("名称未变化".to_string());
            app.mode = AppMode::Normal;
            app.input.clear();
            app.rename_index = None;
            return;
        }
        let old_path = note_file_path(old_name);
        let new_path = note_file_path(&new_name);
        if new_path.exists() {
            app.message = Some(format!("目标笔记已存在: {}", new_name));
            return;
        }
        // 确保目标目录存在
        if let Some(parent) = new_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        match fs::rename(&old_path, &new_path) {
            Ok(()) => {
                cleanup_empty_dirs();
                app.message = Some(format!("已重命名: {} → {}", old_name, new_name));
                app.reload();
            }
            Err(e) => {
                app.message = Some(format!("重命名失败: {}", e));
            }
        }
    }
    app.mode = AppMode::Normal;
    app.input.clear();
    app.rename_index = None;
}

/// 新建目录 Enter 处理
fn enter_mkdir(app: &mut NotebookApp) {
    let dir_name = app.input.trim().to_string();
    if dir_name.is_empty() {
        app.message = Some("目录名为空，已取消".to_string());
        app.mode = AppMode::Normal;
        app.input.clear();
        return;
    }
    let dir_path = notebook_dir().join(&dir_name);
    if dir_path.exists() {
        app.message = Some(format!("目录已存在: {}", dir_name));
        app.mode = AppMode::Normal;
        app.input.clear();
        return;
    }
    match fs::create_dir_all(&dir_path) {
        Ok(()) => {
            app.expanded_dirs.toggle(&dir_name);
            save_expanded_dirs(&app.expanded_dirs);
            app.message = Some(format!("已创建目录: {}", dir_name));
            app.reload();
        }
        Err(e) => {
            app.message = Some(format!("创建目录失败: {}", e));
        }
    }
    app.mode = AppMode::Normal;
    app.input.clear();
}

/// 移动笔记 Enter 处理
fn enter_mv(app: &mut NotebookApp) {
    let target = app.input.trim().to_string();
    if target.is_empty() {
        app.message = Some("目标路径为空，已取消".to_string());
        app.mode = AppMode::Normal;
        app.input.clear();
        return;
    }
    let current_name = app.selected_name().unwrap_or_default();
    if current_name.is_empty() {
        app.message = Some("没有选中的笔记".to_string());
        app.mode = AppMode::Normal;
        app.input.clear();
        return;
    }
    let old_path = note_file_path(&current_name);
    let new_path = note_file_path(&target);
    if !old_path.exists() {
        app.message = Some(format!("源笔记不存在: {}", current_name));
        app.mode = AppMode::Normal;
        app.input.clear();
        return;
    }
    if new_path.exists() {
        app.message = Some(format!("目标笔记已存在: {}", target));
        app.mode = AppMode::Normal;
        app.input.clear();
        return;
    }
    // 确保目标目录存在
    if let Some(parent) = new_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::rename(&old_path, &new_path) {
        Ok(()) => {
            cleanup_empty_dirs();
            app.message = Some(format!("已移动: {} → {}", current_name, target));
            app.reload();
        }
        Err(e) => {
            app.message = Some(format!("移动失败: {}", e));
        }
    }
    app.mode = AppMode::Normal;
    app.input.clear();
}

/// 搜索 Enter 处理
fn enter_search(app: &mut NotebookApp) {
    let keyword = app.input.trim().to_string();
    if keyword.is_empty() {
        app.clear_search();
        app.mode = AppMode::Normal;
    } else {
        app.search_filter = Some(keyword);
        let count = app.filtered_indices().len();
        if count > 0 {
            app.state.select(Some(0));
        } else {
            app.state.select(None);
        }
        app.update_preview();
        app.message = Some(format!(
            "搜索: {} (匹配 {} 条)",
            app.search_filter.as_deref().unwrap_or(""),
            count
        ));
        app.mode = AppMode::Normal;
    }
    app.input.clear();
}

// ========== 命令面板动作分发 ==========

/// 执行命令面板选中的动作
fn execute_cmd_popup_action(app: &mut NotebookApp) {
    let items = app.filtered_cmd_items();
    let Some(&(_orig_idx, cmd_key, _label)) = items.get(app.cmd_popup_selected) else {
        return;
    };

    match cmd_key {
        "search" => {
            app.mode = AppMode::Search;
            app.input.clear();
            app.cursor_pos = 0;
            app.message = None;
        }
        "rename" => {
            if let Some(idx) = app.selected_real_index() {
                app.input = app.notes[idx].path.clone();
                app.cursor_pos = app.input.chars().count();
                app.rename_index = Some(idx);
                app.mode = AppMode::Renaming;
                app.message = None;
            } else {
                app.mode = AppMode::Normal;
                app.message = Some("没有选中的笔记".to_string());
            }
        }
        "delete" => {
            if app.selected_real_index().is_some() {
                app.mode = AppMode::ConfirmDelete;
            } else {
                app.mode = AppMode::Normal;
                app.message = Some("没有选中的笔记".to_string());
            }
        }
        "mkdir" => {
            app.mode = AppMode::Mkdir;
            app.input.clear();
            app.cursor_pos = 0;
            app.message = None;
        }
        "mv" => {
            if let Some(name) = app.selected_name() {
                app.mode = AppMode::Mv;
                app.input = name;
                app.cursor_pos = app.input.chars().count();
                app.message = None;
            } else {
                app.mode = AppMode::Normal;
                app.message = Some("没有选中的笔记".to_string());
            }
        }
        "open" => {
            open_in_finder();
            app.mode = AppMode::Normal;
            app.message = Some("已打开目录".to_string());
        }
        "ratio" => {
            app.mode = AppMode::RatioInput;
            app.input = format!("{}:{}", app.panel_ratio, 100 - app.panel_ratio);
            app.cursor_pos = app.input.chars().count();
            app.message = None;
        }
        "help" => {
            app.mode = AppMode::Help;
        }
        _ => {}
    }
}

// ========== 输入光标辅助函数 ==========

/// 删除光标前一个字符
fn delete_char_before(app: &mut NotebookApp) {
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

/// 删除光标处字符
fn delete_char_at(app: &mut NotebookApp) {
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

/// 在光标处插入字符
fn insert_char(app: &mut NotebookApp, c: char) {
    let byte_idx = app
        .input
        .char_indices()
        .nth(app.cursor_pos)
        .map(|(i, _)| i)
        .unwrap_or(app.input.len());
    app.input.insert(byte_idx, c);
    app.cursor_pos += 1;
}
