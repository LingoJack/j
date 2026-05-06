## File Tools

### Read

Read file contents, supports image formats.

| Parameter | Type | Required | Description |
|------|------|------|------|
| path | string | Yes | File path (absolute or relative) |
| offset | uint | No | Starting line number (0-based, defaults to beginning) |
| limit | uint | No | Number of lines to read (defaults to all) |

**Supported image formats:** PNG, JPG, GIF, WEBP, BMP

```json
{"path": "src/main.rs"}
{"path": "screenshot.png"}
{"path": "large.log", "offset": 100, "limit": 50}
```

### Write

Write to file, auto-creates parent directories. Overwrites existing files.

| Parameter | Type | Required | Description |
|------|------|------|------|
| path | string | Yes | File path |
| content | string | Yes | Content to write |

```json
{"path": "src/new_file.rs", "content": "fn main() {}\n"}
```

### Edit

String replacement editing. `old_string` must uniquely match content in the file.

| Parameter | Type | Required | Description |
|------|------|------|------|
| path | string | Yes | File path |
| old_string | string | Yes | Original string to replace (must be unique) |
| new_string | string | No | Replacement string (empty deletes) |

```json
{"path": "src/main.rs", "old_string": "fn main() {}", "new_string": "fn main() { println!(\"Hello\"); }"}
```

### Glob

Find files by pattern.

| Parameter | Type | Required | Description |
|------|------|------|------|
| pattern | string | Yes | Glob pattern (e.g., `**/*.rs`, `src/**/*.ts`) |
| path | string | No | Search directory (defaults to current) |
| excludePattern | string | No | Exclude pattern (e.g., `**/node_modules/**`) |
| limit | uint | No | Max results (default 100) |

```json
{"pattern": "**/*.rs"}
{"pattern": "src/**/*.tsx", "excludePattern": "**/node_modules/**"}
```

### Grep

Regex search in file contents.

| Parameter | Type | Required | Description |
|------|------|------|------|
| pattern | string | Yes | Regex pattern |
| path | string | No | Search path (defaults to current) |
| glob | string | No | File filter (e.g., `*.rs`) |
| type | string | No | File type (js/py/rust/go/java) |
| output_mode | string | No | Output mode: content/files_with_matches/count |
| context | uint | No | Context lines |
| head_limit | uint | No | Max results |
| ignore_case | bool | No | Case insensitive (default false) |

```json
{"pattern": "fn\\s+\\w+", "type": "rust"}
{"pattern": "TODO", "glob": "*.rs", "output_mode": "count"}
```

## Execution Tools

### Bash

Execute shell commands (macOS / Linux only).

| Parameter | Type | Required | Description |
|------|------|------|------|
| command | string | Yes | Shell command (executes in bash -c) |
| cwd | string | No | Working directory |
| timeout | uint | No | Timeout in seconds (default 120, max 600) |
| run_in_background | bool | No | Run in background (returns task_id) |

**Notes:**
- Interactive commands not supported
- Build commands: recommended timeout 300-600
- Background tasks: use `TaskOutput` to get results

```json
{"command": "cargo build --release", "timeout": 300}
{"command": "npm run dev", "run_in_background": true}
```

### PowerShell

Execute PowerShell commands (Windows only).

| Parameter | Type | Required | Description |
|------|------|------|------|
| command | string | Yes | PowerShell command (executes in powershell.exe -NoProfile -Command) |
| description | string | No | Command description |
| cwd | string | No | Working directory |
| timeout | uint | No | Timeout in seconds (default 120, max 600) |
| run_in_background | bool | No | Run in background (returns task_id) |

**Notes:**
- Interactive commands not supported
- Build commands: recommended timeout 300-600
- Background tasks: use `TaskOutput` to get results

```json
{"command": "cargo build --release", "timeout": 300}
{"command": "npm run dev", "run_in_background": true}
```

### TaskOutput

Get output from background task.

