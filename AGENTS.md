# Repository Guidelines

## Project Structure & Module Organization
`src/` contains the Rust CLI and TUI code. Major areas include `src/command/` for subcommands, `src/interactive/` for the REPL, `src/tui/` for shared terminal UI, `src/config/` for config loading, and `src/util/` for helpers. `web/` is a separate React + TypeScript + Vite site for docs and landing pages. Supporting assets live in `assets/`, Swift helpers in `helpers/`, release patches in `patches/`, and reference material in `docs/` and `.github/`.

## Build, Test, and Development Commands
Use `make build` or `cargo build` for the CLI. Run `make run` for the Rust app with the `browser_cdp` feature and bundled remote assets. Use `make test` for the default Rust test suite and `make test-all` for all features. `make fmt`, `make lint`, and `make check` cover formatting, Clippy, and compile checks. For the frontend, run `cd web && npm run dev` for local development, `npm run build` for production output, and `npm run lint` or `npm run format:check` before submitting changes.

## Coding Style & Naming Conventions
Rust follows `rustfmt` defaults: 4-space indentation, `snake_case` for functions/modules, `CamelCase` for types, and small focused modules. Keep command logic under `src/command/<area>/` when it grows beyond a single file. Frontend code follows Prettier in `web/.prettierrc`: 2 spaces, single quotes, no semicolons, 100-column width. Use `PascalCase` for React components and `camelCase` for hooks and utilities.

## Testing Guidelines
Rust tests are currently inline unit tests under `#[cfg(test)]` blocks, for example in `src/command/chat/sandbox.rs` and `src/command/chat/permission.rs`. Name tests `test_<behavior>` and keep them close to the code they verify. Run `cargo test` locally before opening a PR. There is no frontend test runner configured yet, so changes in `web/` should at minimum pass `npm run build` and `npm run lint`.

## Commit & Pull Request Guidelines
Recent commits use short timestamp-based messages such as `更新: 2026-04-07 20:44:45`; for contributor work, prefer concise imperative messages that describe the change clearly. Keep commits scoped to one concern. PRs should include a short summary, affected areas (`src/`, `web/`, assets), manual verification steps, and screenshots for visible `web/` changes. Tag releases are created from versioned `v*` tags and published through `.github/workflows/release.yml`.
