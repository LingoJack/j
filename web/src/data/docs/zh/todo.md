## 概述

待办管理系统提供轻量级的任务跟踪能力，支持状态流转和优先级管理。

核心特性：
- **快速添加**：一行命令添加待办事项
- **状态管理**：pending、in_progress、completed 状态流转
- **列表查看**：按状态筛选和排序
- **数据持久化**：自动保存到本地文件

## 基本用法

### 查看待办

```bash
j todo              # 查看所有待办
j todo -s pending   # 按状态筛选
```

### 添加待办

```bash
j todo add "完成文档编写"
j todo add "修复Bug" -p high  # 高优先级
```

### 更新状态

```bash
j todo start <id>     # 标记为进行中
j todo done <id>      # 标记为已完成
j todo cancel <id>    # 取消待办
```

### 删除待办

```bash
j todo rm <id>        # 删除指定待办
j todo clear          # 清空已完成
```

## 待办状态

| 状态 | 说明 |
|------|------|
| pending | 待处理 |
| in_progress | 进行中 |
| completed | 已完成 |

## 优先级

| 级别 | 标识 |
|------|------|
| 低 | low |
| 中 | medium（默认） |
| 高 | high |

## 数据存储

待办数据存储在 `~/.jdata/todos.json`：

```json
[
  {
    "id": 1,
    "content": "完成文档编写",
    "status": "pending",
    "priority": "high",
    "created_at": "2024-01-15T10:00:00Z"
  }
]
```

## 使用场景

- 日常任务管理
- 项目进度跟踪
- 个人备忘录
- 团队协作任务分配
