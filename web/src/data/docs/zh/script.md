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

脚本统一存储在 `~/.jdata/scripts/` 目录：

```
~/.jdata/scripts/
├── deploy.sh
├── build.sh
└── test.sh
```

脚本创建后自动注册为别名，可直接通过 `j <name>` 执行。

## 示例

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
