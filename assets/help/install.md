---
name: 安装 & 设置
order: 10
---

## 安装 & 更新

### macOS / Linux 一键安装（推荐）
```bash
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh
```

指定版本安装：
```bash
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh -s -- v1.0.0
```

### Windows 一键安装（推荐）
```powershell
irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex
```

指定版本安装：
```powershell
$v="v1.0.0"; irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex
```

> Windows 安装位置: `%LOCALAPPDATA%\j-cli\j.exe`，自动添加到用户 PATH

### 从源码安装
```bash
cargo install j-cli
# CDP 版本：cargo install j-cli --features browser_cdp
```

### 更新
```bash
j update               # 自动检测安装来源并更新
j update --check       # 仅检查是否有新版本
```

## 卸载

### macOS / Linux
```bash
# 使用安装脚本卸载（推荐）
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh -s -- --uninstall

# 或通过 cargo 卸载
cargo uninstall j-cli

# （可选）删除数据目录
rm -rf ~/.jdata
```

### Windows
```powershell
# 使用安装脚本卸载
powershell -ExecutionPolicy Bypass -Command "irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex" -Uninstall

# 或直接删除
Remove-Item "$env:LOCALAPPDATA\j-cli" -Recurse -Force

# （可选）删除数据目录
Remove-Item "$env:USERPROFILE\.jdata" -Recurse -Force
```

> 卸载命令只会删除二进制文件，用户数据（`~/.jdata/`）会保留。

## 系统设置

| 命令 | 说明 |
|------|------|
| `j log mode <verbose/concise>` | 设置日志模式 |
| `j config <section> <field> <val>` | 直接修改配置字段 |
| `j clear` | 清屏 |
| `j version` / `j v` | 版本信息 |
| `j help` / `j h` | 打开多标签帮助 TUI |
| `j exit` / `j q` / `j quit` | 退出（交互模式，或按 `Ctrl+Q` / `Ctrl+D`） |
| `j completion [shell]` | 生成 shell 补全脚本（支持 `zsh` / `bash`，默认 `zsh`） |

### 帮助界面快捷键

| 按键 | 功能 |
|------|------|
| `←` / `→` / `h` / `l` | 切换帮助 Tab |
| `Tab` / `Shift+Tab` | 切换到下一个 / 上一个 Tab |
| `1`-`0` | 直接跳到指定 Tab |
| `↑` / `↓` / `j` / `k` | 滚动内容 |
| `PageUp` / `PageDown` | 快速滚动 |
| `Home` / `End` | 跳到顶部 / 底部 |
| `q` / `Esc` / `Ctrl+C` | 退出帮助 |

## 使用技巧

- 不带参数运行 `j` 进入**交互模式**，支持 Tab 补全和历史建议
- 交互模式下按 `Ctrl+Q` 快速退出（等同于 `exit` 命令或 `Ctrl+D`）
- 交互模式下用 `!` 前缀执行 shell 命令（如 `!ls -la`），自动注入别名环境变量
- 交互模式下输入 `!`（不带命令）进入交互式 shell 模式（提示符变为绿色 `shell >`），cd 等状态延续，输入 `exit` 或按 `Ctrl+D` 返回 copilot
- 路径含空格时用引号包裹：`j set app "/Applications/My App.app"`
- URL 会自动识别并归类到 `inner_url`，无需手动指定 section
- CLI 工具（如 rg、fzf）注册后可直接在终端执行并支持管道
- 脚本需要后台运行时，使用 `-w` 标志在新窗口中执行（如 `j deploy -w`）
- 启用 shell Tab 补全：`eval "$(j completion zsh)"` 加入 `.zshrc`
- AI 对话中输入 `/` 唤起斜杠命令面板，快速执行常用操作
- AI 对话中输入 `@` 唤起补全弹窗，引用技能、命令或文件
- 使用 `j md` 管理笔记，支持子目录、Markdown 编辑和实时预览

### 平台差异说明

| 功能 | macOS / Linux | Windows |
|------|---------------|---------|
| AI Shell 工具 | Bash | PowerShell |
| 默认脚本 | `.sh` + bash shebang | `.cmd` |
| 自动更新 | `.tar.gz` 解压 | `.zip` 解压 |
| 数据目录 | `~/.jdata/` | `%USERPROFILE%\.jdata\` |
| 安装位置 | `/usr/local/bin/j` | `%LOCALAPPDATA%\j-cli\j.exe` |
| Computer Use | 支持 | 不支持 |
| j-indicator | 菜单栏指示灯 | 不支持 |
