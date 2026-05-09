use super::ToolResult;
use super::background::BackgroundManager;
use crate::command::chat::agent::thread_identity::thread_cwd;
use crate::command::chat::constants::{
    SHELL_AUTO_BG_SECS, SHELL_DEFAULT_TIMEOUT_SECS, SHELL_INTERACTIVE_SILENCE_SECS,
    SHELL_MAX_TIMEOUT_SECS, SHELL_POLL_INTERVAL_MS,
};
use crate::command::chat::tools::{
    PlanDecision, Tool, check_blocking_command, is_dangerous_command, parse_tool_args,
    schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::borrow::Cow;
use std::io::BufRead;
use std::sync::{
    Arc, Mutex,
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
    /// If true, spawn the command with a PTY so that it can accept stdin via the Session tool. Use this for interactive programs (e.g. python REPL, node REPL, mysql, ssh). Returns a sid (same as task_id) that can be used with the Session tool.
    #[serde(default)]
    interactive: bool,
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

    fn description(&self) -> Cow<'_, str> {
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
        - Set run_in_background: true for long-running commands (builds, servers, watchers, etc.) to get a task_id immediately; use TaskOutput to retrieve results. You do not need to poll — you will be notified when it finishes
        - IMPORTANT: When starting servers, dev servers, or any long-running process (e.g. `go run`, `npm run dev`, `python -m http.server`, `docker compose up`, `java -jar`), you MUST use run_in_background: true. Do NOT use shell `&` (e.g. `go run ... & sleep 3 && curl ...`). Instead: start the server with run_in_background: true, then use a separate Bash call for health checks or follow-up commands.
        - Avoid unnecessary `sleep` commands: do not sleep between commands, do not retry in a sleep loop — diagnose the root cause instead
        - For git commands:
          - Prefer creating a new commit rather than amending
          - Before running destructive operations (git reset --hard, git push --force), consider safer alternatives
          - Never skip hooks (--no-verify) unless the user explicitly asks
        - Commands that may run longer than a few seconds (builds, deploys, container ops, custom scripts, make targets) should use run_in_background: true — even if the command has its own detach flag (e.g. `podman compose up -d` or `docker compose up -d` still blocks during image build). If unsure, prefer run_in_background: true; the tool will auto-promote to background after 30s regardless, returning a task_id you can use with TaskOutput.
        - For interactive programs (REPLs, ssh, mysql, python -i, node -i), set interactive: true. This spawns the command with a PTY and returns a sid. Use the Session tool to send stdin and read stdout. The sid is the same as the task_id.
        "#.into()
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

        if params.interactive {
            return self.execute_interactive(&params.command, params.cwd.as_deref(), timeout_secs);
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
    /// 同步执行命令：spawn → 轮询等待 → 收集输出
    /// 超过 SHELL_AUTO_BG_SECS 仍未结束时，自动将进程移交给 BackgroundManager，
    /// 不杀进程，立即返回 task_id，进程在后台继续运行
    fn execute_sync(
        &self,
        command: &str,
        cwd: Option<&str>,
        timeout_secs: u64,
        cancelled: &Arc<AtomicBool>,
    ) -> ToolResult {
        let mut cmd = std::process::Command::new("bash");
        cmd.arg("-c")
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

        // 用 Arc<Mutex<String>> 逐行写入，升级时可直接将 buffer 移交 BackgroundManager
        // last_write_at 追踪最近一次有输出的时刻，用于自动升级前的"疑似交互式"静默检测
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
                // 升级前的"疑似交互式"静默检测：若最近 SHELL_INTERACTIVE_SILENCE_SECS 秒
                // 完全没有任何 stdout/stderr 输出，判定为可能在等待交互输入
                // （交互式命令通常会先打 prompt 然后静默等 stdin；构建/部署类命令持续有输出）
                let silence_elapsed = last_write_at
                    .lock()
                    .ok()
                    .map(|ts| ts.elapsed())
                    .unwrap_or(Duration::ZERO);

                if silence_elapsed >= Duration::from_secs(SHELL_INTERACTIVE_SILENCE_SECS) {
                    // 疑似交互式：kill 进程，返回提示，让 agent 重新发起并加非交互标志
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
                         请检查命令是否需要 stdin 交互：加上非交互标志（-y / --yes / --noconfirm / --no-input 等），\
                         或使用 here-string/echo 管道提供预设输入，然后重新执行。",
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

                // 把当前已缓冲内容复制到 bg_buffer，随后 reader 线程继续写 output_buffer，
                // 但 output_buffer 和 bg_buffer 是两个独立 Arc，需要在后台监控线程中桥接
                let current_output = output_buffer
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                if !current_output.is_empty()
                    && let Ok(mut buf) = bg_buffer.lock()
                {
                    buf.push_str(&current_output);
                }

                // 启动后台监控线程：等待子进程结束，持续从 output_buffer 同步到 bg_buffer，
                // 最终调用 complete_task
                let manager = Arc::clone(&self.manager);
                let tid = task_id.clone();
                let cmd_str = command.to_string();
                let src_buf = Arc::clone(&output_buffer);
                let dst_buf = Arc::clone(&bg_buffer);
                std::thread::spawn(move || {
                    // 等待 reader 线程写完（child 进程结束时 pipe 关闭，reader 自然退出）
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();

                    // 最终同步完整输出
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
                        "ShellTool::auto_bg",
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

    /// 使用 PTY 启动交互式命令，立即返回 sid
    fn execute_interactive(
        &self,
        command: &str,
        cwd: Option<&str>,
        _timeout_secs: u64,
    ) -> ToolResult {
        use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

        let effective_cwd = cwd
            .map(|s| s.to_string())
            .or_else(|| thread_cwd().map(|p| p.to_string_lossy().to_string()));

        let pty_system = NativePtySystem::default();
        let pair = match pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(p) => p,
            Err(e) => {
                return ToolResult {
                    output: format!("创建 PTY 失败: {}", e),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let mut builder = CommandBuilder::new("bash");
        builder.arg("-c");
        builder.arg(command);
        if let Some(ref dir) = effective_cwd {
            builder.cwd(dir);
        }

        let _child = match pair.slave.spawn_command(builder) {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    output: format!("PTY 启动命令失败: {}", e),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        // 注册后台任务
        let (task_id, output_buffer) =
            self.manager
                .spawn_command(command, effective_cwd.clone(), 0, None);

        // 在移动 master 之前先 clone reader 和获取 writer
        let reader = match pair.master.try_clone_reader() {
            Ok(r) => r,
            Err(e) => {
                return ToolResult {
                    output: format!("PTY reader 克隆失败: {}", e),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let writer = match pair.master.take_writer() {
            Ok(w) => w,
            Err(e) => {
                return ToolResult {
                    output: format!("PTY writer 获取失败: {}", e),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        // 保存 PTY writer 句柄（master 保持存活直到 writer drop）
        let _master = pair.master; // 保持 master 活着，drop 在 writer 之后
        self.manager.set_pty_writer(&task_id, writer);

        let buf = Arc::clone(&output_buffer);
        let tid = task_id.clone();
        let mgr = Arc::clone(&self.manager);
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(reader);
            for line in reader.lines().map_while(Result::ok) {
                if let Ok(mut b) = buf.lock() {
                    b.push_str(&line);
                    b.push('\n');
                }
            }
            // PTY reader 结束意味着进程已退出
            mgr.complete_task(&tid, "completed", "交互式会话已结束".to_string());
        });

        ToolResult {
            output: json!({
                "sid": task_id,
                "task_id": task_id,
                "command": command,
                "status": "running",
                "message": "交互式会话已启动，使用 Session 工具的 stdin/stdout 操作"
            })
            .to_string(),
            is_error: false,
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }
}
