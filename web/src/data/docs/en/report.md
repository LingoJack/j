## Overview

Daily/weekly report system with quick logging, week management, and Git sync.

## Basic Commands

```bash
j report <content>        # Quick write to daily report
j report                  # Open TUI editor (prefilled with history + date prefix)
j check [n]               # View last n lines (default 3)
j check open              # Open TUI editor to edit full content
j search <n|all> <keyword> [-f]  # Search reports (-f for fuzzy match)
```

## Week Management (reportctl)

```bash
j reportctl new [date]      # Start a new week
j reportctl sync [date]     # Sync week number and date
j reportctl set-url <url>   # Set Git repository URL
j reportctl push [message]  # Push to remote repository
j reportctl pull            # Pull from remote repository
j reportctl open            # Open TUI editor to edit full content
```

## Report Format

```markdown
# Week1[2024-01-01 - 2024-01-07]
- 【01-01】 Project initialization completed
- 【01-02】 Core features implemented
- 【01-03】 Code review and optimization
```

## Git Sync

Set up remote repository for automatic sync:

```bash
# Initial setup
j reportctl set-url https://github.com/user/reports.git

# Push to remote
j reportctl push "Update report"

# Pull from remote
j reportctl pull
```

## Configuration Files

Report settings are stored in two locations:

| File | macOS / Linux | Windows |
|------|---------------|---------|
| Main config | `~/.jdata/config.yaml` | `%USERPROFILE%\.jdata\config.yaml` |
| Week metadata | `<report_dir>/settings.json` | `<report_dir>\settings.json` |

| File | Description |
|------|-------------|
| Main config | report_file_path, git_repo settings |
| Week metadata | week_num, last_day info |

## Auto Week Switch

When current date exceeds `last_day`, writing to report automatically:
1. Generates new week title `# WeekN[start_date - end_date]`
2. Updates week_num and last_day
