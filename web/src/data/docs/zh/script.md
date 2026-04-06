## 概述

脚本系统允许定义和执行预设的 Shell 命令序列，支持参数化和条件执行。

核心特性：
- **预定义脚本**：将常用命令序列保存为可复用脚本
- **参数化执行**：脚本支持占位符，运行时传入参数
- **多命令串联**：支持命令链式执行
- **环境隔离**：每个脚本在独立 Shell 中执行

## 基本用法

### 执行脚本

```bash
j script <name>           # 执行指定脚本
j script <name> <args...> # 带参数执行
```

### 管理脚本

脚本存放在 `~/.jdata/scripts/` 目录，每个脚本为一个 Markdown 文件：

```
~/.jdata/scripts/
├── deploy.md
├── build.md
└── test.md
```

## 脚本格式

脚本使用 Markdown 格式，支持 frontmatter 配置：

```markdown
---
name: deploy
description: 部署到生产环境
---

#!/bin/bash
set -e

echo "Building..."
npm run build

echo "Deploying..."
rsync -avz dist/ user@server:/var/www/
```

### Frontmatter 字段

| 字段 | 类型 | 必填 | 描述 |
|------|------|------|------|
| name | string | 是 | 脚本名称 |
| description | string | 否 | 脚本描述 |

## 参数化

脚本支持 `{{.param}}` 占位符：

```markdown
---
name: greet
description: 问候脚本
---

#!/bin/bash
name="{{.name}}"
echo "Hello, $name!"
```

执行时传入参数：

```bash
j script greet --name World
```

## 使用场景

- 项目构建和部署
- 代码格式化和检查
- 数据库备份
- 环境初始化
- 定时任务封装
