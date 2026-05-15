---
name: Hook 事件
order: 2
---

## 事件详解

### 一、会话级事件（每个会话触发一次）

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `session_start` | 会话启动时 | `messages` | 仅通知，返回值被忽略 |
| `session_end` | 会话退出时 | `messages` | 仅通知，返回值被忽略 |

### 二、消息发送阶段

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `pre_send_message` | 用户发送消息前 | `user_input`, `messages` | `user_input`, `action=stop`, `retry_feedback` |
| `post_send_message` | 用户发送消息后 | `user_input`, `messages` | 仅通知，返回值被忽略 |

### 三、LLM 请求/回复阶段

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `pre_llm_request` | LLM API 请求前 | `messages`, `system_prompt`, `model` | `messages`, `system_prompt`, `inject_messages`, `additional_context`, `action=stop`, `retry_feedback` |
| `post_llm_response` | LLM 回复完成后 | `assistant_output`, `messages`, `model` | `assistant_output`, `action=stop`, `retry_feedback`, `system_message` |

### 四、工具执行阶段

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `pre_tool_execution` | 工具执行前 | `tool_name`, `tool_arguments` | `tool_arguments`, `action=skip` |
| `post_tool_execution` | 工具执行成功后 | `tool_name`, `tool_result` | `tool_result` |
| `post_tool_execution_failure` | 工具执行失败后 | `tool_name`, `tool_error` | `tool_error`, `additional_context` |

### 五、回复结束阶段

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `stop` | LLM 即将结束回复时（无更多工具调用） | `user_input`, `messages`, `system_prompt`, `model` | `retry_feedback`, `additional_context`, `action=stop` |

### 六、上下文压缩阶段

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `pre_micro_compact` | 轮次级压缩前 | `messages`, `model` | `action=stop` |
| `post_micro_compact` | 轮次级压缩后 | `messages` | `messages` |
| `pre_auto_compact` | 全量压缩前 | `messages`, `system_prompt`, `model` | `additional_context`, `action=stop` |
| `post_auto_compact` | 全量压缩后 | `messages` | `messages` |

## 压缩 Hook 说明

两层压缩各有独立的 Pre/Post hook，构成一个 compact 子管线：

1. `pre_micro_compact` → micro_compact → `post_micro_compact`
2. `pre_auto_compact` → auto_compact → `post_auto_compact`