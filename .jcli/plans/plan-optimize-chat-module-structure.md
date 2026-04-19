# Chat 模块文件结构优化方案

## 一、现状分析

### 当前目录结构（114 个 .rs 文件）

```
src/command/chat/
├── chat.rs                 # 根模块：pub mod + 15 个 pub use 兼容重导出
├── agent/                  # Agent 核心循环
│   ├── agent_loop.rs       # 1396 行 — 主循环 + 重试 + 工具调用处理
│   ├── api.rs              # 635 行  — OpenAI API 交互
│   ├── compact.rs          # 上下文压缩
│   ├── config.rs           # AgentLoopConfig / AgentLoopSharedState
│   ├── thread_identity.rs  # 线程标识
│   └── window.rs           # 498 行  — 消息优先级窗口选择
├── app/                    # 应用状态（Elm 架构）
│   ├── action.rs           # Action 枚举定义
│   ├── agent_handle.rs     # Agent 线程句柄
│   ├── chat_app.rs         # 3773 行 — ChatApp 结构体 + update() + 所有业务方法
│   ├── chat_state.rs       # 后端数据状态
│   ├── tool_executor.rs    # 工具执行状态机
│   ├── types.rs            # StreamMsg / ToolResultMsg / PlanDecision 等
│   └── ui_state.rs         # 前端 UI 状态
├── handler/                # TUI 事件处理器
├── infra/                  # 基础设施（hook/skill/command/sandbox/archive）
├── input/                  # 输入处理
├── markdown/               # Markdown 解析渲染
├── permission/             # 权限系统
├── remote/                 # WebSocket 远程控制
├── render/                 # UI 渲染辅助
├── storage.rs              # 1386 行 — 数据类型 + 配置 + 持久化 + 路径
├── tools/                  # 工具实现（~30 个文件）
├── ui/                     # UI 组件
├── agent_md.rs
├── constants.rs
├── error.rs
├── oneshot.rs
└── teammate/
    ├── manager.rs
    └── teammate_loop.rs
```

### 核心问题

| # | 问题 | 影响的文件 | 行数 | 严重度 |
|---|------|-----------|------|--------|
| P1 | **chat_app.rs 上帝对象**：`ChatApp` 承担消息发送、流式轮询、会话管理、归档、ask 模式、toast、配置编辑、session 持久化/恢复等所有职责 | `app/chat_app.rs` | 3773 | 高 |
| P2 | **storage.rs 职责混杂**：数据结构（ChatMessage/ToolCallItem/SessionEvent）、配置（AgentConfig/ModelProvider）、文件 I/O（load/save）、路径逻辑（SessionPaths）全部堆在一个文件 | `storage.rs` | 1386 | 高 |
| P3 | **agent_loop.rs 过大**：流式读取 + 重试 + 工具调用处理 + compact + hook 全部在一个函数体内，嵌套 4 层循环 | `agent/agent_loop.rs` | 1396 | 中 |
| P4 | **send_message_internal 与 wake_from_teammate_inbox 代码重复**：两处构造 system_prompt_fn、AgentLoopConfig、MainAgentHandle::spawn 的逻辑几乎完全相同 | `app/chat_app.rs` | ~200行重复 | 中 |
| P5 | **chat.rs 过度重导出**：15 个 `pub use` 为向后兼容保留旧路径，增加认知负担 | `chat.rs` | 46行 | 低 |

---

## 二、优化方案

### 阶段 1：拆分 `storage.rs`（P2 — 高收益、低风险）

**目标**：将 `storage.rs` 按职责拆分为 4 个独立文件，消除"万能 storage"。

```
storage.rs (1386行) 拆分为：
  storage/
    mod.rs          — 模块声明 + 公开接口重导出
    types.rs        — ChatMessage, ToolCallItem, ImageData, ChatSession, SessionEvent
    config.rs       — AgentConfig, ModelProvider, load/save 配置相关
    session.rs      — SessionPaths, session CRUD, JSONL 读写, 元数据管理
    persist.rs      — TeammateSnapshotPersist, SubAgentSnapshotPersist 等
                      持久化类型 + save/load 辅助函数
```

**关键原则**：
- `mod.rs` 中使用 `pub use` 重导出所有公开类型，外部模块无需修改任何 `use` 路径
- 数据类型定义与对应的 `impl` 块保持在同一文件中（遵循 AGENT.md 规范）

### 阶段 2：拆分 `chat_app.rs`（P1 — 最高收益、需谨慎）

