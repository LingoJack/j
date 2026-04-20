# CODEBUDDY.md

This file provides guidance to CodeBuddy Code when working with code in this repository.

## Build & Development Commands

```bash
# Build (debug)
cargo build

# Build (release)
cargo build --release

# Build with browser CDP support (optional feature)
cargo build --release --features browser_cdp

# Run (debug, with CDP feature)
cargo run --features browser_cdp

# Install release binary to /usr/local/bin/j
make install

# Format code
cargo fmt

# Lint (clippy, warnings as errors)
cargo clippy -- -D warnings

# Run tests
cargo test

# Run all tests including feature-gated ones
cargo test --all-features

# Type check without building
cargo check

# Pre-commit (format + lint + test)
make pre-commit

# Bump patch version in Cargo.toml
make bump-version
```

## Project Overview

**j-cli** is a fast macOS ARM64 CLI productivity tool written in Rust. Binary name is `j`. User data lives in `~/.jdata/` (configurable via `J_DATA_PATH`).

Core features: alias management, daily reports with git sync, TUI todo system, full TUI AI chat with streaming/tool calls, browser automation (Lite HTTP mode + optional CDP), extensible skill & hook systems, and an interactive REPL mode.

## Rust Edition & Toolchain

- **Edition**: 2024
- **Minimum Rust**: 1.93.1
- **Optional feature**: `browser_cdp` — enables Chromium automation via `chromiumoxide`

## Source Architecture

```
src/
├── main.rs              # Entry point: load config → interactive mode OR clap dispatch
├── cli.rs               # Clap derive definitions (Cli struct, SubCmd enum with 28 variants)
├── constants.rs         # VERSION and global constants
├── assets.rs            # rust-embed for bundled assets
├── config/
│   └── yaml_config.rs   # YamlConfig: 11 BTreeMap sections, YAML serde, file locking
├── interactive/         # REPL mode (rustyline + Tab completion + history)
├── tui/                 # Shared TUI editor widget (used by report + scripts)
├── util/                # Shared utilities
└── command/
    ├── mod.rs           # dispatch(SubCmd, &mut YamlConfig) router
    ├── handler.rs       # command_handlers! macro — generates handler structs
    ├── open.rs          # Alias opening logic (path/URL/script/browser/editor dispatch)
    ├── alias.rs         # set/rm/rename/modify alias commands
    ├── report.rs        # Daily report system (TUI editor, git sync, search)
    ├── todo/            # TUI todo app (app.rs state machine + ui.rs rendering)
    └── chat/            # AI chat subsystem (30+ files, see below)
```

### Command Dispatch Flow

```
main() → Cli::try_parse()
  ├── Ok(Some(SubCmd)) → command::dispatch() → handler for each subcommand
  ├── Ok(None) / Err(_) → command::open::handle_open() (alias fallback)
  └── No args → interactive::run_interactive() (REPL)
```

When clap fails to parse (e.g. `j chrome`), the tool falls back to alias-based open logic — this is intentional.

### Chat Module Architecture (`command/chat/`)

The chat module is the largest subsystem (~30 files). It follows a **Redux-like unidirectional data flow**:

```
KeyEvent → handler() → Action → app.update() → state mutation → render
StreamMsg → poll_stream_actions() → Vec<Action> → app.update() → state mutation → render
```

**Key structs in `app.rs`**:

| Struct | Role |
|--------|------|
| `ChatApp` | Top-level container, owns all state |
| `UIState` | ~55 frontend fields (input, cursor, scroll, popups, mode, caches) |
| `ChatState` | ~7 backend fields (agent_config, session messages, streaming content, loaded skills) |
| `ToolExecutor` | 9 fields + 8 methods (active tool calls, confirmation state, execution channels) |
| `AgentHandle` | 2 fields + 3 methods (stream_rx channel, cancellation token) |

**Action enum** (`app.rs`): ~85 variants covering input, popups, streaming, tool execution, ask-tool interaction, navigation, config editing, archiving, and UI management. The central `update(Action)` method dispatches all state mutations.

**Main event loop** (`handler/tui_loop.rs`): 5-phase architecture:
1. **Tick** — toast expiry
2. **Poll backend** — `poll_stream_actions()` from agent + `poll_tool_exec_results()` from tool threads → Actions
3. **Render** — `draw_chat_ui()` (only if `needs_redraw`)
4. **Collect input** — from InputThread channel, batch-process → Actions
5. **Side effects** — full-screen editors for system prompt / style

**ChatMode enum** (9 modes): Chat, SelectModel, Browse, Help, Config, ArchiveConfirm, ArchiveList, ToolConfirm, ToolToggle/SkillToggle.

**Chat submodule layout**:

