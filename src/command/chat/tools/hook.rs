use crate::command::chat::hook::{HookDef, HookEvent, HookManager, OnError};
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

| event               | 触发时机       | stdin 可读字段                    | stdout 可写字段                                    |
|---------------------|----------------|-----------------------------------|----------------------------------------------------|
| pre_send_message    | 用户消息发送前 | user_input, messages              | user_input, abort                                  |
| post_send_message   | 用户消息发送后 | user_input, messages              | （仅通知，返回值忽略）                             |
| pre_llm_request     | LLM 请求前     | messages, system_prompt, model    | messages, system_prompt, inject_messages, abort     |
| post_llm_response   | LLM 回复后     | assistant_output, messages        | assistant_output                                   |
| pre_tool_execution  | 工具执行前     | tool_name, tool_arguments         | tool_arguments, abort                              |
| post_tool_execution | 工具执行后     | tool_name, tool_result            | tool_result                                        |
| session_start       | 会话开始       | messages                          | （仅通知）                                         |
| session_end         | 会话退出       | messages                          | （仅通知）                                         |

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
  "tool_result": "工具执行结果"
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
  "inject_messages": [{"role":"user","content":"注入消息"}],
  "abort": false
}
```

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

### 示例 3：纯通知（post_send_message / session_end）
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
                    output.push_str(&format!(
                        "  [{}] event={}, source={}{}, label={}, timeout={}, on_error={}\n",
                        i,
                        entry.event.as_str(),
                        entry.source,
                        session_idx_str,
                        entry.label,
                        timeout_str,
                        on_error_str,
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
