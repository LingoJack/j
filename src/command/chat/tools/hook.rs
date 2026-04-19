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
        ToolResult {
            output: r#"# Hook 完整协议文档

## Hook 类型

### bash（默认）
通过 `sh -c` 子进程执行 Shell 命令。
- 参数：`command`（必填）、`timeout`（默认 10s）、`on_error`、`retry`（默认 0）

### llm
通过 prompt 模板调用 LLM，LLM 返回 HookResult JSON。
- 参数：`prompt`（必填，支持 `{{variable}}` 模板变量）、`model`（可选，覆盖当前模型）、`timeout`（默认 30s）、`retry`（默认 1）、`on_error`
- LLM 输出必须为合法 HookResult JSON，系统会自动提取 JSON 并解析
- 解析失败或网络错误 → Err → 按 retry 重试 → 重试耗尽按 on_error 处理
- 可用模板变量：`{{event}}`、`{{user_input}}`、`{{assistant_output}}`、`{{tool_name}}`、`{{tool_arguments}}`、`{{tool_result}}`、`{{model}}`、`{{cwd}}`

## 可用事件及其可读/可写字段

| event                         | 触发时机       | 可读字段                                        | 可写字段                                                        |
|-------------------------------|----------------|-------------------------------------------------|------------------------------------------------------------------------|
| pre_send_message              | 用户消息发送前 | user_input, messages                            | user_input, action=stop, retry_feedback                                |
| post_send_message             | 用户消息发送后 | user_input, messages                            | （仅通知，返回值忽略）                                                 |
| pre_llm_request               | LLM 请求前     | messages, system_prompt, model                  | messages, system_prompt, inject_messages, additional_context, action=stop, retry_feedback |
| post_llm_response             | LLM 回复后     | assistant_output, messages, model               | assistant_output, action=stop, retry_feedback, system_message          |
| pre_tool_execution            | 工具执行前     | tool_name, tool_arguments                       | tool_arguments, action=skip                                            |
| post_tool_execution           | 工具执行后     | tool_name, tool_result                          | tool_result                                                            |
| post_tool_execution_failure   | 工具执行失败后 | tool_name, tool_error                           | tool_error, additional_context                                         |
| stop                          | LLM 即将结束   | user_input(回复), messages, system_prompt, model | retry_feedback, additional_context, action=stop                        |
| pre_micro_compact             | micro_compact前| messages, model                                 | action=stop                                                            |
| post_micro_compact            | micro_compact后| messages                                        | messages                                                               |
| pre_auto_compact              | auto_compact前 | messages, system_prompt, model                  | additional_context, action=stop                                        |
| post_auto_compact             | auto_compact后 | messages                                        | messages                                                               |
| session_start                 | 会话开始       | messages                                        | （仅通知）                                                             |
| session_end                   | 会话退出       | messages                                        | （仅通知）                                                             |

## Bash Hook 脚本协议
- 执行方式：`sh -c "<command>"`
- 工作目录：用户当前目录
- 环境变量：JCLI_HOOK_EVENT（事件名）、JCLI_CWD（当前目录）
- stdin：HookContext JSON
- stdout：HookResult JSON（只返回要修改的字段，空/`{}` 表示无修改）
- exit 0 = 成功，非零 = 失败（按 on_error 策略处理：skip=记录日志继续，abort=中止整条链）
- on_error 默认 "skip"：脚本失败时不中断操作，仅记录错误日志
- retry 默认 0：失败后不重试；设置 >0 则重试指定次数（受链总超时 30s 约束）

## LLM Hook 协议
- 系统自动在 prompt 末尾追加 JSON 格式指令，LLM 需返回 HookResult JSON
- 使用当前活跃 provider 的 API（或通过 model 参数覆盖模型名）
- JSON 提取逻辑：从 LLM 输出中找第一个 `{` 到最后一个 `}` 之间的内容
- 解析失败 → 视为 Err → 按 retry 重试
- retry 默认 1：LLM 返回非法 JSON 或网络失败时重试

## HookResult JSON 结构
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
  "action": "stop",
  "retry_feedback": "审查反馈：请修正XX问题",
  "additional_context": "追加到 system_prompt 的额外上下文",
  "system_message": "展示给用户的提示消息"
}
```

## 关键字段说明
- `action`：控制流动作，字符串 `"stop"` 或 `"skip"`。旧字段 `abort: true` 等价于 `action: "stop"`。
  - `"stop"`：中止当前步骤及其所属子管线
  - `"skip"`：跳过当前步骤，同级步骤继续（仅 `pre_tool_execution` 中使用）
- `retry_feedback`：与 stop 配合使用。在 stop/pre_send_message/post_llm_response 中，stop+retry_feedback 会中止当前操作并将反馈注入为新消息，LLM 带反馈重新生成。这是实现"宪法 AI/纠查官"的核心机制。
- `additional_context`：追加文本到 system_prompt 末尾，不占消息位。适用于注入规则、约束等。
- `system_message`：在 UI 上以 toast/提示形式展示给用户，不影响 LLM 输入。

## action 语义
- `pre_send_message` / `pre_llm_request` / `stop` / `post_llm_response`：`action=stop` 中止当前操作
- `pre_tool_execution`：`action=skip` 跳过该工具调用（其他工具继续执行）
- `pre_micro_compact`：`action=stop` 中止整个 compact 子管线
- `pre_auto_compact`：`action=stop` 中止 auto_compact

## 压缩 Hook 说明
两层压缩各有独立的 Pre/Post hook，构成一个 compact 子管线：
1. `pre_micro_compact` → micro_compact → `post_micro_compact`
2. `pre_auto_compact` → auto_compact → `post_auto_compact`

## 示例

### 示例 1：LLM 纠查官（推荐，type=llm）
```yaml
# ~/.jdata/agent/hooks.yaml
post_llm_response:
  - type: llm
    prompt: |
      检查以下 AI 回复是否包含敏感信息（密码、密钥、token）：
      {{assistant_output}}
      如果包含敏感信息，返回 action=stop + retry_feedback 说明问题。
      如果没有问题，返回空 JSON {}。
    timeout: 30
    retry: 1
    on_error: skip
