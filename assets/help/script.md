---
name: 脚本
order: 4
---

## 脚本 & 倒计时

| 命令 | 说明 |
|------|------|
| `j script <name> "<content>"` | 创建脚本并注册为别名（保存到 `~/.jdata/scripts/`） |
| `j script <name>` | 打开 TUI 编辑器创建或编辑脚本 |
| `j <script> [args...]` | 在当前终端执行脚本 |
| `j <script> -w [args...]` | 在**新终端窗口**中执行脚本 |
| `j time countdown <duration>` | 启动倒计时（支持 `30s` / `5m` / `1h`，不带单位默认按分钟） |

> `-w` 或 `--new-window` 标志可让脚本在新终端窗口中执行，用于需要后台运行的场景

### 脚本环境变量注入

执行脚本时，所有已注册的别名路径会自动注入为环境变量，命名规则为 `J_<别名大写>`（`-` 转为 `_`）：

**macOS / Linux**（`.sh` 脚本）：
```bash
#!/bin/bash
# 已注册: chrome -> /Applications/Google Chrome.app
# 已注册: my-tool -> /usr/local/bin/my-tool

open -a "$J_CHROME" https://example.com
"$J_MY_TOOL" --version
```

**Windows**（`.cmd` 脚本）：
```cmd
@echo off
REM 已注册: notepad -> C:\Windows\notepad.exe
REM 已注册: vscode -> C:\Users\%USERNAME%\AppData\Local\Programs\Microsoft VS Code\Code.exe

start "" "%J_VSCODE%" .\src
"%J_NOTEPAD%" readme.txt
```

> 覆盖 section: `path`、`inner_url`、`outer_url`、`script`
> 路径含空格时，脚本中必须用双引号包裹变量：`"$J_CHROME"` / `"%J_VSCODE%"`
> Windows 脚本扩展名为 `.cmd`，macOS/Linux 为 `.sh`
