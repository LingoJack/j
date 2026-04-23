# 修复 TUI 编辑器搜索模式 Bug

## Bug 描述

在 TUI 编辑器（如 `j report` 打开的编辑器）中，`/` 搜索功能存在两个问题：

1. **按 `/` 进入的是命令面板而非搜索模式** — 用户按 `/` 期望直接进入搜索输入（类似 vim），但实际打开了命令面板列表。
2. **通过命令面板选择 "search" 后，按 Enter 只切回 Normal 模式，不跳转到匹配结果** — 搜索虽然实时执行了，但 Enter 时没有把光标移动到第一个匹配处。

## 根因分析

### 问题 1：`/` 键绑定错误

`vim.rs` 第 380 行：
```rust
Key::Char('/') => Transition::Mode(Mode::CommandPanel(String::new())),
```
应该改为进入 `Mode::Search`：
```rust
Key::Char('/') => Transition::Mode(Mode::Search(String::new())),
```

### 问题 2：Search 模式 Enter 不跳转

`vim.rs` `handle_search_mode`（第 414-420 行）中，Enter 直接返回 `Transition::Mode(Mode::Normal)`，没有触发跳转。由于 vim 层不持有 SearchState，需要在 `editor.rs` 的 `handle_input` 中处理这个 Transition，在模式从 Search 变为 Normal 时自动跳转到当前匹配。

## 修复方案

### 修改 1: `src/tui/editor_core/vim.rs`

将 Normal 模式下 `/` 键从进入 CommandPanel 改为进入 Search：
```rust
Key::Char('/') => Transition::Mode(Mode::Search(String::new())),
```

### 修改 2: `src/tui/editor_core/editor.rs`

在 `handle_input` 的 `Transition::Mode(new_mode)` 分支中，当从 Search 模式退出到 Normal 模式时，自动跳转到当前搜索匹配位置：

```rust
Transition::Mode(new_mode) => {
    // 从 Insert 模式退出时保存 undo 点
    if old_mode == Mode::Insert && new_mode != Mode::Insert {
        self.vim.push_snapshot(...);
    }
    // 从 Search 模式退出时跳转到当前匹配
    if matches!(old_mode, Mode::Search(_)) && new_mode == Mode::Normal {
        if let Some(m) = self.search.current_match() {
            self.buffer.set_cursor(m.line, m.start);
        }
    }
    self.vim.set_mode(new_mode);
    self.rebuild_wrap_cache();
}
```

## 影响范围

- `src/tui/editor_core/vim.rs` — 1 行修改
- `src/tui/editor_core/editor.rs` — ~5 行新增
- 不影响其他模块，仅改变 TUI 编辑器内的按键行为

## 验证方式

1. `cargo build` 编译通过
2. 在编辑器中按 `/` 应直接进入搜索输入框（而非命令面板）
3. 输入搜索词后按 Enter，光标应跳转到第一个匹配位置
4. 按 `n`/`N` 可继续跳转到下一个/上一个匹配
5. 命令面板仍可通过其他方式访问（如果需要的话可以保留其他入口）
