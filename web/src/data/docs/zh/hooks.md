## 概述

Hook 允许在特定事件时运行自定义脚本，通过 `RegisterHook` 工具管理。

## Hook 事件

| 事件 | 触发时机 |
|------|----------|
| `pre_send_message` | 用户消息发送前 |
| `post_send_message` | 用户消息发送后 |
| `pre_llm_request` | LLM 请求前 |
| `post_llm_response` | LLM 回复后 |
| `pre_tool_execution` | 工具执行前 |
| `post_tool_execution` | 工具执行后 |
| `session_start` | 会话开始 |
| `session_end` | 会话结束 |

## 注册 Hook

通过 `RegisterHook` 工具管理 session 级 hook：

```
# 查看 hook 协议文档
RegisterHook action="help"

# 列出已注册的 hook
RegisterHook action="list"

# 注册 hook
RegisterHook event="pre_send_message" command="echo '发送消息...'"

# 移除 hook
RegisterHook action="remove" event="pre_send_message" index=0
```

## 配置文件

Hook 也可以通过配置文件管理：

```yaml
# 用户级: ~/.jdata/agent/hooks.yaml
# 项目级: .jcli/hooks.yaml

hooks:
  - event: pre_send_message
    command: "echo '发送消息...'"
    timeout: 10
```

## Hook 脚本协议

脚本通过 stdin 接收 JSON，通过 stdout 返回修改：

```bash
#!/bin/bash
input=$(cat)
# 修改 user_input
echo '{"user_input": "修改后的消息"}'
```
