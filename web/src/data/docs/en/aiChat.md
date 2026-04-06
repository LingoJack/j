## Overview

AI chat system with multi-model support, context references, and autonomous agent execution.

## Start Chat

```bash
j chat              # Enter TUI chat interface
j chat "Hello"      # Quick question with response printed
j chat -c           # Continue previous session
j chat --session <id>  # Restore specific session
```

## Shortcuts

| Shortcut | Action |
|----------|--------|
| `Enter` | Send message |
| `Esc` | Cancel response/Exit |
| `Ctrl+Y` | Copy last AI reply |
| `Ctrl+B` | Message browse mode |
| `Ctrl+G` | Open log windows |
| `Ctrl+O` | Toggle tool details |
| `Ctrl+E` | Open config panel |
| `F1` or `?` | Show help |

## Slash Commands

Type `/` in the input box to trigger slash commands:

| Command | Action |
|---------|--------|
| `/copy` | Copy last AI reply |
| `/log` | Open log windows |
| `/browse` | Browse message history |
| `/config` | Open config panel |
| `/model` | Switch model |
| `/archive` | Archive current conversation |

## Context References

Type `@` in the input box to trigger completion:

```
@skill:<name>       # Reference a skill
@command:<name>     # Reference a custom command
@file:<path>        # Reference file content (supports images)
```

## Agent Capabilities

AI chat has built-in Agent capabilities for autonomous multi-step task execution:

- **Autonomous Reasoning**: AI plans and executes multi-step tasks
- **Tool Integration**: Automatically uses available tools (Read, Write, Bash, etc.)
- **Task Management**: Task and Todo tools manage complex tasks
- **Plan Mode**: Explore codebase before making a plan

### Plan Mode

For complex tasks, enter plan mode to explore the codebase first:

```
Analyze the project architecture and design a refactoring plan

# AI will:
1. Enter plan mode (read-only tools available)
2. Explore codebase structure
3. Generate detailed plan
4. Submit plan for user confirmation
```

### Tool Permission Configuration

Create `.jcli/permissions.yaml` in project root:

```yaml
permissions:
  allow_all: false
  allow:
    - Read
    - Grep
    - Glob
  deny:
    - Bash
    - Write
```

## Remote Control

```bash
j chat --remote     # Enable remote control (scan QR code with phone)
j chat --remote --port 9390  # Custom port
```
