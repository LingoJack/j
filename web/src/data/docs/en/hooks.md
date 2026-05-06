## Overview

The Hook system allows custom scripts to execute automatically at specific points during the AI chat lifecycle.

## Hook Types

| Hook | Trigger | Use Case |
|------|---------|----------|
| `pre_chat` | Before chat starts | Inject system prompts, set up environment |
| `post_chat` | After chat ends | Cleanup, notifications |
| `pre_tool` | Before tool execution | Security checks, parameter modification |
| `post_tool` | After tool execution | Logging, result processing |
| `on_error` | On error | Error notification, graceful degradation |

## Hook Configuration

Hooks are defined in `.jcli/config.yaml`:

```yaml
hooks:
  pre_chat:
    - name: load_project_context
      command: "cat AGENT.md 2>/dev/null || echo 'No AGENT.md found'"
      timeout: 5
    - name: check_env
      command: "echo 'Environment ready'"
      timeout: 3
  post_tool:
    - name: log_tool_usage
      command: "echo 'Tool executed at $(date)' >> ~/.jdata/tool.log"
      timeout: 5
```

## Execution

### macOS / Linux

Hook commands execute via `sh -c`:

```bash
sh -c "<hook_command>"
```

Supports all standard shell commands including pipes, redirection, and command substitution.

### Windows

Hook commands execute via `cmd /c`:

```cmd
cmd /c "<hook_command>"
```

Supports cmd built-in commands and external programs. Pipe `|`, redirection `>`, etc. are available.

> **Note**: Windows hooks do not support bash syntax (e.g., `$(date)`, `2>/dev/null`). Use cmd equivalents (e.g., `%DATE%`, `2>NUL`).

## Timeout Handling

- Default timeout: 10 seconds
- Process is killed on timeout:
  - macOS / Linux: SIGKILL signal
  - Windows: `taskkill /F /T /PID`
- Timeout does not block the chat flow; the current hook is simply skipped

## Environment Variables

Hooks have access to the following environment variables:

| Variable | Description |
|----------|-------------|
| `J_DATA_PATH` | Data directory path |
| `J_<ALIAS>` | App path for the alias (uppercase, `-` → `_`) |

### macOS / Linux Example

```yaml
hooks:
  pre_chat:
    - name: open_project
      command: "cat $J_DATA_PATH/agent/data/context.txt"
      timeout: 5
```

### Windows Example

```yaml
hooks:
  pre_chat:
    - name: open_project
      command: "type %J_DATA_PATH%\agent\data\context.txt"
      timeout: 5
```

## PATH Injection

The hook directory is automatically prepended to PATH:
- macOS / Linux: uses `:` separator
- Windows: uses `;` separator

## Hook Script Files

In addition to inline commands, you can create standalone script files:

### macOS / Linux

```bash
# ~/.jdata/hooks/pre_chat/my_hook.sh
#!/bin/bash
echo "Loading project context..."
cat AGENT.md 2>/dev/null
```

### Windows

```cmd
@echo off
REM %USERPROFILE%\.jdata\hooks\pre_chat\my_hook.cmd
echo Loading project context...
type AGENT.md 2>NUL
```

## Notes

- Hook failures do not block the conversation
- Command output is injected as a system message into the chat context
- Avoid long-running operations in hooks
- Do not output sensitive information through hooks
