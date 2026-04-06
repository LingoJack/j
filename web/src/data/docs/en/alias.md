## Overview

Alias system for creating short aliases to paths and URLs for quick access.

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

Aliases are stored in `~/.jdata/config.yaml`:

```yaml
path:
  work: /Users/user/Projects/work
  notes: /Users/user/Documents/notes

inner_url:
  gh: https://github.com
  gh-issues: https://github.com/issues
```
