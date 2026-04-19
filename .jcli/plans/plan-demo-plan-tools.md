# 演示计划：Task / TodoWrite / Plan 工具对比

## 概述

本文档演示了三个工具的使用场景和区别。

## 工具对比

| 特性 | TodoWrite | Task | Plan (EnterPlanMode) |
|---|---|---|---|
| 用途 | 会话级轻量待办 | 多步骤任务管理 | 方案设计与审批 |
| 持久性 | 会话内有效 | 会话内有效 | 文件持久化保存 |
| 状态管理 | pending/in_progress/completed/cancelled | pending/in_progress/completed/deleted | 提交审批 |
| 依赖关系 | 无 | 支持 blockedBy | 无 |
| 文件输出 | 无 | 可关联 taskDocPaths | 生成 plan 文件 |
| 适用场景 | 简单步骤跟踪 | 复杂多步骤任务 | 需要用户确认的方案设计 |

## 使用建议

1. **TodoWrite**：用于跟踪当前会话中的简单步骤清单，比如"先读文件、再搜索、最后编辑"
2. **Task**：用于管理有依赖关系的复杂任务，可以拆分、分配、追踪
3. **Plan**：在动手编码前，先设计方案，让用户审阅确认后再执行
