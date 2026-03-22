use super::permission::JcliConfig;
use super::storage::ChatMessage;
use crate::util::log::{write_error_log, write_info_log};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::process::Command;

// ========== 数据结构 ==========

/// Hook 事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreSendMessage,
    PostSendMessage,
    PreLlmRequest,
    PostLlmResponse,
    PreToolExecution,
    PostToolExecution,
    SessionStart,
    SessionEnd,
}

impl HookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookEvent::PreSendMessage => "pre_send_message",
            HookEvent::PostSendMessage => "post_send_message",
            HookEvent::PreLlmRequest => "pre_llm_request",
            HookEvent::PostLlmResponse => "post_llm_response",
            HookEvent::PreToolExecution => "pre_tool_execution",
            HookEvent::PostToolExecution => "post_tool_execution",
            HookEvent::SessionStart => "session_start",
            HookEvent::SessionEnd => "session_end",
        }
    }

    pub fn all() -> &'static [HookEvent] {
        &[
            HookEvent::PreSendMessage,
            HookEvent::PostSendMessage,
            HookEvent::PreLlmRequest,
            HookEvent::PostLlmResponse,
            HookEvent::PreToolExecution,
            HookEvent::PostToolExecution,
            HookEvent::SessionStart,
            HookEvent::SessionEnd,
        ]
    }

    pub fn from_str(s: &str) -> Option<HookEvent> {
        match s {
            "pre_send_message" => Some(HookEvent::PreSendMessage),
            "post_send_message" => Some(HookEvent::PostSendMessage),
            "pre_llm_request" => Some(HookEvent::PreLlmRequest),
            "post_llm_response" => Some(HookEvent::PostLlmResponse),
            "pre_tool_execution" => Some(HookEvent::PreToolExecution),
            "post_tool_execution" => Some(HookEvent::PostToolExecution),
            "session_start" => Some(HookEvent::SessionStart),
            "session_end" => Some(HookEvent::SessionEnd),
            _ => None,
        }
    }
}

/// Hook 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDef {
    pub command: String,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_timeout() -> u64 {
    10
}

/// Hook 执行上下文（传给脚本的 stdin JSON）
#[derive(Debug, Serialize)]
pub struct HookContext {
    pub event: HookEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<ChatMessage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistant_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_arguments: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<String>,
    pub cwd: String,
}

impl Default for HookContext {
    fn default() -> Self {
        Self {
            event: HookEvent::SessionStart,
            messages: None,
            system_prompt: None,
            model: None,
            user_message: None,
            assistant_message: None,
            tool_name: None,
            tool_arguments: None,
            tool_result: None,
            cwd: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".".to_string()),
        }
    }
}

/// Hook 脚本返回结果（从 stdout JSON 解析）
#[derive(Debug, Deserialize, Default)]
pub struct HookResult {
    #[serde(default)]
    pub messages: Option<Vec<ChatMessage>>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub user_message: Option<String>,
    #[serde(default)]
    pub assistant_message: Option<String>,
    #[serde(default)]
    pub tool_arguments: Option<String>,
    #[serde(default)]
    pub tool_result: Option<String>,
    #[serde(default)]
    pub inject_messages: Option<Vec<ChatMessage>>,
    #[serde(default)]
    pub abort: bool,
    // MVP 保留字段，暂不支持
    #[serde(default)]
    pub _switch_model: Option<String>,
}

// ========== HookManager ==========

/// Hook 管理器：管理三级 hook（用户级、项目级、session 级）
#[derive(Debug, Clone, Default)]
pub struct HookManager {
    user_hooks: HashMap<HookEvent, Vec<HookDef>>,
    project_hooks: HashMap<HookEvent, Vec<HookDef>>,
    session_hooks: HashMap<HookEvent, Vec<HookDef>>,
}

