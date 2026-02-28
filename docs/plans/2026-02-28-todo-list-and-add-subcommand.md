# Todo List Output & Add Subcommand Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 `j todo` 命令新增 `--list`/`-l` 标志输出待办列表（Markdown checkbox 格式，通过 `md!` 宏渲染），并将快捷添加方式从 `j todo <content>` 改为 `j todo add <content>`。

**Architecture:** 修改 `cli.rs` 中 `Todo` 子命令，将 `content: Vec<String>` 改为使用带 `add` 子关键字和 `-l/--list` 标志的结构；在 `command/todo/mod.rs` 的 `handle_todo` 函数中增加路由逻辑；新增 `handle_todo_list()` 函数输出 Markdown 格式的待办列表。

**Tech Stack:** Rust, clap derive, ratatui (已有), `md!` 宏 (来自 `crate::md_render`)

---

### Task 1: 修改 `cli.rs` 中 Todo 子命令定义

**Files:**
- Modify: `src/cli.rs:131-136`

**Step 1: 修改 Todo 变体，添加 `-l/--list` 标志和 `content` 参数**

将原来的：
```rust
/// 待办备忘录（无参数进入 TUI 界面，有参数快速添加）
#[command(alias = "td")]
Todo {
    /// 待办内容（支持多个参数拼接）
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    content: Vec<String>,
},
```

替换为：
```rust
/// 待办备忘录（无参数进入 TUI 界面）
#[command(alias = "td")]
Todo {
    /// 列出所有待办（Markdown 格式输出）
    #[arg(short = 'l', long = "list")]
    list: bool,
    /// 子命令或内容（add <content> 快速添加）
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    content: Vec<String>,
},
```

**Step 2: 执行 `cargo check` 确认编译通过**

```bash
cargo check 2>&1 | head -30
```

Expected: 出现编译错误，提示 `handle_todo` 的调用方需要更新（因为参数签名改了）。

---

### Task 2: 修改 `command/mod.rs` 中的 dispatch 调用

**Files:**
- Read: `src/command/mod.rs`（先找到 `SubCmd::Todo` 的 dispatch 分支）
- Modify: `src/command/mod.rs`

**Step 1: 找到 dispatch 中的 Todo 分支**

运行：
```bash
grep -n "Todo" /Users/jacklingo/dev_custom/j/src/command/mod.rs
```

**Step 2: 更新 dispatch 分支，传入新的 `list` 参数**

原来类似：
```rust
SubCmd::Todo { content } => command::todo::handle_todo(&content, config),
```

改为：
```rust
SubCmd::Todo { list, content } => command::todo::handle_todo(list, &content, config),
```

**Step 3: `cargo check` 验证**

```bash
cargo check 2>&1 | head -30
```

Expected: 提示 `handle_todo` 函数签名不匹配。

---

### Task 3: 修改 `command/todo/mod.rs` — 更新 `handle_todo` 并新增 `handle_todo_list`

**Files:**
- Modify: `src/command/todo/mod.rs`

**Step 1: 更新 handle_todo 函数签名和逻辑**

将原来的 `handle_todo` 函数替换为：

```rust
/// 处理 todo 命令: j todo [-l] | j todo add <content>
pub fn handle_todo(list_flag: bool, content: &[String], config: &mut YamlConfig) {
    // -l / --list：输出待办列表
    if list_flag {
        handle_todo_list();
        return;
    }

    if content.is_empty() {
        run_todo_tui(config);
        return;
    }

    // 第一个参数是 "add"
    let first = content[0].as_str();
    if first == "add" {
        let rest = &content[1..];
        let text = rest.join(" ");
        let text = text.trim().trim_matches('"').to_string();
        if text.is_empty() {
            error!("⚠️ 内容为空，无法添加待办");
            return;
        }
        quick_add_todo(&text);
    } else {
        // 不识别的子命令，打印用法提示
        use crate::usage;
        usage!("j todo | j todo add <content> | j todo -l");
    }
}

/// 快速添加一条待办
fn quick_add_todo(text: &str) {
    let mut list = load_todo_list();
    list.items.push(TodoItem {
        content: text.to_string(),
        done: false,
        created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        done_at: None,
    });

    if save_todo_list(&list) {
        info!("✅ 已添加待办: {}", text);
        let undone = list.items.iter().filter(|i| !i.done).count();
        info!("📋 当前未完成待办: {} 条", undone);
    }
}

/// 输出待办列表（Markdown checkbox 格式，通过 md! 宏渲染）
fn handle_todo_list() {
    let list = load_todo_list();

    if list.items.is_empty() {
        info!("📋 暂无待办");
        return;
    }

    let total = list.items.len();
    let done_count = list.items.iter().filter(|i| i.done).count();
    let undone_count = total - done_count;

    let mut md = format!(
        "## 待办备忘录 — 共 {} 条 | ✅ {} | ⬜ {}\n\n",
        total, done_count, undone_count
    );

    for item in &list.items {
        let checkbox = if item.done { "[x]" } else { "[ ]" };
        md.push_str(&format!("- {} {}\n", checkbox, item.content));
    }

    crate::md!("{}", md);
}
```

