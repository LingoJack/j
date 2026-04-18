---
name: Hook & Skill
order: 7
---

## Skill 技能系统

Skill 支持两级目录：

- 用户级：`~/.jdata/agent/skills/`
- 项目级：`.jcli/skills/`（同名时覆盖用户级）

AI 通过 `load_skill` 工具按需加载技能。

**创建 Skill**：

```bash
mkdir -p ~/.jdata/agent/skills/my-skill
cat > ~/.jdata/agent/skills/my-skill/SKILL.md << 'EOF'
---
name: my-skill
description: 技能描述
argument-hint: "[参数说明]"
---

指令正文，$ARGUMENTS 会被替换为参数...
EOF
```

**使用方式**：

| 操作 | 说明 |
|------|------|
| 输入 `@` | 弹出混合选择列表（含 skill: 分类入口） |
| `@skill:` | 直接弹出技能选择列表（支持过滤） |
| `↑↓` 选择 + `Tab/Enter` | 补全技能名称 |
| AI 自动调用 `load_skill` | 从 skills 摘要识别后自动加载 |

> Skill 目录支持 `references/` 子目录存放参考文件，会自动附加到上下文

## Commands 自定义命令

除了 Skill，还支持自定义命令模板：

- 用户级：`~/.jdata/agent/commands/`
- 项目级：`.jcli/commands/`（同名时覆盖用户级）

**创建方式**：

```bash
mkdir -p ~/.jdata/agent/commands/review
cat > ~/.jdata/agent/commands/review/COMMAND.md << 'EOF'
---
name: review
description: 代码审查模板
---

请以 code review 模式检查当前改动，优先找 bug、回归风险和遗漏测试。
EOF
```

**使用方式**：

- 在对话中输入 `@command:` 唤起补全
- 选择后会把命令正文展开到输入内容中
- 配置界面的 `Commands` Tab 可启用/禁用命令

## AGENT.md 项目指令

AGENT.md 是项目级指令系统，让 AI 在每次对话中自动加载项目约定，确保长上下文中始终遵循项目规范。

**创建方式**：

```bash
# 项目级（提交到 git，团队共享）
cat > AGENT.md << 'EOF'
# 项目约定

- 使用 Rust 2024 edition
- 所有公共 API 必须有文档注释
- 错误处理使用 thiserror 而非 anyhow
- 提交信息格式: type(scope): description
EOF

# 项目级（.jcli 目录下，与权限配置同目录）
cat > .jcli/AGENT.md << 'EOF'
# 额外约定
...
EOF

# 个人级（不提交到 git）
cat > AGENT.local.md << 'EOF'
# 个人偏好
...
EOF
```

**加载优先级**（后者覆盖前者）：

| 优先级 | 文件 | 说明 |
|--------|------|------|
| 最低 | `~/.jdata/agent/AGENT.md` | 用户级，所有项目生效 |
| ↓ | `AGENT.md`（从 git root 到 CWD） | 项目级，提交到 git |
| ↓ | `.jcli/AGENT.md`（从 git root 到 CWD） | 项目级，与权限配置同目录 |
| ↓ | `AGENT.local.md`（从 git root 到 CWD） | 个人级，不提交 git |
| 最高 | `.jcli/AGENT.local.md`（从 git root 到 CWD） | 个人级，不提交 git |

> 每个文件上限 200 行 / 25KB。项目级指令自动带有 OVERRIDE 强制力，优先于默认行为。
> 在对话配置界面（Ctrl+E）中选择 "AGENT.md" 字段可编辑用户级指令。

## AI Hook

Hook 允许在对话关键节点注入自定义逻辑。对用户可配置部分，支持三级来源：

1. **用户级**：`~/.jdata/agent/hooks.yaml` — 全局生效
2. **项目级**：`.jcli/hooks.yaml` — 项目目录下生效
3. **Session 级**：通过 `register_hook` 工具由 AI 动态注册 — 仅当前会话

> 运行时实际还存在**内置 hook**，执行顺序是：内置 -> 用户级 -> 项目级 -> Session 级。
> 同一事件按链式执行，前一个 hook 的输出会成为后一个 hook 的输入。

**可用事件**：

