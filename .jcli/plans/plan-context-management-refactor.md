# 上下文管理模块重构规划

## 一、现状盘点

### 1.1 涉及上下文管理的文件分布

| 文件路径 | 职责 | 问题 |
|---------|------|------|
| `agent/compact.rs` | micro_compact + auto_compact 核心逻辑 | 与 tools/compact.rs 同名混淆 |
| `agent/window.rs` | 三阶段消息窗口选择 | 依赖 compact.rs 的 is_exempt_tool |
| `agent/message_compression.rs` | 其他 agent tool call 压缩 | 与 compact.rs 语义重叠 |
| `tools/compact.rs` | CompactTool（Layer 3 工具层） | 仅 55 行，职责简单但命名混淆 |
| `tools/plan.rs` | PlanMode 状态 + 工具定义 + ApprovalQueue | 混合状态管理与工具定义 |
| `constants.rs` | Compact/Window 相关常量 | 常量分散，难以统一维护 |

### 1.2 模块间依赖关系

```
agent_loop.rs
    ├── compact::micro_compact / auto_compact (Layer 1/2)
    ├── window::select_messages (调用前预处理)
    └── tool_processor::process_tool_calls (处理 plan_with_context_clear)

window.rs
    └── compact::is_exempt_tool (共享豁免工具判断)

tool_processor.rs
    ├── compact::CompactTool (检测 compact_requested)
    └── PlanDecision::ApproveAndClearContext (清空上下文信号)

plan.rs (tools)
    ├── PlanModeState (状态管理)
    ├── PlanApprovalQueue (审批队列)
    ├── EnterPlanModeTool / ExitPlanModeTool (工具定义)
    └── PLAN_MODE_WHITELIST (白名单常量)
```

### 1.3 核心问题诊断

**问题 1：职责分散**
- compact 核心逻辑在 `agent/` 目录，但 CompactTool 在 `tools/` 目录
- plan mode 状态管理与工具定义混在同一文件
- 上下文管理没有一个统一的"入口"

**问题 2：命名混淆**
- `agent/compact.rs` 与 `tools/compact.rs` 同名，但职责完全不同：
  - 前者：核心压缩引擎（600+ 行）
  - 后者：LLM 可调用的工具（55 行）
- `message_compression.rs` vs `compact.rs` 语义重叠

**问题 3：常量分散**
- `constants.rs` 中 COMPACT_* 和 WINDOW_* 常量分散
- `plan.rs` 中有 PLAN_MODE_WHITELIST magic value
- 难以统一维护和调整

**问题 4：耦合不清晰**
- `window.rs` 直接 import `compact::is_exempt_tool`
- `tool_processor.rs` 处理 plan 决策，但不直接引用 plan 模块

---

## 二、优化方案

### 2.1 新目录结构（遵循 name.rs + name/ 子目录风格）

```
src/command/chat/
  context.rs           # 模块定义 + 公开接口导出
  context/
    compact.rs         # 核心 compact 引擎（从 agent/compact.rs 移入）
    window.rs           # 消息窗口选择（从 agent/window.rs 移入）
    message_compress.rs # 其他 agent tool call 压缩（重命名）
    plan_state.rs       # PlanMode 状态 + ApprovalQueue（从 tools/plan.rs 分离）
    constants.rs        # 上下文相关常量统一管理
    
  agent/
    compact.rs          # 删除（移至 context/）
    window.rs           # 删除（移至 context/）
    message_compression.rs  # 删除（移至 context/）
    
  tools/
    compact.rs          # 重命名为 compact_tool.rs
    plan.rs             # 保留，只含工具定义（或重命名为 plan_tool.rs）
```

### 2.2 模块定义文件

```rust
// src/command/chat/context.rs
//! 上下文管理模块：统一管理消息压缩、窗口选择、Plan 状态等

pub mod compact;
pub mod constants;
pub mod message_compress;
pub mod plan_state;
pub mod window;

// 公开接口重导出
pub use compact::{micro_compact, auto_compact, CompactConfig, CompactResult};
pub use constants::*;
pub use message_compress::compress_other_agent_toolcalls;
pub use plan_state::{is_allowed_in_plan_mode, PlanApprovalQueue, PlanModeState};
pub use window::select_messages;
```

### 2.3 模块职责划分

#### `context/compact.rs` - 核心压缩引擎
- micro_compact（Layer 1：替换大 tool results）
- auto_compact（Layer 2：LLM 摘要压缩）
- is_exempt_tool（豁免工具判断，共享给 window）
- CompactConfig、CompactResult 等类型定义

#### `context/window.rs` - 消息窗口选择
- select_messages（三阶段优先级选择）
- MessageUnit 枚举及内部逻辑
- 依赖 compact::is_exempt_tool（同模块内部调用）

#### `context/message_compress.rs` - 其他 Agent 压缩
- compress_other_agent_toolcalls
- 针对 teammate/subagent 的广播消息压缩

#### `context/plan_state.rs` - Plan 状态管理
- PlanModeState（plan mode 全局状态）
- PlanApprovalQueue（teammate 审批队列）
- PendingPlanApproval（单条审批请求）
- PLAN_MODE_WHITELIST + is_allowed_in_plan_mode

