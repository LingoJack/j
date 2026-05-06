## 概述

脚本系统，通过 `concat` 命令创建和管理可执行脚本。

## 基本用法

### 创建脚本

```bash
j concat <name>              # 打开 TUI 编辑器编写脚本
j concat <name> "<content>"  # 直接创建脚本
```

### 编辑脚本

```bash
j concat <name>              # 如果脚本已存在，进入编辑模式
```

### 运行脚本

```bash
j <name>           # 直接通过别名运行
j <name> <args...> # 带参数运行
```

### 删除脚本

```bash
j rm <name>        # 删除别名（同时删除脚本文件）
```

## 脚本存储

脚本按平台存储在不同目录：

| 平台 | 存储路径 |
|------|---------|
| macOS / Linux | `~/.jdata/scripts/` |
| Windows | `%USERPROFILE%\.jdata\scripts\` |

```
# macOS / Linux
~/.jdata/scripts/
├── deploy.sh
├── build.sh
└── test.sh

# Windows
%USERPROFILE%\.jdata\scripts\
├── deploy.cmd
├── build.cmd
└── test.cmd
```

脚本创建后自动注册为别名，可直接通过 `j <name>` 执行。

## 平台差异

| 功能 | macOS / Linux | Windows |
|------|---------------|---------|
| 脚本扩展名 | `.sh` | `.cmd` |
| 默认 shebang | `#!/bin/bash` | 无 |
| 执行 shell | Bash | cmd.exe |
| PATH 分隔符 | `:` | `;` |

## 示例

### macOS / Linux

```bash
# 创建部署脚本
j concat deploy

# 在编辑器中输入：
#!/bin/bash
set -e
npm run build
rsync -avz dist/ user@server:/var/www/

# 运行脚本
j deploy
```

### Windows

```powershell
# 创建部署脚本
j concat deploy

# 在编辑器中输入：
@echo off
npm run build
xcopy dist\ \\server\www\ /E /I /Y

# 运行脚本
j deploy
```

## 环境变量注入

执行脚本时，所有已注册的别名路径会自动注入为环境变量，命名规则为 `J_<别名大写>`（`-` 转为 `_`）。

### macOS / Linux（.sh 脚本）

```bash
#!/bin/bash
# 已注册: chrome -> /Applications/Google Chrome.app
# 已注册: my-tool -> /usr/local/bin/my-tool

open -a "$J_CHROME" https://example.com
"$J_MY_TOOL" --version
```

### Windows（.cmd 脚本）

```cmd
@echo off
REM 已注册: notepad -> C:\Windows\notepad.exe
REM 已注册: vscode -> C:\Users\%USERNAME%\AppData\Local\Programs\Microsoft VS Code\Code.exe

start "" "%J_VSCODE%" .\src
"%J_NOTEPAD%" readme.txt
```

> 路径含空格时，脚本中必须用双引号包裹变量：`"$J_CHROME"` / `"%J_VSCODE%"`
