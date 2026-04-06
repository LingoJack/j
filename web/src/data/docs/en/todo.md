## Overview

The todo management system provides lightweight task tracking with status transitions and priority management.

Core features:
- **Quick Add**: Add todo items with a single command
- **Status Management**: pending, in_progress, completed state transitions
- **List View**: Filter and sort by status
- **Data Persistence**: Auto-save to local file

## Basic Usage

### View Todos

```bash
j todo              # View all todos
j todo -s pending   # Filter by status
```

### Add Todo

```bash
j todo add "Finish documentation"
j todo add "Fix bug" -p high  # High priority
```

### Update Status

```bash
j todo start <id>     # Mark as in progress
j todo done <id>      # Mark as completed
j todo cancel <id>    # Cancel todo
```

### Delete Todo

```bash
j todo rm <id>        # Delete specified todo
j todo clear          # Clear completed items
```

## Todo Status

| Status | Description |
|--------|-------------|
| pending | Pending |
| in_progress | In Progress |
| completed | Completed |

## Priority

| Level | Identifier |
|-------|------------|
| Low | low |
| Medium | medium (default) |
| High | high |

## Data Storage

Todo data is stored in `~/.jdata/todos.json`:

```json
[
  {
    "id": 1,
    "content": "Finish documentation",
    "status": "pending",
    "priority": "high",
    "created_at": "2024-01-15T10:00:00Z"
  }
]
```

## Use Cases

- Daily task management
- Project progress tracking
- Personal memo
- Team task assignment
