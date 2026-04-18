---
name: Hook
order: 8
---

## AI Hook

Hook 允许在对话关键节点注入自定义逻辑。对用户可配置部分，支持三级来源：

1. **用户级**：`~/.jdata/agent/hooks.yaml` — 全局生效
2. **项目级**：`.jcli/hooks.yaml` — 项目目录下生效
3. **Session 级**：通过 `register_hook` 工具由 AI 动态注册 — 仅当前会话

> 运行时实际还存在**内置 hook**，执行顺序是：内置 -> 用户级 -> 项目级 -> Session 级。
> 同一事件按链式执行，前一个 hook 的输出会成为后一个 hook 的输入。

### 事件生命周期总览

下面按一条消息从「用户输入」到「AI 回复完成」的完整流程，标注所有 Hook 事件的触发位置：

```
[会话启动] ──→ session_start
[会话退出] ──→ session_end

一轮对话的完整流程（循环）：

  用户输入
    │
    ▼
  ┌─────────────────────┐
  │  pre_send_message   │  ← 可修改/拦截用户消息
  └────────┬────────────┘
           ▼
  ┌─────────────────────┐
  │  post_send_message  │  ← 仅通知，不可修改
  └────────┬────────────┘
           ▼
  ┌─────────────────────┐
  │  pre_llm_request    │  ← 可修改 system_prompt / messages
  └────────┬────────────┘
           ▼
       LLM 请求
           │
           ▼
  ┌─────────────────────┐
  │  post_llm_response  │  ← 可修改 AI 回复内容
  └────────┬────────────┘
           │
           ├── AI 回复中包含工具调用？
           │       │
           │       ▼
           │   ┌──────────────────────────┐
           │   │  pre_tool_execution      │  ← 可修改/跳过工具参数
           │   └────────────┬─────────────┘
           │                ▼
           │           执行工具
           │                │
           │        ┌───────┴────────┐
           │        ▼                ▼
           │   成功:                失败:
           │   post_tool_execution   post_tool_execution_failure
           │   (可修改结果)          (可修改错误信息)
           │        │                │
           │        └───────┬────────┘
           │                ▼
           │       工具结果返回 LLM → 回到 pre_llm_request
           │       (LLM 基于工具结果继续生成)
           │
           ├── AI 回复中不再有工具调用？
           │       │
           │       ▼
           │   ┌─────────────────────┐
           │   │  stop               │  ← LLM 即将结束回复
           │   └────────┬────────────┘
           │            ▼
           │       回到顶部，等待下一轮用户输入
           │
           └── 上下文接近上限时触发压缩：
                   │
                   ├── micro_compact（轮次级压缩）
                   │       │
                   │   pre_micro_compact → 执行压缩 → post_micro_compact
                   │
                   └── auto_compact（全量压缩）
                           │
                       pre_auto_compact → 执行压缩 → post_auto_compact
```

### 事件详解

**一、会话级事件**（每个会话触发一次）

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `session_start` | 会话启动时 | `messages` | 仅通知，返回值被忽略 |
| `session_end` | 会话退出时 | `messages` | 仅通知，返回值被忽略 |

**二、消息发送阶段**

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `pre_send_message` | 用户发送消息前 | `user_input`, `messages` | `user_input`, `action=stop`, `retry_feedback` |
| `post_send_message` | 用户发送消息后 | `user_input`, `messages` | 仅通知，返回值被忽略 |

**三、LLM 请求/回复阶段**

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `pre_llm_request` | LLM API 请求前 | `messages`, `system_prompt`, `model` | `messages`, `system_prompt`, `inject_messages`, `additional_context`, `action=stop`, `retry_feedback` |
| `post_llm_response` | LLM 回复完成后 | `assistant_output`, `messages`, `model` | `assistant_output`, `action=stop`, `retry_feedback`, `system_message` |

**四、工具执行阶段**

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `pre_tool_execution` | 工具执行前 | `tool_name`, `tool_arguments` | `tool_arguments`, `action=skip` |
| `post_tool_execution` | 工具执行成功后 | `tool_name`, `tool_result` | `tool_result` |
| `post_tool_execution_failure` | 工具执行失败后 | `tool_name`, `tool_error` | `tool_error`, `additional_context` |

**五、回复结束阶段**

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `stop` | LLM 即将结束回复时（无更多工具调用） | `user_input`, `messages`, `system_prompt`, `model` | `retry_feedback`, `additional_context`, `action=stop` |

**六、上下文压缩阶段**

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `pre_micro_compact` | 轮次级压缩前 | `messages`, `model` | `action=stop` |
| `post_micro_compact` | 轮次级压缩后 | `messages` | `messages` |
| `pre_auto_compact` | 全量压缩前 | `messages`, `system_prompt`, `model` | `additional_context`, `action=stop` |
| `post_auto_compact` | 全量压缩后 | `messages` | `messages` |

### 配置示例

`~/.jdata/agent/hooks.yaml`：