**Step 2: 确保文件顶部有正确的 use 引用**

检查 `mod.rs` 顶部是否已有 `use crate::{error, info};`，如没有则添加 `usage` 宏的引用（通常是 `use crate::usage;` 或 `use crate::{error, info, usage};`）。

**Step 3: `cargo check` 验证**

```bash
cargo check 2>&1 | head -50
```

Expected: 编译成功，0 errors。

---

### Task 4: 更新交互模式解析器 `interactive/parser.rs`（若有 todo 相关处理）

**Files:**
- Read: `src/interactive/parser.rs` 或 `src/interactive.rs`（找到 todo/td 的解析分支）

**Step 1: 查找交互模式中 todo 的处理**

```bash
grep -n "todo\|Todo\|\"td\"" /Users/jacklingo/dev_custom/j/src/interactive.rs 2>/dev/null || \
grep -rn "todo\|Todo\|\"td\"" /Users/jacklingo/dev_custom/j/src/interactive/ 2>/dev/null | head -20
```

**Step 2: 更新交互模式解析**

找到类似 `"todo" | "td" => SubCmd::Todo { content: rest }` 的分支，改为：
```rust
"todo" | "td" => {
    // 解析 -l/--list 标志
    let list_flag = rest.iter().any(|s| s == "-l" || s == "--list");
    let content: Vec<String> = rest.into_iter()
        .filter(|s| s != "-l" && s != "--list")
        .collect();
    SubCmd::Todo { list: list_flag, content }
}
```

**Step 3: `cargo check`**

```bash
cargo check 2>&1 | head -30
```

Expected: 0 errors。

---

### Task 5: 更新 `assets/help.md` 文档

**Files:**
- Modify: `assets/help.md`

**Step 1: 找到并更新 Todo 部分**

定位到 `## 📋 待办备忘录` 部分（约第 133-192 行），将命令表格中：

```markdown
| `j todo 买牛奶` | 快速添加一条待办 |
```

改为：

```markdown
| `j todo add 买牛奶` | 快速添加一条待办 |
| `j todo -l` / `j td -l` | 输出待办列表（Markdown 渲染）|
```

---

### Task 6: 更新 `README.md` 文档

**Files:**
- Modify: `README.md`

**Step 1: 找到并更新 Phase 22 描述和 5.6 节**

在 Phase 22 行（约第 126 行），将 `快捷添加 \`j todo <content>\`` 改为 `快捷添加 \`j todo add <content>\``，并补充 `j todo -l` 输出列表的说明。

在 `5.6 待办备忘录` 节的入口方式部分，更新：
- 将 `j todo 买牛奶 — 快速添加一条待办` 改为 `j todo add 买牛奶 — 快速添加一条待办`
- 新增：`j todo -l / j td -l — 输出待办列表（Markdown 格式渲染）`

---

### Task 7: 编译并手动验证

**Step 1: 完整编译**

```bash
cargo build 2>&1 | tail -5
```

Expected: `Finished` 无错误。

**Step 2: 验证 -l 输出**

```bash
cargo run -- todo -l
```

Expected: 渲染后的 Markdown 待办列表，或 "暂无待办"。

**Step 3: 验证 add 子命令**

```bash
cargo run -- todo add "测试待办项"
cargo run -- todo -l
```

Expected: 第一条命令输出 `✅ 已添加待办: 测试待办项`，第二条输出列表中包含该项。

**Step 4: 验证无参数进入 TUI**

```bash
# 手动运行并确认进入 TUI 界面（Ctrl+C 退出）
cargo run -- todo
```

**Step 5: 提交**

```bash
git add src/cli.rs src/command/todo/mod.rs src/interactive.rs assets/help.md README.md
git commit -m "feat: todo add 子命令 + -l/--list 输出待办列表"
```
