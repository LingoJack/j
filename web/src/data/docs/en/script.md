## Overview

The script system allows defining and executing preset shell command sequences with parameterization and conditional execution.

Core features:
- **Predefined Scripts**: Save common command sequences as reusable scripts
- **Parameterized Execution**: Scripts support placeholders for runtime arguments
- **Command Chaining**: Support for sequential command execution
- **Environment Isolation**: Each script runs in an independent shell

## Basic Usage

### Execute Script

```bash
j script <name>           # Execute specified script
j script <name> <args...> # Execute with arguments
```

### Manage Scripts

Scripts are stored in `~/.jdata/scripts/` directory, each script as a Markdown file:

```
~/.jdata/scripts/
├── deploy.md
├── build.md
└── test.md
```

## Script Format

Scripts use Markdown format with frontmatter configuration:

```markdown
---
name: deploy
description: Deploy to production
---

#!/bin/bash
set -e

echo "Building..."
npm run build

echo "Deploying..."
rsync -avz dist/ user@server:/var/www/
```

### Frontmatter Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| name | string | Yes | Script name |
| description | string | No | Script description |

## Parameterization

Scripts support `{{.param}}` placeholders:

```markdown
---
name: greet
description: Greeting script
---

#!/bin/bash
name="{{.name}}"
echo "Hello, $name!"
```

Execute with arguments:

```bash
j script greet --name World
```

## Use Cases

- Project build and deployment
- Code formatting and linting
- Database backup
- Environment initialization
- Scheduled task wrapper
