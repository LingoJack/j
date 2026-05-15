---
name: Hook 示例
order: 5
---

## 目录布局

Hook 采用目录布局，每个 hook 是一个目录：

```
~/.jdata/agent/hooks/<hook_name>/
├── HOOK.yaml      # hook 定义（也支持 HOOK.yml；两者共存时取 HOOK.yaml）
└── script.sh      # 可选脚本，command 里直接用文件名引用（hook 目录在 PATH 中）
```

## 基础配置示例

### HOOK.yaml 示例 (bash)

```yaml
# ~/.jdata/agent/hooks/inject_time/HOOK.yaml
events: [pre_send_message]
type: bash
command: inject_time.sh
timeout: 5
on_error: skip
```

### HOOK.yaml 示例 (llm)

```yaml
# ~/.jdata/agent/hooks/safety_check/HOOK.yaml
events: [pre_tool_execution]
type: llm
prompt: |
  审查工具调用是否安全：工具={{tool_name}} 参数={{tool_arguments}}
  如果不安全，返回 {"action":"skip"}，否则返回 {}
filter:
  tool_matcher: "Bash|Shell"
```

### 多事件绑定

```yaml
# 同一个 hook 绑定多个事件
events: [pre_send_message, post_send_message]
type: bash
command: log.sh
```

## 示例 1：LLM 纠查官（推荐，type=llm）

在 AI 回复后自动检查敏感信息，发现问题则要求 LLM 重新生成。

```yaml
# ~/.jdata/agent/hooks/censor/HOOK.yaml
events: [post_llm_response]
type: llm
prompt: |
  检查以下 AI 回复是否包含敏感信息（密码、密钥、token）：
  {{assistant_output}}
  如果包含敏感信息，返回 action=stop + retry_feedback 说明问题。
  如果没有问题，返回空 JSON {}。
timeout: 30
retry: 1
on_error: skip
```

## 示例 2：LLM 消息审查（pre_send_message）

在用户消息发送前进行合规检查。

```yaml
# ~/.jdata/agent/hooks/msg_review/HOOK.yaml
events: [pre_send_message]
type: llm
prompt: |
  审查用户消息是否合规：{{user_input}}
  如有违规返回 action=stop 和 retry_feedback。
model: gpt-4o-mini
timeout: 15
retry: 1
```

## 示例 3：Bash 脚本 - 给消息加时间戳（pre_send_message）

使用 Shell 脚本在用户消息前添加时间戳。

```bash
#!/bin/bash
# ~/.jdata/agent/hooks/inject_time/inject_time.sh
input=$(cat)
msg=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('user_input',''))")
echo "{\"user_input\": \"[$(date '+%H:%M')] $msg\"}"
```

```yaml
# ~/.jdata/agent/hooks/inject_time/HOOK.yaml
events: [pre_send_message]
type: bash
command: inject_time.sh
```

## 示例 4：Bash 脚本 - 跳过危险命令（pre_tool_execution）

拦截可能造成破坏的 Shell 命令。

```bash
#!/bin/bash
# ~/.jdata/agent/hooks/rm_guard/guard.sh
input=$(cat)
tool=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('tool_name',''))")
args=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('tool_arguments',''))")
if [ "$tool" = "Bash" ] && echo "$args" | grep -q "rm -rf"; then
  echo '{"action": "skip"}'
else
  echo '{}'
fi
```

```yaml
# ~/.jdata/agent/hooks/rm_guard/HOOK.yaml
events: [pre_tool_execution]
command: guard.sh
```

## 示例 5：带过滤器的工具审查

使用 filter 字段限制 hook 仅对特定工具生效。

```yaml
# ~/.jdata/agent/hooks/tool_review/HOOK.yaml
events: [pre_tool_execution]
type: llm
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
- 只有 session 级 hook 可通过 RegisterHook 工具管理；用户级/项目级需手动编辑 YAML 配置文件
- 移除 hook 时，使用 list 输出中的 session_idx 作为 index 参数

## Hook 执行指标

每个 hook 自动记录执行次数、成功次数、失败次数、跳过次数、累计耗时，可在配置界面 Hooks Tab 中查看。