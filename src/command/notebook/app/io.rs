//! Notebook 文件 I/O 操作：读写笔记、路径管理、配置持久化。

use crate::command::chat::storage::load_agent_config;
use crate::config::YamlConfig;
use crate::constants::{config_key, section, shell};
use crate::error;
use crate::info;
use crate::theme::Theme;
use chrono::{DateTime, Local};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use super::types::ExpandedDirs;

// ========== 路径工具 ==========

/// 获取 notebook 目录路径: ~/.jdata/notebook/
pub fn notebook_dir() -> PathBuf {
    YamlConfig::notebook_dir()
}

/// 获取笔记文件路径
pub fn note_file_path(name: &str) -> PathBuf {
    notebook_dir().join(format!("{}.md", name))
}

// ========== 配置持久化 ==========

/// 从 YamlConfig setting section 加载面板比例
pub fn load_panel_ratio() -> Option<u16> {
    YamlConfig::load()
        .get_property(section::SETTING, config_key::NOTEBOOK_PANEL_RATIO)
        .and_then(|v| v.parse().ok())
}

/// 保存面板比例到 YamlConfig setting section
pub fn save_panel_ratio(ratio: u16) {
    let mut config = YamlConfig::load();
    if let Err(e) = config.set_property(
        section::SETTING,
        config_key::NOTEBOOK_PANEL_RATIO,
        &ratio.to_string(),
    ) {
        crate::util::log::write_error_log("保存面板比例", &e);
    }
}

/// 从 YamlConfig setting section 加载展开目录列表
pub fn load_expanded_dirs() -> ExpandedDirs {
    YamlConfig::load()
        .get_property(section::SETTING, config_key::NOTEBOOK_EXPANDED_DIRS)
        .and_then(|v| serde_json::from_str(v).ok())
        .unwrap_or_default()
}

/// 保存展开目录列表到 YamlConfig setting section
pub fn save_expanded_dirs(dirs: &ExpandedDirs) {
    if let Ok(json) = serde_json::to_string(dirs) {
        let mut config = YamlConfig::load();
        if let Err(e) =
            config.set_property(section::SETTING, config_key::NOTEBOOK_EXPANDED_DIRS, &json)
        {
            crate::util::log::write_error_log("保存展开目录", &e);
        }
    }
}

// ========== 笔记读写 ==========

/// 从磁盘加载笔记列表（递归子目录），按修改时间倒序
pub fn load_notes() -> Vec<super::types::NoteItem> {
    let dir = notebook_dir();
    let mut notes = Vec::new();
    walk_dir_for_notes(&dir, "", &mut notes);
    notes.sort_by_key(|b| std::cmp::Reverse(b.mtime));
    notes
}

fn walk_dir_for_notes(
    dir: &std::path::Path,
    prefix: &str,
    notes: &mut Vec<super::types::NoteItem>,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // 跳过隐藏目录（以 . 开头）
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                if dir_name.starts_with('.') {
                    continue;
                }
                let sub_prefix = if prefix.is_empty() {
                    dir_name.to_string()
                } else {
                    format!("{}/{}", prefix, dir_name)
                };
                walk_dir_for_notes(&path, &sub_prefix, notes);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                let stem = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let note_path = if prefix.is_empty() {
                    stem
                } else {
                    format!("{}/{}", prefix, stem)
                };
                let mtime = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(std::time::UNIX_EPOCH);
                notes.push(super::types::NoteItem {
                    path: note_path,
                    mtime,
                });
            }
        }
    }
}

/// 列出 notebook 下的所有子目录（相对路径）
pub fn list_dirs() -> Vec<String> {
    let dir = notebook_dir();
    let mut dirs = Vec::new();
    walk_dir_for_dirs(&dir, "", &mut dirs);
    dirs.sort();
    dirs
}

fn walk_dir_for_dirs(dir: &std::path::Path, prefix: &str, dirs: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                if dir_name.starts_with('.') {
                    continue;
                }
                let dir_path = if prefix.is_empty() {
                    dir_name.to_string()
                } else {
                    format!("{}/{}", prefix, dir_name)
                };
                dirs.push(dir_path.clone());
                walk_dir_for_dirs(&path, &dir_path, dirs);
            }
        }
    }
}

/// 读取笔记内容
pub fn read_note_content(name: &str) -> Option<String> {
    let path = note_file_path(name);
    fs::read_to_string(path).ok()
}

/// 格式化 SystemTime 为可读字符串
pub fn format_time(time: std::time::SystemTime) -> String {
    let dt: DateTime<Local> = time.into();
    dt.format("%Y-%m-%d %H:%M").to_string()
}

// ========== 编辑器操作 ==========

