## Start AI Chat

```bash
j chat              # Enter TUI chat interface
j chat "Hello"      # Quick question and print response
j chat -c           # Continue last session
j chat --session <id>  # Resume specific session
```

## Remote Control

```bash
j chat --remote     # Enable remote control (scan QR with phone)
j chat --remote --port 9390  # Specify port
```

## Shortcuts

| Shortcut | Action |
|----------|--------|
| `Enter` | Send message |
| `Esc` | Cancel response/Exit |
| `Ctrl+T` | Switch model |
| `Ctrl+L` | Archive conversation |
| `Ctrl+Y` | Copy last AI reply |
| `Ctrl+B` | Message browse mode |
| `Ctrl+E` | Open config panel |
| `F1` or `?` | Show help |

## Context References

Type `@` in input to trigger completion:

```
@skill:<name>       # Reference skill
@command:<name>     # Reference custom command
@file:<path>        # Reference file content (supports images)
```

## Multi-Model Support

Supports OpenAI, Claude, Gemini, Ollama and more. Use `Ctrl+E` to open config panel.
