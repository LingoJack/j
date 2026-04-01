## 概述

Skill 是扩展 AI 能力的专用提示词模块，通过 `LoadSkill` 工具加载。

## Skill 结构

```
~/.jdata/agent/skills/<skill_name>/
├── SKILL.md          # Skill 定义（必需）
├── references/       # 参考文档
└── scripts/          # 脚本文件
```

## 创建 Skill

```markdown
# SKILL.md
---
name: code-review
description: 代码审查最佳实践
argument-hint: 文件路径  # 可选，提示用户传入的参数
---

你是一个代码审查者。分析代码的：
- 代码质量
- 性能问题
- 安全漏洞
- 最佳实践
```

## 使用 Skill

AI 通过 `LoadSkill` 工具加载 skill：

```
加载 code-review skill
```

## Skill 来源

- **用户级**：`~/.jdata/agent/skills/`
- **项目级**：`.jcli/skills/`（同名时项目级覆盖用户级）
