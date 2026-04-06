## Overview

Commands are reusable prompt snippets that help you quickly invoke preset prompts.

## Slash Commands

Type `/` in the input box to trigger slash commands:

| Command | Description |
|---------|-------------|
| `/copy` | Copy the last AI response |
| `/log` | Open the log window |
| `/browse` | Browse message history |
| `/config` | Open the configuration panel |
| `/model` | Switch model |
| `/archive` | Archive current conversation |

## Custom Commands

### Usage

Reference a command with `@command:<name>` in the input box:

```
@command:review Please review this code
```

### Creating Commands

#### Directory Locations

- **User-level**: `~/.jdata/agent/commands/` - Available across all projects
- **Project-level**: `.jcli/commands/` - Only available in current project (higher priority)

#### File Format

Each command is a Markdown file with YAML frontmatter and prompt body:

```markdown
---
name: review
description: Code review prompt
---
Please perform a comprehensive review of the following code, focusing on:
- Code quality
- Potential issues
- Improvement suggestions
```

#### Two Organization Methods

**Method 1: Single File**

Create `.md` files directly in the commands directory:

```
commands/
  review.md
  test.md
```

**Method 2: Directory Structure**

Create a directory with a `COMMAND.md` inside (suitable for complex commands with resource files):

```
commands/
  review/
    COMMAND.md
    checklist.txt
```

### Example

Create a `plan.md`:

```markdown
---
name: plan
description: Enter PLAN mode
---
Please enter plan mode to plan the task
```

Usage:

```
@command:plan
```

### Managing Commands

Press `Ctrl+E` in the TUI to open the configuration panel, switch to the Commands tab to enable or disable commands.
