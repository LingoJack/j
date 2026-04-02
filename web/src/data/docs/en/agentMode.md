## Overview

Agent mode is an enhanced AI chat mode with autonomous multi-step reasoning and tool usage.

## Start

```bash
j chat              # Enter TUI chat
```

In the conversation, AI automatically uses tools to execute multi-step operations as needed.

> **Auto-apply tools**: Create `.jcli/permissions.yaml` in project root with `allow_all: true` to skip all tool confirmations. See "Tool Permission Configuration" below.

## Features

- **Autonomous Reasoning**: AI plans and executes multi-step tasks
- **Tool Integration**: Automatically uses available tools (Read, Write, Bash, etc.)
- **Task Management**: Task and Todo tools manage complex tasks
- **Plan Mode**: EnterPlanMode allows exploring codebase before making plan
- **Sub-agent**: Agent tool can spawn sub-agents for complex tasks in parallel

## Sub-agent (Agent Tool)

Agent tool allows the main agent to spawn sub-agents for complex tasks:

- **No recursion**: Sub-agent cannot call Agent tool
- **Max rounds**: 30 tool call limit
- **Execution mode**: Foreground (blocking) or background (async)
- **Permission control**: Follows deny/allow rules

```
# Example: Spawn sub-agent to search and organize code
Search all files containing 'TODO' and organize by directory
```

## Plan Mode

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

## Example Tasks

```
Analyze the codebase and suggest improvements

Find all TODO comments in the code and generate a summary

Research React state management best practices and generate a report
```

## Tool Permission Configuration

Configure which tools the AI can use (in `.jcli/permissions.yaml`):

```yaml
permissions:
  allow_all: false   # Set to true to skip all confirmations
  
  allow:             # Skip confirmation if matched
    - Read
    - Grep
    - Glob
    - WebFetch
  
  deny:              # Takes priority over allow, blocks execution
    - Bash
    - Write
```
