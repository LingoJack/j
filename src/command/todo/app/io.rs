// ========== 文件路径与数据读写 ==========

use super::types::TodoList;
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

/// 获取 todo 数据文件路径: ~/.jdata/report/todo.json
pub fn todo_file_path() -> PathBuf {
    todo_dir().join("todo.json")
}

/// 从文件加载待办列表
pub fn load_todo_list() -> TodoList {
    let path = todo_file_path();
    if !path.exists() {
        return TodoList::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            error!("✖️ 解析 todo.json 失败: {}", e);
            TodoList::default()
        }),
        Err(e) => {
            error!("✖️ 读取 todo.json 失败: {}", e);
            TodoList::default()
        }
    }
}

/// 保存待办列表到文件
pub fn save_todo_list(list: &TodoList) -> bool {
    let path = todo_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(list) {
        Ok(json) => match fs::write(&path, json) {
            Ok(_) => true,
            Err(e) => {
                error!("✖️ 保存 todo.json 失败: {}", e);
                false
            }
        },
        Err(e) => {
            error!("✖️ 序列化 todo 列表失败: {}", e);
            false
        }
    }
}
