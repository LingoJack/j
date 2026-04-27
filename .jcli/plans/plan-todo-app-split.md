# 拆分 command/todo/app.rs（867行）

## 现状

`src/command/todo/app.rs` 共 867 行，包含 5 个不同职责区域。

## 目标结构

采用 `app.rs` + `app/` 子目录组织方式（遵循项目弃用 `mod.rs` 的规范）：

```
src/command/todo/
├── app.rs              ← 模块入口，pub mod 声明 + re-export，约 ~30 行
├── app/
│   ├── types.rs        ← 数据结构：TodoItem, TodoList, AppMode, CMD_POPUP_ITEMS，约 ~60 行
│   ├── io.rs           ← 文件路径与数据读写：todo_dir, todo_file_path, load_todo_list, save_todo_list，约 ~55 行
│   ├── state.rs        ← TodoApp 结构体定义 + impl（new, filtered_indices, toggle_done 等状态方法），约 ~240 行
│   ├── handler.rs      ← 所有 handle_* 按键处理函数（从 app.rs 迁移，非 handler.rs 中已有的 TUI 入口），约 ~270 行
│   └── util.rs         ← 工具函数：display_width, truncate_to_width, copy_to_clipboard，约 ~70 行
├── constant.rs         ← 不变
├── handler.rs          ← 不变
├── ui.rs               ← 不变
└── todo.rs             ← 不变（模块根）
```

## 拆分详情

### 1. `app/types.rs`
- `CMD_POPUP_ITEMS` 常量
- `TodoItem` struct + derive
- `TodoList` struct + derive
- `AppMode` enum

### 2. `app/io.rs`
- `todo_dir()`
- `todo_file_path()`
- `load_todo_list()`
- `save_todo_list()`

### 3. `app/state.rs`
- `TodoApp` struct 定义
- `impl Default for TodoApp`
- `impl TodoApp`（所有方法：new, filtered_cmd_items, is_dirty, filtered_indices, selected_real_index, move_down, move_up, toggle_done, add_item, confirm_edit, delete_selected, move_item_up, move_item_down, toggle_filter, save）

### 4. `app/handler.rs`
注意：与已有的 `todo/handler.rs`（TUI 入口）不冲突，这是 `todo/app/handler.rs`
- `handle_normal_mode()`
- `handle_input_mode()`
- `handle_confirm_delete()`
- `handle_help_mode()`
- `handle_confirm_cancel_input()`
- `handle_command_popup_mode()`
- `handle_confirm_report()`

### 5. `app/util.rs`
- `display_width()`
- `truncate_to_width()`
- `copy_to_clipboard()`

### 6. `app.rs`（模块入口）
```rust
mod types;
mod io;
mod state;
mod handler;
mod util;

pub use types::*;
pub use io::*;
pub use state::TodoApp;
pub use handler::*;
pub use util::*;
```

## 对其他文件的影响

- `handler.rs`（todo/handler.rs）已通过 `use super::app::{...}` 导入，re-export 后无需修改
- `ui.rs` 已通过 `use super::app::{...}` 导入，re-export 后无需修改
- 无需修改任何外部消费者

## 验证

- `cargo build` 编译通过
- `cargo clippy -- -D warnings` 无告警
- `cargo fmt` 格式化
