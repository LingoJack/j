# Session 操作追踪（Session Ops Tracking）

## 目标

在 session 级别独立记录 `Edit`/`Write`/`Bash` 三类写入操作的审计日志，以 JSONL 格式追加到 `sessions/<id>/ops.jsonl`，无需 replay 整个 transcript 即可快速查询本次会话修改了哪些文件、执行了哪些命令。

## 数据结构设计

### 新增类型：`SessionOp`（`storage/types.rs`）

```rust
/// session 操作审计记录，追加到 sessions/<id>/ops.jsonl
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionOp {
    /// 操作类型
    pub op: SessionOpKind,
    /// 时间戳（epoch ms）
    pub timestamp_ms: u64,
    /// 是否执行失败
    pub is_error: bool,
}

/// 操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionOpKind {
    /// 文件编辑（Edit 工具）
    Edit {
        /// 被编辑的文件路径
        path: String,
    },
    /// 文件写入（Write 工具）
    Write {
        /// 被写入的文件路径
        path: String,
    },
    /// Shell 命令执行（Bash 工具）
    Bash {
        /// 执行的命令
        command: String,
    },
}
```

示例 JSONL：
```jsonl
{"op":{"kind":"edit","path":"src/main.rs"},"timestamp_ms":1700000000000,"is_error":false}
{"op":{"kind":"write","path":"src/new_file.rs"},"timestamp_ms":1700000000100,"is_error":false}
{"op":{"kind":"bash","command":"cargo build"},"timestamp_ms":1700000000200,"is_error":true}
```

## 存储层设计

### 新增路径（`SessionPaths`）

```rust
/// 操作审计文件：sessions/<id>/ops.jsonl
pub fn ops_file(&self) -> PathBuf {
    self.dir.join("ops.jsonl")
}
```

### 新增持久化函数（`session.rs`）

```rust
/// 追加一条操作审计记录到 ops.jsonl
pub fn append_session_op(session_id: &str, op: &SessionOp) -> bool { ... }

/// 读取 session 的所有操作审计记录
pub fn load_session_ops(session_id: &str) -> Vec<SessionOp> { ... }
```

- `append_session_op`：与 `append_session_event` 同样的 append-only 模式
- `load_session_ops`：逐行反序列化 `SessionOp`

### 新增 `persist.rs` 中的 save/load（可选）

不需要单独的 save/load，因为 ops.jsonl 本身就是 append-only 的。
`load_session_ops` 直接在 `session.rs` 中实现即可。

## 注入点设计

**核心问题**：在哪个位置拦截 tool_call 并提取 Edit/Write/Bash 的操作信息？

### 方案：在 `tool_processor.rs` 的 `process_tool_calls` 中提取

在 `process_tool_calls` 函数中，`tool_items: Vec<ToolCallItem>` 包含了所有工具调用的 name 和 arguments。在收到 tool_results 之后（已经知道 is_error），遍历 tool_items + tool_results 配对，提取感兴趣的记录并追加到 ops.jsonl。

**具体位置**：在 `log_tool_results(&tool_items, &tool_results);` 之后、图片处理之前。

```rust
// ★ 记录写入操作到 ops.jsonl
append_write_ops(&tool_items, &tool_results, ctx.session_id);
```

**提取逻辑**（新增辅助函数 `append_write_ops`）：

```rust
fn append_write_ops(
    tool_items: &[ToolCallItem],
    tool_results: &[ToolResultMsg],
    session_id: &str,
) {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    for (i, item) in tool_items.iter().enumerate() {
        let is_error = tool_results
            .iter()
            .find(|r| r.tool_call_id == item.id)
            .map(|r| r.is_error)
            .unwrap_or(true);

        let op_kind = match item.name.as_str() {
            "Edit" => extract_path_from_args(&item.arguments)
                .map(|path| SessionOpKind::Edit { path }),
            "Write" => extract_path_from_args(&item.arguments)
                .map(|path| SessionOpKind::Write { path }),
            "Bash" => extract_command_from_args(&item.arguments)
                .map(|cmd| SessionOpKind::Bash { command: cmd }),
            _ => None,
        };

        if let Some(op) = op_kind {
            let _ = append_session_op(session_id, &SessionOp {
                op,
                timestamp_ms: now_ms,
                is_error,
            });
        }
    }
}
```

**参数提取**（从 JSON arguments 中提取关键字段）：

```rust
/// 从 Edit/Write 工具的 arguments JSON 中提取 path 字段
fn extract_path_from_args(args: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(args).ok()
        .and_then(|v| v.get("path")?.as_str().map(String::from))
}

/// 从 Bash 工具的 arguments JSON 中提取 command 字段
fn extract_command_from_args(args: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(args).ok()
        .and_then(|v| v.get("command")?.as_str().map(String::from))
}
```

## 涉及修改的文件

| 文件 | 修改内容 |
|------|---------|
| `src/command/chat/storage/types.rs` | 新增 `SessionOp`、`SessionOpKind` 类型定义 |
| `src/command/chat/storage/session.rs` | `SessionPaths` 新增 `ops_file()`；新增 `append_session_op()`、`load_session_ops()` 函数 |
| `src/command/chat/storage/persist.rs` | 新增 `save_session_ops_state()`、`load_session_ops_state()`（可选，取决于是否需要在 `save_session_state` 中处理） |
| `src/command/chat/agent/tool_processor.rs` | 新增 `append_write_ops()`、`extract_path_from_args()`、`extract_command_from_args()` 辅助函数；在 `process_tool_calls` 中调用 |
| `src/command/chat/app/session_mgr.rs` | `clear_session()` 时 ops.jsonl 自然属于旧 session 目录，无需额外清理 |

## 不需要修改的文件

- `session_mgr.rs`：`save_session_state()` 不需要处理 ops（append-only，无需最终快照）
- `session_mgr.rs`：`restore_session_state()` 不需要恢复 ops（纯审计日志）
- `session_mgr.rs`：`clear_session()` 创建新 session 时新目录为空，无需清理

## 边界情况

1. **Teammate / SubAgent 的 ops**：它们的 transcript 是独立的。本次实现只追踪主 session 的 ops。后续可扩展到 `teammates/<name>/ops.jsonl`。
2. **compact**：ops.jsonl 不参与 compact，保留完整历史。
3. **Clear session**：旧 session 目录保留 ops.jsonl；新 session 目录从空开始。
4. **JSON 解析失败**：工具参数可能不是合法 JSON（极端情况），`extract_*` 返回 None 则跳过。
5. **并发安全**：`append_session_op` 使用 append-only `writeln!`，与 `append_session_event` 一致，POSIX 下对小于 PIPE_BUF 的写入是原子的。

## 实施步骤

1. 在 `types.rs` 中定义 `SessionOp` 和 `SessionOpKind`
2. 在 `session.rs` 的 `SessionPaths` 中添加 `ops_file()` 方法
3. 在 `session.rs` 中实现 `append_session_op()` 和 `load_session_ops()`
4. 在 `tool_processor.rs` 中实现 `append_write_ops()` 及参数提取辅助函数
5. 在 `process_tool_calls` 中调用 `append_write_ops`
6. 运行 `cargo fmt` 和 `cargo clippy` 确保代码质量
