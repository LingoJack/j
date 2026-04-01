## Available Tools

| Tool | Description |
|------|-------------|
| `Read` | Read file contents |
| `Write` | Write to files |
| `Edit` | Edit files with string replacement |
| `Glob` | Find files by pattern |
| `Grep` | Search file contents |
| `Bash` | Execute shell commands |
| `WebFetch` | Fetch web page content |
| `WebSearch` | Search the web |
| `Ask` | Ask user for input |
| `TaskOutput` | Get background task output |
| `Task` | Manage tasks |
| `TodoWrite` | Write todo items |
| `TodoRead` | Read todo items |
| `Compact` | Compress conversation context |
| `RegisterHook` | Register hooks |
| `ComputerUse` | macOS desktop control |
| `EnterPlanMode` | Enter plan mode |
| `ExitPlanMode` | Exit plan mode |
| `LoadSkill` | Load skills |

## Permission Configuration

```yaml
# ~/.jdata/agent/data/agent_config.yaml
tools:
  - name: Read
    permission: allow
  - name: Bash
    permission: ask  # Require user confirmation
  - name: Write
    permission: deny
```

## Context References

| Reference | Description |
|-----------|-------------|
| `@file:path` | Include file content |
| `@dir:path` | Include directory structure |
| `@url:url` | Include web page content |
| `@grep:pattern` | Include search results |
