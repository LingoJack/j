## Overview

Alias system for creating short aliases to paths and URLs for quick access.

> **Tip**: Press **Tab** to auto-complete file paths. Supports `~` expansion. E.g., `j set work ~/Pro<Tab>`.

## Basic Usage

### Add Alias

```bash
j set <alias> <path>    # Add path alias
j set <alias> <url>     # Add URL alias
```

### Execute Alias

```bash
j <alias>               # Open path or URL
```

### Manage Aliases

```bash
j rm <alias>            # Remove alias
j rename <old> <new>    # Rename alias
j mf <alias> <new_path> # Modify alias target
```

## Alias Types

### Path Alias

```bash
# Add path
j set work ~/Projects/work
j set notes ~/Documents/notes

# Open path
j work    # Open in file manager
j notes   # Open in file manager
```

### URL Alias

```bash
# Add URL
j set gh https://github.com
j set gh-issues https://github.com/issues

# Open URL
j gh        # Open in browser
j gh-issues # Open in browser
```

## Alias Storage

Aliases are stored in the configuration file under the data directory:

| Platform | Config File Path |
|----------|-----------------|
| macOS / Linux | `~/.jdata/alias.yaml` |
| Windows | `%USERPROFILE%\.jdata\alias.yaml` |

```yaml
# macOS / Linux
path:
  chrome: "/Applications/Google Chrome.app"
  vscode: "/Applications/Visual Studio Code.app"
  work: /Users/user/Projects/work

inner_url:
  gh: https://github.com
  gh-issues: https://github.com/issues

# Windows
path:
  notepad: "C:\\Windows\\notepad.exe"
  vscode: "C:\\Users\\user\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe"
  work: "C:\\Users\\user\\Projects\\work"

inner_url:
  gh: https://github.com
```
