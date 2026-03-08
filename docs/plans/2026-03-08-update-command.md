# Update Command Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a built-in `j update` command that allows j-cli to self-update from GitHub Releases, with intelligent detection of installation source.

**Architecture:**
- Compile-time embedding of installation source via `INSTALL_SOURCE` environment variable
- Use `self_update` crate for GitHub Releases download and binary replacement
- Cargo users get a helpful prompt to use `cargo install j-cli` instead

**Tech Stack:** `self_update` crate v0.42.0, GitHub Releases API, tar.gz archives

---

## Task 1: Add Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add self_update dependency**

Add to `Cargo.toml` dependencies section (after line 41):

```toml
self_update = { version = "0.42", default-features = false, features = ["archive-tar", "compression-flate2", "rustls"] }
```

**Step 2: Verify dependency compiles**

Run: `cargo check`
Expected: Compiles successfully without errors

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat: add self_update dependency for update command"
```

---

## Task 2: Add INSTALL_SOURCE Constant

**Files:**
- Modify: `src/constants.rs`

**Step 1: Add INSTALL_SOURCE constant**

Add after line 16 in `src/constants.rs`:

```rust
/// 安装来源（编译时嵌入）
/// - "github": 从 GitHub Release 安装
/// - "cargo": 从 crates.io 安装
/// - "unknown": 未知来源
pub const INSTALL_SOURCE: &str = option_env!("INSTALL_SOURCE").unwrap_or("cargo");
```

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/constants.rs
git commit -m "feat: add INSTALL_SOURCE compile-time constant"
```

---

## Task 3: Add Update Subcommand

**Files:**
- Modify: `src/cli.rs`

**Step 1: Add Update variant to SubCmd enum**

Add after the `Completion` variant (around line 208):

```rust
    // ========== 自更新 ==========
    /// 更新 j-cli 到最新版本
    Update {
        /// 仅检查版本，不更新
        #[arg(short, long)]
        check: bool,
    },
```

**Step 2: Add command keyword constant**

Modify: `src/constants.rs`

Add to the `cmd` module (after line 230):

```rust
    // 自更新
    pub const UPDATE: &[&str] = &["update", "up"];
```

Update the `all_keywords()` function to include `UPDATE`:

```rust
    pub fn all_keywords() -> Vec<&'static str> {
        let groups: &[&[&str]] = &[
            SET, REMOVE, RENAME, MODIFY, NOTE, DENOTE, LIST, CONTAIN, REPORT, REPORTCTL, CHECK,
            SEARCH, TODO, CHAT, CONCAT, TIME, LOG, CHANGE, CLEAR, VERSION, HELP, EXIT, COMPLETION,
            AGENT, SYSTEM, UPDATE,
        ];
        groups.iter().flat_map(|g| g.iter().copied()).collect()
    }
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/cli.rs src/constants.rs
git commit -m "feat: add Update subcommand definition"
```

---

## Task 4: Create Update Command Handler

**Files:**
- Create: `src/command/update.rs`

**Step 1: Create the update handler module**

Create file `src/command/update.rs`:

```rust
use crate::constants::{INSTALL_SOURCE, VERSION};
use colored::Colorize;

/// 处理 update 命令
pub fn handle_update(check_only: bool) {
    match INSTALL_SOURCE {
        "github" => handle_github_update(check_only),
        "cargo" => show_cargo_update_hint(),
        _ => show_unknown_source_hint(),
    }
}

/// 从 GitHub Releases 更新
fn handle_github_update(check_only: bool) {
    println!("{}", "检测到 GitHub Release 安装方式".green());
    println!("当前版本: {}", VERSION.cyan());

    if check_only {
        check_for_update();
    } else {
        perform_update();
    }
}

/// 检查是否有新版本
fn check_for_update() {
    println!("{}", "正在检查更新...".yellow());

    match self_update::backends::github::ReleaseList::configure()
        .repo_owner("LingoJack")
        .repo_name("j")
        .build()
    {
        Ok(release_list) => match release_list.fetch() {
            Ok(releases) => {
                if let Some(latest) = releases.first() {
                    let latest_version = latest.version.trim_start_matches('v');
                    println!("最新版本: {}", latest_version.cyan());

                    if latest_version == VERSION {
                        println!("{}", "已是最新版本".green());
                    } else {
                        println!("{}", "发现新版本！运行 'j update' 进行更新".yellow());
                    }
                } else {
                    println!("{}", "未找到发布版本".red());
                }
            }
            Err(e) => {
                println!("{} {}", "检查更新失败:".red(), e);
            }
        },
        Err(e) => {
            println!("{} {}", "配置更新源失败:".red(), e);
        }
    }
}

/// 执行更新
fn perform_update() {
    println!("{}", "正在更新...".yellow());

    let result = self_update::backends::github::Update::configure()
        .repo_owner("LingoJack")
        .repo_name("j")
        .bin_name("j")
        .show_download_progress(true)
        .current_version(VERSION)
        .build();

    match result {
        Ok(updater) => match updater.update() {
            Ok(status) => {
                println!(
                    "{} {}",
                    "更新成功！".green(),
                    format!("版本: {}", status.version()).cyan()
                );
            }
            Err(e) => {
                println!("{} {}", "更新失败:".red(), e);
                println!("请尝试手动更新:");
                println!("  curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh");
            }
        },
        Err(e) => {
            println!("{} {}", "配置更新失败:".red(), e);
        }
    }
}

/// 提示 cargo 用户使用正确的更新方式
fn show_cargo_update_hint() {
    println!("{}", "检测到你通过 cargo 安装了 j-cli".yellow());
    println!();
    println!("请使用以下命令更新:");
    println!("  {}", "cargo install j-cli".cyan());
    println!();
    println!("或强制从 GitHub 更新:");
    println!("  {}", "curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh".cyan());
}

/// 未知安装来源的提示
fn show_unknown_source_hint() {
    println!("{}", "无法确定安装来源".yellow());
    println!();
    println!("请选择以下方式更新:");
    println!();
    println!("1. cargo 方式:");
    println!("   {}", "cargo install j-cli".cyan());
    println!();
    println!("2. GitHub Release 方式:");
    println!("   {}", "curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh".cyan());
}
```

