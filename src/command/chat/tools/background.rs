use super::{PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params};
use crate::util::safe_lock;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::AtomicBool};
use std::time::{Duration, Instant};

// ========== BgTask ==========

/// 后台任务状态
pub(super) struct BgTask {
    pub task_id: String,
    pub command: String,
    pub status: String, // "running" | "completed" | "error" | "timeout"
    /// 共享输出缓冲区，reader 线程实时写入，查询时可直接读取中间输出
    pub output_buffer: Arc<Mutex<String>>,
    pub result: Option<String>,
    /// 任务启动时间，用于计算已运行时长
    pub started_at: Instant,
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
    pub(super) tasks: Mutex<HashMap<String, BgTask>>,
    notifications: Mutex<Vec<BgNotification>>,
    next_id: Mutex<u64>,
}

impl Default for BackgroundManager {
    fn default() -> Self {
        Self::new()
    }
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
    /// 返回 task_id 和共享输出缓冲区的 Arc，调用方将 buffer 传给 reader 线程实现实时写入
    pub fn spawn_command(
        &self,
        command: &str,
        _cwd: Option<String>,
        _timeout_secs: u64,
    ) -> (String, Arc<Mutex<String>>) {
        let task_id = self.gen_id();
        let output_buffer = Arc::new(Mutex::new(String::new()));

        let bg_task = BgTask {
            task_id: task_id.clone(),
            command: command.to_string(),
            status: "running".to_string(),
            output_buffer: Arc::clone(&output_buffer),
            result: None,
            started_at: Instant::now(),
        };

        {
            let mut tasks = safe_lock(&self.tasks, "BackgroundManager::spawn_command");
            tasks.insert(task_id.clone(), bg_task);
        }

        (task_id, output_buffer)
    }

