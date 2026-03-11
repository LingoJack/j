---
name: j-cli
description: "j-cli (work-copilot) command-line tool for daily workflow automation. TRIGGER when user wants to: (1) manage daily/weekly reports (日报/周报/report), (2) manage todo items (待办/todo), (3) open apps or URLs via aliases, (4) register/manage app aliases (别名), (5) create or run preset scripts (脚本/concat), (6) install or check j-cli availability. DO NOT TRIGGER for general shell commands unrelated to j-cli."
---

# j-cli (work-copilot)

A CLI tool for workflow automation: aliases, reports, todos, scripts.

## Prerequisites

Before using any j-cli command, verify it is installed:

```bash
bash <skill-path>/scripts/ensure_j.sh
```

If the script is unavailable, check manually:

```bash
command -v j && j version
# Install if missing:
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh
```

## Core Workflows

### 1. Daily/Weekly Report (日报/周报)

Report and weekly report are the same concept — j uses a single `week_report.md` file.

```bash
j report "完成了XX功能开发"       # Write a report entry (auto-prefixed with date)
j check                          # View last 10 lines
j check 20                       # View last 20 lines
j search all <keyword>           # Search in report
j search all <keyword> -f        # Fuzzy search (case-insensitive)
j reportctl open                 # Open report in TUI editor for full editing
j reportctl new                  # Start a new week (week number +1)
j reportctl push "update report" # Push to remote git repo
j reportctl pull                 # Pull from remote git repo
j reportctl set-url <repo_url>   # Set remote git repo URL
```

**IMPORTANT**: Always use `j report` to write entries. NEVER directly edit `~/.jdata/report/week_report.md`. Use `j reportctl push` to push, NOT raw git commands.

### 2. Todo Management (待办)

```bash
j todo add "买牛奶"     # Quick add a todo item
j todo list             # List all todos (rendered markdown)
j todo list --undone    # List only undone items
j todo list --done      # List only done items
j todo                  # Enter TUI management interface (interactive)
j td                    # Same as j todo (alias)
```

The TUI interface supports keyboard navigation, toggling completion, editing, deleting, reordering, and optional write-to-report on completion.

### 3. Alias Management (open apps/URLs)

Register aliases for apps, URLs, or files, then open them with one command:

```bash
# Register
j set chrome "/Applications/Google Chrome.app"
j set github https://github.com           # URL auto-classified as inner_url
j note chrome browser                      # Mark as browser category

# Open
j chrome                   # Open Chrome
j chrome github            # Open github URL in Chrome
j chrome "rust lang"       # Search "rust lang" in Chrome
j vscode ./src             # Open directory in VSCode

# Manage
j ls                       # List common aliases
j ls all                   # List all aliases
j rm <alias>               # Remove alias
j rename <alias> <new>     # Rename alias
j mf <alias> <new_path>    # Change alias target path
```

### 4. Script Preset (脚本预制)

Create reusable scripts stored at `~/.jdata/scripts/`, registered as aliases for quick execution:

```bash
j concat deploy "#!/bin/bash\necho deploying..."   # Create script + register alias
j concat deploy                                     # Edit existing script in TUI editor
j deploy                                            # Execute the script
j deploy -w                                         # Execute in new terminal window
```

Scripts automatically get all registered aliases as environment variables (`J_<ALIAS_UPPER>`):

```bash
#!/bin/bash
# If chrome is registered, $J_CHROME = /Applications/Google Chrome.app
open -a "$J_CHROME" https://example.com
```

### 5. Other Useful Commands

```bash
j version              # Show version
j update               # Update to latest version
j help                 # Show help
j                      # Enter interactive mode (with Tab completion)
j time countdown 5m    # Start a 5-minute countdown
```

## Reference Files

- **Full command reference**: See [references/commands.md](references/commands.md) for complete command list with all options
- **Configuration details**: See [references/configuration.md](references/configuration.md) for config file structures and settings

## Key Reminders

- Use `j report` for writing entries, not file editing
- Use `j reportctl push` for pushing reports, not git commands
- Use `j todo add` for adding todos programmatically
- Use `j concat` for creating scripts, not manually writing to `~/.jdata/scripts/`
- Paths with spaces must be quoted: `j set app "/Applications/My App.app"`
