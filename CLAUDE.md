# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`j-cli` (a.k.a. "work-copilot") is a Rust CLI productivity tool for macOS ARM64. It manages app/URL aliases, daily reports, todos, AI chat, voice-to-text, and more. Binary name is `j`. User data lives in `~/.jdata/` (overridable via `J_DATA_PATH`).
more related info: README.md
REMEBER TO UPDATE README.md AND HELP.md WHEN CHANGES ARE MADE IF NEEDED

## Commands

```bash
# Development
cargo build              # Debug build
cargo build --release    # Release build
cargo run                # Run (enters interactive mode if no args)
cargo run -- <args>      # Run with subcommand, e.g.: cargo run -- list
cargo fmt                # Format code
cargo test               # Run tests
cargo check              # Check without building

# Makefile shortcuts
make install             # Build release and install to /usr/local/bin/j
make uninstall           # Remove /usr/local/bin/j
make fmt                 # cargo fmt
make release             # Build md_render Go binary + cargo build --release
make md_render           # Build the Markdown renderer Go binary (plugin/md_render/code/)
make push                # fmt + git commit + push (commit message: "新增了一些特性")
make tag                 # Create and push a version tag (triggers GitHub Actions release)
```

**Note:** `cargo build --release` requires CMake (`brew install cmake`) because of the `whisper-rs` dependency.

## Architecture

### Entry Point and Routing (`src/main.rs`, `src/cli.rs`)

`main.rs` handles two execution paths:
1. **No args** → `interactive::run_interactive()` (rustyline REPL)
2. **With args** → `Cli::try_parse()` via clap:
   - Success + subcommand → `command::dispatch(sub_cmd, &mut config)`
   - Success + no subcommand + args → `command::open::handle_open(args, &config)` (alias lookup)
   - Parse failure → also falls through to `command::open::handle_open()` (e.g., `j chrome`)

The `SubCmd` enum in `cli.rs` defines all subcommands with their clap derive attributes.

### Config (`src/config/yaml_config.rs`)

All configuration is stored in `~/.jdata/config.yaml` and deserialized into a `YamlConfig` struct via serde. The config contains alias maps, category tags, and runtime settings. The `config` is passed as `&mut YamlConfig` throughout the command layer.

### Command Layer (`src/command/`)

- `mod.rs` — `dispatch()` function routing `SubCmd` variants to handlers; also contains keyword/category constant lists
- `open.rs` — The core alias resolution logic: looks up an alias in config, determines if it's a path/URL/script/app, and spawns the appropriate system process
- `alias.rs` — CRUD for alias entries
- `report.rs` — Daily report writing, searching, and git sync
- `chat/` — Async AI chat TUI using `async-openai` + `tokio`, with streaming and Markdown rendering
- `todo/` — TUI-based todo list backed by `~/.jdata/todo/todo.json`
- `voice.rs` — Offline speech recognition via `whisper-rs` + `cpal` for recording

### TUI Layer (`src/tui/editor.rs`)

A fullscreen multi-line editor built on `ratatui` + `tui-textarea` (patched local copy in `patches/tui-textarea-0.7.0/`). Used for writing reports, scripts, and composing multi-line content.

### Interactive Mode (`src/interactive/`)

- `shell.rs` — REPL loop using `rustyline`
- `completer.rs` — Tab completion that reads alias names from config
- `parser.rs` — Parses raw input lines into command args for dispatch

### Utilities (`src/util/`)

- `log.rs` — `debug_log!` macro (only prints when verbose mode is enabled in config)
- `md_render.rs` — Invokes the external `ask` binary (Go-based Markdown renderer embedded in `~/.jdata/bin/ask`) for rendering
- `fuzzy.rs` — Simple fuzzy string matching used by `search` command

### Embedded Assets (`src/assets.rs`, `assets/`)

`assets/help.md` and `assets/version.md` are embedded at compile time via `include_str!`. The Go Markdown renderer binary is also embedded and extracted to `~/.jdata/bin/ask` on first run.

### Markdown Renderer Plugin (`plugin/md_render/code/`)

A Go program (`main.go`) that renders Markdown to ANSI-colored terminal output. Built separately with `make md_render` and embedded into the Rust binary. The `go.mod` uses `github.com/charmbracelet/glamour`.

### Release Pipeline

GitHub Actions (`.github/workflows/release.yml`) triggers on `v*` tags, builds for `aarch64-apple-darwin` with `CFLAGS=-march=armv8-a` (to avoid ARM i8mm issues with whisper.cpp), and publishes a GitHub Release with `j-darwin-arm64.tar.gz`.

## Key Design Decisions

- **Patched dependency**: `tui-textarea` is patched via `[patch.crates-io]` in `Cargo.toml` to use the local copy in `patches/`. Any changes to textarea behavior go there.
- **Cargo registry**: `.cargo/config.toml` uses `rsproxy-sparse` (Chinese mirror). This is intentional for the development environment.
- **Alias open fallback**: When clap parse fails (e.g., `j chrome`), args are treated as alias names rather than returning an error.
- **Verbose logging**: Controlled by a config field; use `debug_log!(config, "...")` for debug output that respects this setting.
