use crate::command::chat::hook::{HookDef, HookEvent, HookFilter, HookManager, OnError};
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
    /// Shell command to execute (required for register)
    #[serde(default)]
    command: Option<String>,
    /// Timeout in seconds (default 10)
    #[serde(default)]
    timeout: Option<u64>,
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
        Actions: register (requires event+command), list, remove (requires event+index), help (view stdin/stdout JSON schema and script examples).
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
                    let command = params.command.as_deref().unwrap_or("?");
                    let on_error = params.on_error.as_deref().unwrap_or("skip");
                    format!(
                        "Register hook: event={}, command={}, on_error={}",
                        event, command, on_error
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
        ToolResult {
            output: r#"# Hook 完整协议文档

## 可用事件及其可读/可写字段

| event                         | 触发时机       | stdin 可读字段                                  | stdout 可写字段                                                        |
|-------------------------------|----------------|-------------------------------------------------|------------------------------------------------------------------------|
| pre_send_message              | 用户消息发送前 | user_input, messages                            | user_input, abort, retry_feedback                                      |
| post_send_message             | 用户消息发送后 | user_input, messages                            | （仅通知，返回值忽略）                                                 |
| pre_llm_request               | LLM 请求前     | messages, system_prompt, model                  | messages, system_prompt, inject_messages, additional_context, abort    |
| post_llm_response             | LLM 回复后     | assistant_output, messages, model               | assistant_output, abort, retry_feedback, system_message                |
| pre_tool_execution            | 工具执行前     | tool_name, tool_arguments                       | tool_arguments, abort                                                  |
| post_tool_execution           | 工具执行后     | tool_name, tool_result                          | tool_result                                                            |
| post_tool_execution_failure   | 工具执行失败后 | tool_name, tool_error                           | tool_error, additional_context                                         |
| stop                          | LLM 即将结束   | user_input(回复), messages, system_prompt, model | retry_feedback, additional_context, abort                              |
| pre_compact                   | 上下文压缩前   | messages, system_prompt, model, compact_trigger | additional_context, abort                                              |
| post_compact                  | 上下文压缩后   | messages, compact_trigger                       | messages                                                               |
| session_start                 | 会话开始       | messages                                        | （仅通知）                                                             |
| session_end                   | 会话退出       | messages                                        | （仅通知）                                                             |

## 脚本协议
- 执行方式：`sh -c "<command>"`
- 工作目录：用户当前目录
- 环境变量：JCLI_HOOK_EVENT（事件名）、JCLI_CWD（当前目录）
- stdin：HookContext JSON
- stdout：HookResult JSON（只返回要修改的字段，空/`{}` 表示无修改）
- exit 0 = 成功，非零 = 失败（按 on_error 策略处理：skip=记录日志继续，abort=中止整条链）
- on_error 默认 "skip"：脚本失败时不中断操作，仅记录错误日志

## stdin HookContext JSON 结构
```json
{
  "event": "pre_send_message",
  "cwd": "/path/to/project",
  "user_input": "用户输入文本",
  "messages": [{"role": "user", "content": "..."}],
  "system_prompt": "系统提示词",
  "model": "gpt-4o",
  "assistant_output": "AI 回复文本",
  "tool_name": "Bash",
  "tool_arguments": "{\"command\": \"ls\"}",
  "tool_result": "工具执行结果",
  "tool_error": "工具错误信息",
  "compact_trigger": "auto"
}
```
各字段按事件类型选择性出现，未填充的不会出现在 JSON 中。

## stdout HookResult JSON 结构
```json
{
  "user_input": "修改后的用户消息",
  "assistant_output": "修改后的 AI 回复",
  "messages": [{"role":"user","content":"..."}],
  "system_prompt": "修改后的提示词",
  "tool_arguments": "修改后的工具参数",
  "tool_result": "修改后的工具结果",
  "tool_error": "修改后的错误信息",
  "inject_messages": [{"role":"user","content":"注入消息"}],
  "retry_feedback": "审查反馈：请修正XX问题",
  "additional_context": "追加到 system_prompt 的额外上下文",
  "system_message": "展示给用户的提示消息",
  "abort": false
}
```

## 关键字段说明
- `retry_feedback`：与 abort 配合使用。在 stop/pre_send_message/post_llm_response 中，abort+retry_feedback 会中止当前操作并将反馈注入为新消息，LLM 带反馈重新生成。这是实现"宪法 AI/纠查官"的核心机制。
- `additional_context`：追加文本到 system_prompt 末尾，不占消息位。适用于注入规则、约束等。
- `system_message`：在 UI 上以 toast/提示形式展示给用户，不影响 LLM 输入。

## 脚本示例

### 示例 1：给用户消息加时间戳（pre_send_message）
```bash
#!/bin/bash
input=$(cat)
msg=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('user_input',''))")
echo "{\"user_input\": \"[$(date '+%H:%M')] $msg\"}"
```

### 示例 2：拦截危险命令（pre_tool_execution）
```bash
#!/bin/bash
input=$(cat)
tool=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('tool_name',''))")
args=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('tool_arguments',''))")
if [ "$tool" = "Bash" ] && echo "$args" | grep -q "rm -rf"; then
  echo '{"abort": true}'
else
  echo '{}'
fi
```

### 示例 3：宪法 AI 纠查官（stop）
```bash
#!/bin/bash
input=$(cat)
reply=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('user_input',''))")
if echo "$reply" | grep -qiE 'password|secret|api.key'; then
  echo '{"abort": true, "retry_feedback": "回复包含敏感信息，请重新组织回答避免泄露密码/密钥"}'
else
  echo '{}'
fi
```

### 示例 4：压缩保护（pre_compact）
```bash
#!/bin/bash
echo '{"additional_context": "压缩时必须保留所有宪法规则和关键约束，不可丢弃。"}'
```

### 示例 5：纯通知（post_send_message / session_end）
```bash
#!/bin/bash
cat > /dev/null  # 必须读 stdin，否则可能 SIGPIPE
```

## 注意事项
- 先用 Write/Bash 工具创建脚本文件，再用本工具注册
- 脚本必须从 stdin 读取（至少 `cat > /dev/null`），否则可能 SIGPIPE
- timeout 默认 10 秒，超时后脚本被 kill
- on_error 默认 "skip"（记录日志继续），设为 "abort" 则脚本失败时中止整条 hook 链
- 只有 session 级 hook 可通过本工具管理；用户级/项目级需手动编辑配置文件
- 移除 hook 时，使用 list 输出中的 session_idx 作为 index 参数"#
                .to_string(),
            is_error: false,
                    images: vec![],
                plan_decision: PlanDecision::None,
        }
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

        let command = match params.command.as_deref() {
            Some(c) => c.to_string(),
            None => {
                return ToolResult {
                    output: "缺少 command 参数".to_string(),
                    is_error: true,
                    images: vec![],
                    plan_decision: PlanDecision::None,
                };
            }
        };

        let timeout = params.timeout.unwrap_or(10);

        let on_error = match params.on_error.as_deref() {
            Some("abort") => OnError::Abort,
            _ => OnError::Skip, // 默认 skip
        };

        let on_error_str = match on_error {
            OnError::Skip => "skip",
            OnError::Abort => "abort",
        };

        let hook_def = HookDef {
            command: command.clone(),
            timeout,
            on_error,
            filter: HookFilter::default(),
        };

        match self.hook_manager.lock() {
            Ok(mut manager) => {
                manager.register_session_hook(event, hook_def);
                ToolResult {
                    output: format!(
                        "已注册 session hook: event={}, command={}, timeout={}s, on_error={}",
                        event_str, command, timeout, on_error_str
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
                        "  [{}] event={}, source={}{}, label={}, timeout={}, on_error={}{}{}\n",
                        i,
                        entry.event.as_str(),
                        entry.source,
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
