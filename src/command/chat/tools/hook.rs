use crate::command::chat::hook::{HookDef, HookEvent, HookManager};
use crate::command::chat::tools::{Tool, ToolResult};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex, atomic::AtomicBool};

/// register_hook 工具：让 LLM 动态注册/管理 session 级 hook
pub struct RegisterHookTool {
    pub hook_manager: Arc<Mutex<HookManager>>,
}

impl Tool for RegisterHookTool {
    fn name(&self) -> &str {
        "RegisterHook"
    }

    fn description(&self) -> &str {
        r#"
        Register, list, remove session-level hooks, or view the full protocol documentation.
        Actions: register (requires event+command), list, remove (requires event+index), help (view stdin/stdout JSON schema and script examples).
        Call action="help" first to learn the script protocol before registering hooks.
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "Action type: register (default), list, remove, help",
                    "enum": ["register", "list", "remove", "help"]
                },
                "event": {
                    "type": "string",
                    "description": "Hook event name (required for register/remove)",
                    "enum": [
                        "pre_send_message", "post_send_message",
                        "pre_llm_request", "post_llm_response",
                        "pre_tool_execution", "post_tool_execution",
                        "session_start", "session_end"
                    ]
                },
                "command": {
                    "type": "string",
                    "description": "Shell command to execute (required for register)"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (default 10)"
                },
                "index": {
                    "type": "integer",
                    "description": "Index of the hook to remove (required for remove)"
                }
            }
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let parsed: Value = match serde_json::from_str(arguments) {
            Ok(v) => v,
            Err(e) => {
                return ToolResult {
                    output: format!("参数解析失败: {}", e),
                    is_error: true,
                };
            }
        };

        let action = parsed
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("register");

        match action {
            "help" => Self::handle_help(),
            "list" => self.handle_list(),
            "remove" => self.handle_remove(&parsed),
            _ => self.handle_register(&parsed),
        }
    }

    fn requires_confirmation(&self) -> bool {
        true // 注册 hook 需要用户确认
    }

    fn confirmation_message(&self, arguments: &str) -> String {
        let parsed: Value = serde_json::from_str(arguments).unwrap_or(Value::Null);
        let action = parsed
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("register");

        match action {
            "help" => "View Hook protocol documentation".to_string(),
            "list" => "List all registered hooks".to_string(),
            "remove" => {
                let event = parsed.get("event").and_then(|v| v.as_str()).unwrap_or("?");
                let index = parsed.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                format!("Remove hook: event={}, index={}", event, index)
            }
            _ => {
                let event = parsed.get("event").and_then(|v| v.as_str()).unwrap_or("?");
                let command = parsed
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                format!("Register hook: event={}, command={}", event, command)
            }
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
- exit 0 = 成功，非零 = abort

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
- 只有 session 级 hook 可通过本工具管理；用户级/项目级需手动编辑配置文件"#
                .to_string(),
            is_error: false,
        }
    }

    fn handle_register(&self, parsed: &Value) -> ToolResult {
        let event_str = match parsed.get("event").and_then(|v| v.as_str()) {
            Some(e) => e,
            None => {
                return ToolResult {
                    output: "缺少 event 参数".to_string(),
                    is_error: true,
                };
            }
        };

        let event = match HookEvent::from_str(event_str) {
            Some(e) => e,
            None => {
                return ToolResult {
                    output: format!("未知事件: {}", event_str),
                    is_error: true,
                };
            }
        };

        let command = match parsed.get("command").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => {
                return ToolResult {
                    output: "缺少 command 参数".to_string(),
                    is_error: true,
                };
            }
        };

        let timeout = parsed.get("timeout").and_then(|v| v.as_u64()).unwrap_or(10);

        let hook_def = HookDef {
            command: command.clone(),
            timeout,
        };

        match self.hook_manager.lock() {
            Ok(mut manager) => {
                manager.register_session_hook(event, hook_def);
                ToolResult {
                    output: format!(
                        "已注册 session hook: event={}, command={}, timeout={}s",
                        event_str, command, timeout
                    ),
                    is_error: false,
                }
            }
            Err(e) => ToolResult {
                output: format!("获取 HookManager 锁失败: {}", e),
                is_error: true,
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
                    };
                }

                let mut output = String::from("已注册的 hook:\n");
                for (i, (event, def, source)) in hooks.iter().enumerate() {
                    output.push_str(&format!(
                        "  [{}] event={}, source={}, command={}, timeout={}s\n",
                        i,
                        event.as_str(),
                        source,
                        def.command,
                        def.timeout
                    ));
                }
                ToolResult {
                    output,
                    is_error: false,
                }
            }
            Err(e) => ToolResult {
                output: format!("获取 HookManager 锁失败: {}", e),
                is_error: true,
            },
        }
    }

    fn handle_remove(&self, parsed: &Value) -> ToolResult {
        let event_str = match parsed.get("event").and_then(|v| v.as_str()) {
            Some(e) => e,
            None => {
                return ToolResult {
                    output: "缺少 event 参数".to_string(),
                    is_error: true,
                };
            }
        };

        let event = match HookEvent::from_str(event_str) {
            Some(e) => e,
            None => {
                return ToolResult {
                    output: format!("未知事件: {}", event_str),
                    is_error: true,
                };
            }
        };

        let index = parsed.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        match self.hook_manager.lock() {
            Ok(mut manager) => {
                if manager.remove_session_hook(event, index) {
                    ToolResult {
                        output: format!(
                            "已移除 session hook: event={}, index={}",
                            event_str, index
                        ),
                        is_error: false,
                    }
                } else {
                    ToolResult {
                        output: format!(
                            "移除失败：event={} 的 session hook 索引 {} 不存在",
                            event_str, index
                        ),
                        is_error: true,
                    }
                }
            }
            Err(e) => ToolResult {
                output: format!("获取 HookManager 锁失败: {}", e),
                is_error: true,
            },
        }
    }
}
