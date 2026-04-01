## 概述

Agent 模式支持自主多步推理和工具调用。

## 启动

```bash
j agent
```

## 功能特性

- **自主推理**：AI 规划并执行多步任务
- **工具集成**：自动使用可用工具
- **任务管理**：分解复杂请求

## 示例任务

```bash
# 代码分析
分析代码库并提出改进建议

# 文件操作
查找代码中的所有 TODO 注释并生成摘要

# 研究
研究 React 状态管理的最佳实践并生成报告
```

## 工具权限配置

配置 agent 可以使用的工具：

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
