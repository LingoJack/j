use super::ToolResult;
use super::background::BackgroundManager;
use crate::command::chat::agent::thread_identity::thread_cwd;
use crate::command::chat::constants::{
    SHELL_DEFAULT_TIMEOUT_SECS, SHELL_MAX_TIMEOUT_SECS, SHELL_POLL_INTERVAL_MS,
};
use crate::command::chat::tools::{
    PlanDecision, Tool, check_blocking_command, is_dangerous_command, parse_tool_args,
    schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::BufRead;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

/// ShellTool 参数
#[derive(Deserialize, JsonSchema)]
struct ShellParams {
    /// The shell command to execute (runs in bash -c). Interactive input is not supported; use non-interactive flags (e.g. -y, --yes, --no-input).
    command: String,
    /// A short description of the command (5-10 words), displayed in the UI
    #[serde(default)]
    #[allow(dead_code)] // 仅用于 UI 展示，通过 arguments JSON 提取
    description: Option<String>,
    /// Working directory for the command (absolute path). Defaults to the current process working directory if not specified.
    #[serde(default)]
    cwd: Option<String>,
    /// Timeout in seconds, default 120, max 600. The process is automatically killed on timeout and partial output is returned. For build commands (npm run build, cargo build, etc.) use 300-600.
    #[serde(default)]
    timeout: Option<u64>,
    /// If true, run the command in background and return a task_id immediately. Use TaskOutput to retrieve results.
    #[serde(default)]
    run_in_background: bool,
}

// ========== ShellTool ==========

/// 执行 shell 命令的工具
#[derive(Debug)]
pub struct ShellTool {
    pub manager: Arc<BackgroundManager>,
}

impl ShellTool {
    pub const NAME: &'static str = "Bash";
}

impl Tool for ShellTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Execute shell commands on the current system, returning stdout and stderr. Each call creates a new process; state does not persist.

        IMPORTANT: Avoid using this tool to run `find`, `grep`, `cat`, `head`, `tail`, `sed`, `awk`, or `echo` commands, unless explicitly instructed. Instead, use the appropriate dedicated tool:
        - File search: Use Glob (NOT find or ls)
        - Content search: Use Grep (NOT grep or rg)
        - Read files: Use Read (NOT cat/head/tail)
        - Edit files: Use Edit (NOT sed/awk)
        - Write files: Use Write (NOT echo >/cat <<EOF)

        Important limitations:
        - Interactive commands are not supported (stdin is not connected)
        - Commands that exceed the timeout (default 120s) are automatically terminated and partial output is returned
        - For build commands, increase the timeout value as needed (max 600)

        Usage tips:
        - If your command will create new directories or files, first run `ls` to verify the parent directory exists
        - Always quote file paths containing spaces with double quotes
        - Try to maintain your current working directory by using absolute paths and avoiding `cd`
        - When issuing multiple commands:
          - If commands are independent and can run in parallel, make multiple Bash tool calls in a single response
          - If commands depend on each other, use && to chain them sequentially
          - Use ; only when you don't care if earlier commands fail
          - DO NOT use newlines to separate commands
        - Set run_in_background: true for long-running commands (builds, servers, etc.) to get a task_id immediately; use TaskOutput to retrieve results. You do not need to poll — you will be notified when it finishes
        - Avoid unnecessary `sleep` commands: do not sleep between commands, do not retry in a sleep loop — diagnose the root cause instead
        - For git commands:
          - Prefer creating a new commit rather than amending
          - Before running destructive operations (git reset --hard, git push --force), consider safer alternatives
          - Never skip hooks (--no-verify) unless the user explicitly asks
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<ShellParams>()
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: ShellParams = match parse_tool_args(arguments) {
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

        // ===== 同步执行（原有逻辑） =====
        let mut cmd = std::process::Command::new("bash");
        cmd.arg("-c")
            .arg(&params.command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // 设置工作目录：优先用显式 cwd，其次用 thread-local worktree CWD
        let effective_dir = params
            .cwd
            .clone()
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

        // 先取走 stdout/stderr 句柄，在独立线程中读取，避免管道缓冲区满导致死锁
        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        let stdout_thread = std::thread::spawn(move || {
            stdout_handle.map(|mut r| {
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut r, &mut buf).ok();
                buf
            })
        });
        let stderr_thread = std::thread::spawn(move || {
            stderr_handle.map(|mut r| {
                let mut buf = Vec::new();
                std::io::Read::read_to_end(&mut r, &mut buf).ok();
                buf
            })
        });

        // 轮询等待子进程完成，同时检测取消信号和超时
        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        let status = loop {
            // 检测用户取消
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

            // 检测超时
            if start.elapsed() > timeout {
                let _ = child.kill();
                let _ = child.wait();

                // 等待读取线程结束，收集已有输出
                let stdout_bytes = stdout_thread.join().ok().flatten().unwrap_or_default();
                let stderr_bytes = stderr_thread.join().ok().flatten().unwrap_or_default();
                let partial = build_output(&stdout_bytes, &stderr_bytes);

                let timeout_msg = format!(
                    "[超时] 命令执行超过 {}s 已自动终止。可能原因：命令等待交互输入（尝试加 --yes 等非交互标志）或命令长时间运行（尝试增大 timeout 值）。",
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

        let stdout_bytes = stdout_thread.join().ok().flatten().unwrap_or_default();
        let stderr_bytes = stderr_thread.join().ok().flatten().unwrap_or_default();

        let result = build_output(&stdout_bytes, &stderr_bytes);

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

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        if let Ok(params) = serde_json::from_str::<ShellParams>(arguments) {
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

impl ShellTool {
    /// 在后台线程执行命令，立即返回 task_id
    fn execute_background(
        &self,
        command: String,
        cwd: Option<String>,
        timeout_secs: u64,
    ) -> ToolResult {
        // 显式 cwd 优先，其次取 thread-local worktree CWD（后台任务在新线程中运行，
        // 所以要在这里捕获，而不是在新线程中重新读取）
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
            let mut child_cmd = std::process::Command::new("bash");
            child_cmd
                .arg("-c")
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

            // 独立 reader 线程防死锁，逐行读取并实时写入共享 buffer
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

                    // 等待 reader 线程结束，确保 buffer 已写入完整
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

            // 等待 reader 线程结束，确保 buffer 已写入完整
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

/// 将 stdout 和 stderr 字节拼接为最终输出字符串（剥离 ANSI 转义码 + 清理控制字符）
pub(super) fn build_output(stdout_bytes: &[u8], stderr_bytes: &[u8]) -> String {
    use crate::util::text::sanitize_tool_output;
    let mut result = String::new();
    let stdout = String::from_utf8_lossy(stdout_bytes);
    let stderr = String::from_utf8_lossy(stderr_bytes);

    if !stdout.is_empty() {
        result.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !result.is_empty() {
            result.push_str("\n[stderr]\n");
        } else {
            result.push_str("[stderr]\n");
        }
        result.push_str(&stderr);
    }
    sanitize_tool_output(&result)
}
