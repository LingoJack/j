use super::model::ChatMessage;
use crate::error;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ========== 数据结构 ==========

/// 归档数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatArchive {
    /// 归档名称
    pub name: String,
    /// 创建时间（ISO 8601 格式）
    pub created_at: String,
    /// 消息列表
    pub messages: Vec<ChatMessage>,
}

// ========== 文件路径 ==========

/// 获取归档目录路径: ~/.jdata/agent/data/archives/
pub fn get_archives_dir() -> PathBuf {
    super::model::agent_data_dir().join("archives")
}

/// 确保归档目录存在
pub fn ensure_archives_dir() -> std::io::Result<()> {
    let dir = get_archives_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(())
}

// ========== 归档操作 ==========

/// 列出所有归档文件
pub fn list_archives() -> Vec<ChatArchive> {
    let dir = get_archives_dir();
    if !dir.exists() {
        return Vec::new();
    }

    let mut archives = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    match serde_json::from_str::<ChatArchive>(&content) {
                        Ok(archive) => archives.push(archive),
                        Err(e) => {
                            error!("[list_archives] 解析归档文件失败: {:?}, 错误: {}", path, e);
                        }
                    }
                }
            }
        }
    }

    // 按创建时间倒序排列
    archives.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    archives
}

/// 创建新归档
pub fn create_archive(name: &str, messages: Vec<ChatMessage>) -> Result<ChatArchive, String> {
    // 校验名称
    validate_archive_name(name)?;

    // 确保归档目录存在
    if let Err(e) = ensure_archives_dir() {
        return Err(format!("创建归档目录失败: {}", e));
    }

    let now: DateTime<Utc> = Utc::now();
    let archive = ChatArchive {
        name: name.to_string(),
        created_at: now.to_rfc3339(),
        messages,
    };

    let path = get_archive_path(name);
    let json =
        serde_json::to_string_pretty(&archive).map_err(|e| format!("序列化归档失败: {}", e))?;

    fs::write(&path, json).map_err(|e| format!("写入归档文件失败: {}", e))?;

    Ok(archive)
}

/// 从归档恢复消息
pub fn restore_archive(name: &str) -> Result<Vec<ChatMessage>, String> {
    let path = get_archive_path(name);

    if !path.exists() {
        return Err(format!("归档文件不存在: {}", name));
    }

    let content = fs::read_to_string(&path).map_err(|e| format!("读取归档文件失败: {}", e))?;

    let archive: ChatArchive =
        serde_json::from_str(&content).map_err(|e| format!("解析归档文件失败: {}", e))?;

    Ok(archive.messages)
}

/// 删除归档
pub fn delete_archive(name: &str) -> Result<(), String> {
    let path = get_archive_path(name);

    if !path.exists() {
        return Err(format!("归档文件不存在: {}", name));
    }

    fs::remove_file(&path).map_err(|e| format!("删除归档文件失败: {}", e))?;

    Ok(())
}

/// 校验归档名称合法性
pub fn validate_archive_name(name: &str) -> Result<(), String> {
    // 检查名称长度
    if name.is_empty() {
        return Err("归档名称不能为空".to_string());
    }
    if name.len() > 50 {
        return Err("归档名称过长，最多 50 字符".to_string());
    }

    // 检查非法字符
    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    for c in invalid_chars {
        if name.contains(c) {
            return Err(format!("归档名称包含非法字符: {}", c));
        }
    }

    Ok(())
}

/// 生成默认归档名称（格式：archive-YYYY-MM-DD，重名时自动添加后缀）
pub fn generate_default_archive_name() -> String {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let base_name = format!("archive-{}", today);

    // 如果基础名称不存在，直接使用
    if !archive_exists(&base_name) {
        return base_name;
    }

    // 重名时添加后缀 (1), (2), ...
    let mut suffix = 1;
    loop {
        let name = format!("{}({})", base_name, suffix);
        if !archive_exists(&name) {
            return name;
        }
        suffix += 1;
    }
}

// ========== 辅助函数 ==========

/// 检查归档是否存在
pub fn archive_exists(name: &str) -> bool {
    let path = get_archives_dir().join(format!("{}.json", name));
    path.exists()
}

/// 获取归档文件路径
fn get_archive_path(name: &str) -> PathBuf {
    get_archives_dir().join(format!("{}.json", name))
}
