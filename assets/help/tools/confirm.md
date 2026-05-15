---
name: 工具确认快捷键
order: 5
parent: tools
---

## 工具确认快捷键

当工具需要用户确认时（如 Bash、Write、Edit 等标记为 "Yes" 的工具），可使用以下快捷键：

| 按键 | 功能 |
|------|------|
| `Y` / `Enter` | 执行工具 |
| `N` / `Esc` | 拒绝执行 |

### 需确认的工具

以下工具在执行前需要用户确认：

- `Bash` - 执行 shell 命令
- `PowerShell` - 执行 PowerShell 命令
- `Write` - 写入文件
- `Edit` - 编辑文件
- `ComputerUse` - 控制 macOS 桌面
- `RegisterHook` - 注册 hook
- `EnterWorktree` / `ExitWorktree` - 创建/退出 git worktree

可通过 `.jcli/permissions.yaml` 配置自动放行规则，跳过确认步骤。详见 [权限配置](permissions.md)。
