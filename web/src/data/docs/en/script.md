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

Scripts are stored in `~/.jdata/scripts/` directory:

```
~/.jdata/scripts/
├── deploy.sh
├── build.sh
└── test.sh
```

Scripts are automatically registered as aliases after creation, executable via `j <name>`.

## Example

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
