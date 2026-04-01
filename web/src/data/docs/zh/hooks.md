## 概述

Hook 允许在特定事件时运行自定义脚本。

## Hook 事件

| 事件 | 触发时机 |
|------|----------|
| `pre_send_message` | 发送消息给 AI 之前 |
| `post_llm_response` | 收到 AI 响应之后 |
| `pre_tool_execution` | 工具执行之前 |
| `post_tool_execution` | 工具执行之后 |
| `session_start` | 会话开始时 |
| `session_end` | 会话结束时 |

## 注册 Hook

```bash
# 注册 hook
j hook register pre_send_message "echo '发送消息...'"

# 列出 hook
j hook list

# 移除 hook
j hook remove pre_send_message 0
```

## Hook 脚本

Hook 脚本通过 stdin 接收 JSON 数据：

```json
{
  "event": "pre_send_message",
  "data": {
    "message": "用户消息"
  }
}
```

脚本应通过 stdout 输出 JSON 来修改数据。
