## 概述

别名系统允许为常用命令和应用程序创建简短别名，提高命令行效率。

核心特性：
- **命令别名**：将长命令简化为短别名
- **应用别名**：快速打开常用应用和网址
- **分组管理**：按项目或类别组织别名
- **动态扩展**：支持运行时添加和删除

## 基本用法

### 执行别名

```bash
j <alias>            # 执行别名命令
j <alias> <args...>  # 带参数执行
```

### 管理别名

```bash
j alias              # 列出所有别名
j alias add <name> <command>  # 添加别名
j alias rm <name>    # 删除别名
```

## 别名类型

### 命令别名

将常用命令简化：

```bash
# 添加别名
j alias add gs "git status"
j alias add gp "git push"

# 使用别名
j gs
j gp origin main
```

### 应用别名

快速打开应用或网址：

```bash
# 打开应用
j alias add chrome "open -a 'Google Chrome'"
j alias add vscode "open -a 'Visual Studio Code'"

# 打开网址
j alias add gh "open https://github.com"

# 使用
j chrome
j gh
```

## 别名文件

别名单独存放在 `~/.jdata/aliases/` 目录：

```
~/.jdata/aliases/
├── git.json      # Git 相关别名
├── apps.json     # 应用别名
└── work.json     # 工作相关别名
```

### 别名文件格式

```json
[
  {
    "name": "gs",
    "command": "git status",
    "description": "查看 Git 状态"
  },
  {
    "name": "chrome",
    "command": "open -a 'Google Chrome'",
    "description": "打开 Chrome 浏览器"
  }
]
```

## 参数化别名

别名支持 `$1`、`$2` 等参数占位符：

```bash
# 添加带参数的别名
j alias add find-file "find . -name '$1' -type f"

# 使用
j find-file "*.rs"
```

## 使用场景

- Git 命令简化
- 项目快速切换
- 应用快速启动
- 常用网址书签
- SSH 连接快捷方式
