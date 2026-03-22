use super::{Tool, ToolResult};
use crate::util::safe_lock;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::AtomicBool};
use std::time::Instant;

// ========== BgTask ==========

/// 后台任务状态
struct BgTask {
    task_id: String,
    command: String,
    status: String, // "running" | "completed" | "error" | "timeout"
    result: Option<String>,
    #[allow(dead_code)]
    started_at: Instant,
}

/// 后台任务完成通知
pub struct BgNotification {
    pub task_id: String,
    pub command: String,
    pub status: String,
    pub result: String,
}

// ========== BackgroundManager ==========

/// 后台任务管理器（Send + Sync，可跨线程共享）
pub struct BackgroundManager {
    tasks: Mutex<HashMap<String, BgTask>>,
    notifications: Mutex<Vec<BgNotification>>,
    next_id: Mutex<u64>,
}

impl BackgroundManager {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
            notifications: Mutex::new(Vec::new()),
            next_id: Mutex::new(1),
        }
    }

    /// 生成唯一的后台任务 ID
    fn gen_id(&self) -> String {
        let mut id = safe_lock(&self.next_id, "BackgroundManager::gen_id");
        let current = *id;
        *id += 1;
        format!("bg_{}", current)
    }

    /// 注册后台命令为 running 状态，返回 task_id（实际 spawn 在调用方完成）
    fn spawn_command(&self, command: &str) -> String {
        let task_id = self.gen_id();

        let bg_task = BgTask {
            task_id: task_id.clone(),
            command: command.to_string(),
            status: "running".to_string(),
            result: None,
            started_at: Instant::now(),
        };

        {
            let mut tasks = safe_lock(&self.tasks, "BackgroundManager::spawn_command");
            tasks.insert(task_id.clone(), bg_task);
        }

        task_id
    }

    /// 内部方法：标记任务完成并添加通知
    fn complete_task(&self, task_id: &str, status: &str, result: String) {
        let command;
        {
            let mut tasks = safe_lock(&self.tasks, "BackgroundManager::complete_task");
            if let Some(task) = tasks.get_mut(task_id) {
                task.status = status.to_string();
                task.result = Some(result.clone());
                command = task.command.clone();
            } else {
                return;
            }
        }
        {
            let mut notifs = safe_lock(&self.notifications, "BackgroundManager::complete_notify");
            notifs.push(BgNotification {
                task_id: task_id.to_string(),
                command,
                status: status.to_string(),
                result,
            });
        }
    }

    /// Drain 所有待处理的通知（agent loop 每轮调用）
    pub fn drain_notifications(&self) -> Vec<BgNotification> {
        let mut notifs = safe_lock(
            &self.notifications,
            "BackgroundManager::drain_notifications",
        );
        std::mem::take(&mut *notifs)
    }

    /// 查询单个后台任务状态
    fn get_task_status(&self, task_id: &str) -> Option<Value> {
        let tasks = safe_lock(&self.tasks, "BackgroundManager::get_task_status");
        tasks.get(task_id).map(|t| {
            json!({
                "task_id": t.task_id,
                "command": t.command,
                "status": t.status,
                "result": t.result,
            })
        })
    }

    /// 列出所有后台任务状态
    fn list_all(&self) -> Vec<Value> {
        let tasks = safe_lock(&self.tasks, "BackgroundManager::list_all");
        let mut items: Vec<Value> = tasks
            .values()
            .map(|t| {
                json!({
                    "task_id": t.task_id,
                    "command": t.command,
                    "status": t.status,
                    "has_result": t.result.is_some(),
                })
            })
            .collect();
        items.sort_by(|a, b| {
            a.get("task_id")
                .and_then(|v| v.as_str())
                .cmp(&b.get("task_id").and_then(|v| v.as_str()))
        });
        items
    }
}

// ========== BackgroundRunTool ==========

/// 后台运行命令的工具
pub struct BackgroundRunTool {
    pub manager: Arc<BackgroundManager>,
}

impl Tool for BackgroundRunTool {
    fn name(&self) -> &str {
        "BackgroundRun"
    }

