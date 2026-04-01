## 概述

Skill 是扩展 AI 能力的专用提示词。

## Skill 结构

```
~/.jdata/agent/skills/<skill_name>/
├── skill.md         # Skill 定义
├── assets/          # 支持文件
└── examples/        # 使用示例
```

## 创建 Skill

```markdown
# skill.md
---
name: code-review
description: 代码审查最佳实践
trigger: 代码审查
---

你是一个代码审查者。分析代码的：
- 代码质量
- 性能问题
- 安全漏洞
- 最佳实践
```

## 使用 Skill

```bash
# 在 AI 对话中
> 代码审查这个文件 @file:src/main.rs
```

## 内置 Skill

- `code-review`：代码分析
- `test-gen`：生成测试
- `doc-gen`：生成文档
- `refactor`：重构建议