```
command/chat/
├── app.rs               # ChatApp + UIState + ChatState + ToolExecutor + Action enum + update()
├── mod.rs               # run_chat() entry point
├── agent.rs             # Background async agent loop (tokio, streams LLM responses)
├── api.rs               # OpenAI SDK wrapper (async-openai), message format conversion
├── compact.rs           # Context window compaction (micro_compact + auto_compact)
├── permission.rs        # .jcli permission rule matching (allow/deny/allow_all)
├── skill.rs             # Skill loading from ~/.jdata/agent/skills/
├── hook.rs              # 4-level hook system (builtin/user/project/session), directory-based layout, 14 event types
├── storage.rs           # Persistence: agent_config.json, chat_history.json, archives
├── theme.rs             # 6 built-in themes (Dark, Light, Dracula, Gruvbox, Monokai, Nord)
├── render_cache.rs      # Per-message rendering cache with throttled streaming
├── input_thread.rs      # Spawns thread for crossterm event reading (non-blocking)
├── autocomplete.rs      # @skill and @file: popup completion
├── archive.rs           # Chat session archive/restore
├── constants.rs         # Chat-specific constants
├── ui_helpers.rs        # Shared UI helper functions
├── handler/             # Key event handlers per mode (all emit Actions)
│   ├── tui_loop.rs      # Main 5-phase event loop
│   ├── chat_handler.rs  # Chat mode key bindings
│   ├── browse_handler.rs
│   ├── config_handler.rs
│   └── ...
├── ui/                  # Rendering (all read-only, take &ChatApp)
│   ├── chat.rs          # Main chat UI drawing (messages, input, status)
│   ├── config.rs        # Config screen rendering
│   ├── archive.rs       # Archive list UI
│   └── ...
├── markdown/            # Markdown → ratatui spans (pulldown-cmark + syntax highlighting)
│   └── highlight.rs     # Language-specific syntax highlighting
└── tools/               # Tool implementations
    ├── mod.rs           # ToolRegistry + Tool trait definition
    ├── shell.rs         # ShellTool (Bash command execution)
    ├── file.rs          # Read/Write/Edit/Glob tools
    ├── grep.rs          # GrepTool (ripgrep-style search)
    ├── browser.rs       # BrowserTool (CDP + Lite dual mode)
    ├── web_fetch.rs     # WebFetchTool (HTTP + HTML→markdown)
    ├── web_search.rs    # WebSearchTool (search API)
    ├── ask.rs           # AskTool (interactive questionnaire)
    ├── background.rs    # BackgroundRun + CheckBackground
    ├── task/            # TaskCreate/List/Get/Update tools
    ├── compact.rs       # CompactTool (context compression trigger)
    └── ...
```

### Tool System

Tools implement the `Tool` trait:
```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;       // JSON Schema for LLM
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult;
    fn requires_confirmation(&self) -> bool;
    fn confirmation_message(&self, arguments: &str) -> String;
}
```

Tools are registered in `ToolRegistry`. The LLM generates `tool_calls` from the schema; the main thread shows confirmation UI (unless auto-allowed by `.jcli` permissions), then spawns OS threads for execution. Results flow back via channels.

### Concurrency Model

The chat system uses **channels** for cross-thread communication:
- `stream_rx`: Agent thread → Main (streaming chunks, tool call requests, done/error)
- `tool_result_tx`: Main → Agent thread (tool execution results)
- `tool_exec_rx`: Tool OS threads → Main (execution completion)
- `ask_request_rx`: AskTool → Main (interactive question requests)

Shared mutable state uses `Arc<Mutex<T>>` or `Arc<AtomicBool>` for: streaming content, queued tasks, pending user messages, and tool cancellation flags.

### Config Layer

- **`YamlConfig`** (`config/yaml_config.rs`): Main app config with 11 `BTreeMap<String,String>` sections (path, inner_url, outer_url, browser, editor, vpn, script, setting, log, report, version). Uses `serde_yaml` + `fs2` file locking.
- **`AgentConfig`** (`chat/storage.rs`): AI chat config (providers, active model, system prompt, tools_enabled, theme, etc.). Stored as JSON at `~/.jdata/agent/data/agent_config.json`.
- **`.jcli`** (project root): YAML permissions file for tool auto-approval rules. Program searches upward from cwd.

### Permissions (`.jcli`)

Rule format: `ToolName`, `Bash(prefix:*)`, `Write(path:dir/*)`, `WebFetch(domain:x)`. Deny rules take priority over allow rules. `allow_all: true` bypasses all confirmation.

### Skill System

Skills live in `~/.jdata/agent/skills/<name>/SKILL.md` with YAML frontmatter (name, description, argument-hint). The system prompt includes skill summaries; the AI calls `load_skill` to inject full instructions. Skills can have `references/` and `scripts/` subdirectories.

### Hook System

Three levels (user `~/.jdata/agent/hooks/<name>/HOOK.yaml`, project `.jcli/hooks/<name>/HOOK.yaml`, session via `register_hook` tool). Each hook is a directory containing `HOOK.yaml` (with `events` list, type, command/prompt, timeout, retry, on_error, filter) and optional script files. Shell hooks execute with cwd set to the hook directory. 14 events: pre/post_send_message, pre/post_llm_request, pre/post_tool_execution, post_tool_execution_failure, stop, pre/post_micro_compact, pre/post_auto_compact, session_start/end. Hooks receive JSON via stdin, return JSON modifications via stdout.

## Key Conventions

- All Chinese UI text is used throughout (error messages, help text, log output) — maintain this convention
- The `debug_log!` macro is used for verbose logging (controlled by `j log mode verbose/concise`)
- No unit tests currently exist in `src/`; testing is done via CLI integration
- The `.claude/settings.local.json` contains Claude Code tool permission rules — distinct from the `.jcli` runtime permissions