    fn description(&self) -> &str {
        r#"
        Execute a shell command in a background thread, returning a task_id immediately without blocking the conversation.
        Suitable for long-running commands (e.g. builds, downloads, tests); a notification is sent upon completion.
        Use CheckBackground to query status and results.
        Note: background tasks do not support interactive input and are not preserved across session restarts.
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "description": {
                    "type": "string",
                    "description": "Short description of the command"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds, default 300, max 600"
                }
            },
            "required": ["command"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let parsed: Value = match serde_json::from_str(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        let command = match parsed.get("command").and_then(|c| c.as_str()) {
            Some(cmd) => cmd.to_string(),
            None => {
                return ToolResult {
                    output: "缺少 command 参数".to_string(),
                    is_error: true,
                };
            }
        };

        let _timeout_secs = parsed
            .get("timeout")
            .and_then(|t| t.as_u64())
            .unwrap_or(300)
            .min(600);

        // 安全检查
        if super::is_dangerous_command(&command) {
            return ToolResult {
                output: "该命令被安全策略拒绝执行".to_string(),
                is_error: true,
            };
        }

        let task_id = self.manager.spawn_command(&command);
        let manager = Arc::clone(&self.manager);
        let tid = task_id.clone();
        let cmd = command.clone();

        // Spawn 后台线程执行命令
        std::thread::spawn(move || {
            let mut child_cmd = std::process::Command::new("bash");
            child_cmd
                .arg("-c")
                .arg(&cmd)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            let child = match child_cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    manager.complete_task(&tid, "error", format!("启动失败: {}", e));
                    return;
                }
            };

            let output = match child.wait_with_output() {
                Ok(o) => o,
                Err(e) => {
                    manager.complete_task(&tid, "error", format!("等待进程失败: {}", e));
                    return;
                }
            };

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let mut result = String::new();
            if !stdout.is_empty() {
                result.push_str(&stdout);
            }
            if !stderr.is_empty() {
                if !result.is_empty() {
                    result.push_str("\n[stderr]\n");
                }
                result.push_str(&stderr);
            }
            if result.is_empty() {
                result = "(无输出)".to_string();
            } else {
                result = crate::util::text::sanitize_tool_output(&result);
            }

            let status = if output.status.success() {
                "completed"
            } else {
                "error"
            };
            manager.complete_task(&tid, status, result);
        });

        ToolResult {
            output: json!({
                "task_id": task_id,
                "command": command,
                "status": "running",
                "message": "命令已在后台启动，使用 CheckBackground 查询状态"
            })
            .to_string(),
            is_error: false,
        }
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        let parsed = serde_json::from_str::<Value>(arguments).ok();
        let cmd = parsed
            .as_ref()
            .and_then(|v| v.get("command").and_then(|c| c.as_str()))
            .unwrap_or(arguments);
        format!("Background execute: {}", cmd)
    }
}

// ========== CheckBackgroundTool ==========

/// 查询后台任务状态的工具
pub struct CheckBackgroundTool {
    pub manager: Arc<BackgroundManager>,
}

impl Tool for CheckBackgroundTool {
    fn name(&self) -> &str {
        "CheckBackground"
    }

    fn description(&self) -> &str {
        r#"
        Query the status and result of background tasks.
        Provide a task_id to query a single task, or omit it to list all background tasks.
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "Background task ID to query. Omit to list all tasks."
                }
            }
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let parsed: Value = serde_json::from_str(arguments).unwrap_or(json!({}));

        if let Some(task_id) = parsed.get("task_id").and_then(|v| v.as_str()) {
            // 查询单个任务
            match self.manager.get_task_status(task_id) {
                Some(info) => ToolResult {
                    output: serde_json::to_string_pretty(&info).unwrap_or_default(),
                    is_error: false,
                },
                None => ToolResult {
                    output: format!("后台任务 {} 不存在", task_id),
                    is_error: true,
                },
            }
        } else {
            // 列出所有任务
            let all = self.manager.list_all();
            if all.is_empty() {
                ToolResult {
                    output: "没有后台任务".to_string(),
                    is_error: false,
                }
            } else {
                ToolResult {
                    output: serde_json::to_string_pretty(&all).unwrap_or_default(),
                    is_error: false,
                }
            }
        }
    }
}