impl HookManager {
    /// 加载用户级 + 项目级 hook
    pub fn load() -> Self {
        let mut manager = HookManager::default();

        // 加载用户级 hooks：~/.jdata/agent/hooks.yaml
        let user_hooks_path = super::storage::hooks_config_path();
        if user_hooks_path.is_file() {
            match std::fs::read_to_string(&user_hooks_path) {
                Ok(content) => {
                    match serde_yaml::from_str::<HashMap<String, Vec<HookDef>>>(&content) {
                        Ok(hooks_map) => {
                            for (event_name, defs) in hooks_map {
                                if let Some(event) = HookEvent::from_str(&event_name) {
                                    manager.user_hooks.entry(event).or_default().extend(defs);
                                } else {
                                    write_error_log(
                                        "HookManager::load",
                                        &format!("未知 hook 事件: {}", event_name),
                                    );
                                }
                            }
                            write_info_log(
                                "HookManager::load",
                                &format!("已加载用户级 hooks: {}", user_hooks_path.display()),
                            );
                        }
                        Err(e) => {
                            write_error_log(
                                "HookManager::load",
                                &format!("解析用户级 hooks.yaml 失败: {}", e),
                            );
                        }
                    }
                }
                Err(e) => {
                    write_error_log("HookManager::load", &format!("读取 hooks.yaml 失败: {}", e));
                }
            }
        }

        // 加载项目级 hooks：从 .jcli 文件
        let jcli_config = JcliConfig::load();
        for (event_name, defs) in &jcli_config.hooks {
            if let Some(event) = HookEvent::from_str(event_name) {
                manager
                    .project_hooks
                    .entry(event)
                    .or_default()
                    .extend(defs.clone());
            } else {
                write_error_log(
                    "HookManager::load",
                    &format!("项目级 .jcli 中未知 hook 事件: {}", event_name),
                );
            }
        }

        if !manager.project_hooks.is_empty() {
            write_info_log("HookManager::load", "已加载项目级 hooks (from .jcli)");
        }

        manager
    }

    /// 注册 session 级 hook
    pub fn register_session_hook(&mut self, event: HookEvent, def: HookDef) {
        self.session_hooks.entry(event).or_default().push(def);
    }

    /// 移除 session 级 hook
    pub fn remove_session_hook(&mut self, event: HookEvent, index: usize) -> bool {
        if let Some(hooks) = self.session_hooks.get_mut(&event) {
            if index < hooks.len() {
                hooks.remove(index);
                return true;
            }
        }
        false
    }

    /// 列出所有 hook（含来源标记）
    pub fn list_hooks(&self) -> Vec<(HookEvent, &HookDef, &str)> {
        let mut result = Vec::new();
        for event in HookEvent::all() {
            if let Some(hooks) = self.user_hooks.get(event) {
                for hook in hooks {
                    result.push((*event, hook, "user"));
                }
            }
            if let Some(hooks) = self.project_hooks.get(event) {
                for hook in hooks {
                    result.push((*event, hook, "project"));
                }
            }
            if let Some(hooks) = self.session_hooks.get(event) {
                for hook in hooks {
                    result.push((*event, hook, "session"));
                }
            }
        }
        result
    }

    /// 链式执行所有 hook（用户→项目→session）
    /// 返回 Some(HookResult) 如果有任何修改或 abort，否则 None
    pub fn execute(&self, event: HookEvent, mut context: HookContext) -> Option<HookResult> {
        let mut all_hooks: Vec<&HookDef> = Vec::new();

        if let Some(hooks) = self.user_hooks.get(&event) {
            all_hooks.extend(hooks.iter());
        }
        if let Some(hooks) = self.project_hooks.get(&event) {
            all_hooks.extend(hooks.iter());
        }
        if let Some(hooks) = self.session_hooks.get(&event) {
            all_hooks.extend(hooks.iter());
        }

        if all_hooks.is_empty() {
            return None;
        }

        write_info_log(
            "HookManager::execute",
            &format!(
                "执行 {} 个 hook (事件: {})",
                all_hooks.len(),
                event.as_str()
            ),
        );

        let mut had_modification = false;
        let mut final_result = HookResult::default();

        for hook in all_hooks {
            match execute_single_hook(hook, &context) {
                Ok(result) => {
                    if result.abort {
                        write_info_log(
                            "HookManager::execute",
                            &format!("Hook abort (cmd: {})", hook.command),
                        );
                        return Some(HookResult {
                            abort: true,
                            ..Default::default()
                        });
                    }

                    // 合并结果到 context（链式传递）
                    if let Some(ref msgs) = result.messages {
                        write_info_log(
                            "HookManager::execute",
                            &format!(
                                "Hook 修改了 messages (cmd: {}, count: {})",
                                hook.command,
                                msgs.len()
                            ),
                        );
                        context.messages = Some(msgs.clone());
                        final_result.messages = Some(msgs.clone());
                        had_modification = true;
                    }
                    if let Some(ref sp) = result.system_prompt {
                        write_info_log(
                            "HookManager::execute",
                            &format!("Hook 修改了 system_prompt (cmd: {})", hook.command),
                        );
                        context.system_prompt = Some(sp.clone());
                        final_result.system_prompt = Some(sp.clone());
                        had_modification = true;
                    }
                    if let Some(ref um) = result.user_message {
                        write_info_log(
                            "HookManager::execute",
                            &format!("Hook 修改了 user_message (cmd: {})", hook.command),
                        );
                        context.user_message = Some(um.clone());
                        final_result.user_message = Some(um.clone());
                        had_modification = true;
                    }
                    if let Some(ref am) = result.assistant_message {
                        write_info_log(
                            "HookManager::execute",
                            &format!("Hook 修改了 assistant_message (cmd: {})", hook.command),
                        );
                        context.assistant_message = Some(am.clone());
                        final_result.assistant_message = Some(am.clone());
                        had_modification = true;
                    }
                    if let Some(ref ta) = result.tool_arguments {
                        write_info_log(
                            "HookManager::execute",
                            &format!("Hook 修改了 tool_arguments (cmd: {})", hook.command),
                        );
                        context.tool_arguments = Some(ta.clone());
                        final_result.tool_arguments = Some(ta.clone());
                        had_modification = true;
                    }
                    if let Some(ref tr) = result.tool_result {
                        write_info_log(
                            "HookManager::execute",
                            &format!("Hook 修改了 tool_result (cmd: {})", hook.command),
                        );
                        context.tool_result = Some(tr.clone());
                        final_result.tool_result = Some(tr.clone());
                        had_modification = true;
                    }
                    if let Some(ref inject) = result.inject_messages {
                        // inject_messages 累积追加
                        write_info_log(
                            "HookManager::execute",
                            &format!(
                                "Hook 注入了 {} 条消息 (cmd: {})",
                                inject.len(),
                                hook.command
                            ),
                        );
                        let existing = final_result.inject_messages.get_or_insert_with(Vec::new);
                        existing.extend(inject.clone());
                        had_modification = true;
                    }
                }
                Err(e) => {
                    // 非零退出 / 超时 → 视为 abort
                    write_error_log(
                        "HookManager::execute",
                        &format!("Hook 执行失败 (cmd: {}): {}", hook.command, e),
                    );
                    return Some(HookResult {
                        abort: true,
                        ..Default::default()
                    });
                }
            }
        }

        if had_modification {
            Some(final_result)
        } else {
            None
        }
    }
}

