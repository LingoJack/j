## Overview

Agent mode enables autonomous multi-step reasoning with tool calling.

## Activation

```bash
j ai
```

## Features

- **Autonomous reasoning**: AI plans and executes multi-step tasks
- **Tool integration**: Uses available tools automatically
- **Task management**: Breaks down complex requests

## Example Tasks

```bash
# Code analysis
Analyze the codebase and suggest improvements

# File operations
Find all TODO comments in the code and create a summary

# Research
Research the best practices for React state management and create a report
```

## Tool Permissions

Configure which tools the agent can use:

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
