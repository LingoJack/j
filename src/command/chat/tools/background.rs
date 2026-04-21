use super::{PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params};
use crate::command::chat::constants::{
    BG_TASK_CMD_DISPLAY_MAX_CHARS, BG_TASK_DEFAULT_TIMEOUT_MS, BG_TASK_MAX_TIMEOUT_MS,
};
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
    pub status: String, // "running" | "completed" | "error" | "timeout" | "dead"
    /// 共享输出缓冲区，reader 线程实时写入，查询时可直接读取中间输出
    pub output_buffer: Arc<Mutex<String>>,
    pub result: Option<String>,
    /// 任务启动时间，用于计算已运行时长
    pub started_at: Instant,
    /// 子进程 PID，用于存活检测（仅 shell 后台任务有值，SubAgent 后台无子进程）
    pub child_pid: Option<u32>,
}

/// 后台任务完成通知
#[derive(Debug)]
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

impl std::fmt::Debug for BackgroundManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tasks_count = self.tasks.lock().map_or(0, |t| t.len());
        let notif_count = self.notifications.lock().map_or(0, |n| n.len());
        let next_id = self.next_id.lock().map_or(0, |id| *id);
        f.debug_struct("BackgroundManager")
            .field("tasks_count", &tasks_count)
            .field("notifications_count", &notif_count)
            .field("next_id", &next_id)
            .finish()
    }
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
            child_pid: None,
        };

        {
            let mut tasks = safe_lock(&self.tasks, "BackgroundManager::spawn_command");
            tasks.insert(task_id.clone(), bg_task);
        }

        (task_id, output_buffer)
    }

    /// 更新子进程 PID（执行线程在 spawn 成功后调用）
    pub fn update_child_pid(&self, task_id: &str, pid: u32) {
        let mut tasks = safe_lock(&self.tasks, "BackgroundManager::update_child_pid");
        if let Some(task) = tasks.get_mut(task_id) {
            task.child_pid = Some(pid);
            crate::info!("[BgTask] 后台任务 {} 已关联子进程 PID: {}", task_id, pid);
        }
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
                    let truncated: String = t
                        .command
                        .chars()
                        .take(BG_TASK_CMD_DISPLAY_MAX_CHARS)
                        .collect();
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

    /// 清理已死进程：双重验证（PID 存在 + command 匹配）
    /// 在每次 LLM request 前调用，确保 system prompt 中的后台任务状态准确
    pub fn cleanup_dead_tasks(&self) {
        let mut tasks = safe_lock(&self.tasks, "BackgroundManager::cleanup_dead_tasks");
        let running_count = tasks.values().filter(|t| t.status == "running").count();

        if running_count == 0 {
            return;
        }

        crate::util::log::write_info_log(
            "BgTask::cleanup_dead_tasks",
            &format!("开始存活检测，共 {} 个 running 任务", running_count),
        );

        let mut dead_tasks = Vec::new();

        for task in tasks.values() {
            if task.status != "running" {
                continue;
            }

            let confirmed_alive = if let Some(pid) = task.child_pid {
                // PID 存活检测：kill(pid, 0) 判断进程是否仍存在
                if !process_exists(pid) {
                    crate::util::log::write_info_log(
                        "BgTask::cleanup_dead_tasks",
                        &format!("任务 {} (PID: {}) 进程不存在", task.task_id, pid),
                    );
                    false
                } else {
                    true
                }
            } else {
                // 无 PID（SubAgent 等），用 pgrep 备选验证
                crate::util::log::write_info_log(
                    "BgTask::cleanup_dead_tasks",
                    &format!(
                        "任务 {} 无 PID，使用 command 匹配检测 (cmd: {})",
                        task.task_id, task.command
                    ),
                );
                is_process_alive_by_command(&task.command)
            };

            if !confirmed_alive {
                dead_tasks.push((task.task_id.clone(), task.command.clone(), task.child_pid));
            }
        }

        // 更新状态并生成通知
        for (task_id, command, pid) in &dead_tasks {
            if let Some(task) = tasks.get_mut(task_id) {
                task.status = "dead".to_string();
                let pid_info = pid.map_or(String::new(), |p| format!(" (PID: {})", p));
                task.result = Some(format!(
                    "进程已终止{}：被外部杀死、崩溃或 PID 被复用",
                    pid_info
                ));
            }
            let pid_info = pid.map_or(String::new(), |p| format!("PID: {}", p));
            crate::util::log::write_info_log(
                "BgTask::cleanup_dead_tasks",
                &format!(
                    "任务 {} ({} cmd: {}) 已确认为 dead",
                    task_id, pid_info, command
                ),
            );
        }

        crate::util::log::write_info_log(
            "BgTask::cleanup_dead_tasks",
            &format!("存活检测完成，发现 {} 个 dead 任务", dead_tasks.len()),
        );

        // 将通知加入队列
        if !dead_tasks.is_empty() {
            let mut notifs = safe_lock(
                &self.notifications,
                "BackgroundManager::cleanup_dead_tasks_notify",
            );
            for (task_id, command, pid) in dead_tasks {
                let pid_info = pid.map_or(String::new(), |p| format!(" (PID: {})", p));
                notifs.push(BgNotification {
                    task_id,
                    command,
                    status: "dead".to_string(),
                    result: format!("进程已终止{}：被外部杀死、崩溃或 PID 被复用", pid_info),
                });
            }
        }
    }
}

