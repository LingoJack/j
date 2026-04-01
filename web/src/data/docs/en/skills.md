## Overview

Skills are specialized prompts that extend AI capabilities.

## Skill Structure

```
~/.jdata/agent/skills/<skill_name>/
├── skill.md         # Skill definition
├── assets/          # Supporting files
└── examples/        # Example usage
```

## Creating Skills

```markdown
# skill.md
---
name: code-review
description: Review code for best practices
trigger: code review
---

You are a code reviewer. Analyze code for:
- Code quality
- Performance issues
- Security vulnerabilities
- Best practices
```

## Using Skills

```bash
# In AI chat
> code review this file @file:src/main.rs
```

## Built-in Skills

- `code-review`: Code analysis
- `test-gen`: Generate tests
- `doc-gen`: Generate documentation
- `refactor`: Refactoring suggestions
