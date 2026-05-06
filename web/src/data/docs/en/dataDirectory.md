## Overview

All j-cli data is stored in a unified user data directory, with support for custom paths via environment variable.

## Data Directory Path

| Platform | Default Path | Environment Variable Override |
|----------|-------------|------------------------------|
| macOS / Linux | `~/.jdata/` | `J_DATA_PATH=/custom/path` |
| Windows | `%USERPROFILE%\.jdata\` | `$env:J_DATA_PATH="C:\custom\path"` |

### Environment Variable Override

```bash
# macOS / Linux
export J_DATA_PATH=/custom/path
j chat  # Data will be stored in /custom/path/

# Windows (PowerShell)
$env:J_DATA_PATH="C:\custom\path"
j chat  # Data will be stored in C:\custom\path\
```

## Directory Structure

```
~/.jdata/                          # macOS / Linux
%USERPROFILE%\.jdata\              # Windows
├── config.yaml                    # Main configuration
├── alias.yaml                     # Alias definitions
├── report/                        # Daily/weekly reports
│   ├── report.md                  # Report file
│   ├── todo.json                  # Todo data
│   └── settings.json              # Week metadata
├── scripts/                       # User scripts
│   ├── deploy.sh                  # macOS / Linux
│   └── deploy.cmd                 # Windows
├── agent/                         # AI Agent data
│   ├── data/                      # Agent runtime data
│   │   ├── messages/              # Chat history
│   │   └── agent_config.json      # Agent configuration
│   └── skills/                    # User-defined skills
│       └── <skill_name>/
│           └── SKILL.md
└── hooks/                         # Hook scripts
    └── pre_chat/
        └── my_hook.sh             # macOS / Linux
        └── my_hook.cmd            # Windows
```

## Configuration Files

### config.yaml

Main configuration file for global settings:

```yaml
# API Configuration
api_key: "your-api-key"
base_url: "https://api.openai.com/v1"
model: "gpt-4"

# Report Configuration
report_file_path: "~/.jdata/report/report.md"

# Browser Configuration
settings:
  browser_headless: true
```

### alias.yaml

Alias definitions for apps and URLs:

```yaml
# macOS / Linux
chrome:
  path: "/Applications/Google Chrome.app"
  note: "browser"

# Windows
notepad:
  path: "C:\\Windows\\notepad.exe"
  note: "editor"

# URL Alias
github:
  path: "https://github.com"
  type: "inner_url"
```

## Data Migration

### Backup

```bash
# macOS / Linux
cp -r ~/.jdata ~/.jdata.backup

# Windows
Copy-Item "$env:USERPROFILE\.jdata" "$env:USERPROFILE\.jdata.backup" -Recurse
```

### Restore

```bash
# macOS / Linux
cp -r ~/.jdata.backup ~/.jdata

# Windows
Copy-Item "$env:USERPROFILE\.jdata.backup" "$env:USERPROFILE\.jdata" -Recurse
```

### Cross-platform Migration

The data directory structure is consistent across all platforms. You can copy it directly:

1. Back up the source platform's data directory
2. Copy to the target platform's corresponding location
3. Adjust script file extensions (`.sh` → `.cmd`)
4. Update paths in alias.yaml to the target platform's format
