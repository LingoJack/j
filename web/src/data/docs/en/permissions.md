## Permission Configuration File

Permissions are configured in `.jcli/permissions.yaml` in your project directory:

```yaml
permissions:
  # Allow all tools without confirmation
  allow_all: false
  
  # Allow list (skip confirmation if matched)
  allow:
    - Read
    - Grep
    - Glob
    - "Bash(cargo build:*)"
    - "Bash(git status:*)"
  
  # Deny list (takes priority over allow, blocks execution)
  deny:
    - "Bash(rm -rf:*)"
    - "Bash(/.*sudo.*/)"    # Regex match
```

## Rule Formats

| Format | Description | Example |
|--------|-------------|---------|
| `*` | Match all tools | `*` |
| `ToolName` | Match all calls to this tool | `Read`, `Grep` |
| `ToolName(prefix:*)` | Prefix match | `Bash(cargo build:*)` |
| `ToolName(path:/dir/*)` | Path match | `Write(path:/src/*)` |
| `ToolName(domain:example.com)` | Domain match | `WebFetch(domain:docs.rs)` |
| `ToolName(/regex/)` | Regex match | `Bash(/^cargo (build\|test)/)` |

## Match Priority

```
deny > allow > default requires confirmation
```

- `deny` list has highest priority, blocks execution if matched
- `allow` list skips confirmation if matched
- `allow_all: true` skips all confirmations (but deny still takes priority)

## Tool-Specific Rules

### Platform Differences

| Tool | macOS / Linux | Windows |
|------|---------------|---------|
| Shell | `Bash(...)` | `PowerShell(...)` |
| Computer Use | `ComputerUse(...)` | Not available |

> **Note**: On Windows, use `PowerShell(...)` rules instead of `Bash(...)`.

### Bash / PowerShell Command Matching

macOS / Linux:

```yaml
allow:
  - "Bash(cargo:*)"        # cargo build, cargo test, etc.
  - "Bash(git status:*)"   # git status
  - "Bash(ls:*)"           # ls, ls -la, etc.

deny:
  - "Bash(rm -rf:*)"       # Block rm -rf
  - "Bash(/.*sudo.*/)"     # Block all sudo commands
```

Windows:

```yaml
allow:
  - "PowerShell(cargo:*)"        # cargo build, cargo test, etc.
  - "PowerShell(git status:*)"   # git status
  - "PowerShell(dir:*)"          # dir, dir /s, etc.

deny:
  - "PowerShell(Remove-Item -Recurse -Force:*)"  # Block recursive force delete
  - "PowerShell(/.*Format-.*/)"                   # Block format commands
```

### File Path Matching (Write/Edit/Read)

```yaml
# macOS / Linux
allow:
  - "Write(path:/src/*)"   # Allow writes to /src directory
  - "Edit(path:/lib/*)"    # Allow edits to /lib directory

deny:
  - "Write(path:/etc/*)"   # Block writes to /etc

# Windows
allow:
  - "Write(path:C:\\Projects\\myapp\\src\\*)"  # Allow writes to project src directory
  - "Edit(path:C:\\Projects\\myapp\\lib\\*)"   # Allow edits to project lib directory

deny:
  - "Write(path:C:\\Windows\\System32\\*)"     # Block writes to system directory
```

### URL Domain Matching (WebFetch)

```yaml
allow:
  - "WebFetch(domain:docs.rs)"
  - "WebFetch(domain:github.com)"
  - "WebFetch(domain:/.*\\.google\\.com$/)"  # Regex match all google subdomains
```
