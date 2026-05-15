---
name: 使用技巧
order: 34
---

## 使用技巧

### 交互模式

- 不带参数运行 `j` 进入**交互模式**，支持 Tab 补全和历史建议
- 交互模式下按 `Ctrl+Q` 快速退出（等同于 `exit` 命令或 `Ctrl+D`）

### Shell 命令

- 交互模式下用 `!` 前缀执行 shell 命令（如 `!ls -la`），自动注入别名环境变量
- 交互模式下输入 `!`（不带命令）进入交互式 shell 模式（提示符变为绿色 `shell >`），cd 等状态延续，输入 `exit` 或按 `Ctrl+D` 返回 copilot

### 路径与 URL

- 路径含空格时用引号包裹：`j set app "/Applications/My App.app"`
- URL 会自动识别并归类到 `inner_url`，无需手动指定 section

### CLI 工具与脚本

- CLI 工具（如 rg、fzf）注册后可直接在终端执行并支持管道
- 脚本需要后台运行时，使用 `-w` 标志在新窗口中执行（如 `j deploy -w`）

### Shell Tab 补全

- 启用 shell Tab 补全：`eval "$(j completion zsh)"` 加入 `.zshrc`

### AI 对话

- AI 对话中输入 `/` 唤起斜杠命令面板，快速执行常用操作
- AI 对话中输入 `@` 唤起补全弹窗，引用技能、命令或文件

### 笔记管理

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
