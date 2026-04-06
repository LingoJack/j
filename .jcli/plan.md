# Plan: 调查 Context 占用追踪机制的实现方式、准确性以及 Compact 后的更新行为

## 调查发现

### 1. Token 估算机制
- **位置**: `src/command/chat/compact.rs:60-62`
- **实现**: `estimate_tokens(messages) = serde_json::to_string(messages).len() / 4`
- 这是一个粗略估算，假设每 4 个字符约等于 1 个 token

### 2. UI 显示逻辑
- **位置**: `src/command/chat/ui/chat.rs:103-104`
- 标题栏每帧绘制时调用 `estimate_tokens(&app.state.session.messages)`
- **数据源**: `app.state.session.messages`

### 3. 消息同步架构
```
Agent Thread                          UI Thread
─────────────                         ─────────────
messages (本地)          ──────>      shared_messages (Arc<Mutex>)
                                       │
                                       ↓ (增量同步)
                                  session.messages
                                       │
                                       ↓
                                  estimate_tokens()
```

### 4. 问题根源

**核心问题**: `auto_compact` 执行后，消息同步链路中存在数据不一致。

#### 问题细节:
1. `auto_compact(&mut messages, ...)` 清空并替换 `messages`（agent.rs:79-81）
2. 新的摘要消息通过 `push_shared()` 追加到 `shared_messages`（compact.rs:232-258）
3. **但 `session.messages` 中的旧消息从未被清除**
4. UI 的增量同步机制只会追加，不会删除旧消息

#### 代码路径分析:
```rust
// agent.rs:79-81
if let Err(e) = compact::auto_compact(&mut messages, &provider).await {
    write_error_log("agent_loop", &format!("auto_compact failed: {}", e));
}

// compact.rs:232-258 - auto_compact 内部
messages.clear();  // 清空 agent 本地的 messages
messages.push(summary_user_msg);
messages.push(understood_assistant_msg);
messages.push(system_notification);
// 注意：这些新消息会被后续操作 push_shared 到 shared_messages
```

#### UI 增量同步逻辑 (chat_app.rs:1911-1925):
```rust
let shared = safe_lock(&self.shared_agent_messages, "poll::shared_msgs");
let new_count = shared.len();
if new_count > self.shared_messages_read_cursor {
    for msg in &shared[self.shared_messages_read_cursor..] {
        self.state.session.messages.push(msg.clone());  // 只追加，不清除
    }
    self.shared_messages_read_cursor = new_count;
}
```

### 5. 影响范围

1. **Context Token 显示不准确**: `session.messages` 包含压缩前的旧消息 + 压缩后的新消息
2. **消息重复显示**: UI 可能同时显示旧消息和摘要消息
3. **micro_compact 同样受影响**: 它修改 `messages` 内容但不反映到 UI

## 解决方案

### 方案 A: 在 UI 侧处理 compact 通知
1. `auto_compact` 推入一条特殊系统消息（带标记）
2. UI 检测到该消息后清空 `session.messages`，再执行增量同步
3. **优点**: 改动最小，保持现有架构
4. **缺点**: 需要特殊消息类型

### 方案 B: 添加 compact 事件通道
1. 新增 `compact_event: Arc<AtomicBool>` 到共享状态
2. `auto_compact` 完成后设置标志
3. UI 检测到标志后重置 `session.messages` 和 cursor
4. **优点**: 逻辑清晰，不依赖消息内容
5. **缺点**: 需要新增共享状态

### 方案 C: 重构消息同步机制（推荐）
1. 将 `shared_messages` 设计为单一数据源
2. `auto_compact` 时：清空 `shared_messages`，重置 cursor，推入新消息
3. UI 检测到 `shared_messages` 缩小时自动清空 `session.messages`
4. **优点**: 从根本上解决问题，架构更健壮
5. **缺点**: 改动较大

## 推荐方案

采用 **方案 B**（compact 事件通道），原因：
1. 改动范围适中
2. 逻辑清晰可靠
3. 不需要特殊消息类型
4. 易于测试和验证

### 实施步骤

1. 在 `AgentSharedState` 中添加 `compact_happened: Arc<AtomicBool>`
2. 在 `auto_compact` 完成后设置该标志
3. 在 `poll_stream_actions` 中检测标志，执行重置逻辑
4. 确保重置后 token 估算基于新的 `session.messages`

