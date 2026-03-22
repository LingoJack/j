use super::ToolResult;
use crate::command::chat::tools::{Tool, is_dangerous_command};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

/// 默认超时秒数
const DEFAULT_TIMEOUT_SECS: u64 = 120;

// ========== ShellTool ==========

/// 执行 shell 命令的工具
pub struct ShellTool;

impl Tool for ShellTool {
    fn name(&self) -> &str {
        "Bash"
    }

    fn description(&self) -> &str {
        r#"
        Execute shell commands on the current system, returning stdout and stderr. Each call creates a new process; state does not persist.
        Important limitations:
        - Interactive commands are not supported (stdin is not connected)
        - Background/long-running services (e.g. npm run dev, python -m http.server) are not supported; processes are killed on timeout
        - Commands that exceed the timeout (default 120s) are automatically terminated and partial output is returned
        - For build commands, increase the timeout value as needed (max 600)
        Usage tips:
        - Chain independent commands with && instead of making multiple calls
        - Use absolute paths; avoid relying on cd to switch directories
        - Quote file paths containing spaces with double quotes
        - Prefer Read/Write/Edit tools for file operations instead of cat/sed/echo
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute (runs in bash -c). Interactive input is not supported; use non-interactive flags (e.g. -y, --yes, --no-input)."
                },
                "description": {
                    "type": "string",
                    "description": "A short description of the command (5-10 words), displayed in the UI"
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory for the command (absolute path). Defaults to the current process working directory if not specified."
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds, default 120, max 600. The process is automatically killed on timeout and partial output is returned. For build commands (npm run build, cargo build, etc.) use 300-600."
                }
            },
            "required": ["command"]
        })
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let parsed = match serde_json::from_str::<Value>(arguments) {
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
                    output: "参数缺少 command 字段".to_string(),
                    is_error: true,
                };
            }
        };

        let cwd = parsed
            .get("cwd")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        let timeout_secs = parsed
            .get("timeout")
            .and_then(|t| t.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(600);

        // 安全过滤
        if is_dangerous_command(&command) {
            return ToolResult {
                output: "该命令被安全策略拒绝执行".to_string(),
                is_error: true,
            };
        }

        let mut cmd = std::process::Command::new("bash");
        cmd.arg("-c")
            .arg(&command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // 设置工作目录
        if let Some(ref dir) = cwd {
            let path = std::path::Path::new(dir);
            if !path.is_dir() {
                return ToolResult {
                    output: format!("指定的工作目录不存在: {}", dir),
                    is_error: true,
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
                };
            }

            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                Err(e) => {
                    return ToolResult {
                        output: format!("等待进程失败: {}", e),
                        is_error: true,
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

        let cwd = parsed
            .as_ref()
            .and_then(|v| v.get("cwd").and_then(|c| c.as_str()));

        match cwd {
            Some(dir) => format!("Execute: {} (cwd: {})", cmd, dir),
            None => format!("Execute: {}", cmd),
        }
    }
}

/// 将 stdout 和 stderr 字节拼接为最终输出字符串（剥离 ANSI 转义码 + 清理控制字符）
fn build_output(stdout_bytes: &[u8], stderr_bytes: &[u8]) -> String {
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
