## 概述

j-cli 的所有数据存储在统一的用户数据目录中，支持通过环境变量自定义路径。

## 数据目录路径

| 平台 | 默认路径 | 环境变量覆盖 |
|------|---------|-------------|
| macOS / Linux | `~/.jdata/` | `J_DATA_PATH=/custom/path` |
| Windows | `%USERPROFILE%\.jdata\` | `$env:J_DATA_PATH="C:\custom\path"` |

### 环境变量覆盖

```bash
# macOS / Linux
export J_DATA_PATH=/custom/path
j chat  # 数据将存储在 /custom/path/

# Windows (PowerShell)
$env:J_DATA_PATH="C:\custom\path"
j chat  # 数据将存储在 C:\custom\path\
```

## 目录结构

```
~/.jdata/                          # macOS / Linux
%USERPROFILE%\.jdata\              # Windows
├── config.yaml                    # 主配置文件
├── alias.yaml                     # 别名定义
├── report/                        # 日报/周报
│   ├── report.md                  # 日报文件
│   ├── todo.json                  # 待办数据
│   └── settings.json              # 周报元数据
├── scripts/                       # 用户脚本
│   ├── deploy.sh                  # macOS / Linux
│   └── deploy.cmd                 # Windows
├── agent/                         # AI Agent 数据
│   ├── data/                      # Agent 运行时数据
│   │   ├── messages/              # 对话历史
│   │   └── agent_config.json      # Agent 配置
│   └── skills/                    # 用户自定义 Skill
│       └── <skill_name>/
│           └── SKILL.md
└── hooks/                         # Hook 脚本
    └── pre_chat/
        └── my_hook.sh             # macOS / Linux
        └── my_hook.cmd            # Windows
```

## 配置文件说明

### config.yaml

主配置文件，存储全局设置：

```yaml
# API 配置
api_key: "your-api-key"
base_url: "https://api.openai.com/v1"
model: "gpt-4"

# 报表配置
report_file_path: "~/.jdata/report/report.md"

# 浏览器配置
settings:
  browser_headless: true
```

### alias.yaml

别名定义，存储应用和 URL 别名：

```yaml
# macOS / Linux
chrome:
  path: "/Applications/Google Chrome.app"
  note: "browser"

# Windows
notepad:
  path: "C:\\Windows\\notepad.exe"
  note: "editor"

# URL 别名
github:
  path: "https://github.com"
  type: "inner_url"
```

## 数据迁移

### 备份

```bash
# macOS / Linux
cp -r ~/.jdata ~/.jdata.backup

# Windows
Copy-Item "$env:USERPROFILE\.jdata" "$env:USERPROFILE\.jdata.backup" -Recurse
```

### 恢复

```bash
# macOS / Linux
cp -r ~/.jdata.backup ~/.jdata

# Windows
Copy-Item "$env:USERPROFILE\.jdata.backup" "$env:USERPROFILE\.jdata" -Recurse
```

### 跨平台迁移

数据目录结构在所有平台上一致，可以直接复制迁移：

1. 备份源平台数据目录
2. 复制到目标平台对应位置
3. 调整脚本文件扩展名（`.sh` → `.cmd`）
4. 更新 alias.yaml 中的路径格式
