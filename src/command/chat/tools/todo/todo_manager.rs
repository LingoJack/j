use super::entity::TodoItem;
use crate::command::chat::permission::JcliConfig;
use crate::util::safe_lock;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

/// Todo 管理器：轻量级方向跟踪，单文件持久化
pub struct TodoManager {
    items: Mutex<Vec<TodoItem>>,
    file_path: PathBuf,
    /// 距离上次 TodoWrite 调用的轮数（用于 nag reminder）
    turns_without_call: AtomicU32,
}

impl TodoManager {
    pub fn new() -> Self {
        // 优先使用 .jcli/todos.json，找不到则在 cwd 下创建 .jcli/todos.json
        let config_dir = JcliConfig::find_config_dir().or_else(JcliConfig::ensure_config_dir);
        let file_path = match config_dir {
            Some(dir) => {
                let _ = fs::create_dir_all(&dir);
                dir.join("todos.json")
            }
            None => {
                // 极端 fallback：使用全局目录
                let data_dir = crate::config::YamlConfig::data_dir();
                let dir = data_dir.join("agent").join("data");
                let _ = fs::create_dir_all(&dir);
                dir.join("todos.json")
            }
        };

        // 从磁盘加载已有数据
        let items = if file_path.exists() {
            fs::read_to_string(&file_path)
                .ok()
                .and_then(|data| serde_json::from_str::<Vec<TodoItem>>(&data).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Self {
            items: Mutex::new(items),
            file_path,
            turns_without_call: AtomicU32::new(0),
        }
    }

    /// 写入 todos。merge=false 替换全部；merge=true 按 id 合并更新。
    /// 返回写入后的完整列表。
    pub fn write_todos(
        &self,
        new_items: Vec<TodoItem>,
        merge: bool,
    ) -> Result<Vec<TodoItem>, String> {
        let mut items = safe_lock(&self.items, "TodoManager::write_todos");

        if merge {
            // 合并模式：按 id 更新已有项，添加新项
            for new_item in new_items {
                if let Some(existing) = items.iter_mut().find(|i| i.id == new_item.id) {
                    existing.content = new_item.content;
                    existing.status = new_item.status;
                } else {
                    // 新项，自动分配 id（如果为空）
                    let item = if new_item.id.is_empty() {
                        TodoItem {
                            id: self.next_id_from(&items),
                            ..new_item
                        }
                    } else {
                        new_item
                    };
                    items.push(item);
                }
            }
        } else {
            // 替换模式：用新列表替换全部
            let mut final_items = Vec::with_capacity(new_items.len());
            for (idx, item) in new_items.into_iter().enumerate() {
                let item = if item.id.is_empty() {
                    TodoItem {
                        id: (idx + 1).to_string(),
                        ..item
                    }
                } else {
                    item
                };
                final_items.push(item);
            }
            *items = final_items;
        }

        // 强制只允许一个 in_progress：最后设为 in_progress 的胜出，其余降为 pending
        self.enforce_single_in_progress(&mut items);

        // 重置 nag 计数器
        self.turns_without_call.store(0, Ordering::Relaxed);

        // 持久化
        self.save(&items)?;
        Ok(items.clone())
    }

    #[allow(dead_code)]
    pub fn list_todos(&self) -> Vec<TodoItem> {
        let items = safe_lock(&self.items, "TodoManager::list_todos");
        items.clone()
    }

    pub fn has_todos(&self) -> bool {
        let items = safe_lock(&self.items, "TodoManager::has_todos");
        !items.is_empty()
    }

    /// 格式化当前 todos 为可读字符串（供 nag reminder 使用）
    pub fn format_todos_summary(&self) -> String {
        let items = safe_lock(&self.items, "TodoManager::format_todos_summary");
        if items.is_empty() {
            return "No active todos.".to_string();
        }
        let mut summary = String::new();
        for item in items.iter() {
            let icon = match item.status.as_str() {
                "completed" => "✅",
                "in_progress" => "🔄",
                "cancelled" => "❌",
                _ => "⬜",
            };
            summary.push_str(&format!(
                "{} [{}] {}: {}\n",
                icon, item.id, item.status, item.content
            ));
        }
        summary.trim_end().to_string()
    }

    /// 每轮 agent loop 调用，递增计数器
    pub fn increment_turn(&self) {
        self.turns_without_call.fetch_add(1, Ordering::Relaxed);
    }

    /// 获取距离上次调用的轮数
    pub fn turns_since_last_call(&self) -> u32 {
        self.turns_without_call.load(Ordering::Relaxed)
    }

    // ── 内部方法 ──

    /// 生成下一个 id（基于现有最大数字 id + 1）
    fn next_id_from(&self, items: &[TodoItem]) -> String {
        let max_id = items
            .iter()
            .filter_map(|i| i.id.parse::<u64>().ok())
            .max()
            .unwrap_or(0);
        (max_id + 1).to_string()
    }

    /// 确保只有一个 in_progress：保留最后一个，其余降为 pending
    fn enforce_single_in_progress(&self, items: &mut [TodoItem]) {
        let in_progress_indices: Vec<usize> = items
            .iter()
            .enumerate()
            .filter(|(_, i)| i.status == "in_progress")
            .map(|(idx, _)| idx)
            .collect();

        if in_progress_indices.len() > 1 {
            // 保留最后一个 in_progress，其余降为 pending
            for &idx in &in_progress_indices[..in_progress_indices.len() - 1] {
                items[idx].status = "pending".to_string();
            }
        }
    }

    fn save(&self, items: &[TodoItem]) -> Result<(), String> {
        let data = serde_json::to_string_pretty(items)
            .map_err(|e| format!("Failed to serialize todos: {}", e))?;
        fs::write(&self.file_path, data).map_err(|e| format!("Failed to write todos: {}", e))
    }
}
