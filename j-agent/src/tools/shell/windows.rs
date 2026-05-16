use super::ToolResult;
#[cfg(windows)]
use super::background::BackgroundManager;
#[cfg(windows)]
use crate::agent::thread_identity::thread_cwd;
#[cfg(windows)]
use crate::constants::{
    SHELL_AUTO_BG_SECS, SHELL_DEFAULT_TIMEOUT_SECS, SHELL_INTERACTIVE_SILENCE_SECS,
    SHELL_MAX_TIMEOUT_SECS, SHELL_POLL_INTERVAL_MS,
};
#[cfg(windows)]
use crate::tools::{
    PlanDecision, Tool, check_blocking_command, is_dangerous_command, parse_tool_args,
    schema_to_tool_params,
};
#[cfg(windows)]
use schemars::JsonSchema;
#[cfg(windows)]
use serde::Deserialize;
#[cfg(windows)]
use serde_json::{Value, json};
#[cfg(windows)]
use std::borrow::Cow;
#[cfg(windows)]
use std::io::BufRead;
#[cfg(windows)]
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
#[cfg(windows)]
use std::time::{Duration, Instant};

/// PowerShellTool 参数
#[cfg(windows)]
#[derive(Deserialize, JsonSchema)]
struct PowerShellParams {
    /// The PowerShell command to execute (runs in powershell -Command). Interactive input is not supported; use non-interactive flags.
    command: String,
    /// A short description of the command (5-10 words), displayed in the UI
    #[serde(default)]
    #[allow(dead_code)]
    description: Option<String>,
    /// Working directory for the command (absolute path). Defaults to the current process working directory if not specified.
    #[serde(default)]
    cwd: Option<String>,
    /// Timeout in seconds, default 120, max 600. The process is automatically killed on timeout and partial output is returned. For build commands use 300-600.
    #[serde(default)]
    timeout: Option<u64>,
    /// If true, run the command in background and return a task_id immediately. Use TaskOutput to retrieve results.
    #[serde(default)]
    run_in_background: bool,
}

// ========== PowerShellTool ==========

/// 执行 PowerShell 命令的工具（仅 Windows 可用）
#[cfg(windows)]
#[derive(Debug)]
pub struct PowerShellTool {
    pub manager: Arc<BackgroundManager>,
}

#[cfg(windows)]
impl PowerShellTool {
    // 对 agent 暴露统一工具名为 Shell，避免 Windows/Unix 在提示词、权限规则、
    // 渲染与分类链路中出现名称分叉；底层实现仍然执行 PowerShell。
    pub const NAME: &'static str = "Shell";
}