**Step 2: Export the module**

Modify: `src/command/mod.rs`

Add the module export:

```rust
pub mod update;
```

And add the public export:

```rust
pub use update::handle_update;
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/command/update.rs src/command/mod.rs
git commit -m "feat: implement update command handler"
```

---

## Task 5: Wire Up Handler Dispatch

**Files:**
- Modify: `src/command/handler.rs`

**Step 1: Add UpdateCmd handler**

Add to the `command_handlers!` macro (after the CompletionCmd entry):

```rust
    // ========== 自更新 ==========
    UpdateCmd { check: bool } => |self, _config| {
        crate::command::update::handle_update(self.check);
    },
```

**Step 2: Add SubCmd match arm**

Add to the `into_handler` match statement (after the Completion arm):

```rust
            SubCmd::Update { check } => Box::new(UpdateCmd { check }),
```

**Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/command/handler.rs
git commit -m "feat: wire up update command to handler dispatch"
```

---

## Task 6: Update GitHub Actions Workflow

**Files:**
- Modify: `.github/workflows/release.yml`

**Step 1: Add INSTALL_SOURCE environment variable**

Modify the Build step (line 27-33) to include `INSTALL_SOURCE`:

```yaml
      - name: Build
        run: cargo build --release --target aarch64-apple-darwin
        env:
          INSTALL_SOURCE: github
          CFLAGS: -march=armv8-a
          CXXFLAGS: -march=armv8-a
```

**Step 2: Verify workflow syntax**

Run: `cat .github/workflows/release.yml`
Expected: Shows the updated workflow with INSTALL_SOURCE

**Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "feat: add INSTALL_SOURCE env var to GitHub release build"
```

---

## Task 7: Update Documentation

**Files:**
- Modify: `assets/help.md`
- Modify: `README.md`

**Step 1: Add update command to help.md**

Add a new section in `assets/help.md` after the "系统设置" section:

```markdown
## 🔄 自更新

| 命令 | 说明 |
|------|------|
| `j update` | 更新到最新版本（仅限 GitHub Release 安装方式） |
| `j update --check` | 仅检查是否有新版本 |

> **注意**：
> - 通过 `cargo install j-cli` 安装的用户，请使用 `cargo install j-cli` 更新
> - 通过 GitHub Release 安装的用户，可使用 `j update` 自动更新
```

**Step 2: Update README.md**

Add `update` to the command table in `README.md`:

```markdown
| **更新** | `j update` | 自更新（仅 GitHub Release 安装） |
```

**Step 3: Commit**

```bash
git add assets/help.md README.md
git commit -m "docs: add update command documentation"
```

---

## Task 8: Add Interactive Mode Completion

**Files:**
- Modify: `src/interactive/completer.rs`

**Step 1: Add update to first-word completions**

Find the `complete_first_word` function and add `update` and `up` to the command list.

**Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/interactive/completer.rs
git commit -m "feat: add update command to interactive completion"
```

---

## Task 9: Test and Verify

**Step 1: Build locally**

Run: `cargo build --release`
Expected: Builds successfully

**Step 2: Test update command (cargo mode)**

Run: `./target/release/j update`
Expected: Shows "检测到你通过 cargo 安装了 j-cli" message

**Step 3: Test update --check**

Run: `./target/release/j update --check`
Expected: Shows cargo update hint

**Step 4: Test with INSTALL_SOURCE=github**

Run: `INSTALL_SOURCE=github cargo run --release -- update --check`
Expected: Shows "检测到 GitHub Release 安装方式" and checks for updates

**Step 5: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix: any test-driven fixes"
```

---

## Summary

| Task | Description | Files |
|------|-------------|-------|
| 1 | Add dependencies | `Cargo.toml` |
| 2 | Add INSTALL_SOURCE constant | `src/constants.rs` |
| 3 | Add Update subcommand | `src/cli.rs`, `src/constants.rs` |
| 4 | Create update handler | `src/command/update.rs`, `src/command/mod.rs` |
| 5 | Wire up dispatch | `src/command/handler.rs` |
| 6 | Update CI workflow | `.github/workflows/release.yml` |
| 7 | Update docs | `assets/help.md`, `README.md` |
| 8 | Add completion | `src/interactive/completer.rs` |
| 9 | Test and verify | - |
