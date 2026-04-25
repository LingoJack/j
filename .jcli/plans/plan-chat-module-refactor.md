# Chat 模块代码规范重构计划

> 对照 `AGENT.md` 8 大规范维度，对 `src/command/chat/` 进行合规性重构。
> 按优先级分批次执行，每批完成后运行 `cargo check` + `cargo clippy` 验证。

---

## 第一批：error.rs 改用 thiserror（P1，独立文件）

### 1.1 `error.rs` — 使用 `thiserror` 派生替代手写 `Display`/`Error`

**当前问题**：项目已引入 `thiserror = "2"`，但 `error.rs` 手动实现了 `Display` 和 `std::error::Error`，违反"编写库建议使用 thiserror"规范。

**改动内容**：
- 添加 `#[derive(thiserror::Error)]` 到 `ChatError`
- 为每个变体添加 `#[error("...")]` 属性，替代手动 `Display` impl
- 删除手动 `impl std::fmt::Display for ChatError`
- 删除手动 `impl std::error::Error for ChatError`
- 保留 `display_message()`、`From<LlmError>`、`From<reqwest::Error>` 等逻辑不变
- 保留所有辅助函数和测试不变

---

## 第二批：路径简化 — 消除逻辑代码中的内联长路径（P0）

### 2.1 `app/chat_app.rs` — 8 处内联长路径

**改动内容**：在文件顶部 `use` 区添加导入，替换逻辑代码中的内联长路径。

需要添加的 `use` 语句：
```rust
use crate::command::chat::tools::todo::TodoManager;
use crate::command::chat::remote::bridge::WsBridge;
use crate::command::chat::tools::derived_shared::AgentContextConfig;
use crate::command::chat::context::compact::InvokedSkillsMap;
use crate::command::chat::context::compact;
use crate::command::chat::agent::thread_identity;
use crate::command::chat::ui::quotes;
use crate::command::chat::app::types::PlanDecision;
use crate::command::chat::storage::{load_session_meta_file, save_session_meta_file};
```

涉及修改的行：
- L65: 结构体字段类型 `TodoManager`
- L81: 结构体字段类型 `WsBridge`
- L90: 结构体字段类型 `AgentContextConfig`
- L116: 结构体字段类型 `InvokedSkillsMap`
- L202: `compact::new_invoked_skills_map()`
- L210: `SessionPaths::new()`
- L248: `AgentContextConfig { ... }` 字面量
- L277/282/288/293: 工具构造（SubAgentTool/TeammateTool/SendMessageTool/IgnoreMessageTool）
- L444: `thread_identity::current_agent_name()`
- L536: `quotes::quotes_count()`
- L2035: `PlanDecision::Reject`
- L2135/2138: `load_session_meta_file`/`save_session_meta_file`

### 2.2 `tools/sub_agent.rs` — 3 处内联长路径

添加 `use` 并替换：
- L77: `AgentContextConfig` 参数类型
- L516: `select_messages()`
- L524: `micro_compact()`

### 2.3 `teammate/teammate_loop.rs` — 2 处内联长路径

添加 `use` 并替换：
- L229: `select_messages()`
- L237: `micro_compact()`

### 2.4 其他文件散落内联长路径

- `app/system_prompt.rs:89` — `agent_md::load_agent_md()`
- `app/tool_executor.rs:348` — `permission::generate_allow_rule()`
- `app/archive.rs:78` — `archive::list_archives()`
- `app/message.rs:226` — `ModelProvider` 类型
- `agent/api.rs:512` — `API_ERROR_BODY_MAX_LEN`
- `tools/derived_shared.rs:381` — `LlmClient` 类型

---

## 第三批：类型设计改进 — 添加缺失的 Trait derive + pub(crate) 缩窄（P1）

### 3.1 `app/types.rs` — 添加缺失 derive

- `StreamMsg` (L31): 添加 `#[derive(Debug, Clone)]`（注意 `ToolCallItem` 和 `ChatError` 都已 derive `Clone`）
- `ToolCallStatus` (L73): 添加 `Clone` derive
- `ToolResultMsg` (L85): 添加 `Clone` derive
- `CompletedToolResult` (L98): 添加 `Clone, PartialEq` derive
- `AskAnswer` (L121): 添加 `#[derive(Debug, Clone, PartialEq)]`
- `AskRequest` (L131): 添加 `Clone` derive