## 关于物理存储的说明

### 对话记录存储机制
1. **Archive（归档）**: 用户手动归档，保存在 `~/.jdata/agent/data/archives/{name}.json`
2. **Transcript**: `auto_compact` 前自动保存完整对话，保存在 `~/.jdata/agent/data/transcripts/transcript_{timestamp}.jsonl`

### 方案 C 对存储的影响
**方案 C 不影响物理存储**。原因：
1. 方案 C 只涉及**内存中的消息同步机制**
   - `shared_messages` (Arc<Mutex<Vec<ChatMessage>>>)
   - `session.messages` (UI 本地缓存)
2. `auto_compact` 在清空前已调用 `save_transcript()` 保存完整对话
3. Archive 是用户手动触发的，与 compact 机制无关

### 数据流图
```
auto_compact 触发
    │
    ├─→ save_transcript() → transcripts/transcript_xxx.jsonl (物理存储，不受影响)
    │
    └─→ messages.clear() + push(summary) → shared_messages (内存，方案C修改这里)
                                              │
                                              ↓
                                         session.messages (内存，方案C修改这里)
```

## 方案 C 详细实现

### 核心思路
让 `shared_messages` 成为消息的**唯一数据源**，UI 的 `session.messages` 只是它的完整副本。当 `shared_messages` 被重置时，UI 自动清空并重建。

### 实现步骤

#### Step 1: 修改 `auto_compact` 接收 `shared_messages` 参数

```rust
// compact.rs
pub async fn auto_compact(
    messages: &mut Vec<ChatMessage>,
    shared_messages: &Arc<Mutex<Vec<ChatMessage>>>,  // 新增参数
    provider: &ModelProvider,
) -> Result<(), String> {
    // ... 摘要逻辑不变 ...

    // 替换 messages 并同步到 shared_messages
    messages.clear();
    messages.push(summary_user_msg.clone());
    messages.push(understood_assistant_msg.clone());
    messages.push(system_notification.clone());

    // ★ 关键修改：清空并重建 shared_messages
    if let Ok(mut shared) = shared_messages.lock() {
        shared.clear();
        shared.push(summary_user_msg);
        shared.push(understood_assistant_msg);
        shared.push(system_notification);
    }

    Ok(())
}
```

#### Step 2: 修改 `run_agent_loop` 传递 `shared_messages`

```rust
// agent.rs:79-81
if compact::estimate_tokens(&messages) > compact_config.token_threshold {
    write_info_log("agent_loop", "auto_compact triggered");
    // 传入 shared_messages
    if let Err(e) = compact::auto_compact(&mut messages, &shared_messages, &provider).await {
        write_error_log("agent_loop", &format!("auto_compact failed: {}", e));
    }
}

// agent.rs:411 和 483 - compact tool 触发也需要传递
if compact_requested && compact_config.enabled {
    let _ = compact::auto_compact(&mut messages, &shared_messages, &provider).await;
}
```

#### Step 3: 修改 UI 增量同步逻辑检测 `shared_messages` 缩小

```rust
// chat_app.rs:1911-1926
{
    let shared = safe_lock(&self.shared_agent_messages, "poll::shared_msgs");
    let new_count = shared.len();

    // ★ 检测 shared_messages 是否缩小（compact 发生）
    if new_count < self.shared_messages_read_cursor {
        // compact 发生，清空 session.messages 并重建
        self.state.session.messages.clear();
        self.shared_messages_read_cursor = 0;
        // 重置消息渲染缓存
        self.ui.msg_lines_cache = None;
        write_info_log("poll_stream", "检测到 compact，已清空 session.messages");
    }

    // 正常增量同步
    if new_count > self.shared_messages_read_cursor {
        for msg in &shared[self.shared_messages_read_cursor..] {
            self.state.session.messages.push(msg.clone());
        }
        self.shared_messages_read_cursor = new_count;
        self.ui.msg_lines_cache = None;
        self.ui.auto_scroll = true;
        self.ui.scroll_offset = u16::MAX;
    }
}
```

### 修改文件清单

