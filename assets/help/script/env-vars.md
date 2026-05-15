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