### 3.2 缩窄 `pub` 字段为 `pub(crate)` — `app/types.rs`

以下结构体仅在 crate 内部使用，字段全部改为 `pub(crate)`：
- `ToolCallStatus` 的所有字段
- `ToolResultMsg` 的所有字段
- `CompletedToolResult` 的所有字段
- `AskOption` 的字段
- `AskQuestion` 的字段
- `AskRequest` 的字段

### 3.3 其他结构体缺失 derive

- `infra/hook.rs` L565 `BuiltinHook`: 添加 `#[derive(Debug)]`
- `infra/hook.rs` L851 `HookEntry`: 添加 `#[derive(Debug, Clone)]`
- `teammate/manager.rs` L243 `FileLockGuard`: 添加 `#[derive(Debug)]`
- `teammate/teammate_loop.rs` L31 `TeammateLoopConfig`: 添加 `#[derive(Debug)]`

---

## 第四批：oneshot.rs — unwrap 消除（P0）

### 4.1 替换 `lock().unwrap()` 为已有的 `safe_lock()`

项目已有 `crate::util::safe_lock(mutex, context)` 工具函数（`src/util/sync.rs`），遇到 poison 自动 recover 并记录日志。

**替换 6 处** `Mutex::lock().unwrap()` 为 `safe_lock()`：
- L591: `streaming_content.lock().unwrap()` → `safe_lock(&streaming_content, "streaming_content")`
- L1070: 同上
- L1116: 同上
- L1151: `context_messages.lock().unwrap()` → `safe_lock(&context_messages, "context_messages")`
- L1176: 同上
- L1197: 同上

需要在文件顶部添加 `use crate::util::safe_lock;`。

---

## 第五批：魔法数字提取为常量（P2）

### 5.1 在 `constants.rs` 中添加新常量

```rust
// --- Remote ---
pub const WS_BUFFER_SIZE: usize = 4096;
pub const WS_CHANNEL_CAPACITY: usize = 256;
pub const WS_CONNECT_SETTLE_MS: u64 = 100;
pub const WS_LOG_TRUNCATE_LEN: usize = 200;
pub const SOCKET_LISTEN_BACKLOG: i32 = 128;

// --- UI ---
pub const POPUP_MAX_VISIBLE_ITEMS: usize = 15;
pub const TUI_RENDER_THROTTLE_MS: u64 = 150;

// --- Error ---
pub const ERROR_MSG_TRUNCATE_LEN: usize = 150;

// --- Channel ---
pub const TOOL_RESULT_CHANNEL_CAPACITY: usize = 16;
```

### 5.2 在各文件中替换魔法数字为新常量

- `remote/server.rs`: 替换 `4096`、`100`、`200` 等
- `remote/bridge.rs`: 替换 `256`
- `remote/setup.rs`: 替换 `128`、`500`
- `error.rs`: 替换 `150`
- `handler/tui_loop.rs`: 替换 `150`、`10`
- `app/agent_handle.rs`: 替换 `16`
- `ui/popup.rs`: 替换 `15`

---

## 第六批：文档注释补充（P1）

### 6.1 为缺失文档的 `pub` 成员添加 `///` 注释

优先补充核心 API（约 116 个缺失项中最关键的）：
- `tools.rs` 中 27 个工具名称常量
- `ui/palette.rs` 中 8 个调色板常量
- `app/system_prompt.rs` 中 4 个公开 API
- 各工具模块的 `pub const NAME`

---

## 执行策略

1. **每批独立提交**，每批完成后执行：
   ```bash
   cargo fmt
   cargo clippy --all-targets --all-features 2>&1 | head -50
   cargo test --lib -- chat:: 2>&1 | tail -20
   ```
2. **严格按批次顺序**，因为后续批次可能依赖前面的改动
3. **不涉及功能变更**，纯粹是合规性重构，不改变任何运行时行为
4. **不拆分超长函数**（如 `update` 1520行），这属于架构级重构，需要单独规划

---

## 不在本轮重构范围内

以下项目属于架构级改动，建议单独规划：
- `app/chat_app.rs::update` 函数拆分（1520行 → 需要设计 Action 分组架构）
- `agent/agent_loop.rs::run_main_agent_loop` 拆分（1169行）
- `markdown/parser.rs::markdown_to_lines` 拆分（847行）
- 参数超过 4 个的函数封装为 Config 结构体
