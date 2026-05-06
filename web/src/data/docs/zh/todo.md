## 概述

待办管理系统，支持任务状态流转和 TUI 交互界面。

## 基本用法

### 进入 TUI 界面

```bash
j todo              # 打开待办管理 TUI
```

### 命令行操作

```bash
j todo list              # 列出所有待办
j todo list --done       # 只列出已完成
j todo list --undone     # 只列出未完成
j todo add "完成文档"     # 快速添加待办
```

## TUI 操作

| 快捷键 | 功能 |
|--------|------|
| `j/k` | 上下移动 |
| `Enter` | 切换完成状态 |
| `a` | 添加新待办 |
| `e` | 编辑当前项 |
| `d` | 删除当前项 |
| `r` | 写入日报 |
| `Tab` | 切换筛选（全部/未完成/已完成） |
| `?` | 显示帮助 |
| `q/Esc` | 退出 |

## 待办状态

| 状态 | 显示 |
|------|------|
| 未完成 | `[ ]` |
| 已完成 | `[x]` |

## 数据存储

待办数据存储在数据目录下：

| 平台 | 路径 |
|------|------|
| macOS / Linux | `~/.jdata/report/todo.json` |
| Windows | `%USERPROFILE%\.jdata\report\todo.json` |
