## Overview

Hooks allow running custom scripts on specific events, managed via the `RegisterHook` tool.

## Hook Events

| Event | When Triggered |
|-------|----------------|
| `pre_send_message` | Before sending user message |
| `post_send_message` | After sending user message |
| `pre_llm_request` | Before LLM request |
| `post_llm_response` | After LLM response |
| `pre_tool_execution` | Before tool execution |
| `post_tool_execution` | After tool execution |
| `session_start` | When session starts |
| `session_end` | When session ends |

## Register Hooks

Manage session-level hooks via `RegisterHook` tool:

```
# View hook protocol documentation
RegisterHook action="help"

# List registered hooks
RegisterHook action="list"

# Register a hook
RegisterHook event="pre_send_message" command="echo 'Sending...'"

# Remove a hook
RegisterHook action="remove" event="pre_send_message" index=0
```

## Configuration Files

Hooks can also be managed via config files:

```yaml
# User level: ~/.jdata/agent/hooks.yaml
# Project level: .jcli/hooks.yaml

hooks:
  - event: pre_send_message
    command: "echo 'Sending...'"
    timeout: 10
```

## Hook Script Protocol

Scripts receive JSON via stdin, return modifications via stdout:

```bash
#!/bin/bash
input=$(cat)
# Modify user_input
echo '{"user_input": "Modified message"}'
```