/// 执行单个 hook 脚本
fn execute_single_hook(hook: &HookDef, context: &HookContext) -> Result<HookResult, String> {
    let context_json =
        serde_json::to_string(context).map_err(|e| format!("序列化 context 失败: {}", e))?;

    let cwd = std::env::current_dir().map_err(|e| format!("获取 cwd 失败: {}", e))?;

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(&hook.command)
        .current_dir(&cwd)
        .env("JCLI_HOOK_EVENT", context.event.as_str())
        .env("JCLI_CWD", cwd.display().to_string())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("启动 hook 进程失败: {}", e))?;

    // 写入 stdin
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(context_json.as_bytes());
    }

    // 等待完成（带超时）
    let timeout = std::time::Duration::from_secs(hook.timeout);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(format!("Hook 退出码: {:?}", status.code()));
                }

                let output = child
                    .wait_with_output()
                    .map_err(|e| format!("读取输出失败: {}", e))?;
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stdout = stdout.trim();

                if stdout.is_empty() || stdout == "{}" {
                    return Ok(HookResult::default());
                }

                let result: HookResult = serde_json::from_str(stdout)
                    .map_err(|e| format!("解析 hook 输出 JSON 失败: {} (输出: {})", e, stdout))?;

                write_info_log(
                    "execute_single_hook",
                    &format!("Hook 完成 (cmd: {}), abort={}", hook.command, result.abort),
                );

                return Ok(result);
            }
            Ok(None) => {
                // 进程还在运行
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(format!("Hook 超时 ({}s): {}", hook.timeout, hook.command));
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => {
                return Err(format!("等待 hook 进程失败: {}", e));
            }
        }
    }
}

