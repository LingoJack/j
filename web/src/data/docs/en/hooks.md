## Overview

Hooks allow custom scripts to run at specific events.

## Hook Events

| Event | When it runs |
|-------|--------------|
| `pre_send_message` | Before sending message to AI |
| `post_llm_response` | After receiving AI response |
| `pre_tool_execution` | Before tool execution |
| `post_tool_execution` | After tool execution |
| `session_start` | When session starts |
| `session_end` | When session ends |

## Registering Hooks

```bash
# Register a hook
j hook register pre_send_message "echo 'Sending message...'"

# List hooks
j hook list

# Remove hook
j hook remove pre_send_message 0
```

## Hook Scripts

Hook scripts receive JSON via stdin:

```json
{
  "event": "pre_send_message",
  "data": {
    "message": "user message"
  }
}
```

Scripts should output JSON to stdout to modify the data.
