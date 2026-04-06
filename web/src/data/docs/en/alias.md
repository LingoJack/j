## Overview

The alias system allows creating short aliases for common commands and applications, improving command-line efficiency.

Core features:
- **Command Aliases**: Simplify long commands into short aliases
- **App Aliases**: Quickly open frequently used apps and URLs
- **Group Management**: Organize aliases by project or category
- **Dynamic Extension**: Support runtime addition and deletion

## Basic Usage

### Execute Alias

```bash
j <alias>            # Execute alias command
j <alias> <args...>  # Execute with arguments
```

### Manage Aliases

```bash
j alias              # List all aliases
j alias add <name> <command>  # Add alias
j alias rm <name>    # Remove alias
```

## Alias Types

### Command Aliases

Simplify common commands:

```bash
# Add alias
j alias add gs "git status"
j alias add gp "git push"

# Use alias
j gs
j gp origin main
```

### App Aliases

Quickly open apps or URLs:

```bash
# Open application
j alias add chrome "open -a 'Google Chrome'"
j alias add vscode "open -a 'Visual Studio Code'"

# Open URL
j alias add gh "open https://github.com"

# Usage
j chrome
j gh
```

## Alias Files

Aliases are stored in `~/.jdata/aliases/` directory:

```
~/.jdata/aliases/
├── git.json      # Git related aliases
├── apps.json     # App aliases
└── work.json     # Work related aliases
```

### Alias File Format

```json
[
  {
    "name": "gs",
    "command": "git status",
    "description": "Show Git status"
  },
  {
    "name": "chrome",
    "command": "open -a 'Google Chrome'",
    "description": "Open Chrome browser"
  }
]
```

## Parameterized Aliases

Aliases support `$1`, `$2` parameter placeholders:

```bash
# Add alias with parameters
j alias add find-file "find . -name '$1' -type f"

# Usage
j find-file "*.rs"
```

## Use Cases

- Git command simplification
- Quick project switching
- Fast app launching
- URL bookmarks
- SSH connection shortcuts
