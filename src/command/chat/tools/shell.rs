use super::ToolResult;
use crate::command::chat::tools::{Tool, is_dangerous_command};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

// ========== ShellTool ==========

/// 执行 shell 命令的工具
pub struct ShellTool;

impl Tool for ShellTool {
    fn name(&self) -> &str {
        "RunShell"
    }

    fn description(&self) -> &str {
        "在当前系统上执行 shell 命令，返回命令的 stdout 和 stderr 输出；注意每次调用 run_shell 都会创建一个新的进程，状态是不延续的"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "要执行的 shell 命令（在 bash 中执行）"
                }
            },
            "required": ["command"]
        })
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let command = match serde_json::from_str::<Value>(arguments) {
            Ok(v) => match v.get("command").and_then(|c| c.as_str()) {
                Some(cmd) => cmd.to_string(),
                None => {
                    return ToolResult {
                        output: "参数缺少 command 字段".to_string(),
                        is_error: true,
                    };
                }
            },
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        // 安全过滤
        if is_dangerous_command(&command) {
            return ToolResult {
                output: "该命令被安全策略拒绝执行".to_string(),
                is_error: true,
            };
        }

        let mut child = match std::process::Command::new("bash")
            .arg("-c")
            .arg(&command)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
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

        // 轮询等待子进程完成，同时检测取消信号
        let status = loop {
            if cancelled.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                return ToolResult {
                    output: "[已取消]".to_string(),
                    is_error: true,
                };
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
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

        let mut result = String::new();
        let stdout = String::from_utf8_lossy(&stdout_bytes);
        let stderr = String::from_utf8_lossy(&stderr_bytes);

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

        if result.is_empty() {
            result = "(无输出)".to_string();
        }

        let is_error = !status.success();
        ToolResult {
            output: result,
            is_error,
        }
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        // 尝试解析 command 字段
        let cmd = serde_json::from_str::<Value>(arguments)
            .ok()
            .and_then(|v| {
                v.get("command")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| arguments.to_string());
        format!("即将执行: {}", cmd)
    }
}
