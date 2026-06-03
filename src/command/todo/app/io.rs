// ========== 文件路径与数据读写 ==========

use super::types::{TodoItem, TodoList};
use crate::config::YamlConfig;
use crate::error;
use std::fs;
use std::path::PathBuf;

/// 获取 todo 数据目录: ~/.jdata/report/
pub fn todo_dir() -> PathBuf {
    let dir = YamlConfig::data_dir().join("report");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 获取 todo 数据文件路径: ~/.jdata/report/todo.jsonl
pub fn todo_file_path() -> PathBuf {
    todo_dir().join("todo.jsonl")
}

/// 从 JSONL 文件加载待办列表（每行一个 JSON 对象，对应一条 TodoItem）
pub fn load_todo_list() -> TodoList {
    let path = todo_file_path();
    if !path.exists() {
        return TodoList::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => {
            let mut items = Vec::new();
            for (line_no, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                match serde_json::from_str::<TodoItem>(trimmed) {
                    Ok(item) => items.push(item),
                    Err(e) => {
                        error!("✖️ todo.jsonl 第 {} 行解析失败，已跳过: {}", line_no + 1, e);
                    }
                }
            }
            TodoList { items }
        }
        Err(e) => {
            error!("✖️ 读取 todo.jsonl 失败: {}", e);
            TodoList::default()
        }
    }
}

/// 保存待办列表到 JSONL 文件（每行一个 JSON 对象，对应一条 TodoItem）
pub fn save_todo_list(list: &TodoList) -> bool {
    let path = todo_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut lines = Vec::with_capacity(list.items.len());
    for item in &list.items {
        match serde_json::to_string(item) {
            Ok(json) => lines.push(json),
            Err(e) => {
                error!("✖️ 序列化待办项失败: {}", e);
                return false;
            }
        }
    }
    let content = lines.join("\n");
    // 确保文件末尾有换行
    let content = if content.is_empty() {
        String::new()
    } else {
        format!("{}\n", content)
    };
    match fs::write(&path, content) {
        Ok(_) => true,
        Err(e) => {
            error!("✖️ 保存 todo.jsonl 失败: {}", e);
            false
        }
    }
}
