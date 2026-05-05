# 新增 PowerShell Tool（Windows 专属 Shell Tool）

## 目标

- Shell Tool (`Bash`) 仅在 Unix/macOS 上可用，Windows 上不注册
- 新增 `PowerShell` Tool，仅在 Windows 上注册，使用 `powershell.exe -Command` 执行命令
- 两者共用 `BackgroundManager`、安全检查等基础设施
- PowerShell Tool 的 description 和提示信息针对 PowerShell 语法和 Windows 环境定制

## 改动清单

### 1. 新增 `src/command/chat/tools/powershell.rs`

基于 `shell.rs` 的架构，创建独立的 PowerShell tool：

- **Tool 名称**: `PowerShell`
- **执行方式**: `powershell.exe -NoProfile -Command <command>`
- **参数结构体**: `PowerShellParams`（与 `ShellParams` 相同字段：command, description, cwd, timeout, run_in_background）
- **description**: 针对 PowerShell 语法重写（去掉 bash 相关提示，加入 PowerShell 特定用法如 `;` 分隔、`$env:VAR` 环境变量等）
- **同步执行 `execute_sync`**: 与 shell.rs 逻辑相同，但用 `powershell.exe` 替代 `bash -c`
- **后台执行 `execute_background`**: 同理，用 `powershell.exe` 替代 `bash -c`
- **整个文件用 `#[cfg(windows)]` 包裹**

### 2. 修改 `src/command/chat/tools/shell.rs`

- 在 ShellTool 上添加 `is_available()` 方法，返回 `!cfg!(windows)` — Windows 上 Shell Tool 不可用
- 或直接在注册时不注册（更干净）

### 3. 修改 `src/command/chat/tools.rs`

- 添加 `#[cfg(windows)] mod powershell;`
- 在 `tool_names` 中添加 `#[cfg(windows)] POWERSHELL`
- 在 `ToolRegistry::new()` 中：
  - ShellTool 注册加 `#[cfg(unix)]` 条件
  - PowerShellTool 注册加 `#[cfg(windows)]` 条件

### 4. 修改 `src/command/chat/constants.rs`

- PowerShell 的默认超时等常量可复用 Shell 的，无需新增

### 5. 修改 `src/util/shell_safety.rs`（可选，后续扩展）

- 当前安全检查基于 Unix 命令，PowerShell 有不同的危险命令模式
- 初始版本可先复用现有检查，后续再添加 PowerShell 专属规则

## 文件影响

| 文件 | 操作 |
|------|------|
| `src/command/chat/tools/powershell.rs` | **新增** |
| `src/command/chat/tools.rs` | 修改（mod 声明 + 注册 + tool_names） |
| `src/command/chat/tools/shell.rs` | 修改（无需改动，注册层面控制） |

## 不改动的部分

- `BackgroundManager` — 共用，无需改动
- `shell_safety.rs` — 初始版本复用，后续扩展
- `definition.rs` — Tool trait 无需改动
