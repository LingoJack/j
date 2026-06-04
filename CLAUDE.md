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
