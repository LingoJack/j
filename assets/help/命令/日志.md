---
name: 日志设置
order: 30
---

## 日志模式

通过 `j log mode` 命令切换日志输出的详细程度：

```bash
j log mode verbose    # 详细模式，输出完整日志信息
j log mode concise    # 精简模式，仅输出关键信息
```

### 模式说明

| 模式 | 说明 |
|------|------|
| `verbose` | 详细输出，适合调试和排查问题 |
| `concise` | 精简输出，适合日常使用 |
