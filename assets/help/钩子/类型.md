---
name: Hook 类型
order: 3
---

## bash（默认）

通过 `sh -c` 子进程执行 Shell 命令。

### 参数

| 参数 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `command` | 是 | - | Shell 命令或脚本文件名 |
| `timeout` | 否 | 10s | 执行超时时间 |
| `on_error` | 否 | skip | 失败策略：skip（记录日志继续）或 stop（中止链） |
| `retry` | 否 | 0 | 失败重试次数 |

### Bash Hook 脚本协议

- **执行方式**：`sh -c "<command>"`
- **工作目录**：用户当前目录
- **PATH**：目录布局下，hook 目录前置到 PATH，脚本可直接用文件名调用（如 `script.sh`）
- **环境变量**：
  - `JCLI_HOOK_EVENT`：事件名
  - `JCLI_CWD`：用户当前目录
  - `JCLI_HOOK_DIR`：hook 目录（目录布局下有值）
- **stdin**：HookContext JSON
- **stdout**：HookResult JSON（只返回要修改的字段，空/`{}` 表示无修改）
- **exit 0** = 成功，非零 = 失败（按 on_error 策略处理）

## llm

通过 prompt 模板调用 LLM，LLM 返回 HookResult JSON。

### 参数

| 参数 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `prompt` | 是 | - | 提示词模板，支持 `{{variable}}` 模板变量 |
| `model` | 否 | 当前模型 | 覆盖当前模型名称 |
| `timeout` | 否 | 30s | 执行超时时间 |
| `retry` | 否 | 1 | LLM 返回非法 JSON 或网络失败时重试次数 |
| `on_error` | 否 | skip | 失败策略 |

### LLM Hook 协议

- 系统自动在 prompt 末尾追加 JSON 格式指令，LLM 需返回 HookResult JSON
- 使用当前活跃 provider 的 API（或通过 model 参数覆盖模型名）
- JSON 提取逻辑：从 LLM 输出中找第一个 `{` 到最后一个 `}` 之间的内容
- 解析失败 → 视为 Err → 按 retry 重试

### 可用模板变量

| 变量 | 说明 |
|------|------|
| `{{event}}` | 当前事件类型 |
| `{{user_input}}` | 用户输入文本 |
| `{{assistant_output}}` | AI 回复文本 |
| `{{tool_name}}` | 工具名称 |
| `{{tool_arguments}}` | 工具参数 JSON |
| `{{tool_result}}` | 工具执行结果 |
| `{{model}}` | 当前模型名称 |
| `{{cwd}}` | 当前工作目录 |