// ========== 单元测试 ==========

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_roundtrip() {
        for event in HookEvent::all() {
            let s = event.as_str();
            let parsed = HookEvent::from_str(s).unwrap();
            assert_eq!(*event, parsed);
        }
    }

    #[test]
    fn test_hook_event_from_str_invalid() {
        assert!(HookEvent::from_str("unknown_event").is_none());
    }

    #[test]
    fn test_hook_def_default_timeout() {
        let yaml = r#"command: "echo hello""#;
        let def: HookDef = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.timeout, 10);
    }

    #[test]
    fn test_hook_result_empty_json() {
        let result: HookResult = serde_json::from_str("{}").unwrap();
        assert!(!result.abort);
        assert!(result.messages.is_none());
        assert!(result.user_message.is_none());
    }

    #[test]
    fn test_hook_result_with_abort() {
        let json = r#"{"abort": true}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert!(result.abort);
    }

    #[test]
    fn test_hook_result_with_user_message() {
        let json = r#"{"user_message": "[modified] hello"}"#;
        let result: HookResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.user_message.as_deref(), Some("[modified] hello"));
    }

    #[test]
    fn test_hook_context_serialization() {
        let ctx = HookContext {
            event: HookEvent::PreSendMessage,
            user_message: Some("hello".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_string(&ctx).unwrap();
        assert!(json.contains("pre_send_message"));
        assert!(json.contains("hello"));
        // skip_serializing_if 应跳过 None 字段
        assert!(!json.contains("messages"));
        assert!(!json.contains("tool_name"));
    }

    #[test]
    fn test_execute_single_hook_echo() {
        let hook = HookDef {
            command: r#"echo '{"user_message": "hooked"}'"#.to_string(),
            timeout: 5,
        };
        let ctx = HookContext {
            event: HookEvent::PreSendMessage,
            user_message: Some("original".to_string()),
            ..Default::default()
        };
        let result = execute_single_hook(&hook, &ctx).unwrap();
        assert_eq!(result.user_message.as_deref(), Some("hooked"));
        assert!(!result.abort);
    }

    #[test]
    fn test_execute_single_hook_empty_output() {
        let hook = HookDef {
            command: "echo ''".to_string(),
            timeout: 5,
        };
        let ctx = HookContext::default();
        let result = execute_single_hook(&hook, &ctx).unwrap();
        assert!(!result.abort);
        assert!(result.user_message.is_none());
    }

    #[test]
    fn test_execute_single_hook_nonzero_exit() {
        let hook = HookDef {
            command: "exit 1".to_string(),
            timeout: 5,
        };
        let ctx = HookContext::default();
        let result = execute_single_hook(&hook, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_single_hook_reads_stdin() {
        // Script that reads stdin and echoes back a modified user_message
        let hook = HookDef {
            command: r#"input=$(cat); event=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('event',''))" 2>/dev/null || echo ""); echo '{"user_message": "got_input"}'"#.to_string(),
            timeout: 5,
        };
        let ctx = HookContext {
            event: HookEvent::PreSendMessage,
            user_message: Some("test".to_string()),
            ..Default::default()
        };
        let result = execute_single_hook(&hook, &ctx).unwrap();
        assert_eq!(result.user_message.as_deref(), Some("got_input"));
    }

    #[test]
    fn test_hook_manager_empty() {
        let manager = HookManager::default();
        assert!(manager.list_hooks().is_empty());
        let result = manager.execute(HookEvent::PreSendMessage, HookContext::default());
        assert!(result.is_none());
    }

    #[test]
    fn test_hook_manager_session_hooks() {
        let mut manager = HookManager::default();
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: r#"echo '{"user_message": "session_hooked"}'"#.to_string(),
                timeout: 5,
            },
        );

        let hooks = manager.list_hooks();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].2, "session");

        let result = manager
            .execute(
                HookEvent::PreSendMessage,
                HookContext {
                    event: HookEvent::PreSendMessage,
                    user_message: Some("original".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(result.user_message.as_deref(), Some("session_hooked"));
    }

    #[test]
    fn test_hook_manager_remove_session_hook() {
        let mut manager = HookManager::default();
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: "echo test".to_string(),
                timeout: 5,
            },
        );
        assert_eq!(manager.list_hooks().len(), 1);

        assert!(manager.remove_session_hook(HookEvent::PreSendMessage, 0));
        assert!(manager.list_hooks().is_empty());

        // 移除不存在的索引
        assert!(!manager.remove_session_hook(HookEvent::PreSendMessage, 0));
    }

    #[test]
    fn test_hook_chain_execution() {
        let mut manager = HookManager::default();

        // 注册两个 session hook，第一个设置 user_message，第二个也设置
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: r#"echo '{"user_message": "first"}'"#.to_string(),
                timeout: 5,
            },
        );
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: r#"echo '{"user_message": "second"}'"#.to_string(),
                timeout: 5,
            },
        );

        let result = manager
            .execute(
                HookEvent::PreSendMessage,
                HookContext {
                    event: HookEvent::PreSendMessage,
                    user_message: Some("original".to_string()),
                    ..Default::default()
                },
            )
            .unwrap();

        // 最后一个 hook 的输出应该覆盖之前的
        assert_eq!(result.user_message.as_deref(), Some("second"));
    }

    #[test]
    fn test_hook_abort_stops_chain() {
        let mut manager = HookManager::default();

        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: "exit 1".to_string(), // 非零退出 → abort
                timeout: 5,
            },
        );
        manager.register_session_hook(
            HookEvent::PreSendMessage,
            HookDef {
                command: r#"echo '{"user_message": "should_not_reach"}'"#.to_string(),
                timeout: 5,
            },
        );

        let result = manager
            .execute(
                HookEvent::PreSendMessage,
                HookContext {
                    event: HookEvent::PreSendMessage,
                    ..Default::default()
                },
            )
            .unwrap();

        assert!(result.abort);
        assert!(result.user_message.is_none());
    }
}