| 文件 | 修改内容 |
|------|----------|
| `src/command/chat/compact.rs` | `auto_compact` 添加 `shared_messages` 参数，在清空 `messages` 后同步清空 `shared_messages` |
| `src/command/chat/agent.rs` | 三处调用 `auto_compact` 时传入 `shared_messages` |
| `src/command/chat/app/chat_app.rs` | `poll_stream_actions` 中检测 `shared_messages.len() < cursor` 触发清空逻辑 |

### 边界情况处理

1. **并发安全**: `shared_messages.lock()` 在 `auto_compact` 内部获取，确保原子性
2. **UI 渲染中断**: 清空 `session.messages` 后立即重建，用户只看到短暂闪烁
3. **micro_compact**: 不影响消息数量，只修改内容，无需特殊处理

### 优点分析

1. **架构简洁**: `shared_messages` 成为唯一数据源，消除不一致可能
2. **向后兼容**: 不改变 `push_shared` 等现有接口
3. **物理存储独立**: 只影响内存，不影响 transcript 和 archive

## Breaking Change 风险分析

### 公共 API 影响

| 函数/类型 | 可见性 | 调用方 | 影响 |
|-----------|--------|--------|------|
| `auto_compact()` | `pub async fn` | 仅 `agent.rs` 内部 | **无外部调用** |
| `micro_compact()` | `pub fn` | 仅 `agent.rs:73` | 不修改签名 |
| `estimate_tokens()` | `pub fn` | `agent.rs`, `ui/chat.rs` | 不修改签名 |
| `CompactConfig` | `pub struct` | 多个模块（配置加载） | 不修改 |

### 详细影响评估

#### 1. 函数签名变更
```rust
// 当前
pub async fn auto_compact(
    messages: &mut Vec<ChatMessage>,
    provider: &ModelProvider,
) -> Result<(), String>

// 方案 C 修改后
pub async fn auto_compact(
    messages: &mut Vec<ChatMessage>,
    shared_messages: &Arc<Mutex<Vec<ChatMessage>>>,  // 新增参数
    provider: &ModelProvider,
) -> Result<(), String>
```

**结论**：这是**内部 API 变更**，不是 breaking change。原因：
- `auto_compact` 虽然是 `pub`，但只在 `src/command/chat/agent.rs` 中调用
- 没有外部 crate 或其他模块依赖此函数

#### 2. 调用点修改
所有三个调用点都在 `run_agent_loop` 内部：
- `agent.rs:79` - token 阈值触发
- `agent.rs:411` - fallback 模式 compact tool 触发
- `agent.rs:483` - 流式模式 compact tool 触发

`run_agent_loop` 已拥有 `shared_messages`（从 `AgentSharedState` 解构）：
```rust
let AgentSharedState {
    streaming_content,
    pending_user_messages,
    background_manager,
    todo_manager,
    shared_messages,  // ← 已存在
} = shared;
```

**结论**：**无需新增数据传递**，所有调用点都能直接获取 `shared_messages`。

#### 3. UI 增量同步变更
`chat_app.rs:1911-1926` 的增量同步逻辑需要修改，这是**行为变更**，但：
- 不影响公共 API
- 只改变内部同步逻辑
- 向后兼容：当 `shared_messages` 增长时行为不变，只在缩小时触发清空

#### 4. oneshot 模式
`run_oneshot_agent`（mod.rs:211-783）不使用 `auto_compact`，不受影响。

### Breaking Change 结论

**方案 C 不会造成 breaking change**：
1. 只修改内部函数签名，无外部依赖
2. 所有调用点都在同一模块内，可同步修改
3. UI 同步逻辑变更是内部实现细节
4. 物理存储（transcript/archive）完全独立

## 最终方案

采用 **方案 C**，实施步骤如下：

### Step 1: 修改 `compact.rs` - 添加 `shared_messages` 参数

```rust
// 在 auto_compact 函数中：
pub async fn auto_compact(
    messages: &mut Vec<ChatMessage>,
    shared_messages: &Arc<Mutex<Vec<ChatMessage>>>,  // 新增
    provider: &ModelProvider,
) -> Result<(), String> {
    // ... 现有摘要逻辑 ...

    // 替换 messages
    messages.clear();
    let summary_msg = ChatMessage { ... };
    let understood_msg = ChatMessage { ... };
    let system_msg = ChatMessage { ... };
    
    messages.push(summary_msg.clone());
    messages.push(understood_msg.clone());
    messages.push(system_msg.clone());

    // ★ 新增：同步清空并重建 shared_messages
    if let Ok(mut shared) = shared_messages.lock() {
        shared.clear();
        shared.push(summary_msg);
        shared.push(understood_msg);
        shared.push(system_msg);
    }

    Ok(())
}
```