**目标**：将 `ChatApp` 的方法按功能域拆分到独立文件，主文件只保留结构体定义和 `new()`。

```
app/chat_app.rs (3773行) 拆分为：
  app/
    mod.rs              — ChatApp 结构体定义 + new() + update() 调度
    action.rs           — Action 枚举（已有）
    agent_handle.rs     — MainAgentHandle（已有）
    chat_state.rs       — ChatState（已有）
    tool_executor.rs    — ToolExecutor（已有）
    types.rs            — StreamMsg 等（已有）
    ui_state.rs         — UIState（已有）
    
    message.rs          — send_message, send_message_internal, wake_from_teammate_inbox
                          提取公共的 agent_spawn_context() 消除重复
    stream_poll.rs      — poll_stream_actions, init_ask_mode, ask_submit_answer
    session_mgr.rs      — persist_new_messages, save_session_state, 
                          restore_session_state, clear_runtime_state, clear_session
                          restore_teammate_transcripts, switch_session 相关
    config_edit.rs      — config 相关的 Action 处理辅助方法
    archive.rs          — start_archive_confirm, do_archive, do_restore 等
    browse.rs           — browse_filtered_indices, browse_jump_to_first_match
    system_prompt.rs    — apply_static_placeholders, StaticPlaceholderValues
                          build_current_system_prompt, build_api_messages
```

**关键原则**：
- 拆分后通过 `impl ChatApp` 在各文件中扩展方法，不需要修改外部调用方
- `update()` 方法保留在 `mod.rs` 中（它是 Redux reducer，是唯一的调度入口）
- 提取 `agent_spawn_context()` 辅助函数消除 `send_message_internal` 和 `wake_from_teammate_inbox` 的重复代码（P4）

### 阶段 3：拆分 `agent_loop.rs`（P3 — 中等收益）

**目标**：将 `run_main_agent_loop` 中的大块逻辑提取为独立函数/子模块。

```
agent/agent_loop.rs (1396行) 拆分为：
  agent/
    agent_loop.rs       — run_main_agent_loop() 保留主循环框架
    retry.rs            — RetryPolicy, retry_policy_for(), backoff_delay_ms()
    stream_reader.rs    — 流式响应读取 + chunk 处理逻辑
    tool_processor.rs   — process_tool_calls(), ToolCallContext, ToolCallResult
                          drain_pending_user_messages, push_ui, flush_streaming_as_message
```

**关键原则**：
- `run_main_agent_loop` 变为纯编排函数（调用子模块函数）
- 重试策略独立为 `retry.rs`，便于单独测试

### 阶段 4：清理 `chat.rs` 根模块重导出（P5 — 低收益）

**目标**：清理向后兼容的 `pub use` 重导出，更新所有内部引用。

- 审计所有 `pub use` 的外部使用情况
- 将直接使用旧路径（如 `crate::command::chat::hook`）的调用方更新为新路径（`crate::command::chat::infra::hook`）
- 保留确实需要跨模块使用的重导出

---

## 三、执行计划

### 优先级排序

| 顺序 | 阶段 | 风险 | 预计改动文件数 | 理由 |
|------|------|------|---------------|------|
| 1 | 阶段 1：拆分 storage.rs | 低 | ~5 | 纯文件拆分 + pub use，外部无需改动 |
| 2 | 阶段 2：拆分 chat_app.rs | 中 | ~12 | 方法拆分，需确保 impl 块正确 |
| 3 | 阶段 3：拆分 agent_loop.rs | 中 | ~5 | 函数提取，逻辑不变 |
| 4 | 阶段 4：清理重导出 | 低 | ~20+ | 搜索替换为主 |

### 每个阶段的验证标准

1. `cargo build` 编译通过
2. `cargo clippy` 无新增告警
3. `cargo fmt` 格式化通过
4. 现有测试全部通过（`cargo test`）
5. 无任何外部 `use` 路径需要修改（阶段 1-3）

---

## 四、不纳入本次优化的部分

以下部分虽然也有优化空间，但风险较高或收益不明显，暂不纳入：

- `app/action.rs` 的 Action 枚举膨胀（100+ 变体）：这是 Elm 架构的固有特征，拆分反而破坏内聚性
- `tools/` 目录的工具实现：每个工具已经是独立文件，结构合理
- `ui/` 和 `render/` 的渲染逻辑：职责清晰，无需调整
- `infra/` 的各子模块：已按功能语义拆分（hook/skill/command/sandbox/archive），符合规范
