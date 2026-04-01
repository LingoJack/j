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
