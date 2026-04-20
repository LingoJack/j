use crate::assets::Assets;
use crate::command::chat::infra::hook::{
    HookDef, HookEvent, HookFilter, HookManager, HookType, OnError,
};
use crate::command::chat::tools::{
    PlanDecision, Tool, ToolResult, parse_tool_args, schema_to_tool_params,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex, atomic::AtomicBool};

/// RegisterHookTool 参数
#[derive(Deserialize, JsonSchema)]
struct RegisterHookParams {
    /// Action type: register (default), list, remove, help
    #[serde(default = "default_action")]
    action: String,
    /// Hook event name (required for register/remove)
    #[serde(default)]
    event: Option<String>,
    /// Hook type: "bash" (default) or "llm"
    #[serde(default)]
    r#type: Option<String>,
    /// Shell command to execute (required for type=bash)
    #[serde(default)]
    command: Option<String>,
    /// LLM prompt template (required for type=llm, supports {{variable}} template vars)
    #[serde(default)]
    prompt: Option<String>,
    /// LLM model name override (optional for type=llm)
    #[serde(default)]
    model: Option<String>,
    /// Timeout in seconds (default 10 for bash, 30 for llm)
    #[serde(default)]
    timeout: Option<u64>,
    /// Retry count on error (default 0 for bash, 1 for llm; only applies to Err path)
    #[serde(default)]
    retry: Option<u32>,
    /// Index of the session hook to remove (required for remove). Use session_idx from list output.
    #[serde(default)]
    index: Option<usize>,
    /// Error handling strategy: "skip" (default, log and continue) or "abort" (stop hook chain)
    #[serde(default)]
    on_error: Option<String>,
}

fn default_action() -> String {
    "register".to_string()
}

/// register_hook 工具：让 LLM 动态注册/管理 session 级 hook
#[derive(Debug)]
pub struct RegisterHookTool {
    pub hook_manager: Arc<Mutex<HookManager>>,
}

impl RegisterHookTool {
    pub const NAME: &'static str = "RegisterHook";
}

impl Tool for RegisterHookTool {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn description(&self) -> &str {
        r#"
        Register, list, remove session-level hooks, or view the full protocol documentation.
        Actions: register (requires event+command or event+prompt), list, remove (requires event+index), help (view stdin/stdout JSON schema and script examples).
        Supports two hook types: "bash" (shell command, default) and "llm" (LLM prompt template).
        Call action="help" first to learn the script protocol before registering hooks.
        "#
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<RegisterHookParams>()
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: RegisterHookParams = match parse_tool_args(arguments) {
            Ok(p) => p,
            Err(e) => return e,
        };

        match params.action.as_str() {
            "help" => Self::handle_help(),
            "list" => self.handle_list(),
            "remove" => self.handle_remove(&params),
            _ => self.handle_register(&params),
        }
    }

    fn requires_confirmation(&self) -> bool {
        true // 注册 hook 需要用户确认
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        if let Ok(params) = serde_json::from_str::<RegisterHookParams>(arguments) {
            match params.action.as_str() {
                "help" => "View Hook protocol documentation".to_string(),
                "list" => "List all registered hooks".to_string(),
                "remove" => {
                    let event = params.event.as_deref().unwrap_or("?");
                    let index = params.index.unwrap_or(0);
                    format!("Remove hook: event={}, index={}", event, index)
                }
                _ => {
                    let event = params.event.as_deref().unwrap_or("?");
                    let hook_type = params.r#type.as_deref().unwrap_or("bash");
                    let desc = if hook_type == "llm" {
                        let prompt_preview = params
                            .prompt
                            .as_deref()
                            .map(|p| if p.len() > 60 { &p[..60] } else { p })
                            .unwrap_or("?");
                        format!("type=llm, prompt={}", prompt_preview)
                    } else {
                        let cmd = params.command.as_deref().unwrap_or("?");
                        format!("type=bash, command={}", cmd)
                    };
                    let on_error = params.on_error.as_deref().unwrap_or("skip");
                    format!(
                        "Register hook: event={}, {}, on_error={}",
                        event, desc, on_error
                    )
                }
            }
        } else {
            "RegisterHook operation".to_string()
        }
    }
}

