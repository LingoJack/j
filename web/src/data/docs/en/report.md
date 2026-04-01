## Commands

| Command | Description |
|---------|-------------|
| `j report [text]` | Write to daily report (opens TUI if no text) |
| `j check [n]` | View recent n lines (default: 10) |
| `j search <keyword>` | Search reports with fuzzy matching |

## Examples

```bash
# Quick write
j report "Completed user authentication module"
j report "Meeting with team" "Discussed sprint planning"

# View reports
j check          # View recent 10 lines
j check 20       # View recent 20 lines

# Search
j search authentication
j search "user module" -fuzzy
```

## TUI Editor

Running `j report` without arguments opens the TUI editor:

- **Multi-line editing**: Write longer entries with proper formatting
- **History suggestions**: Auto-complete from previous entries
- **Tab completion**: Quickly insert common phrases

## Git Integration

```bash
# Initialize Git sync
cd ~/.jdata/report
git init
git remote add origin <your-repo>

# Daily workflow
j report "Completed feature"
j reportctl push   # Sync to remote
```
