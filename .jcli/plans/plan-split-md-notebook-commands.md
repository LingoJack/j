# 拆分 `j md` 和 `j notebook` 命令

## 目标

将当前混在一起的 `j md` 和 `j notebook` 拆分为两个完全独立的命令：

- **`j md [file_path]`** — Markdown 文件编辑器，类似 `vim`，在当前目录（或指定路径）编辑文件
- **`j notebook`** / **`j nb`** — 集成笔记本管理（TUI、list、search、delete 等全部子命令）

## 变更清单

### 1. CLI 定义 (`src/cli.rs`)

**修改 `Md` 变体：**
- 描述改为：`Markdown 文件编辑器（类似 vim）`
- 参数改为可选的单个文件路径 `file: Option<String>`（不再是 `Vec<String>`）
- 去掉 alias `markdown` 或保留都行（用户确认）

**修改 `Notebook` 变体：**
- 取消 `hide = true`，使其正式可见
- 描述改为：`笔记本管理（TUI 浏览、搜索、增删改）`
- 保持 `alias = "nb"`

### 2. 命令分发 (`src/command/handler.rs`)

**`MdCmd`：**
- 改为调用新的 `crate::command::notebook::handle_md(file)` 函数
- `handle_md` 只做一件事：用 Markdown 编辑器打开指定文件路径

**`NotebookCmd`：**
- 保持调用 `crate::command::notebook::handle_notebook(&args)`
- `handle_notebook` 只处理笔记本相关逻辑（去掉文件编辑分支）

### 3. Notebook handler (`src/command/notebook/handler.rs`)

**新增 `handle_md` 函数：**
- 从当前 handler 中提取 `edit_file_with_editor` + `expand_tilde` 逻辑
- `handle_md(file: Option<&str>)`：
  - `None` → 提示用法（或打开当前目录下的 README.md / 新文件）
  - `Some(path)` → 调用 `edit_file_with_editor(path)`

**修改 `handle_notebook` 函数：**
- 去掉 `_ =>` 分支中的 `is_file_path` 判断和 `edit_file_with_editor` 调用
- `_ =>` 分支改为 `edit_note_with_editor(&joined)`（只处理笔记本笔记）
- 或者直接报错提示使用 `j md` 来编辑文件

**删除 `is_file_path` 函数**（不再需要）

### 4. 笔记本用法提示 (`src/command/notebook/app/io.rs`)

- `edit_note_with_editor` 保持不变（仅供 notebook 内部使用）

### 5. 常量 (`src/constants.rs`)

- 无需修改，`notebook_action` 常量继续由 `handle_notebook` 使用

## 文件变更汇总

| 文件 | 变更类型 |
|---|---|
| `src/cli.rs` | 修改 `Md` 和 `Notebook` 变体定义 |
| `src/command/handler.rs` | 修改 `MdCmd` 分发逻辑 |
| `src/command/notebook/handler.rs` | 新增 `handle_md`，修改 `handle_notebook`，删除 `is_file_path` |
| `src/command/notebook.rs` | 导出新增的 `handle_md` |

## 用户使用方式对比

| 操作 | 现在 | 改后 |
|---|---|---|
| 编辑当前目录文件 | `j md ./file.md`（需带路径标记） | `j md file.md` |
| 编辑任意路径文件 | `j md ~/notes/a.md` | `j md ~/notes/a.md` |
| 编辑笔记本笔记 | `j md mynote`（靠 `is_file_path` 猜测） | `j notebook mynote` 或 TUI 内操作 |
| 打开笔记本 TUI | `j md`（无参数） | `j notebook`（无参数） |
| 笔记本 list/search/delete | `j md list` 等 | `j notebook list` 等 |
| `j md` 无参数 | 打开笔记本 TUI | 提示用法或创建新文件 |
