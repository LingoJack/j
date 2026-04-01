## Commands

| Command | Description |
|---------|-------------|
| `j todo` | Open TUI todo manager |
| `j todo add <text>` | Quick add todo item |
| `j todo done <id>` | Mark todo as done |
| `j todo list` | List todos (supports --done/--undone) |

## Examples

```bash
# Quick add
j todo add Buy milk
j todo add Review pull request

# List todos
j todo list              # All todos
j todo list --undone     # Only pending
j todo list --done       # Only completed

# Mark as done
j todo done 1
j todo done 1 --report   # Also write to daily report
```

## TUI Manager

Running `j todo` opens the interactive TUI:

- **Add/Edit/Delete**: Manage todos interactively
- **Priority**: Set priority levels
- **Due dates**: Add deadlines
- **Categories**: Organize by project/context

## Markdown Integration

Todos can be written in daily reports using Markdown:

```markdown
- [x] Completed task
- [ ] Pending task
- [ ] Another pending task
```