### Step 2: 修改 `agent.rs` - 三处调用点传入 `shared_messages`

```rust
// Line 79
if let Err(e) = compact::auto_compact(&mut messages, &shared_messages, &provider).await {

// Line 411
let _ = compact::auto_compact(&mut messages, &shared_messages, &provider).await;

// Line 483
let _ = compact::auto_compact(&mut messages, &shared_messages, &provider).await;
```

### Step 3: 修改 `chat_app.rs` - 检测 shared_messages 缩小

```rust
// poll_stream_actions 中，增量同步逻辑修改为：
{
    let shared = safe_lock(&self.shared_agent_messages, "poll::shared_msgs");
    let new_count = shared.len();

    // ★ 检测 compact：shared_messages 缩小说明发生了压缩
    if new_count < self.shared_messages_read_cursor {
        self.state.session.messages.clear();
        self.shared_messages_read_cursor = 0;
        self.ui.msg_lines_cache = None;
    }

    // 正常增量同步
    if new_count > self.shared_messages_read_cursor {
        for msg in &shared[self.shared_messages_read_cursor..] {
            self.state.session.messages.push(msg.clone());
        }
        self.shared_messages_read_cursor = new_count;
        self.ui.msg_lines_cache = None;
        self.ui.auto_scroll = true;
        self.ui.scroll_offset = u16::MAX;
    }
}
```

### 修改文件清单

| 文件 | 行号 | 修改内容 |
|------|------|----------|
| `compact.rs` | 180-260 | 函数签名 + 清空 shared_messages |
| `agent.rs` | 79, 411, 483 | 传入 `&shared_messages` |
| `chat_app.rs` | 1911-1926 | 检测缩小并清空 |

## UI 影响分析

### 方案C 对用户界面的影响

#### 1. 视觉效果
当 `auto_compact` 触发时，用户会看到：
```
[长对话消息列表] 
     ↓ (compact 触发，约 1 帧)
[闪烁：清空]
     ↓ (同一帧内重建)
[3条摘要消息]
```

**这是预期行为**：
- Compact 的目的就是压缩历史，UI 应该反映这一变化
- 用户看到压缩后的摘要，知道上下文已被精简
- 标题栏的 Context Token 会立即显示正确的压缩后数值

#### 2. 消息渲染缓存机制
UI 使用 `msg_lines_cache` 缓存消息渲染结果：
```rust
// chat.rs:268-278
let cache_hit = if let Some(ref cache) = app.ui.msg_lines_cache {
    cache.msg_count == msg_count  // 消息数量变化 → 缓存失效
        && cache.last_msg_len == last_msg_len
        // ...
};
```

方案C在 `poll_stream_actions` 中设置 `msg_lines_cache = None`，触发下一帧重新渲染。

#### 3. 滚动位置
```rust
// 方案C 会重置：
self.shared_messages_read_cursor = 0;
self.ui.msg_lines_cache = None;
self.ui.auto_scroll = true;
self.ui.scroll_offset = u16::MAX;  // 滚动到底部
```

用户会被带到摘要消息底部，这是合理的行为——用户需要阅读新的摘要内容。

#### 4. 与当前行为的对比

| 场景 | 当前行为（Bug） | 方案C修复后 |
|------|-----------------|-------------|
| Compact 触发 | 旧消息保留 + 新摘要追加 → 消息重复 | 旧消息清除 → 只显示摘要 |
| Context Token | 显示错误（旧+新） | 显示正确（仅新消息） |
| 用户感知 | 困惑（消息重复、token不准） | 清晰（看到压缩后的摘要） |

### 结论

**方案C 对 UI 的影响是正向的**：
1. 修复了消息重复显示的 bug
2. Context Token 显示正确
3. 用户能看到压缩结果，符合预期
4. 无需额外的 UI 组件或状态

## 待确认

- [x] 方案 C 不影响物理存储
- [x] 方案 C 实现细节已明确
- [x] 方案 C 不造成 breaking change
- [x] 方案 C 对 UI 影响是正向的（修复 bug）
- [ ] 是否批准方案 C 并开始实施？