#[cfg(windows)]
impl Tool for PowerShellTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> Cow<'_, str> {
        r#"
        Execute PowerShell commands on the current Windows system, returning stdout and stderr. Each call creates a new process; state does not persist.

        IMPORTANT: Avoid using this tool to run commands that have dedicated tools, unless explicitly instructed. Instead, use the appropriate dedicated tool:
        - File search: Use Glob
        - Content search: Use Grep
        - Read files: Use Read
        - Edit files: Use Edit
        - Write files: Use Write

        Important limitations:
        - Interactive commands are not supported (stdin is not connected)
        - Commands that exceed the timeout (default 120s) are automatically terminated and partial output is returned
        - For build commands, increase the timeout value as needed (max 600)

        Usage tips:
        - If your command will create new directories or files, first verify the parent directory exists
        - Always quote file paths containing spaces
        - Try to maintain your current working directory by using absolute paths and avoiding `cd`
        - When issuing multiple commands:
          - If commands are independent and can run in parallel, make multiple Shell tool calls in a single response
          - If commands depend on each other, use `;` to chain them sequentially
          - Use `&&` for conditional execution (run next only if previous succeeded)
        - Set run_in_background: true for long-running commands (builds, servers, watchers, etc.) to get a task_id immediately; use TaskOutput to retrieve results.
        - IMPORTANT: When starting servers, dev servers, or any long-running process, you MUST use run_in_background: true.
        - Avoid unnecessary `Start-Sleep` commands: do not sleep between commands — diagnose the root cause instead
        - For git commands:
          - Prefer creating a new commit rather than amending
          - Before running destructive operations (git reset --hard, git push --force), consider safer alternatives
          - Never skip hooks (--no-verify) unless the user explicitly asks
        - Commands that may run longer than a few seconds should use run_in_background: true. If unsure, prefer run_in_background: true; the tool will auto-promote to background after 30s regardless.
        "#.into()
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<PowerShellParams>()
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: PowerShellParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        let timeout_secs = params
            .timeout
            .unwrap_or(SHELL_DEFAULT_TIMEOUT_SECS)
            .min(SHELL_MAX_TIMEOUT_SECS);

        // 安全过滤
        if is_dangerous_command(&params.command) {
            return ToolResult {
                output: "该命令被安全策略拒绝执行".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        // 阻塞式命令检测（仅非后台运行时）
        if !params.run_in_background
            && let Some(msg) = check_blocking_command(&params.command)
        {
            return ToolResult {
                output: format!("该命令被阻断: {}", msg),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        if params.run_in_background {
            return self.execute_background(params.command, params.cwd, timeout_secs);
        }

        self.execute_sync(
            &params.command,
            params.cwd.as_deref(),
            timeout_secs,
            cancelled,
        )
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        if let Ok(params) = serde_json::from_str::<PowerShellParams>(arguments) {
            let prefix = if params.run_in_background {
                "Background execute"
            } else {
                "Execute"
            };

            match params.cwd {
                Some(dir) => format!("{}: {} (cwd: {})", prefix, params.command, dir),
                None => format!("{}: {}", prefix, params.command),
            }
        } else {
            format!("Execute: {}", arguments)
        }
    }
}

#[cfg(windows)]
impl PowerShellTool {
    /// 同步执行 PowerShell 命令：spawn → 轮询等待 → 收集输出
    /// 超过 SHELL_AUTO_BG_SECS 仍未结束时，自动将进程移交给 BackgroundManager
    fn execute_sync(
        &self,
        command: &str,
        cwd: Option<&str>,
        timeout_secs: u64,
        cancelled: &Arc<AtomicBool>,
    ) -> ToolResult {
        let mut cmd = std::process::Command::new("powershell.exe");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command"])
            .arg(command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // 设置工作目录：优先用显式 cwd，其次用 thread-local worktree CWD
        let effective_dir = cwd
            .map(|s| s.to_string())
            .or_else(|| thread_cwd().map(|p| p.to_string_lossy().to_string()));
        if let Some(ref dir) = effective_dir {
            let path = std::path::Path::new(dir);
            if !path.is_dir() {
                return ToolResult {
                    output: format!("指定的工作目录不存在: {}", dir),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
            cmd.current_dir(path);
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    output: format!("执行失败: {}", e),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let pid = child.id();

        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();
        let output_buffer: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
        let last_write_at: Arc<Mutex<Instant>> = Arc::new(Mutex::new(Instant::now()));
        let stdout_buf = Arc::clone(&output_buffer);
        let stderr_buf = Arc::clone(&output_buffer);
        let stdout_write_ts = Arc::clone(&last_write_at);
        let stderr_write_ts = Arc::clone(&last_write_at);

        let stdout_thread = std::thread::spawn(move || {
            if let Some(r) = stdout_handle {
                let reader = std::io::BufReader::new(r);
                for line in reader.lines().map_while(Result::ok) {
                    if let Ok(mut buf) = stdout_buf.lock() {
                        buf.push_str(&line);
                        buf.push('\n');
                    }
                    if let Ok(mut ts) = stdout_write_ts.lock() {
                        *ts = Instant::now();
                    }
                }
            }
        });
        let stderr_thread = std::thread::spawn(move || {
            if let Some(r) = stderr_handle {
                let reader = std::io::BufReader::new(r);
                for line in reader.lines().map_while(Result::ok) {
                    if let Ok(mut buf) = stderr_buf.lock() {
                        buf.push_str(&line);
                        buf.push('\n');
                    }
                    if let Ok(mut ts) = stderr_write_ts.lock() {
                        *ts = Instant::now();
                    }
                }
            }
        });

        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);
        let auto_bg_threshold = Duration::from_secs(SHELL_AUTO_BG_SECS);

        let status = loop {
            if cancelled.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                return ToolResult {
                    output: "[已取消]".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }

            // 自动升级：超过阈值仍未结束，不杀进程，移交后台
            if start.elapsed() >= auto_bg_threshold {
                let silence_elapsed = last_write_at
                    .lock()
                    .ok()
                    .map(|ts| ts.elapsed())
                    .unwrap_or(Duration::ZERO);

                if silence_elapsed >= Duration::from_secs(SHELL_INTERACTIVE_SILENCE_SECS) {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                    let partial = output_buffer
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();
                    let hint = format!(
                        "[疑似交互式命令] 命令运行 {}s 后最近 {}s 无任何输出，判定为可能在等待交互输入，已自动终止。\
                         请检查命令是否需要 stdin 交互：加上非交互标志，或使用管道提供预设输入，然后重新执行。",
                        start.elapsed().as_secs(),
                        SHELL_INTERACTIVE_SILENCE_SECS
                    );
                    return ToolResult {
                        output: if partial.is_empty() {
                            hint
                        } else {
                            format!("{}\n{}", partial, hint)
                        },
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }

                let (task_id, bg_buffer) = self.manager.adopt_process(command, pid, start);

                let current_output = output_buffer
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                if !current_output.is_empty()
                    && let Ok(mut buf) = bg_buffer.lock()
                {
                    buf.push_str(&current_output);
                }

                let manager = Arc::clone(&self.manager);
                let tid = task_id.clone();
                let cmd_str = command.to_string();
                let src_buf = Arc::clone(&output_buffer);
                let dst_buf = Arc::clone(&bg_buffer);
                std::thread::spawn(move || {
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();

                    let final_output = src_buf.lock().unwrap_or_else(|e| e.into_inner()).clone();
                    {
                        let mut buf = dst_buf.lock().unwrap_or_else(|e| e.into_inner());
                        *buf = final_output.clone();
                    }

                    let result = if final_output.is_empty() {
                        "(无输出)".to_string()
                    } else {
                        final_output
                    };
                    manager.complete_task(&tid, "completed", result);
                    crate::util::log::write_info_log(
                        "PowerShellTool::auto_bg",
                        &format!("自动后台任务 {} 已完成: {}", tid, cmd_str),
                    );
                });

                return ToolResult {
                    output: json!({
                        "task_id": task_id,
                        "command": command,
                        "status": "running",
                        "message": format!(
                            "命令运行超过 {}s 仍未结束，已自动转为后台任务。使用 TaskOutput 查询结果。",
                            SHELL_AUTO_BG_SECS
                        )
                    })
                    .to_string(),
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }

            if start.elapsed() > timeout {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();
                let partial = output_buffer
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                let timeout_msg = format!(
                    "[超时] 命令执行超过 {}s 已自动终止。可能原因：命令等待交互输入或命令长时间运行（尝试增大 timeout 值）。",
                    timeout_secs
                );
                return ToolResult {
                    output: if partial.is_empty() {
                        timeout_msg
                    } else {
                        format!("{}\n{}", partial, timeout_msg)
                    },
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }

            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => std::thread::sleep(Duration::from_millis(SHELL_POLL_INTERVAL_MS)),
                Err(e) => {
                    return ToolResult {
                        output: format!("等待进程失败: {}", e),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
            }
        };

        let _ = stdout_thread.join();
        let _ = stderr_thread.join();
        let raw = output_buffer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let result = crate::util::text::sanitize_tool_output(&raw);

        let is_error = !status.success();
        ToolResult {
            output: if result.is_empty() {
                "(无输出)".to_string()
            } else {
                result
            },
            is_error,
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }

    /// 在后台线程执行 PowerShell 命令，立即返回 task_id
    fn execute_background(
        &self,
        command: String,
        cwd: Option<String>,
        timeout_secs: u64,
    ) -> ToolResult {
        let effective_cwd = cwd
            .clone()
            .or_else(|| thread_cwd().map(|p| p.to_string_lossy().to_string()));

        let (task_id, output_buffer) =
            self.manager
                .spawn_command(&command, effective_cwd.clone(), timeout_secs, None);
        let manager = Arc::clone(&self.manager);
        let tid = task_id.clone();
        let cmd = command.clone();

        std::thread::spawn(move || {
            let mut child_cmd = std::process::Command::new("powershell.exe");
            child_cmd
                .args(["-NoProfile", "-NonInteractive", "-Command"])
                .arg(&cmd)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            if let Some(ref dir) = effective_cwd {
                let path = std::path::Path::new(dir);
                if path.is_dir() {
                    child_cmd.current_dir(path);
                }
            }

            let mut child = match child_cmd.spawn() {
                Ok(c) => {
                    let pid = c.id();
                    manager.update_child_pid(&tid, pid);
                    c
                }
                Err(e) => {
                    let mut buf = output_buffer.lock().unwrap_or_else(|e| e.into_inner());
                    *buf = format!("启动失败: {}", e);
                    drop(buf);
                    manager.complete_task(&tid, "error", format!("启动失败: {}", e));
                    return;
                }
            };

            let stdout_handle = child.stdout.take();
            let stderr_handle = child.stderr.take();
            let stdout_buf = Arc::clone(&output_buffer);
            let stderr_buf = Arc::clone(&output_buffer);

            let stdout_thread = std::thread::spawn(move || {
                if let Some(r) = stdout_handle {
                    let reader = std::io::BufReader::new(r);
                    for line in reader.lines().map_while(Result::ok) {
                        if let Ok(mut buf) = stdout_buf.lock() {
                            buf.push_str(&line);
                            buf.push('\n');
                        }
                    }
                }
            });
            let stderr_thread = std::thread::spawn(move || {
                if let Some(r) = stderr_handle {
                    let reader = std::io::BufReader::new(r);
                    for line in reader.lines().map_while(Result::ok) {
                        if let Ok(mut buf) = stderr_buf.lock() {
                            buf.push_str(&line);
                            buf.push('\n');
                        }
                    }
                }
            });

            let start = Instant::now();
            let timeout = Duration::from_secs(timeout_secs);

            let final_status = loop {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();

                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();

                    let output = output_buffer
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();
                    let timeout_msg = format!("[超时] 命令执行超过 {}s 已自动终止。", timeout_secs);
                    let result = if output.is_empty() {
                        timeout_msg
                    } else {
                        format!("{}\n{}", output, timeout_msg)
                    };
                    manager.complete_task(&tid, "timeout", result);
                    return;
                }

                match child.try_wait() {
                    Ok(Some(status)) => break status,
                    Ok(None) => std::thread::sleep(Duration::from_millis(SHELL_POLL_INTERVAL_MS)),
                    Err(e) => {
                        let mut buf = output_buffer.lock().unwrap_or_else(|e| e.into_inner());
                        *buf = format!("等待进程失败: {}", e);
                        drop(buf);
                        manager.complete_task(&tid, "error", format!("等待进程失败: {}", e));
                        return;
                    }
                }
            };

            let _ = stdout_thread.join();
            let _ = stderr_thread.join();

            let output = output_buffer
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            let result = if output.is_empty() {
                "(无输出)".to_string()
            } else {
                output
            };

            let status_str = if final_status.success() {
                "completed"
            } else {
                "error"
            };
            manager.complete_task(&tid, status_str, result);
        });

        ToolResult {
            output: json!({
                "task_id": task_id,
                "command": command,
                "status": "running",
                "message": "命令已在后台启动，使用 TaskOutput 查询状态和结果"
            })
            .to_string(),
            is_error: false,
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }
}
