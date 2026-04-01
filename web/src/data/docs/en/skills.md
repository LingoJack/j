## Overview

Skills are specialized prompt modules that extend AI capabilities, loaded via the `LoadSkill` tool.

## Skill Structure

```
~/.jdata/agent/skills/<skill_name>/
├── SKILL.md          # Skill definition (required)
├── references/       # Reference documents
└── scripts/          # Script files
```

## Creating a Skill

```markdown
# SKILL.md
---
name: code-review
description: Code review best practices
argument-hint: file path  # optional, hints the argument user passes
---

You are a code reviewer. Analyze code for:
- Code quality
- Performance issues
- Security vulnerabilities
- Best practices
```

## Using Skills

AI loads skills via the `LoadSkill` tool:

```
Load the code-review skill
```

## Skill Sources

- **User level**: `~/.jdata/agent/skills/`
- **Project level**: `.jcli/skills/` (project level overrides user level when names conflict)
