## 概述

Skill 是扩展 AI 能力的专用提示词模块，通过 `LoadSkill` 工具加载。

## Skill 结构

```
~/.jdata/agent/skills/<skill_name>/
├── SKILL.md          # Skill 定义（必需）
├── references/       # 参考文档（AI 按需 Read 读取）
└── scripts/          # 脚本文件（AI 按需 Bash 执行）
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

加载后，AI 会：
1. 获取 skill 的 body 内容作为上下文
2. 列出 references/ 和 scripts/ 目录中的文件路径
3. 按需使用 Read 工具读取参考文档
4. 按需使用 Bash 工具执行脚本

## Skill 来源

| 来源 | 路径 | 优先级 |
|------|------|--------|
| 用户级 | `~/.jdata/agent/skills/` | 低 |
| 项目级 | `.jcli/skills/` | 高（覆盖用户级） |

## 禁用 Skill

在 TUI 配置界面中禁用特定 skill，配置保存在 `~/.jdata/agent/data/agent_config.json`：

```json
{
  "disabled_skills": ["skill-name-1", "skill-name-2"]
}
```
