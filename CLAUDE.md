# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**j-cli** (`j`) is a fast CLI productivity tool written in Rust. It provides alias management, daily reports, todo management, AI chat with tool-calling, browser automation, a skill/hook system, and an interactive REPL. Binary name is `j`.

- Rust 2024 edition, minimum rustc 1.93.1
- Published to crates.io as `j-cli`
- All UI text is in Chinese (中文)

## Common Commands

```bash
# Build
cargo build                        # Debug build
cargo build --release              # Release build
cargo build --features browser_cdp # Build with Chrome DevTools Protocol support

# Test
cargo test                         # Run all tests
cargo test --all-features          # Run tests including CDP features
cargo test <test_name>             # Run a single test

# Code quality
cargo fmt                          # Format code
cargo clippy -- -D warnings        # Lint (treats warnings as errors)

# Pre-commit (format + lint + test)
make pre-commit

# Install locally
make install                       # Builds release and copies to /usr/local/bin/j

# Version management
make bump-version                  # Increment patch version in Cargo.toml
make set-version V=x.y.z           # Set specific version
make publish                       # Bump version + build + tag + push + publish to crates.io
```

## Architecture

### Entry Flow

`main.rs` → If no args, enters interactive REPL (`interactive::run_interactive`). Otherwise, attempts clap parsing (`cli::Cli`). If clap fails (unrecognized subcommand), falls back to alias-open logic (`command::open::handle_open`).

### Module Structure

```
src/
├── main.rs              # Entry point, arg routing
├── cli.rs               # Clap command/subcommand definitions (SubCmd enum)
├── constants.rs         # Global constants (paths, version, data dirs)
├── assets.rs            # Embedded assets (rust-embed)
├── config/              # YAML config loading/saving (~/.jdata/config.yaml)
├── command/
│   ├── handler.rs       # CommandHandler trait + command_handlers! macro
│   ├── mod.rs           # dispatch() function routing SubCmd → handlers
│   ├── alias.rs         # set/remove/rename/modify aliases
│   ├── category.rs      # note/denote category tagging
│   ├── list.rs          # ls command
│   ├── open.rs          # Alias open logic (apps, URLs, editors, browsers)
│   ├── report.rs        # Daily report system
│   ├── script.rs        # Script creation/execution (j concat)
│   ├── system.rs        # System commands (contain, change, clear, help)
│   ├── time.rs          # Countdown timer
│   ├── update.rs        # Self-update
│   ├── todo/            # Todo TUI (ratatui-based)
│   ├── help/            # Help text
│   └── chat/            # AI chat system (~largest module)
├── interactive/         # REPL mode (rustyline, tab completion, shell mode)
├── tui/                 # Shared TUI editor widget
└── util/                # Logging macros, markdown rendering, fuzzy search, HTML extract
```

### Chat Module (`command/chat/`) — Largest Subsystem

The AI chat is a full TUI application with tool-calling, streaming, and context management:

- **handler/** — TUI event loop (`tui_loop.rs`), chat logic (`chat.rs`), config UI, archive restore, tool confirmation dialog, message browse mode
- **agent.rs** — Agent loop: sends messages to LLM, processes tool calls in a loop until done
- **api.rs** — OpenAI-compatible API client (streaming via `async-openai`)
- **tools/** — 15+ built-in tools (Bash, Read, Write, Edit, Glob, Grep, WebFetch, WebSearch, Browser, Ask, BackgroundRun, Compact, Task*, LoadSkill, RegisterHook). Each tool is a separate file
- **tools/file/** — File operation tools (read, write, edit, glob, grep)
- **tools/task/** — Task management tools (create, list, get, update)
- **compact.rs** — 3-layer context compaction (micro_compact → auto_compact → Compact tool)
- **permission.rs** — `.jcli` file permission system (allow/deny rules for tool execution)
- **hook.rs** — 3-tier hook system (user → project → session level)
- **skill.rs** — Skill loading from `~/.jdata/agent/skills/`
- **storage.rs** — Chat history persistence, agent config loading
- **ui/** — UI rendering (chat view, config view, archive list)
- **markdown/** — Markdown parsing and syntax highlighting for chat display
- **theme.rs** — Color themes (dark, light, dracula, gruvbox, monokai, nord)
- **render_cache.rs** — Rendered message caching
- **archive.rs** — Conversation archive/restore
- **autocomplete.rs** — `@skill` autocomplete in input

### Key Patterns

- **CommandHandler trait + `command_handlers!` macro**: Declarative command registration in `handler.rs`. Each command is a struct implementing `CommandHandler::execute()`.
- **`dispatch()` function**: Maps `SubCmd` enum variants to handler structs in `command/mod.rs`.
- **Logging macros**: `info!`, `error!`, `usage!`, `debug_log!(config, ...)` defined in `util/log.rs`. `debug_log!` only prints in verbose mode.
- **Agent file logging**: `log_info()` and `log_error()` in `util/log.rs` write to `~/.jdata/agent/logs/`.
- **Config**: `YamlConfig` wraps `~/.jdata/config.yaml`. Agent config is separate JSON at `~/.jdata/agent/data/agent_config.json`.
- **Data directory**: All user data lives under `~/.jdata/` (customizable via `J_DATA_PATH` env var).
- **Optional feature**: `browser_cdp` feature flag enables Chrome DevTools Protocol support via `chromiumoxide` crate. Without it, browser tools use HTTP-based "Lite" mode.
- **Patched dependency**: `tui-textarea` is patched locally (see `patches/tui-textarea-0.7.0/` and `[patch.crates-io]` in Cargo.toml).