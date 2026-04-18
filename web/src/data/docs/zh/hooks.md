## 概述

Hook 允许在特定事件时运行自定义脚本，通过 `RegisterHook` 工具或配置文件管理。

## Hook 事件

| 事件 | 触发时机 | 可读字段 | 可写字段 |
|------|----------|----------|----------|
| `pre_send_message` | 用户消息发送前 | user_input, messages | user_input, abort |
| `post_send_message` | 用户消息发送后 | user_input, messages | 仅通知，返回值忽略 |
| `pre_llm_request` | LLM 请求前 | messages, system_prompt, model | messages, system_prompt, inject_messages, abort |
| `post_llm_response` | LLM 回复后 | assistant_output, messages | assistant_output |
| `pre_tool_execution` | 工具执行前 | tool_name, tool_arguments | tool_arguments, abort |
| `post_tool_execution` | 工具执行后 | tool_name, tool_result | tool_result |
| `session_start` | 会话开始 | messages | 仅通知 |
| `session_end` | 会话结束 | messages | 仅通知 |

## 使用 RegisterHook 工具

在 AI 对话中通过工具管理 session 级 hook：

```
# 查看协议文档
RegisterHook action="help"

# 列出已注册的 hook
RegisterHook action="list"

# 注册 hook
RegisterHook event="pre_send_message" command="echo '{\"user_input\": \"[modified]\"}'" timeout=10

# 注册 hook（失败时中止链）
RegisterHook event="pre_tool_execution" command="./guard.sh" on_error="abort"

# 移除 hook（使用 list 中的 session_idx）
RegisterHook action="remove" event="pre_send_message" index=0
```

## 配置文件

通过 YAML 文件管理持久化 hook：

```yaml
# 用户级: ~/.jdata/agent/hooks.yaml
# 项目级: .jcli/hooks.yaml

pre_send_message:
  - command: "echo '{\"user_input\": \"[timestamp] \" + $(cat | jq -r .user_input)}'"
    timeout: 5

pre_tool_execution:
  - command: |
      input=$(cat)
      tool=$(echo "$input" | jq -r .tool_name)
      if [ "$tool" = "Bash" ]; then
        echo '{"abort": true}'
      else
        echo '{}'
      fi
    timeout: 10
    on_error: abort  # 此 hook 失败时中止操作（默认为 skip）
    filter:          # 条件过滤（可选）
      tool_name: Bash        # 仅对 Bash 工具生效
      # model_prefix: gpt-4  # 仅对 gpt-4 系列模型生效
```

## 脚本协议

### 执行环境

| 项目 | 说明 |
|------|------|
| 执行方式 | `sh -c "<command>"` |
| 工作目录 | 用户当前目录 |
| 环境变量 | `JCLI_HOOK_EVENT`（事件名）、`JCLI_CWD`（当前目录） |

### stdin/stdout

| 项目 | 说明 |
|------|------|
| stdin | HookContext JSON |
| stdout | HookResult JSON（空或 `{}` 表示无修改） |
| exit 0 | 成功 |
| exit 非 0 | 按 `on_error` 策略处理（默认 `skip`：记录日志继续；`abort`：中止链） |

### stdin HookContext 示例

```json
{
  "event": "pre_send_message",
  "cwd": "/path/to/project",
  "session_id": "20260418_143022_abc",
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

### stdout HookResult 示例

```json
{
  "user_input": "修改后的用户消息",
  "assistant_output": "修改后的 AI 回复",
  "messages": [{"role": "user", "content": "..."}],
  "system_prompt": "修改后的提示词",
  "tool_arguments": "修改后的工具参数",
  "tool_result": "修改后的工具结果",
  "inject_messages": [{"role": "user", "content": "注入消息"}],
  "abort": false
}
```

## 脚本示例

### 给用户消息加时间戳

```bash
#!/bin/bash
input=$(cat)
msg=$(echo "$input" | jq -r .user_input)
echo "{\"user_input\": \"[$(date '+%H:%M')] $msg\"}"
```

### 拦截危险命令

```bash
#!/bin/bash
input=$(cat)
tool=$(echo "$input" | jq -r .tool_name)
args=$(echo "$input" | jq -r .tool_arguments)

if [ "$tool" = "Bash" ] && echo "$args" | grep -q "rm -rf"; then
  echo '{"abort": true}'
else
  echo '{}'
fi
```

### 纯通知 hook

```bash
#!/bin/bash
cat > /dev/null  # 必须读取 stdin，否则可能 SIGPIPE
```

## 三级 Hook 优先级

Hook 分三个级别，执行顺序：用户级 → 项目级 → Session 级

| 级别 | 配置位置 | 生命周期 |
|------|----------|----------|
| 用户级 | `~/.jdata/agent/hooks.yaml` | 持久化 |
| 项目级 | `.jcli/hooks.yaml` | 项目内持久化 |
| Session 级 | RegisterHook 工具 | 仅当前会话 |

链式执行时，前一个 hook 的输出会更新到 context 中，成为下一个 hook 的输入。任何 `abort` 立即中止整条链。

## 注意事项

- 先用 Write/Bash 工具创建脚本文件，再用 RegisterHook 注册
- 脚本必须从 stdin 读取（至少 `cat > /dev/null`），否则可能 SIGPIPE
- timeout 默认 10 秒，超时后脚本被 kill
- `on_error` 默认 `skip`（记录日志继续），设为 `abort` 则脚本失败时中止整条 hook 链
- 只有 session 级 hook 可通过工具管理；用户级/项目级需手动编辑配置文件
- 移除 hook 时，使用 `list` 输出中的 `session_idx` 作为 `index` 参数
