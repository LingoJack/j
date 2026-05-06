## 概述

Hook 系统允许在 AI 对话生命周期的特定节点自动执行自定义脚本。

## Hook 类型

| Hook | 触发时机 | 用途 |
|------|---------|------|
| `pre_chat` | 对话开始前 | 注入系统提示、设置环境 |
| `post_chat` | 对话结束后 | 清理、通知 |
| `pre_tool` | 工具执行前 | 安全检查、参数修改 |
| `post_tool` | 工具执行后 | 日志、结果处理 |
| `on_error` | 发生错误时 | 错误通知、降级处理 |

## Hook 配置

Hook 在 `.jcli/config.yaml` 中定义：

```yaml
hooks:
  pre_chat:
    - name: load_project_context
      command: "cat AGENT.md 2>/dev/null || echo 'No AGENT.md found'"
      timeout: 5
    - name: check_env
      command: "echo 'Environment ready'"
      timeout: 3
  post_tool:
    - name: log_tool_usage
      command: "echo 'Tool executed at $(date)' >> ~/.jdata/tool.log"
      timeout: 5
```

## 执行方式

### macOS / Linux

Hook 命令通过 `sh -c` 执行：

```bash
sh -c "<hook_command>"
```

支持所有标准 shell 命令，包括管道、重定向、命令替换等。

### Windows

Hook 命令通过 `cmd /c` 执行：

```cmd
cmd /c "<hook_command>"
```

支持 cmd 内置命令和外部程序，管道 `|`、重定向 `>` 等语法可用。

> **注意**：Windows hook 不支持 bash 语法（如 `$(date)`、`2>/dev/null`），需使用对应的 cmd 语法（如 `%DATE%`、`2>NUL`）。

## 超时处理

- 默认超时 10 秒
- 超时后自动终止进程：
  - macOS / Linux：发送 SIGKILL 信号
  - Windows：使用 `taskkill /F /T /PID` 终止
- 超时不会阻断对话流程，仅跳过当前 hook

## 环境变量

Hook 执行时可访问以下环境变量：

| 变量 | 描述 |
|------|------|
| `J_DATA_PATH` | 数据目录路径 |
| `J_<ALIAS>` | 别名对应的应用路径（大写，`-` 转 `_`） |

### macOS / Linux 示例

```yaml
hooks:
  pre_chat:
    - name: open_project
      command: "cat $J_DATA_PATH/agent/data/context.txt"
      timeout: 5
```

### Windows 示例

```yaml
hooks:
  pre_chat:
    - name: open_project
      command: "type %J_DATA_PATH%\agent\data\context.txt"
      timeout: 5
```

## PATH 注入

Hook 目录会自动注入到 PATH 环境变量中：
- macOS / Linux：使用 `:` 分隔符
- Windows：使用 `;` 分隔符

## Hook 脚本文件

除了内联命令，也可以创建独立的脚本文件：

### macOS / Linux

```bash
# ~/.jdata/hooks/pre_chat/my_hook.sh
#!/bin/bash
echo "Loading project context..."
cat AGENT.md 2>/dev/null
```

### Windows

```cmd
@echo off
REM %USERPROFILE%\.jdata\hooks\pre_chat\my_hook.cmd
echo Loading project context...
type AGENT.md 2>NUL
```

## 注意事项

- Hook 执行失败不会阻断对话
- 命令输出会作为系统消息注入到对话上下文中
- 避免在 hook 中执行耗时操作
- 敏感信息不要通过 hook 输出
