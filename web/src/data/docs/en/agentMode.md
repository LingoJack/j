## Overview

Agent mode is an enhanced AI chat mode with autonomous multi-step reasoning and tool usage.

## Start

```bash
j chat              # Enter TUI chat
```

In the conversation, AI automatically uses tools to execute multi-step operations as needed.

## Features

- **Autonomous Reasoning**: AI plans and executes multi-step tasks
- **Tool Integration**: Automatically uses available tools (Read, Write, Bash, etc.)
- **Task Management**: Task and Todo tools manage complex tasks

## Example Tasks

```
Analyze the codebase and suggest improvements

Find all TODO comments in the code and generate a summary

Research React state management best practices and generate a report
```

## Tool Permission Configuration

Configure which tools the AI can use:

```yaml
# ~/.jdata/agent/data/agent_config.yaml
permissions:
  allow:
    - Read
    - Grep
    - Glob
    - WebFetch
  deny:
    - Bash
    - Write
```
