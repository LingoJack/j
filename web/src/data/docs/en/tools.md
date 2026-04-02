## Available Tools

| Tool | Description |
|------|-------------|
| `Read` | Read file contents (supports images: png/jpg/gif/webp/bmp) |
| `Write` | Write to files (auto-create directories) |
| `Edit` | Edit files with string replacement (old_string must match uniquely) |
| `Glob` | Find files by pattern (supports glob like `**/*.rs`) |
| `Grep` | Regex search file contents (supports context, pagination) |
| `Bash` | Execute shell commands (supports background execution) |
| `WebFetch` | Fetch web page content (convert to Markdown or text) |
| `WebSearch` | Search the web via Exa (requires EXA_API_KEY) |
| `Ask` | Ask user for structured input (single/multi-select) |
| `TaskOutput` | Get background task output |
| `Task` | Manage tasks (create/get/list/update) |
| `TodoWrite` | Write todo items (only one in_progress allowed) |
| `TodoRead` | Read todo items list |
| `Compact` | Compress conversation context (auto-triggered) |
| `RegisterHook` | Register session-level hooks |
| `ComputerUse` | macOS desktop control (screenshot, click, type) |
| `EnterPlanMode` | Enter plan mode (read-only tools) |
| `ExitPlanMode` | Exit plan mode (submit plan) |
| `LoadSkill` | Load skills (registered on demand) |
| `Agent` | Sub-agent for complex tasks (prevents recursion) |
| `Browser` | Browser tool (Lite/CDP mode) |

## Permission Configuration

Permissions are configured in `.jcli/permissions.yaml` with three rule types:

```yaml
# .jcli/permissions.yaml
permissions:
  # Allow all tools without confirmation
  allow_all: false
  
  # Allow list (skip confirmation if matched, supports regex)
  allow:
    - Read
    - Grep
    - Glob
    - "Bash:ls.*"       # Regex match command arguments
    - "Bash:git status"
  
  # Deny list (takes priority over allow, blocks execution)
  deny:
    - "Bash:rm -rf.*"   # Block dangerous commands
    - "Bash:.*sudo.*"   # Block sudo commands
```

### Rule Matching

- **Simple match**: Tool name (e.g., `Read`, `Bash`)
- **Regex match**: `ToolName:regex_pattern` (e.g., `Bash:rm.*` matches Bash tool's command argument)
- **Priority**: deny > allow > default requires confirmation

## Context References

| Reference | Description |
|-----------|-------------|
| `@file:path` | Include file content |
| `@dir:path` | Include directory structure |
| `@url:url` | Include web page content |
| `@grep:pattern` | Include search results |