#### `context/constants.rs` - 常量统一
```rust
// Compact 相关
pub const MICRO_COMPACT_BYTES_THRESHOLD: usize = 800;
pub const COMPACT_TOKEN_THRESHOLD: usize = 256 * 800;
pub const COMPACT_KEEP_RECENT: usize = 10;
pub const COMPACT_KEEP_RECENT_USER_MESSAGES: usize = 5;
pub const COMPACT_SUMMARY_MAX_TOKENS: u32 = 20000;
pub const COMPACT_TRUNCATE_MAX_CHARS: usize = 80_000;

// Window 相关
pub const WINDOW_KEEP_RECENT_MULTIPLIER: usize = 2;
pub const WINDOW_QUOTA_USER: f32 = 0.35;
pub const WINDOW_QUOTA_ASST_TEXT: f32 = 0.25;
pub const WINDOW_QUOTA_TOOL_GROUP: f32 = 0.40;

// Plan 相关
pub const PLAN_MODE_WHITELIST: &[&str] = &[...];
```

#### `tools/compact_tool.rs` - Compact 工具定义
- 仅保留 CompactTool 结构体和 Tool trait 实现
- 命名改为 CompactTool 避免与 context/compact 混淆

#### `tools/plan.rs` - Plan 工具定义（保留或重命名为 plan_tool.rs）
- EnterPlanModeTool
- ExitPlanModeTool
- 依赖 context::plan_state

---

## 三、迁移步骤

### Phase 1：创建 context 模块骨架
1. 创建 `src/command/chat/context/` 目录
2. 创建 `src/command/chat/context.rs` 模块定义
3. 创建 `context/constants.rs` 提取上下文常量

### Phase 2：迁移核心文件
1. `agent/compact.rs` → `context/compact.rs`
2. `agent/window.rs` → `context/window.rs`
3. `agent/message_compression.rs` → `context/message_compress.rs`

### Phase 3：分离 plan 状态
1. 从 `tools/plan.rs` 提取 PlanModeState / PlanApprovalQueue
2. 创建 `context/plan_state.rs`
3. 保留工具定义在 `tools/plan.rs`

### Phase 4：重命名 tools 目录文件
1. `tools/compact.rs` → `tools/compact_tool.rs`
2. 更新 tools.rs 的 mod 声明

### Phase 5：更新所有 import 路径
- agent_loop.rs: `use super::compact` → `use crate::command::chat::context`
- tool_processor.rs: 同上
- 其他引用文件（约 14 个）

### Phase 6：删除旧文件
- 删除 `agent/compact.rs`
- 删除 `agent/window.rs`
- 删除 `agent/message_compression.rs`
- 更新 `agent.rs` 导出

### Phase 7：更新 chat.rs 模块声明
```rust
// src/command/chat.rs
pub mod context;  // 新增
```

### Phase 8：验证与测试
1. 运行 `cargo build` 确保编译通过
2. 运行 `cargo clippy` 检查告警
3. 运行 `cargo test` 验证单元测试

---

## 四、影响范围

### 需要修改 import 的文件（约 14 个）

| 文件 | 当前 import | 新 import |
|------|------------|-----------|
| `agent/agent_loop.rs` | `super::compact` | `crate::command::chat::context` |
| `agent/tool_processor.rs` | `super::compact`, `tools::compact::CompactTool` | `context`, `tools::compact_tool::CompactTool` |
| `agent/config.rs` | `super::compact::CompactConfig` | `context::CompactConfig` |
| `handler/chat.rs` | `agent::window::select_messages` | `context::select_messages` |
| `oneshot.rs` | `agent::window::select_messages` | `context::select_messages` |
| `app/system_prompt.rs` | `agent::window::select_messages` | `context::select_messages` |
| `tools/definition.rs` | `agent::compact::EXEMPT_TOOLS` | `context::EXEMPT_TOOLS` |
| `tools/skill.rs` | `agent::compact::InvokedSkillsMap` | `context::InvokedSkillsMap` |
| `tools/sub_agent.rs` | `agent::compact::InvokedSkillsMap` | `context::InvokedSkillsMap` |
| `teammate/teammate_loop.rs` | `agent::compact::CompactConfig` | `context::CompactConfig` |
| `tools/derived_shared.rs` | `agent::compact::InvokedSkillsMap` | `context::InvokedSkillsMap` |
| `storage/config.rs` | `agent::compact::CompactConfig` | `context::CompactConfig` |
| `render/helpers.rs` | `agent::compact::EXEMPT_TOOLS` | `context::EXEMPT_TOOLS` |
| `ui/config/global.rs` | `agent::compact::CompactConfig` | `context::CompactConfig` |

---

## 五、预期收益

1. **职责清晰**：上下文管理统一在 `context/` 模块
2. **命名明确**：`compact.rs`（引擎）vs `compact_tool.rs`（工具）
3. **常量集中**：所有上下文常量在 `context/constants.rs`
4. **依赖简化**：外部模块只需 `use crate::command::chat::context`
5. **易于扩展**：新增上下文策略只需在 `context/` 中添加文件

---

## 六、风险评估

- **风险 1**：import 路径变更可能遗漏部分文件
  - 缓解：使用 `grep` 全量搜索旧路径
  
- **风险 2**：window.rs 与 compact.rs 的内部依赖需要调整
  - 缓解：同模块内调用，路径改为 `super::compact`
  
- **风险 3**：测试可能因路径变更失败
  - 缓解：逐步迁移，每步运行 `cargo test`

---

## 七、备选方案

### 方案 B：最小改动
- 仅重命名 `tools/compact.rs` → `tools/compact_tool.rs`
- 仅分离 `tools/plan.rs` 的状态管理部分到 `agent/plan_state.rs`
- 不移动 agent 目录文件

**优点**：改动量小，风险低
**缺点**：职责仍然分散，未解决核心问题

### 方案 C：保持现状，仅文档化
- 添加 README 说明各文件职责
- 不做代码移动

**优点**：零风险
**缺点**：命名混淆和职责分散问题持续存在

---

**推荐方案：方案 A（创建 context 模块，使用 name.rs + name/ 子目录风格）**