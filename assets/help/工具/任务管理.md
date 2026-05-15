---
name: 任务管理
order: 3
parent: tools
---

## 任务管理（`Task` 工具）

共享任务系统，所有 Agent 均可操作，持久化到 `.jcli/tasks/`。

### 操作表格

| 操作 | 参数 | 说明 |
|------|------|------|
| `action: "create"` | `title`（必需）、`description`、`blockedBy`、`taskDocPaths` | 创建待办任务 |
| `action: "get"` | `taskId` | 获取任务详情 |
| `action: "list"` | `ready: true`（可选，仅显示无阻塞的待办任务） | 列出所有任务 |
| `action: "update"` | `taskId` + `status`/`title`/`description`/`owner`/`addBlockedBy` | 更新任务 |

### 任务状态流转

```
pending --> in_progress --> completed
                         \-> deleted
```

### 任务依赖（blockedBy）

- `blockedBy`：任务依赖 DAG，前置任务完成后自动清理引用
- `list` 时传入 `ready: true` 可仅列出无阻塞的待办任务

### 任务持久化

- 任务 ID 自增
- 持久化为 `.jcli/tasks/task_{id}.json`

### 其他字段

- `owner`：负责该任务的 Agent 名称
- `description`：任务的详细描述
- `taskDocPaths`：关联的文档路径列表
