## Overview

Skills are specialized prompt modules that extend AI capabilities, loaded via the `LoadSkill` tool.

## Skill Structure

```
~/.jdata/agent/skills/<skill_name>/
├── SKILL.md          # Skill definition (required)
├── references/       # Reference documents (AI reads on demand via Read tool)
└── scripts/          # Script files (AI executes on demand via Bash tool)
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

After loading, AI will:
1. Get the skill's body content as context
2. List file paths in references/ and scripts/ directories
3. Read reference documents on demand via Read tool
4. Execute scripts on demand via Bash tool

## Skill Sources

| Source | Path | Priority |
|--------|------|----------|
| User level | `~/.jdata/agent/skills/` | Low |
| Project level | `.jcli/skills/` | High (overrides user level) |

## Disabling Skills

Disable specific skills via the TUI configuration interface. Settings are saved in `~/.jdata/agent/data/agent_config.json`:

```json
{
  "disabled_skills": ["skill-name-1", "skill-name-2"]
}
```
