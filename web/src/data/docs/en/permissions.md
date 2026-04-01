## Permission Levels

| Level | Description |
|-------|-------------|
| `allow` | Always allowed |
| `ask` | Ask for confirmation |
| `deny` | Always denied |

## Configuration

```yaml
# ~/.jdata/agent/data/agent_config.yaml
permissions:
  # Read operations - always allowed
  - tool: Read
    permission: allow
  
  # Write operations - ask for confirmation
  - tool: Write
    permission: ask
  
  # Shell commands - ask for confirmation
  - tool: Bash
    permission: ask
    rules:
      - pattern: "ls *"        # Allow ls commands
        permission: allow
      - pattern: "rm *"        # Always ask for rm
        permission: ask
  
  # Web access - always allowed
  - tool: WebFetch
    permission: allow
  - tool: WebSearch
    permission: allow
```

## Fine-grained Rules

```yaml
permissions:
  - tool: Bash
    permission: ask
    rules:
      # Allow specific patterns
      - pattern: "git status"
        permission: allow
      - pattern: "cargo build"
        permission: allow
      
      # Deny dangerous patterns
      - pattern: "rm -rf /*"
        permission: deny
```