/// 用 Markdown 编辑器编辑笔记（在已有 terminal 上），返回是否有内容变化
#[allow(dead_code)]
pub fn edit_note_on_terminal(
    title: &str,
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
) -> bool {
    let file_path = note_file_path(title);
    let (content, is_new) = read_note_content_or_new(&file_path);

    let editor_title = if is_new {
        format!("{} (新笔记)", title)
    } else {
        title.to_string()
    };

    let theme = Theme::from_name(&load_agent_config().theme);
    match crate::tui::editor_markdown::open_markdown_editor_on_terminal(
        terminal,
        &editor_title,
        &content,
        &theme,
    ) {
        Ok((Some(new_content), _)) => {
            save_if_changed(&file_path, title, &content, &new_content, is_new)
        }
        Ok((None, _)) => {
            info!("已取消编辑");
            false
        }
        Err(e) => {
            error!("编辑器启动失败: {}", e);
            false
        }
    }
}

/// 用 Markdown 编辑器编辑笔记（独立终端），返回是否有内容变化
pub fn edit_note_with_editor(title: &str) -> bool {
    let file_path = note_file_path(title);
    let (content, is_new) = read_note_content_or_new(&file_path);

    let editor_title = if is_new {
        format!("{} (新笔记)", title)
    } else {
        title.to_string()
    };

    let theme = Theme::from_name(&load_agent_config().theme);
    match crate::tui::editor_markdown::open_markdown_editor(&editor_title, &content, &theme) {
        Ok((Some(new_content), _)) => {
            save_if_changed(&file_path, title, &content, &new_content, is_new)
        }
        Ok((None, _)) => {
            info!("已取消编辑");
            false
        }
        Err(e) => {
            error!("编辑器启动失败: {}", e);
            false
        }
    }
}

/// 读取笔记文件内容，不存在则返回空字符串
fn read_note_content_or_new(path: &std::path::Path) -> (String, bool) {
    if path.exists() {
        match fs::read_to_string(path) {
            Ok(c) => (c, false),
            Err(e) => {
                error!("读取笔记失败: {}", e);
                (String::new(), true)
            }
        }
    } else {
        (String::new(), true)
    }
}

/// 仅当内容变化时才写入文件；新文件（文件不存在）时始终写入
fn save_if_changed(
    file_path: &std::path::Path,
    title: &str,
    old_content: &str,
    new_content: &str,
    is_new: bool,
) -> bool {
    if is_new || new_content != old_content {
        if let Some(parent) = file_path.parent()
            && !parent.exists()
            && let Err(e) = fs::create_dir_all(parent)
        {
            error!("创建目录失败: {} - {}", parent.display(), e);
            return false;
        }
        match fs::write(file_path, new_content) {
            Ok(()) => {
                info!("笔记已保存: {}", title);
                return true;
            }
            Err(e) => error!("保存笔记失败: {}", e),
        }
    } else {
        info!("内容未变化，跳过保存");
    }
    false
}

// ========== 剪切板 ==========

/// 复制内容到系统剪切板（使用 arboard 库，支持 macOS/Linux/Windows）
#[allow(dead_code)]
pub fn copy_to_clipboard(content: &str) -> bool {
    use arboard::Clipboard;

    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(_) => return false,
    };
    clipboard.set_text(content).is_ok()
}

// ========== 外部操作 ==========

/// 在 Finder 中打开 notebook 目录
pub fn open_in_finder() {
    let dir = notebook_dir();
    let path = dir.to_string_lossy().to_string();
    let os = std::env::consts::OS;
    let result = if os == shell::MACOS_OS {
        Command::new("open").arg(&path).status()
    } else if os == shell::WINDOWS_OS {
        Command::new(shell::WINDOWS_CMD)
            .args([shell::WINDOWS_CMD_FLAG, "start", "", &path])
            .status()
    } else {
        Command::new("xdg-open").arg(&path).status()
    };

    if let Err(e) = result {
        error!("打开目录失败: {}", e);
    }
}

/// 清理 notebook 下的空目录（递归，从深层到浅层）
pub fn cleanup_empty_dirs() {
    let dir = notebook_dir();
    let all_dirs = list_dirs();
    // 按路径深度倒序排列，先删除最深的空目录
    let mut sorted_dirs: Vec<String> = all_dirs;
    sorted_dirs.sort_by_key(|b| std::cmp::Reverse(b.matches('/').count()));
    for dir_path in &sorted_dirs {
        let full_path = dir.join(dir_path);
        if full_path.is_dir() {
            // 检查目录是否为空（只有子目录且子目录也已为空也算空）
            if let Ok(entries) = fs::read_dir(&full_path) {
                let has_content = entries
                    .flat_map(|e| e.ok())
                    .any(|e| e.path().is_file() || e.path().is_dir());
                if !has_content {
                    let _ = fs::remove_dir(&full_path);
                }
            }
        }
    }
}

/// 解析比例字符串 (如 "20:80") 为左侧面板百分比
pub fn parse_ratio(input: &str) -> Option<u16> {
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let left: u16 = parts[0].parse().ok()?;
    let right: u16 = parts[1].parse().ok()?;
    if left == 0 || right == 0 {
        return None;
    }
    let pct = left * 100 / (left + right);
    Some(pct.clamp(15, 60))
}