    /// 内部方法：标记任务完成并添加通知
    pub fn complete_task(&self, task_id: &str, status: &str, result: String) {
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

    /// 查询单个后台任务状态（包括中间输出）
    pub fn get_task_status(&self, task_id: &str) -> Option<Value> {
        let tasks = safe_lock(&self.tasks, "BackgroundManager::get_task_status");
        tasks.get(task_id).map(|t| {
            // 优先从 output_buffer 读取（包含实时中间输出），回退到 result（最终结果）
            let output = {
                let buf = safe_lock(&t.output_buffer, "BgTask::output_buffer");
                if buf.is_empty() {
                    t.result.clone()
                } else {
                    Some(buf.clone())
                }
            };
            json!({
                "task_id": t.task_id,
                "command": t.command,
                "status": t.status,
                "output": output,
            })
        })
    }

    /// 检查任务是否仍在运行
    pub fn is_running(&self, task_id: &str) -> bool {
        let tasks = safe_lock(&self.tasks, "BackgroundManager::is_running");
        tasks
            .get(task_id)
            .map(|t| t.status == "running")
            .unwrap_or(false)
    }

    /// 列出当前所有 status == "running" 的任务，用于注入 LLM 上下文
    /// 返回 (task_id, command 摘要, 已运行秒数) 的列表，按 task_id 排序
    pub fn list_running(&self) -> Vec<(String, String, u64)> {
        let tasks = safe_lock(&self.tasks, "BackgroundManager::list_running");
        let now = Instant::now();
        let mut out: Vec<_> = tasks
            .values()
            .filter(|t| t.status == "running")
            .map(|t| {
                let elapsed = now.duration_since(t.started_at).as_secs();
                // command 截断到 80 字符，避免污染上下文
                let cmd_summary = if t.command.chars().count() > 80 {
                    let truncated: String = t.command.chars().take(77).collect();
                    format!("{}...", truncated)
                } else {
                    t.command.clone()
                };
                (t.task_id.clone(), cmd_summary, elapsed)
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

/// 构建运行中后台任务摘要，用于系统提示词的 {{.background_tasks}} 占位符
pub fn build_running_summary(manager: &Arc<BackgroundManager>) -> String {
    let running = manager.list_running();
    if running.is_empty() {
        return String::new();
    }
    let mut md = String::from(
        "## Background Tasks\n\n\
         The following background tasks are still running. \
         Use TaskOutput to wait for or check their results when needed. \
         Do not re-spawn these commands.\n",
    );
    for (id, cmd, elapsed) in &running {
        md.push_str(&format!(
            "- {} (running {}): {}\n",
            id,
            format_elapsed(*elapsed),
            cmd
        ));
    }
    md.trim_end().to_string()
}

fn format_elapsed(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m{}s", secs / 60, secs % 60)
    } else {
        format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
    }
}

// ========== TaskOutputTool ==========

/// TaskOutputTool 参数
#[derive(Deserialize, JsonSchema)]
struct TaskOutputParams {
    /// The task ID to get output from (returned by Bash with run_in_background: true)
    task_id: String,
    /// Whether to wait for task completion (default: true). Set to false for a non-blocking check of current status.
    #[serde(default = "default_block")]
    block: bool,
    /// Max wait time in milliseconds when block=true (default: 30000, max: 600000)
    #[serde(default = "default_timeout_ms")]
    timeout: u64,
}

fn default_block() -> bool {
    true
}

fn default_timeout_ms() -> u64 {
    30_000
}

/// 查询后台任务输出的工具（替代 CheckBackgroundTool）
pub struct TaskOutputTool {
    pub manager: Arc<BackgroundManager>,
}

impl Tool for TaskOutputTool {
    fn name(&self) -> &str {
        "TaskOutput"
    }

    fn description(&self) -> &str {
        r#"
        Retrieves output from a running or completed background task (started via Bash with run_in_background: true).
        Use block=true (default) to wait for task completion; use block=false for a non-blocking status check.
        Returns the task output along with status information.
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<TaskOutputParams>()
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: TaskOutputParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let timeout_ms = params.timeout.min(600_000);

        // 若任务不存在，直接报错
        if self.manager.get_task_status(&params.task_id).is_none() {
            return ToolResult {
                output: format!("后台任务 {} 不存在", params.task_id),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        // block=true 且任务仍在运行时，轮询等待
        if params.block && self.manager.is_running(&params.task_id) {
            let deadline = Instant::now() + Duration::from_millis(timeout_ms);
            loop {
                if !self.manager.is_running(&params.task_id) {
                    break;
                }
                // ★ 检查取消信号：用户取消请求时应立即中断等待
                if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
                    let info = self
                        .manager
                        .get_task_status(&params.task_id)
                        .unwrap_or(json!({}));
                    let mut obj = info.clone();
                    if let Some(map) = obj.as_object_mut() {
                        map.insert(
                            "note".to_string(),
                            json!("cancelled: request was cancelled while waiting for task output"),
                        );
                    }
                    return ToolResult {
                        output: serde_json::to_string_pretty(&obj).unwrap_or_default(),
                        is_error: false,
                        images: vec![],
                plan_decision: PlanDecision::None,
                    };
                }
                if Instant::now() >= deadline {
                    // 超时，返回当前状态
                    let info = self
                        .manager
                        .get_task_status(&params.task_id)
                        .unwrap_or(json!({}));
                    let mut obj = info.clone();
                    if let Some(map) = obj.as_object_mut() {
                        map.insert(
                            "note".to_string(),
                            json!("still running: timeout waiting for completion"),
                        );
                    }
                    return ToolResult {
                        output: serde_json::to_string_pretty(&obj).unwrap_or_default(),
                        is_error: false,
                        images: vec![],
                plan_decision: PlanDecision::None,
                    };
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        // 返回当前状态（已完成或 block=false）
        match self.manager.get_task_status(&params.task_id) {
            Some(info) => ToolResult {
                output: serde_json::to_string_pretty(&info).unwrap_or_default(),
                is_error: false,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
            None => ToolResult {
                output: format!("后台任务 {} 不存在", params.task_id),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }
}