```yaml
pre_send_message:
  - command: "python3 ~/.jdata/agent/hooks/inject_time.py"
    timeout: 5
    on_error: skip
session_start:
  - command: "echo '{\"inject_messages\": [{\"role\": \"user\", \"content\": \"当前用户: jack\"}]}'"
pre_tool_execution:
  - type: llm
    prompt: |
      审查工具调用是否安全：工具={{tool_name}} 参数={{tool_arguments}}
      如果不安全，返回 {"action":"skip"}，否则返回 {}
    filter:
      tool_matcher: "Bash|Shell"
```

### 支持类型

- `bash`：默认类型，通过 `sh -c` 执行 `command`
- `llm`：通过 `prompt` 模板调用当前 provider，要求返回 HookResult JSON

### 脚本协议

- stdin 接收 `HookContext` JSON，stdout 输出 `HookResult` JSON；空字符串或 `{}` 表示无修改
- `bash` hook 默认超时 10 秒，`llm` hook 默认超时 30 秒
- 失败策略由 `on_error` 控制：`skip` 为记录错误并继续，`abort` 为中止当前 hook 链
- `action` 控制流字段：`stop` 中止当前步骤及其所属子管线，`skip` 跳过当前步骤（同级继续，主要用于 `pre_tool_execution`）
- 旧字段 `abort: true` 向后兼容，等价于 `action: "stop"`
- 整条 hook 链有 30 秒总超时，超时后剩余 hook 不再执行

### HookContext 字段（stdin JSON）

| 字段 | 类型 | 说明 |
|------|------|------|
| `event` | string | 当前触发的事件类型（如 `"pre_send_message"`） |
| `messages` | array | 当前对话消息列表（部分事件可读） |
| `system_prompt` | string | 当前系统提示词 |
| `model` | string | 当前使用的模型名称 |
| `user_input` | string | 本轮用户输入文本 |
| `assistant_output` | string | 本轮 AI 回复文本 |
| `tool_name` | string | 当前工具调用的工具名 |
| `tool_arguments` | string | 当前工具调用的参数 JSON |
| `tool_result` | string | 工具执行结果 |
| `tool_error` | string | 工具执行失败原因 |
| `session_id` | string | 当前会话 ID |
| `cwd` | string | 当前工作目录 |

> 各字段按事件类型有选择性地填充，未填充的字段序列化时省略

### HookResult 字段（stdout JSON）

| 字段 | 生效事件 | 说明 |
|------|----------|------|
| `user_input` | PreSendMessage | 替换用户即将发送的消息 |
| `assistant_output` | PostLlmResponse | 替换 AI 最终展示的回复 |
| `messages` | PreLlmRequest, PostMicroCompact, PostAutoCompact | 替换消息列表 |
| `system_prompt` | PreLlmRequest | 替换系统提示词 |
| `tool_arguments` | PreToolExecution | 替换工具调用参数 |
| `tool_result` | PostToolExecution | 替换工具返回结果 |
| `tool_error` | PostToolExecutionFailure | 替换工具错误信息 |
| `inject_messages` | PreLlmRequest | 追加消息到消息列表末尾 |
| `retry_feedback` | Pre*/Stop/PostLlmResponse | 中止并带反馈重试（注入为 user message 重新请求 LLM） |
| `additional_context` | PreLlmRequest, Stop, PreAutoCompact | 纯文本追加到 system_prompt 末尾 |
| `system_message` | 所有事件 | 展示给用户的提示消息（toast） |
| `action` | 大部分事件 | `"stop"` 中止当前步骤及其子管线；`"skip"` 跳过当前步骤（同级继续） |

### HookFilter 条件过滤

所有字段可选，未设置不参与过滤；多字段同时设置取 AND 关系：

| 字段 | 说明 |
|------|------|
| `tool_name` | 工具名精确匹配（仅工具相关事件） |
| `tool_matcher` | 工具名模式匹配，管道分隔（如 `"Write\|Edit\|Bash"`），优先级低于 `tool_name` |
| `model_prefix` | 模型名前缀过滤（如 `"gpt-4"` 匹配 `"gpt-4o"`） |

### LLM Hook

- `type: llm` 的 hook 通过 `prompt` 模板调用当前 LLM，要求返回 HookResult JSON
- `prompt` 支持 `{{variable}}` 模板变量：`{{event}}`、`{{user_input}}`、`{{assistant_output}}`、`{{tool_name}}`、`{{tool_arguments}}`、`{{tool_result}}`、`{{model}}`、`{{cwd}}`
- 可选 `model` 字段覆盖当前活跃模型
- 默认超时 30 秒，默认重试 1 次
- 自动拼接 JSON 格式指令到 prompt 末尾，LLM 只需返回 JSON 对象

### Shell Hook 环境变量

| 环境变量 | 说明 |
|----------|------|
| `JCLI_HOOK_EVENT` | 当前事件名（如 `"pre_send_message"`） |
| `JCLI_CWD` | 当前工作目录 |

### Hook 执行指标

每个 hook 自动记录执行次数、成功次数、失败次数、跳过次数、累计耗时，可在配置界面 Hooks Tab 中查看
