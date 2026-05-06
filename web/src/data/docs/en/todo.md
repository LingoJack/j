## Overview

Todo management system with status transitions and TUI interface.

## Basic Usage

### Open TUI Interface

```bash
j todo              # Open todo management TUI
```

### Command Line Operations

```bash
j todo list              # List all todos
j todo list --done       # List completed only
j todo list --undone     # List undone only
j todo add "Finish docs"  # Quick add todo
```

## TUI Operations

| Shortcut | Action |
|----------|--------|
| `j/k` | Move up/down |
| `Enter` | Toggle completion |
| `a` | Add new todo |
| `e` | Edit current item |
| `d` | Delete current item |
| `r` | Write to daily report |
| `Tab` | Toggle filter (all/undone/done) |
| `?` | Show help |
| `q/Esc` | Exit |

## Todo Status

| Status | Display |
|--------|---------|
| Undone | `[ ]` |
| Done | `[x]` |

## Data Storage

Todo data is stored in the data directory:

| Platform | Path |
|----------|------|
| macOS / Linux | `~/.jdata/report/todo.json` |
| Windows | `%USERPROFILE%\.jdata\report\todo.json` |
