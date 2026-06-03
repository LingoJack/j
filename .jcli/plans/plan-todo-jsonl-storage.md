# Todo 存储：从 JSON 改为 JSONL 格式

## 背景

当前 `src/command/todo/app/io.rs` 中的 `load_todo_list` 和 `save_todo_list` 使用 JSON 格式（`serde_json::to_string_pretty` / `serde_json::from_str`）将整个 `TodoList`（包含 `items: Vec<TodoItem>`）序列化为一个 JSON 文件 `todo.json`。

用户希望改为 JSONL（JSON Lines）格式：每行一个独立的 JSON 对象（即每条 `TodoItem` 占一行）。

## 改动范围

仅需修改 **1 个文件**：`src/command/todo/app/io.rs`

其他文件（`types.rs`、`state.rs`、`handler.rs` 等）中的 `load_todo_list()` / `save_todo_list()` 调用接口不变，无需修改。

## 具体改动

### `io.rs` 修改内容

1. **`todo_file_path()`**：文件名从 `todo.json` 改为 `todo.jsonl`
2. **`load_todo_list()`**：
   - 读取文件后，按行分割
   - 每行用 `serde_json::from_str::<TodoItem>()` 反序列化
   - 忽略空行和解析失败的行（打印警告日志）
   - 收集为 `Vec<TodoItem>` 构建 `TodoList`
3. **`save_todo_list()`**：
   - 遍历 `list.items`，每个 `TodoItem` 用 `serde_json::to_string()` 序列化为一行
   - 写入文件，行与行之间用 `\n` 分隔
4. **兼容性**（可选）：如果旧文件 `todo.json` 存在但 `todo.jsonl` 不存在，自动迁移一次（读取旧 JSON 格式，以 JSONL 格式写入新文件）。考虑到这是个人工具，这个步骤不是必须的，可以简化为直接切换。

### 日志消息更新

错误日志中的文件名从 `todo.json` 更新为 `todo.jsonl`。

## 优势

- **追加友好**：JSONL 天然支持追加写入，未来如果需要只 append 新条目，可以直接在文件末尾追加一行，无需重写整个文件
- **行级容错**：某一行损坏不影响其他行的读取
- **git diff 友好**：每条待办独立一行，版本控制 diff 更清晰
- **流式处理**：可以逐行读取，不需要一次性加载整个文件到内存解析

## 不受影响的部分

- `TodoItem` / `TodoList` 数据结构（`types.rs`）不变
- `TodoApp` 状态管理（`state.rs`）不变
- TUI 按键处理（`handler.rs`）不变
- 命令行入口（`command/todo/handler.rs`）不变
- UI 渲染（`ui.rs`）不变
