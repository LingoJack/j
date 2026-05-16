use crate::constants::{HOOK_LLM_MAX_TOKENS, HOOK_LLM_TEMPERATURE};
use crate::infra::hook::definition::*;
use crate::infra::hook::types::*;
use crate::storage::ModelProvider;
use crate::util::log::write_info_log;
use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

// ========== Hook 执行分派 ==========

/// 执行单个 hook（分派到 Shell / LLM / Builtin），不处理重试
pub(crate) fn execute_hook_with_provider(
    kind: &HookKind,
    context: &HookContext,
    provider: &Option<Arc<Mutex<ModelProvider>>>,
) -> Result<HookResult, String> {
    match kind {
        HookKind::Shell(shell) => execute_shell_hook(shell, context),
        HookKind::Llm(llm) => execute_llm_hook(llm, context, provider),
        HookKind::Builtin(builtin) => match (builtin.handler)(context) {
            Some(result) => Ok(result),
            None => Ok(HookResult::default()),
        },
    }
}

// ========== LLM Hook ==========

/// LLM hook 的 JSON 格式指令（拼接到 prompt 末尾）
const LLM_HOOK_FORMAT_INSTRUCTION: &str = r#"

---
You are a hook function. You MUST respond with ONLY a valid JSON object matching this schema (no markdown, no explanation outside JSON):
{
  "user_input": "string (optional, replace user message)",
  "assistant_output": "string (optional, replace assistant output)",
  "messages": [{"role":"user","content":"..."}] (optional, replace message list),
  "system_prompt": "string (optional, replace system prompt)",
  "tool_arguments": "string (optional, replace tool arguments JSON)",
  "tool_result": "string (optional, replace tool result)",
  "tool_error": "string (optional, replace tool error)",
  "inject_messages": [{"role":"user","content":"..."}] (optional, append messages),
  "action": "stop" or "skip" (optional, stop=abort pipeline, skip=skip current step),
  "retry_feedback": "string (optional, feedback to retry with)",
  "additional_context": "string (optional, append to system_prompt)",
  "system_message": "string (optional, show toast to user)"
}
Return {} if no modification needed."#;

/// 模板变量替换
pub(crate) fn render_prompt_template(template: &str, context: &HookContext) -> String {
    let mut result = template.to_string();
    result = result.replace("{{event}}", context.event.as_str());
    result = result.replace("{{cwd}}", &context.cwd);
    result = result.replace(
        "{{user_input}}",
        context.user_input.as_deref().unwrap_or(""),
    );
    result = result.replace(
        "{{assistant_output}}",
        context.assistant_output.as_deref().unwrap_or(""),
    );
    result = result.replace("{{tool_name}}", context.tool_name.as_deref().unwrap_or(""));
    result = result.replace(
        "{{tool_arguments}}",
        context.tool_arguments.as_deref().unwrap_or(""),
    );
    result = result.replace(
        "{{tool_result}}",
        context.tool_result.as_deref().unwrap_or(""),
    );
    result = result.replace("{{model}}", context.model.as_deref().unwrap_or(""));
    result
}

/// 从 LLM 输出文本中提取 JSON（找第一个 { 到最后一个 } 之间的内容）
pub(crate) fn extract_json_from_llm_output(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    // 从末尾找最后一个 }
    let end = text.rfind('}')?;
    if end > start {
        Some(&text[start..=end])
    } else {
        None
    }
}

/// 执行 LLM hook
///
/// 协议：
/// - 将 prompt 模板渲染后 + JSON 格式指令拼接为完整 prompt
/// - 使用当前活跃 provider（或 LlmHook.model 覆盖）调用 LLM API（非流式）
/// - 解析 LLM 输出为 HookResult JSON
/// - JSON 解析失败 → Err → 触发重试
pub(crate) fn execute_llm_hook(
    hook: &LlmHook,
    context: &HookContext,
    provider_opt: &Option<Arc<Mutex<ModelProvider>>>,
) -> Result<HookResult, String> {
    let provider_arc = provider_opt
        .as_ref()
        .ok_or("LLM hook 无法执行：未注入 provider")?;

    let provider = provider_arc
        .lock()
        .map_err(|e| format!("获取 provider 锁失败: {}", e))?
        .clone();

    // 如果 LlmHook 指定了 model，覆盖 provider 的 model
    let provider = if let Some(ref model) = hook.model {
        let mut p = provider;
        p.model = model.clone();
        p
    } else {
        provider
    };

    // 渲染 prompt 模板 + 拼接格式指令
    let rendered = render_prompt_template(&hook.prompt, context);
    let full_prompt = format!("{}{}", rendered, LLM_HOOK_FORMAT_INSTRUCTION);

    // 构造 API 请求消息
    let system_msg = "You are a hook function. Respond ONLY with the JSON object as instructed.";
    let user_msg = full_prompt.as_str();

    // 使用 reqwest 发送非流式请求（复用 api.rs 中的逻辑模式）
    let url = format!(
        "{}/chat/completions",
        provider.api_base.trim_end_matches('/')
    );
    let request_body = serde_json::json!({
        "model": provider.model,
        "messages": [
            {"role": "system", "content": system_msg},
            {"role": "user", "content": user_msg}
        ],
        "temperature": HOOK_LLM_TEMPERATURE,
        "max_tokens": HOOK_LLM_MAX_TOKENS,
    });
    let request_str = serde_json::to_string(&request_body)
        .map_err(|e| format!("序列化 LLM hook 请求失败: {}", e))?;

    // 在新 tokio runtime 中阻塞执行
    let timeout_secs = hook.timeout;
    let rt =
        tokio::runtime::Runtime::new().map_err(|e| format!("创建 tokio runtime 失败: {}", e))?;

    rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| format!("创建 HTTP client 失败: {}", e))?;

        let resp = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", provider.api_key))
            .body(request_str)
            .send()
            .await
            .map_err(|e| format!("LLM hook 请求失败: {}", e))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| format!("读取 LLM hook 响应失败: {}", e))?;

        if !status.is_success() {
            return Err(format!(
                "LLM hook API 错误: HTTP {} (body: {})",
                status,
                &body[..body.len().min(500)]
            ));
        }

        // 解析 OpenAI 兼容响应
        let parsed: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| format!("解析 LLM hook 响应 JSON 失败: {}", e))?;

        let content = parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim();

        if content.is_empty() || content == "{}" {
            return Ok(HookResult::default());
        }

        // 从 LLM 输出中提取 JSON
        let json_str = match extract_json_from_llm_output(content) {
            Some(s) => s,
            None => {
                return Err(format!(
                    "LLM hook 输出中未找到 JSON (输出: {})",
                    &content[..content.len().min(500)]
                ));
            }
        };

        let hook_result: HookResult = serde_json::from_str(json_str).map_err(|e| {
            format!(
                "解析 LLM hook JSON 失败: {} (提取的 JSON: {})",
                e,
                &json_str[..json_str.len().min(500)]
            )
        })?;

        write_info_log(
            "execute_llm_hook",
            &format!(
                "LLM hook 完成 (prompt_len={}, model={}), action={:?}",
                hook.prompt.len(),
                provider.model,
                hook_result.action
            ),
        );

        Ok(hook_result)
    })
}

