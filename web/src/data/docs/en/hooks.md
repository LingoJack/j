## Overview

Hooks allow running custom scripts on specific events, managed via the `RegisterHook` tool or config files.

## Hook Events

| Event | When Triggered | Readable Fields | Writable Fields |
|-------|----------------|-----------------|-----------------|
| `pre_send_message` | Before sending user message | user_input, messages | user_input, abort |
| `post_send_message` | After sending user message | user_input, messages | Notification only |
| `pre_llm_request` | Before LLM request | messages, system_prompt, model | messages, system_prompt, inject_messages, abort |
| `post_llm_response` | After LLM response | assistant_output, messages | assistant_output |
| `pre_tool_execution` | Before tool execution | tool_name, tool_arguments | tool_arguments, abort |
| `post_tool_execution` | After tool execution | tool_name, tool_result | tool_result |
| `session_start` | Session starts | messages | Notification only |
| `session_end` | Session ends | messages | Notification only |

## Using RegisterHook Tool

Manage session-level hooks via tool in AI chat:

```
# View protocol documentation
RegisterHook action="help"

# List registered hooks
RegisterHook action="list"

# Register a hook
RegisterHook event="pre_send_message" command="echo '{\"user_input\": \"[modified]\"}'" timeout=10

# Register a hook (abort on failure)
RegisterHook event="pre_tool_execution" command="./guard.sh" on_error="abort"

# Remove a hook (use session_idx from list output)
RegisterHook action="remove" event="pre_send_message" index=0
```

## Configuration Files

Manage persistent hooks via YAML files:

```yaml
# User level: ~/.jdata/agent/hooks.yaml
# Project level: .jcli/hooks.yaml

pre_send_message:
  - command: "echo '{\"user_input\": \"[timestamp] \" + $(cat | jq -r .user_input)}'"
    timeout: 5

pre_tool_execution:
  - command: |
      input=$(cat)
      tool=$(echo "$input" | jq -r .tool_name)
      if [ "$tool" = "Bash" ]; then
        echo '{"abort": true}'
      else
        echo '{}'
      fi
    timeout: 10
    on_error: abort  # Abort the chain if this hook fails (default: skip)
```

## Script Protocol

### Execution Environment

| Item | Description |
|------|-------------|
| Execution | `sh -c "<command>"` |
| Working Directory | User's current directory |
| Environment Variables | `JCLI_HOOK_EVENT` (event name), `JCLI_CWD` (current directory) |

### stdin/stdout

| Item | Description |
|------|-------------|
| stdin | HookContext JSON |
| stdout | HookResult JSON (empty or `{}` means no modification) |
| exit 0 | Success |
| exit non-zero | Handled per `on_error` strategy (default `skip`: log and continue; `abort`: stop chain) |

### stdin HookContext Example

```json
{
  "event": "pre_send_message",
  "cwd": "/path/to/project",
  "user_input": "User input text",
  "messages": [{"role": "user", "content": "..."}],
  "system_prompt": "System prompt",
  "model": "gpt-4o",
  "assistant_output": "AI response text",
  "tool_name": "Bash",
  "tool_arguments": "{\"command\": \"ls\"}",
  "tool_result": "Tool execution result"
}
```

### stdout HookResult Example

```json
{
  "user_input": "Modified user message",
  "assistant_output": "Modified AI response",
  "messages": [{"role": "user", "content": "..."}],
  "system_prompt": "Modified prompt",
  "tool_arguments": "Modified tool arguments",
  "tool_result": "Modified tool result",
  "inject_messages": [{"role": "user", "content": "Injected message"}],
  "abort": false
}
```

## Script Examples

### Add Timestamp to User Message

```bash
#!/bin/bash
input=$(cat)
msg=$(echo "$input" | jq -r .user_input)
echo "{\"user_input\": \"[$(date '+%H:%M')] $msg\"}"
```

### Block Dangerous Commands

```bash
#!/bin/bash
input=$(cat)
tool=$(echo "$input" | jq -r .tool_name)
args=$(echo "$input" | jq -r .tool_arguments)

if [ "$tool" = "Bash" ] && echo "$args" | grep -q "rm -rf"; then
  echo '{"abort": true}'
else
  echo '{}'
fi
```

### Notification-only Hook

```bash
#!/bin/bash
cat > /dev/null  # Must read stdin to avoid SIGPIPE
```

## Three-Level Hook Priority

Hooks exist at three levels, executed in order: User → Project → Session

| Level | Config Location | Lifecycle |
|-------|-----------------|-----------|
| User | `~/.jdata/agent/hooks.yaml` | Persistent |
| Project | `.jcli/hooks.yaml` | Persistent within project |
| Session | RegisterHook tool | Current session only |

During chain execution, the previous hook's output updates the context for the next hook. Any `abort` immediately stops the entire chain.

## Notes

- Create script files with Write/Bash tools first, then register with RegisterHook
- Scripts must read from stdin (at least `cat > /dev/null`) to avoid SIGPIPE
- Default timeout is 10 seconds; scripts are killed on timeout
- `on_error` defaults to `skip` (log and continue); set to `abort` to stop the hook chain on failure
- Only session-level hooks can be managed via tool; user/project levels require manual config editing
- When removing hooks, use the `session_idx` from list output as the `index` parameter