impl RegisterHookTool {
    fn handle_help() -> ToolResult {
        let content = Assets::get("help/hook.md")
            .map(|asset| {
                let raw = String::from_utf8_lossy(&asset.data);
                // 去掉 frontmatter（---...---），只返回 body
                Self::strip_frontmatter(&raw).to_string()
            })
            .unwrap_or_else(|| "Hook 文档加载失败".to_string());

        ToolResult {
            output: content,
            is_error: false,
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }

    /// 去掉 YAML frontmatter（`---` ... `---`），返回 body 部分
    fn strip_frontmatter(content: &str) -> &str {
        let trimmed = content.trim_start();
        if !trimmed.starts_with("---") {
            return trimmed;
        }
        let after_first = &trimmed[3..];
        if let Some(end) = after_first.find("\n---") {
            return after_first[end + 4..].trim_start();
        }
        trimmed
    }

    fn handle_register(&self, params: &RegisterHookParams) -> ToolResult {
        let event_str = match params.event.as_deref() {
            Some(e) => e,
            None => {
                return ToolResult {
                    output: "缺少 event 参数".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let event = match HookEvent::parse(event_str) {
            Some(e) => e,
            None => {
                return ToolResult {
                    output: format!("未知事件: {}", event_str),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        // 解析 hook 类型
        let hook_type = match params.r#type.as_deref() {
            Some("llm") => HookType::Llm,
            _ => HookType::Bash, // 默认 bash
        };

        // 校验必填字段
        match hook_type {
            HookType::Bash => {
                if params.command.is_none() {
                    return ToolResult {
                        output: "bash hook 缺少 command 参数".to_string(),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
            }
            HookType::Llm => {
                if params.prompt.is_none() {
                    return ToolResult {
                        output: "llm hook 缺少 prompt 参数".to_string(),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }
            }
        }

        let timeout = params.timeout.unwrap_or(match hook_type {
            HookType::Bash => 10,
            HookType::Llm => 30,
        });

        let retry = params.retry.unwrap_or(match hook_type {
            HookType::Bash => 0,
            HookType::Llm => 1,
        });

        let on_error = match params.on_error.as_deref() {
            Some("abort") => OnError::Abort,
            _ => OnError::Skip, // 默认 skip
        };

        let on_error_str = match on_error {
            OnError::Skip => "skip",
            OnError::Abort => "abort",
        };

        let hook_def = HookDef {
            r#type: hook_type,
            command: params.command.clone(),
            prompt: params.prompt.clone(),
            model: params.model.clone(),
            timeout,
            retry,
            on_error,
            filter: HookFilter::default(),
        };

        match self.hook_manager.lock() {
            Ok(mut manager) => {
                manager.register_session_hook(event, hook_def);
                let type_str = format!("{}", hook_type);
                let detail = match hook_type {
                    HookType::Bash => {
                        format!("command={}", params.command.as_deref().unwrap_or("?"))
                    }
                    HookType::Llm => {
                        let prompt_preview = params
                            .prompt
                            .as_deref()
                            .map(|p| if p.len() > 60 { &p[..60] } else { p })
                            .unwrap_or("?");
                        format!("prompt={}", prompt_preview)
                    }
                };
                ToolResult {
                    output: format!(
                        "已注册 session hook: event={}, type={}, {}, timeout={}s, retry={}, on_error={}",
                        event_str, type_str, detail, timeout, retry, on_error_str
                    ),
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
            Err(e) => ToolResult {
                output: format!("获取 HookManager 锁失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    fn handle_list(&self) -> ToolResult {
        match self.hook_manager.lock() {
            Ok(manager) => {
                let hooks = manager.list_hooks();
                if hooks.is_empty() {
                    return ToolResult {
                        output: "当前没有已注册的 hook".to_string(),
                        is_error: false,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    };
                }

                let mut output = String::from("已注册的 hook:\n");
                for (i, entry) in hooks.iter().enumerate() {
                    let timeout_str = entry
                        .timeout
                        .map(|t| format!("{}s", t))
                        .unwrap_or_else(|| "-".to_string());
                    let on_error_str = entry
                        .on_error
                        .map(|e| match e {
                            OnError::Skip => "skip",
                            OnError::Abort => "abort",
                        })
                        .unwrap_or("-");
                    let session_idx_str = entry
                        .session_index
                        .map(|idx| format!(", session_idx={}", idx))
                        .unwrap_or_default();
                    let filter_str = entry
                        .filter
                        .as_ref()
                        .map(|f| {
                            let mut parts = Vec::new();
                            if let Some(ref t) = f.tool_name {
                                parts.push(format!("tool={}", t));
                            }
                            if let Some(ref m) = f.model_prefix {
                                parts.push(format!("model={}*", m));
                            }
                            if parts.is_empty() {
                                String::new()
                            } else {
                                format!(", filter=[{}]", parts.join(","))
                            }
                        })
                        .unwrap_or_default();
                    let metrics_str = entry
                        .metrics
                        .as_ref()
                        .map(|m| {
                            format!(
                                ", runs={}/ok={}/fail={}/skip={}/{}ms",
                                m.executions,
                                m.successes,
                                m.failures,
                                m.skipped,
                                m.total_duration_ms
                            )
                        })
                        .unwrap_or_default();
                    output.push_str(&format!(
                        "  [{}] event={}, source={}, type={}{}, label={}, timeout={}, on_error={}{}{}\n",
                        i,
                        entry.event.as_str(),
                        entry.source,
                        entry.hook_type,
                        session_idx_str,
                        entry.label,
                        timeout_str,
                        on_error_str,
                        filter_str,
                        metrics_str,
                    ));
                }
                ToolResult {
                    output,
                    is_error: false,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                }
            }
            Err(e) => ToolResult {
                output: format!("获取 HookManager 锁失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }

    fn handle_remove(&self, params: &RegisterHookParams) -> ToolResult {
        let event_str = match params.event.as_deref() {
            Some(e) => e,
            None => {
                return ToolResult {
                    output: "缺少 event 参数".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let event = match HookEvent::parse(event_str) {
            Some(e) => e,
            None => {
                return ToolResult {
                    output: format!("未知事件: {}", event_str),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let index = params.index.unwrap_or(0);

        match self.hook_manager.lock() {
            Ok(mut manager) => {
                if manager.remove_session_hook(event, index) {
                    ToolResult {
                        output: format!(
                            "已移除 session hook: event={}, index={}",
                            event_str, index
                        ),
                        is_error: false,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    }
                } else {
                    ToolResult {
                        output: format!(
                            "移除失败：event={} 的 session hook 索引 {} 不存在",
                            event_str, index
                        ),
                        is_error: true,
                        images: vec![],
                        plan_decision: PlanDecision::None,
                    }
                }
            }
            Err(e) => ToolResult {
                output: format!("获取 HookManager 锁失败: {}", e),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            },
        }
    }
}
