## Start AI Chat

```bash
j chat              # Enter TUI chat interface
j chat "Hello"      # Quick question and print response
j chat -c           # Continue last session
j chat --session <id>  # Resume specific session
```

## Shortcuts

| Shortcut | Action |
|----------|--------|
| `Enter` | Send message |
| `Esc` | Cancel response/Exit |
| `Ctrl+T` | Switch model |
| `Ctrl+L` | Archive conversation |
| `Ctrl+Y` | Copy last AI reply |
| `Ctrl+B` | Message browse mode |
| `Ctrl+E` | Open config panel |
| `F1` or `?` | Show help |

## Slash Commands

Type `/` in the input box to trigger slash commands:

| Command | Action |
|---------|--------|
| `/copy` | Copy last AI reply |
| `/log` | Open log window |
| `/browse` | Browse message history |
| `/config` | Open config panel |
| `/model` | Switch model |
| `/archive` | Archive current conversation |

## Context References

Type `@` in input to trigger completion:

```
@skill:<name>       # Reference skill
@command:<name>     # Reference custom command
@file:<path>        # Reference file content (supports images)
```

## Agent Capabilities

AI chat has built-in Agent capabilities for autonomous multi-step task execution:

- **Autonomous Reasoning**: AI plans and executes multi-step tasks
- **Tool Integration**: Automatically uses available tools (Read, Write, Bash, etc.)
- **Task Management**: Task and Todo tools manage complex tasks
- **Plan Mode**: Explore codebase before making plan
- **Sub-agent**: Spawn sub-agents for complex tasks in parallel

### Plan Mode

For complex tasks, enter plan mode to explore codebase first:

```
# Enter plan mode
Analyze this project's architecture and design a refactoring plan

# AI will:
1. Enter plan mode (read-only tools available)
2. Explore codebase structure
3. Generate detailed plan
4. Submit plan for user approval
5. Execute after approval
```

### Sub-agent

Agent tool allows spawning sub-agents for complex tasks:

- **No recursion**: Sub-agent cannot call Agent tool
- **Max rounds**: 30 tool call limit
- **Execution mode**: Foreground (blocking) or background (async)

```
# Example: Spawn sub-agent to search and organize code
Search all files containing 'TODO' and organize by directory
```

### Tool Permission Configuration

Create `.jcli/permissions.yaml` in project root to configure tool permissions:

```yaml
permissions:
  allow_all: false   # Set to true to skip all confirmations
  
  allow:             # Skip confirmation if matched
    - Read
    - Grep
    - Glob
  
  deny:              # Takes priority over allow, blocks execution
    - Bash
    - Write
```

> See [AI Tools](tools) for details.

## Remote Control

```bash
j chat --remote     # Enable remote control (scan QR with phone)
j chat --remote --port 9390  # Specify port
```

## Multi-Model Support

Supports OpenAI, Claude, Gemini, Ollama and more. Use `Ctrl+E` to open config panel.