| 事件 | 触发时机 |
|------|----------|
| `pre_send_message` | 用户发送消息前 |
| `post_send_message` | 用户发送消息后 |
| `pre_llm_request` | LLM API 请求前 |
| `post_llm_response` | LLM 回复完成后 |
| `pre_tool_execution` | 工具执行前 |
| `post_tool_execution` | 工具执行后 |
| `post_tool_execution_failure` | 工具执行失败后 |
| `stop` | LLM 即将结束回复时 |
| `pre_micro_compact` | `micro_compact` 前 |
| `post_micro_compact` | `micro_compact` 后 |
| `pre_auto_compact` | `auto_compact` 前 |
| `post_auto_compact` | `auto_compact` 后 |
| `session_start` | 会话启动时 |
| `session_end` | 会话退出时 |

**配置示例**（`~/.jdata/agent/hooks.yaml`）：
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

**支持类型**：

- `bash`：默认类型，通过 `sh -c` 执行 `command`
- `llm`：通过 `prompt` 模板调用当前 provider，要求返回 HookResult JSON

**脚本协议**：

- stdin 接收 `HookContext` JSON，stdout 输出 `HookResult` JSON；空字符串或 `{}` 表示无修改
- `bash` hook 默认超时 10 秒，`llm` hook 默认超时 30 秒
- 失败策略由 `on_error` 控制：`skip` 为记录错误并继续，`abort` 为中止当前 hook 链
- `action` 控制流字段：`stop` 中止当前步骤及其所属子管线，`skip` 跳过当前步骤（同级继续，主要用于 `pre_tool_execution`）
- 旧字段 `abort: true` 向后兼容，等价于 `action: "stop"`
- 整条 hook 链有 30 秒总超时，超时后剩余 hook 不再执行

**HookContext 字段（stdin JSON）**：

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

**HookResult 字段（stdout JSON）**：

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

**各事件可读/可写字段一览**：

| 事件 | 可读字段 | 可写字段 |
|------|----------|----------|
| `pre_send_message` | `user_input`, `messages` | `user_input`, `action=stop`, `retry_feedback` |
| `post_send_message` | `user_input`, `messages` | 仅通知，返回值被忽略 |
| `pre_llm_request` | `messages`, `system_prompt`, `model` | `messages`, `system_prompt`, `inject_messages`, `additional_context`, `action=stop`, `retry_feedback` |
| `post_llm_response` | `assistant_output`, `messages`, `model` | `assistant_output`, `action=stop`, `retry_feedback`, `system_message` |
| `pre_tool_execution` | `tool_name`, `tool_arguments` | `tool_arguments`, `action=skip` |
| `post_tool_execution` | `tool_name`, `tool_result` | `tool_result` |
| `post_tool_execution_failure` | `tool_name`, `tool_error` | `tool_error`, `additional_context` |
| `stop` | `user_input`, `messages`, `system_prompt`, `model` | `retry_feedback`, `additional_context`, `action=stop` |
| `pre_micro_compact` | `messages`, `model` | `action=stop` |
| `post_micro_compact` | `messages` | `messages` |
| `pre_auto_compact` | `messages`, `system_prompt`, `model` | `additional_context`, `action=stop` |
| `post_auto_compact` | `messages` | `messages` |
| `session_start` | `messages` | 仅通知，返回值被忽略 |
| `session_end` | `messages` | 仅通知，返回值被忽略 |

**HookFilter 条件过滤**：

所有字段可选，未设置不参与过滤；多字段同时设置取 AND 关系：

| 字段 | 说明 |
|------|------|
| `tool_name` | 工具名精确匹配（仅工具相关事件） |
| `tool_matcher` | 工具名模式匹配，管道分隔（如 `"Write\|Edit\|Bash"`），优先级低于 `tool_name` |
| `model_prefix` | 模型名前缀过滤（如 `"gpt-4"` 匹配 `"gpt-4o"`） |

**LLM Hook**：

- `type: llm` 的 hook 通过 `prompt` 模板调用当前 LLM，要求返回 HookResult JSON
- `prompt` 支持 `{{variable}}` 模板变量：`{{event}}`、`{{user_input}}`、`{{assistant_output}}`、`{{tool_name}}`、`{{tool_arguments}}`、`{{tool_result}}`、`{{model}}`、`{{cwd}}`
- 可选 `model` 字段覆盖当前活跃模型
- 默认超时 30 秒，默认重试 1 次
- 自动拼接 JSON 格式指令到 prompt 末尾，LLM 只需返回 JSON 对象

**Shell Hook 环境变量**：

| 环境变量 | 说明 |
|----------|------|
| `JCLI_HOOK_EVENT` | 当前事件名（如 `"pre_send_message"`） |
| `JCLI_CWD` | 当前工作目录 |

**Hook 执行指标**：

每个 hook 自动记录执行次数、成功次数、失败次数、跳过次数、累计耗时，可在配置界面 Hooks Tab 中查看
