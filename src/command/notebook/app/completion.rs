//! 输入框路径补全（mv / 新建 / 重命名 / 新建目录）。
//!
//! 交互：
//! - 在 Adding/Renaming/Mv/Mkdir 任一模式下按 `Tab` 弹出候选弹窗。
//! - ↑↓ 选择候选，Enter / Tab 确认，Esc 关闭（不退出输入模式）。
//! - 输入字符或退格时刷新候选列表。
//!
//! 候选源：
//! - Mv / Mkdir / Renaming：所有目录（每条带 `/` 后缀）。
//! - Adding：所有目录 + 现有笔记完整路径（便于参考、避免冲突）。
//!
//! 替换语义：补全只替换「光标前最后一个 `/` 之后的子串」，已输入的目录前缀保留。

use super::io::list_dirs;
use super::types::{AppMode, NotebookApp};

/// 进入补全模式：根据当前 input 与 mode 构建候选并打开弹窗。
pub fn open_completion(app: &mut NotebookApp) {
    app.completion_active = true;
    rebuild_candidates(app);
    if app.completion_candidates.is_empty() {
        // 没候选时直接关闭
        close_completion(app);
    }
}

/// 关闭补全弹窗，重置状态。
pub fn close_completion(app: &mut NotebookApp) {
    app.completion_active = false;
    app.completion_candidates.clear();
    app.completion_selected = 0;
    app.completion_replace_start = 0;
}

/// 根据当前 input/cursor 重新过滤候选。供输入字符/退格后调用。
pub fn rebuild_candidates(app: &mut NotebookApp) {
    let (replace_start, prefix) = locate_replace_segment(app);
    app.completion_replace_start = replace_start;

    let lower = prefix.to_lowercase();
    let candidates = candidate_pool(app.mode.clone());

    let mut filtered: Vec<String> = candidates
        .into_iter()
        .filter(|c| {
            // 前缀匹配（大小写不敏感）；空 prefix 时全部展示
            lower.is_empty() || c.to_lowercase().starts_with(&lower)
        })
        .collect();

    filtered.sort();
    filtered.dedup();

    app.completion_candidates = filtered;
    if app.completion_selected >= app.completion_candidates.len() {
        app.completion_selected = 0;
    }
}

/// 接受当前选中候选：把 `input[replace_start..cursor_pos]` 替换为候选；光标移到末尾。
pub fn accept_completion(app: &mut NotebookApp) {
    if !app.completion_active || app.completion_candidates.is_empty() {
        return;
    }
    let chosen = app.completion_candidates[app.completion_selected].clone();

    // 字符索引 → 字节索引
    let start_byte = char_idx_to_byte(&app.input, app.completion_replace_start);
    let cursor_byte = char_idx_to_byte(&app.input, app.cursor_pos);
    let (start, end) = if start_byte <= cursor_byte {
        (start_byte, cursor_byte)
    } else {
        (cursor_byte, cursor_byte)
    };

    app.input.replace_range(start..end, &chosen);
    // 重新计算光标 → 替换片段末尾
    let new_cursor_byte = start + chosen.len();
    app.cursor_pos = app.input[..new_cursor_byte].chars().count();

    close_completion(app);
}

/// 移动候选高亮。
pub fn move_completion_up(app: &mut NotebookApp) {
    let n = app.completion_candidates.len();
    if n == 0 {
        return;
    }
    app.completion_selected = if app.completion_selected == 0 {
        n - 1
    } else {
        app.completion_selected - 1
    };
}

pub fn move_completion_down(app: &mut NotebookApp) {
    let n = app.completion_candidates.len();
    if n == 0 {
        return;
    }
    app.completion_selected = (app.completion_selected + 1) % n;
}

// ========== 内部辅助 ==========

/// 定位「光标前最后一个 `/` 之后的子串」：
/// 返回 (替换起点字符索引, 该片段字符串副本)。
fn locate_replace_segment(app: &NotebookApp) -> (usize, String) {
    let chars: Vec<char> = app.input.chars().collect();
    let upto = app.cursor_pos.min(chars.len());
    // 在 chars[..upto] 中找最后一个 '/'
    let mut last_slash: Option<usize> = None;
    for (i, ch) in chars[..upto].iter().enumerate() {
        if *ch == '/' {
            last_slash = Some(i);
        }
    }
    let start = last_slash.map(|i| i + 1).unwrap_or(0);
    let prefix: String = chars[start..upto].iter().collect();
    (start, prefix)
}

/// 不同模式下的候选池。
fn candidate_pool(mode: AppMode) -> Vec<String> {
    let dirs = list_dirs();
    let with_slash: Vec<String> = dirs.into_iter().map(|d| format!("{}/", d)).collect();

    match mode {
        AppMode::Adding => {
            // 目录 + 已有笔记路径（让用户能参考但不强制选）
            let mut pool = with_slash;
            for n in super::io::load_notes() {
                pool.push(n.path);
            }
            pool
        }
        AppMode::Mv | AppMode::Renaming | AppMode::Mkdir => with_slash,
        _ => Vec::new(),
    }
}

/// 字符索引转字节索引。
fn char_idx_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}
