## Commands

| Command | Description |
|---------|-------------|
| `j concat <name> [content]` | Create/edit script |
| `j <script> [args]` | Execute script with arguments |

## Creating Scripts

```bash
# Create script with content
j concat open "open $1"

# Create script with TUI editor
j concat deploy

# Create in new window
j concat build -w
```

## Executing Scripts

```bash
# Execute script
j open README.md         # Passes README.md as $1
j build                  # Execute without arguments

# Execute in new window
j open -w README.md
```

## Environment Variables

Scripts can use environment variables:

| Variable | Description |
|----------|-------------|
| `$1`, `$2`, ... | Script arguments |
| `$@` | All arguments |
| `$J_DATA_PATH` | Data directory path |

## Examples

```bash
# Deployment script
j concat deploy "git pull && cargo build --release && systemctl restart myapp"

# Backup script
j concat backup "cp -r $1 ~/.jdata/backups/$(date +%Y%m%d)"

# Open in editor
j concat edit "code $1"
```