| Parameter | Type | Required | Description |
|------|------|------|------|
| task_id | string | Yes | Background task ID |
| block | bool | No | Wait for completion (default true) |
| timeout | uint | No | Wait timeout in ms (default 30000, max 600000) |

## Network Tools

### WebFetch

Fetch web content, auto-converts to Markdown or plain text.

| Parameter | Type | Required | Description |
|------|------|------|------|
| url | string | Yes | Full URL (http:// or https://) |
| extract_mode | string | No | Output format: markdown/text |
| max_chars | uint | No | Max characters (default 50000) |
| headers | object | No | Custom headers |
| authorization | string | No | Authorization header |

```json
{"url": "https://docs.rs/serde"}
{"url": "https://api.github.com/repos/rust-lang/rust", "headers": {"Accept": "application/vnd.github.v3+json"}}
```

### WebSearch

Search the web using Exa (requires `EXA_API_KEY` environment variable).

| Parameter | Type | Required | Description |
|------|------|------|------|
| query | string | Yes | Search keywords |
| count | uint | No | Number of results (1-10, default 5) |
| type | string | No | Search type: auto/keyword/neural |

## Interaction Tools

### Ask

Request structured input from user, supports single/multi-select.

| Parameter | Type | Required | Description |
|------|------|------|------|
| questions | array | Yes | List of questions (1-4) |

**Question structure:**

| Field | Type | Description |
|------|------|------|
| header | string | Short tag |
| question | string | Full question text |
| options | array | Option list (2-4) |
| multi_select | bool | Allow multiple selections |

```json
{
  "questions": [{
    "header": "Style",
    "question": "Choose code style",
    "options": ["Concise", "Detailed", "Standard"],
    "multi_select": false
  }]
}
```

## Task Tools

### Task

Manage tasks (create/get/list/update).

**Actions:**

| action | Description |
|--------|------|
| create | Create task (requires title) |
| get | Get task details (requires taskId) |
| list | List all tasks |
| update | Update task status |

**Task status flow:** `pending` → `in_progress` → `completed`

```json
{"action": "create", "title": "Implement user auth", "description": "Add login/register functionality"}
{"action": "update", "taskId": 1, "status": "in_progress"}
{"action": "list", "ready": true}
```

### TodoWrite

Manage todo list.

| Parameter | Type | Description |
|------|------|------|
| todos | array | Todo list |
| merge | bool | Merge update |

**Todo structure:**

| Field | Type | Description |
|------|------|------|
| id | string/int | Todo ID |
| content | string | Content |
| status | string | Status: pending/in_progress/completed |

**Rule:** Only ONE item can be `in_progress` at any time

### TodoRead

Read current todo list. Returns id, content, and status for all items.

## Plan Tools

### EnterPlanMode

Enter plan mode. Read-only tools available, write tools blocked.

**Use cases:** Explore codebase and design implementation approach before writing code.

**When to use:**
- New feature with architectural decisions
- Multiple valid approaches need user choice
- Code changes affect existing behavior
- Multi-file changes (more than 2-3 files)

### ExitPlanMode

Exit plan mode, submit plan for user approval.

| Parameter | Type | Description |
|------|------|------|
| allowedPrompts | array | Prompt permissions needed for implementation |

## Extension Tools

### LoadSkill

Load specified skill into context.

| Parameter | Type | Required | Description |
|------|------|------|------|
| name | string | Yes | Skill name |
| arguments | string | No | Arguments to pass to skill |

**Available skills:**

| Skill | Description |
|------|------|
| j-cli | CLI workflow automation |
| skill-creator | Guide for creating skills |
| swift-ios-app-gen | iOS native app development |

### Agent

Launch sub-agent for complex multi-step tasks. Sub-agent uses fresh context, can use all tools except Agent.

| Parameter | Type | Required | Description |
|------|------|------|------|
| prompt | string | Yes | Task description for sub-agent |
| description | string | No | Brief description (3-5 words) |
| run_in_background | bool | No | Run in background |

### RegisterHook

Register, list, remove session-level hooks.

**Actions:**

| action | Description |
|--------|------|
| register | Register hook (requires event + command) |
| list | List all hooks |
| remove | Remove hook (requires event + index) |
| help | View protocol documentation |

## Session Tools

### Compact

Compress conversation context to free up context window.

**Use when:**
- Conversation getting long
- Multiple failed attempts at solving a problem

## ComputerUse Tool

macOS desktop control tool supporting screenshots, clicks, typing, etc. (macOS only).

### Screenshot Operations

#### screenshot

Capture screen with SoM (Set-of-Mark) annotations.

| Parameter | Type | Description |
|------|------|------|
| som | bool | Enable SoM annotations (default true) |
| app | string | Target app name |

**SoM annotations:**
- Draws numbered bounding boxes for each interactive element
- Returns element index for click reference
- Supports clicking via `element` parameter

### Mouse Operations

#### click

Click at position or element.

| Parameter | Type | Description |
|------|------|------|
| x, y | number | Coordinates (logical points) |
| element | uint | SoM element number |

#### double_click

Double-click at position.

#### right_click

Right-click at position.

#### drag

Drag operation.

| Parameter | Type | Description |
|------|------|------|
| start_x, start_y | number | Start coordinates |
| end_x, end_y | number | End coordinates |
| start_element, end_element | uint | SoM element numbers |
| duration_ms | uint | Drag duration |

#### scroll

Scroll operation.

| Parameter | Type | Description |
|------|------|------|
| dx, dy | int | Scroll amount (negative = up) |

### Keyboard Operations

#### type

Type text.

| Parameter | Type | Description |
|------|------|------|
| text | string | Text to type |
| delay_ms | uint | Keystroke delay in ms |

#### key

Press single key.

| Parameter | Type | Description |
|------|------|------|
| key | string | Key name (e.g., enter, tab, escape) |

#### keys

Key combination.

| Parameter | Type | Description |
|------|------|------|
| keys | array | Key list (e.g., `["cmd", "c"]`) |

**Supported keys:**
- Letters: a-z
- Numbers: 0-9
- Special: enter, tab, space, escape, delete, backspace
- Arrows: up, down, left, right
- Function: f1-f12, home, end, pageup, pagedown
- Modifiers: cmd, shift, alt/option, ctrl

### Helper Operations

#### find_element

Find element.

| Parameter | Type | Description |
|------|------|------|
| query | string | Search query |

#### ax_tree

Query accessibility tree.

| Parameter | Type | Description |
|------|------|------|
| app | string | Target app |
| depth | uint | Tree depth limit |
| role | string | Role filter (e.g., AXButton) |
| clickable | bool | Show only clickable elements |

## Permission Configuration

Permissions are configured in `.jcli/permissions.yaml`, supporting three rule types:

```yaml
# .jcli/permissions.yaml
permissions:
  # Allow all (skip all tool confirmations)
  allow_all: false
  
  # Allow list (skip confirmation if matched, supports regex)
  allow:
    - Read
    - Grep
    - Glob
    - "Bash:ls.*"       # Regex match on command parameter
    - "Bash:git status"
  
  # Deny list (takes priority over allow, direct reject if matched)
  deny:
    - "Bash:rm -rf.*"   # Block dangerous commands
    - "Bash:.*sudo.*"   # Block sudo commands
```

### Rule Matching

- **Simple match**: Tool name (e.g., `Read`, `Bash`)
- **Regex match**: `ToolName:regex_pattern` (e.g., `Bash:rm.*` matches Bash tool's command parameter)
- **Priority**: deny > allow > default confirmation

## Context References

| Reference | Description |
|------|------|
| `@file:path` | Include file content (auto-read and inject into context) |
| `@skill:name` | Load and activate specified skill |
