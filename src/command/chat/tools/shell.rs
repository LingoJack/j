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
        在当前系统上执行 shell 命令，返回 stdout 和 stderr。每次调用创建新进程，状态不延续。
        重要限制：
        - 不支持交互式命令（stdin 未连接）。所有需要用户输入的命令必须使用非交互标志：
          - npm init -> npm init -y
          - npx create-react-app -> npx create-react-app my-app（自动跳过提示）
          - apt install -> apt install -y
          - git commit -> git commit -m "message"（不要依赖编辑器打开）
          - pip install 需要确认时 -> echo "y" | pip install
        - 不支持后台/长期运行的服务（如 npm run dev、python -m http.server），进程会在超时后被终止
        - 如果命令超时（默认 120s），会自动终止并返回已有输出
        - 对于构建类命令（npm run build 等），可适当增大 timeout 值（最大 600）
        使用建议：
        - 多个独立命令用 && 串联，而非分多次调用
        - 用绝对路径，避免依赖 cd 切换目录
        - 包含空格的文件路径用双引号包裹
        - 文件操作优先使用 ReadFile/WriteFile/EditFile 工具，而非 cat/sed/echo
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "要执行的 shell 命令（在 bash -c 中执行）。不支持交互式输入，必须使用非交互标志（如 -y、--yes、--no-input）。"
                },
                "description": {
                    "type": "string",
                    "description": "命令的简短描述（5-10 字），用于在 UI 中展示"
                },
                "cwd": {
                    "type": "string",
                    "description": "命令执行的工作目录（绝对路径）。不指定则使用当前进程工作目录。"
                },
                "timeout": {
                    "type": "integer",
                    "description": "超时秒数，默认 120，最大 600。超时后自动终止进程并返回已有输出。构建类命令（npm run build、cargo build 等）建议设置 300-600。"
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
            Some(dir) => format!("即将执行: {} (工作目录: {})", cmd, dir),
            None => format!("即将执行: {}", cmd),
        }
    }
}

/// 将 stdout 和 stderr 字节拼接为最终输出字符串
fn build_output(stdout_bytes: &[u8], stderr_bytes: &[u8]) -> String {
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
    result
}