// ========== Shell Hook ==========

/// 执行 Shell hook 脚本
///
/// 协议：
/// - 执行方式: `sh -c "<command>"`
/// - 工作目录: 用户当前目录（目录布局下，hook 目录会前置到 PATH）
/// - 环境变量: `JCLI_HOOK_EVENT`（事件名）、`JCLI_CWD`（用户当前目录）、`JCLI_HOOK_DIR`（hook 目录）
/// - PATH: 目录布局下，hook 目录前置到 PATH，脚本可直接用文件名调用（如 `script.sh`）
/// - stdin: HookContext JSON
/// - stdout: HookResult JSON（可为空字符串/空 JSON `{}`，表示无修改）
/// - exit 0: 成功
/// - exit ≠0: 视为失败（调用方按 on_error 策略处理）
/// - 超时: kill 子进程，返回 Err
pub(crate) fn execute_shell_hook(
    hook: &ShellHook,
    context: &HookContext,
) -> Result<HookResult, String> {
    let context_json =
        serde_json::to_string(context).map_err(|e| format!("序列化 context 失败: {}", e))?;

    // cwd 始终使用用户当前目录
    let user_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let hook_dir_str = hook
        .dir_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(&hook.command)
        .current_dir(&user_cwd)
        .env("JCLI_HOOK_EVENT", context.event.as_str())
        .env("JCLI_CWD", user_cwd.display().to_string())
        .env("JCLI_HOOK_DIR", &hook_dir_str);

    // 目录布局下，将 hook 目录前置到 PATH，脚本可直接用文件名调用
    if let Some(ref hook_dir) = hook.dir_path {
        let existing_path = env::var("PATH").unwrap_or_default();
        let separator = if cfg!(windows) { ";" } else { ":" };
        let new_path = if existing_path.is_empty() {
            hook_dir.display().to_string()
        } else {
            format!("{}{}{}", hook_dir.display(), separator, existing_path)
        };
        cmd.env("PATH", new_path);
    }

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 hook 进程失败: {}", e))?;

    // 保存 PID 用于超时 kill
    let pid = child.id();

    // 写入 stdin 后关闭（drop stdin handle）
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(context_json.as_bytes());
    }

    // 子线程中 wait_with_output（阻塞等待进程退出 + 一次性读取 stdout/stderr）
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });

    let timeout = Duration::from_secs(hook.timeout);
    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => {
            // 捕获 stderr 并记录日志
            let stderr_str = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if !stderr_str.is_empty() {
                write_info_log(
                    "execute_shell_hook",
                    &format!("Hook stderr ({}): {}", hook.command, stderr_str),
                );
            }

            if !output.status.success() {
                let mut err = format!("Hook 退出码: {:?}", output.status.code());
                if !stderr_str.is_empty() {
                    err.push_str(&format!(", stderr: {}", stderr_str));
                }
                return Err(err);
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stdout = stdout.trim();

            if stdout.is_empty() || stdout == "{}" {
                return Ok(HookResult::default());
            }

            let result: HookResult = serde_json::from_str(stdout)
                .map_err(|e| format!("解析 hook 输出 JSON 失败: {} (输出: {})", e, stdout))?;

            write_info_log(
                "execute_shell_hook",
                &format!(
                    "Hook 完成 (cmd: {}), action={:?}",
                    hook.command, result.action
                ),
            );

            Ok(result)
        }
        Ok(Err(e)) => Err(format!("等待 hook 进程失败: {}", e)),
        Err(_) => {
            // 超时：终止进程
            #[cfg(unix)]
            {
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid as i32),
                    nix::sys::signal::Signal::SIGKILL,
                );
            }
            #[cfg(windows)]
            {
                let _ = Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &pid.to_string()])
                    .status();
            }
            Err(format!("Hook 超时 ({}s): {}", hook.timeout, hook.command))
        }
    }
}
