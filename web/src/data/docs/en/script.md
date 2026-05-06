## Overview

Script system for creating and managing executable scripts via `concat` command.

## Basic Usage

### Create Script

```bash
j concat <name>              # Open TUI editor to write script
j concat <name> "<content>"  # Create script directly
```

### Edit Script

```bash
j concat <name>              # Enter edit mode if script exists
```

### Run Script

```bash
j <name>           # Run via alias directly
j <name> <args...> # Run with arguments
```

### Delete Script

```bash
j rm <name>        # Remove alias (also deletes script file)
```

## Script Storage

Scripts are stored in platform-specific directories:

| Platform | Storage Path |
|----------|-------------|
| macOS / Linux | `~/.jdata/scripts/` |
| Windows | `%USERPROFILE%\.jdata\scripts\` |

```
# macOS / Linux
~/.jdata/scripts/
├── deploy.sh
├── build.sh
└── test.sh

# Windows
%USERPROFILE%\.jdata\scripts\
├── deploy.cmd
├── build.cmd
└── test.cmd
```

Scripts are automatically registered as aliases after creation, executable via `j <name>`.

## Platform Differences

| Feature | macOS / Linux | Windows |
|---------|---------------|---------|
| Script extension | `.sh` | `.cmd` |
| Default shebang | `#!/bin/bash` | (none) |
| Shell | Bash | cmd.exe |
| PATH separator | `:` | `;` |

## Example

### macOS / Linux

```bash
# Create deploy script
j concat deploy

# In editor, input:
#!/bin/bash
set -e
npm run build
rsync -avz dist/ user@server:/var/www/

# Run script
j deploy
```

### Windows

```powershell
# Create deploy script
j concat deploy

# In editor, input:
@echo off
npm run build
xcopy dist\ \\server\www\ /E /I /Y

# Run script
j deploy
```

## Environment Variables

All registered alias paths are automatically injected as environment variables when executing scripts. Naming rule: `J_<ALIAS_UPPERCASE>` (hyphens converted to underscores).

### macOS / Linux (.sh)

```bash
#!/bin/bash
# Registered: chrome -> /Applications/Google Chrome.app
# Registered: my-tool -> /usr/local/bin/my-tool

open -a "$J_CHROME" https://example.com
"$J_MY_TOOL" --version
```

### Windows (.cmd)

```cmd
@echo off
REM Registered: notepad -> C:\Windows\notepad.exe
REM Registered: vscode -> C:\Users\%USERNAME%\AppData\Local\Programs\Microsoft VS Code\Code.exe

start "" "%J_VSCODE%" .\src
"%J_NOTEPAD%" readme.txt
```

> Paths with spaces must be wrapped in double quotes: `"$J_CHROME"` / `"%J_VSCODE%"`