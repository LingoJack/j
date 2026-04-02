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

### Bash Command Matching

```yaml
allow:
  - "Bash(cargo:*)"        # cargo build, cargo test, etc.
  - "Bash(git status:*)"   # git status
  - "Bash(ls:*)"           # ls, ls -la, etc.
  
deny:
  - "Bash(rm -rf:*)"       # Block rm -rf
  - "Bash(/.*sudo.*/)"     # Block all sudo commands
```

### File Path Matching (Write/Edit/Read)

```yaml
allow:
  - "Write(path:/src/*)"   # Allow writes to /src directory
  - "Edit(path:/lib/*)"    # Allow edits to /lib directory
  
deny:
  - "Write(path:/etc/*)"   # Block writes to /etc
```

### URL Domain Matching (WebFetch)

```yaml
allow:
  - "WebFetch(domain:docs.rs)"
  - "WebFetch(domain:github.com)"
  - "WebFetch(domain:/.*\\.google\\.com$/)"  # Regex match all google subdomains
```
