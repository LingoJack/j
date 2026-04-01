## 概述

Agent 模式是 AI 对话的增强模式，支持自主多步推理和工具调用。

## 启动

```bash
j chat              # 进入 TUI 对话
```

在对话中，AI 会根据任务需要自动使用工具执行多步操作。

## 功能特性

- **自主推理**：AI 规划并执行多步任务
- **工具集成**：自动使用可用工具（Read、Write、Bash 等）
- **任务管理**：Task 和 Todo 工具管理复杂任务

## 示例任务

```
分析代码库并提出改进建议

查找代码中的所有 TODO 注释并生成摘要

研究 React 状态管理的最佳实践并生成报告
```

## 工具权限配置

配置 AI 可以使用的工具：

```yaml
# ~/.jdata/agent/data/agent_config.yaml
permissions:
  allow:
    - Read
    - Grep
    - Glob
    - WebFetch
  deny:
    - Bash
    - Write
```