// ========== 进程存活检测辅助函数 ==========

/// 第一层检测：通过 PID 检测进程是否存在
/// 使用 kill(pid, None) 发送 signal 0，只检测进程是否存在，不实际发送信号
#[cfg(unix)]
fn process_exists(pid: u32) -> bool {
    use nix::errno::Errno;
    use nix::sys::signal::kill;
    use nix::unistd::Pid;

    match kill(Pid::from_raw(pid as i32), None) {
        Ok(_) => true,
        Err(Errno::ESRCH) => false, // 进程不存在
        Err(_) => true,             // 其他错误（如权限不足），保守返回 true
    }
}

#[cfg(not(unix))]
fn process_exists(_pid: u32) -> bool {
    true
}

/// 第二层检测：读取 /proc/<pid>/cmdline 验证命令是否匹配
/// 用于防止 PID 复用导致的误判
#[cfg(unix)]
fn command_matches_pid(pid: u32, expected_command: &str) -> bool {
    use std::fs::File;
    use std::io::Read;

    let cmdline_path = format!("/proc/{}/cmdline", pid);
    let mut file = match File::open(&cmdline_path) {
        Ok(f) => f,
        Err(_) => return false, // 无法读取，可能进程已死
    };

    let mut cmdline_bytes = Vec::new();
    if file.read_to_end(&mut cmdline_bytes).is_err() {
        return false;
    }

    // /proc/<pid>/cmdline 格式：参数以 \0 分隔，需替换为空格
    let cmdline = String::from_utf8_lossy(&cmdline_bytes)
        .replace('\0', " ")
        .trim_end()
        .to_string();

    // 命令匹配逻辑：
    // expected_command 是 "bash -c xxx"，cmdline 是 "/bin/bash -c xxx"
    // 只需检查核心部分是否匹配（去除路径前缀）
    let expected_parts: Vec<&str> = expected_command.split_whitespace().collect();
    let cmdline_parts: Vec<&str> = cmdline.split_whitespace().collect();

    if expected_parts.is_empty() || cmdline_parts.is_empty() {
        return false;
    }

    // 检查命令名（去除路径）是否匹配
    let expected_cmd = expected_parts[0];
    let actual_cmd = cmdline_parts[0]
        .rsplit('/')
        .next()
        .unwrap_or(cmdline_parts[0]);

    if expected_cmd != actual_cmd && !expected_cmd.ends_with(actual_cmd) {
        return false;
    }

    // 检查参数是否包含 expected_command 的核心部分
    cmdline.contains(expected_command) || expected_parts.iter().skip(1).all(|p| cmdline.contains(p))
}

#[cfg(not(unix))]
fn command_matches_pid(_pid: u32, _expected_command: &str) -> bool {
    true
}

/// 无 PID 时的备选检测：通过 pgrep + command 验证
#[cfg(unix)]
fn is_process_alive_by_command(command: &str) -> bool {
    use std::process::Command;

    let cmd_name = command.split_whitespace().next().unwrap_or(command);
    let output = Command::new("pgrep").arg("-x").arg(cmd_name).output();
    match output {
        Ok(o) => o.status.success(),
        Err(_) => true, // pgrep 不存在或执行失败，保守返回 true
    }
}

#[cfg(not(unix))]
fn is_process_alive_by_command(_command: &str) -> bool {
    true
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
    BG_TASK_DEFAULT_TIMEOUT_MS
}

/// 查询后台任务输出的工具（替代 CheckBackgroundTool）
#[derive(Debug)]
pub struct TaskOutputTool {
    pub manager: Arc<BackgroundManager>,
}

impl TaskOutputTool {
    pub const NAME: &'static str = "TaskOutput";
}

impl Tool for TaskOutputTool {
    fn name(&self) -> &str {
        Self::NAME
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

        let timeout_ms = params.timeout.min(BG_TASK_MAX_TIMEOUT_MS);

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
