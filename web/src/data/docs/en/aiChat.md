## Starting AI Chat

```bash
j chat              # Open TUI chat
j chat "Hello"      # Quick question
```

## Features

- **Multi-model support**: OpenAI, Claude, Gemini, Ollama
- **Streaming output**: Real-time responses
- **Tool calling**: AI can use tools
- **Context management**: Include files and URLs

## Context Reference

```bash
# Include local files
@file:src/main.rs Explain this code

# Include directories
@dir:src/ Analyze this codebase

# Include URLs
@url:https://example.com Summarize this page
```

## Commands

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/compact` | Compress conversation context |
| `/clear` | Clear conversation history |
| `/model` | Switch AI model |
| `/export` | Export conversation |

## Web Search

Enable web search to let AI fetch latest information:

```bash
What are the new features in React 19?
```