```

### 示例 2：LLM 消息审查（pre_send_message）
```yaml
pre_send_message:
  - type: llm
    prompt: |
      审查用户消息是否合规：{{user_input}}
      如有违规返回 action=stop 和 retry_feedback。
    model: gpt-4o-mini
    timeout: 15
    retry: 1
```

### 示例 3：Bash 脚本 - 给消息加时间戳（pre_send_message）
```bash
#!/bin/bash
input=$(cat)
msg=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('user_input',''))")
echo "{\"user_input\": \"[$(date '+%H:%M')] $msg\"}"
```

### 示例 4：Bash 脚本 - 跳过危险命令（pre_tool_execution）
```bash
#!/bin/bash
input=$(cat)
tool=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('tool_name',''))")
args=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('tool_arguments',''))")
if [ "$tool" = "Bash" ] && echo "$args" | grep -q "rm -rf"; then
  echo '{"action": "skip"}'
else
  echo '{}'
fi
```

### 示例 5：YAML 配置 - 带过滤器的工具审查
```yaml
pre_tool_execution:
  - type: llm
    prompt: |
      审查工具调用是否安全：工具={{tool_name}}, 参数={{tool_arguments}}
      如果不安全，返回 action=skip。
    filter:
      tool_matcher: "Bash|Shell"
    timeout: 15
    retry: 1
```

## 注意事项
- LLM hook 使用当前活跃的 provider API（可通过 model 参数覆盖模型名）
- bash hook 必须从 stdin 读取（至少 `cat > /dev/null`），否则可能 SIGPIPE
- retry 只对 Err 路径生效（超时、非零退出、LLM JSON 解析失败、网络失败）
- 重试受链总超时（30s）约束
- 只有 session 级 hook 可通过本工具管理；用户级/项目级需手动编辑 YAML 配置文件
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
