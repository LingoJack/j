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

    // TODO 这里 AI 根本不知道如何创建自己的 hook，不会使用
    fn description(&self) -> &str {
        r#"
        注册、列出或移除 session 级 hook。Hook 允许在关键事件节点注入自定义脚本。

        支持三种操作：
        1. 注册 hook：提供 event + command（可选 timeout）
        2. 列出所有 hook：action="list"
        3. 移除 session hook：action="remove" + event + index

        可用事件：
        - pre_send_message: 用户发送消息前
        - post_send_message: 用户发送消息后
        - pre_llm_request: LLM 请求前
        - post_llm_response: LLM 回复后
        - pre_tool_execution: 工具执行前
        - post_tool_execution: 工具执行后
        - session_start: 会话开始
        - session_end: 会话结束

        脚本通过 stdin 接收 HookContext JSON，stdout 输出 HookResult JSON。
        exit 0 表示成功，非零退出表示 abort。
        "#
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "description": "操作类型：register（默认）、list、remove",
                    "enum": ["register", "list", "remove"]
                },
                "event": {
                    "type": "string",
                    "description": "Hook 事件名称（register/remove 时必填）",
                    "enum": [
                        "pre_send_message", "post_send_message",
                        "pre_llm_request", "post_llm_response",
                        "pre_tool_execution", "post_tool_execution",
                        "session_start", "session_end"
                    ]
                },
                "command": {
                    "type": "string",
                    "description": "要执行的 shell 命令（register 时必填）"
                },
                "timeout": {
                    "type": "integer",
                    "description": "超时秒数（默认 10）"
                },
                "index": {
                    "type": "integer",
                    "description": "要移除的 hook 索引（remove 时必填）"
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
            "list" => self.handle_list(),
            "remove" => self.handle_remove(&parsed),
            "register" | _ => self.handle_register(&parsed),
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
            "list" => "列出所有已注册的 hook".to_string(),
            "remove" => {
                let event = parsed.get("event").and_then(|v| v.as_str()).unwrap_or("?");
                let index = parsed.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                format!("移除 hook: event={}, index={}", event, index)
            }
            _ => {
                let event = parsed.get("event").and_then(|v| v.as_str()).unwrap_or("?");
                let command = parsed
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                format!("注册 hook: event={}, command={}", event, command)
            }
        }
    }
}

impl RegisterHookTool {
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
