---
name: Hook 协议
order: 4
---

## HookContext 字段（stdin JSON）

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
| `cwd` | string | 用户当前工作目录（JCLI_CWD 环境变量，与 hook 执行目录 JCLI_HOOK_DIR 可能不同） |

> 各字段按事件类型有选择性地填充，未填充的字段序列化时省略

## HookResult 字段（stdout JSON）

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

## HookResult JSON 示例

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

### action

控制流动作，字符串 `"stop"` 或 `"skip"`：

- `"stop"`：中止当前步骤及其所属子管线
- `"skip"`：跳过当前步骤，同级步骤继续（仅 `pre_tool_execution` 中使用）

### retry_feedback

与 stop 配合使用。在 stop/pre_send_message/post_llm_response 中，stop+retry_feedback 会中止当前操作并将反馈注入为新消息，LLM 带反馈重新生成。这是实现"宪法 AI/纠查官"的核心机制。

### additional_context

追加文本到 system_prompt 末尾，不占消息位。适用于注入规则、约束等。

### system_message

在 UI 上以 toast/提示形式展示给用户，不影响 LLM 输入。

## action 语义

| 事件 | action 语义 |
|------|-------------|
| `pre_send_message` | `action=stop` 中止当前操作 |
| `pre_llm_request` | `action=stop` 中止当前操作 |
| `stop` | `action=stop` 中止当前操作 |
| `post_llm_response` | `action=stop` 中止当前操作 |
| `pre_tool_execution` | `action=skip` 跳过该工具调用（其他工具继续执行） |
| `pre_micro_compact` | `action=stop` 中止整个 compact 子管线 |
| `pre_auto_compact` | `action=stop` 中止 auto_compact |

## HookFilter 条件过滤

所有字段可选，未设置不参与过滤；多字段同时设置取 AND 关系：

| 字段 | 说明 |
|------|------|
| `tool_name` | 工具名精确匹配（仅工具相关事件） |
| `tool_matcher` | 工具名模式匹配，管道分隔（如 `"Write\|Edit\|Bash"`），优先级低于 `tool_name` |
| `model_prefix` | 模型名前缀过滤（如 `"gpt-4"` 匹配 `"gpt-4o"`） |