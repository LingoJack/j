# 修复消息恢复顺序问题

## 问题分析

### 当前架构

消息有多个来源：
1. **主 transcript** (`sessions/<id>/transcript.jsonl`) - Main Agent 消息
2. **Teammate transcript** (`sessions/<id>/teammates/<name>/transcript.jsonl`) - 每个 teammate 的独立消息
3. **SubAgent transcript** (`sessions/<id>/subagents/<sub_id>/transcript.jsonl`) - 每个 subagent 的独立消息

### 实时运行时（正确）

Teammate/SubAgent 的消息通过 `push_both` 同步推送到 `display_messages` 和 `context_messages`，`poll_stream_actions` 增量同步到 `session.messages`，**保证顺序正确**。

### 恢复时的问题（错误）

1. `load_session` 只从主 transcript 加载消息
2. `restore_teammate_transcripts` 简单地将 teammate 消息追加到末尾，**不考虑时间戳交错**
3. **SubAgent transcript 完全没有恢复逻辑**

### 问题代码位置

`src/command/chat/app/session_mgr.rs` 中的 `restore_teammate_transcripts` 方法：

```rust
fn restore_teammate_transcripts(&mut self, sid: &str, teammate_names: &[String]) {
    // 只是简单地追加到 session.messages 末尾，丢失了时间顺序
}
```

## 解决方案

### 核心思路：基于时间戳的全局排序恢复

所有 `SessionEvent::Msg` 都包含 `timestamp_ms` 字段，可以使用它来恢复正确的消息顺序。

### 实施步骤

#### Step 1: 新增辅助函数 `read_all_transcripts_with_source`

从所有 transcript 来源读取消息，标记来源类型：

```rust
enum MessageSource {
    Main,
    Teammate { name: String },
    SubAgent { id: String, description: String },
}

struct TimestampedMessage {
    message: ChatMessage,
    timestamp_ms: u64,
    source: MessageSource,
}
```

#### Step 2: 重写 `restore_teammate_transcripts` 为 `restore_all_transcripts`

1. 从主 transcript 读取消息
2. 从所有 teammate transcript 读取消息（需要合成为 `<Teammate@Name> xxx` 格式）
3. 从所有 subagent transcript 读取消息（从 `subagents.json` 获取列表）
4. 按 `timestamp_ms` 全局排序
5. 替换 `session.messages`

#### Step 3: 处理 teammate/subagent 消息的合成

Teammate/SubAgent transcript 存储的是原始消息，需要合成为显示格式：

- `<Teammate@Name> 文本内容`
- `<Teammate@Name> [调用工具 ToolName]`
- `<SubAgent@Description> 文本内容`
- `<SubAgent@Description> [调用工具 ToolName]`

#### Step 4: 处理 SendMessage 工具调用

SendMessage 工具调用不应该显示（在 teammate_loop.rs 中已过滤），只有非 SendMessage 的工具调用才需要显示。

### 修改文件清单

1. **`src/command/chat/app/session_mgr.rs`**
   - 新增 `MessageSource` 和 `TimestampedMessage` 类型
   - 新增 `read_all_transcripts_with_source` 辅助函数
   - 重写 `restore_teammate_transcripts` 为 `restore_all_transcripts`
   - 新增 `restore_subagent_transcripts` 逻辑（或合并到上述方法）

2. **`src/command/chat/storage/session.rs`**
   - 可能需要新增 `list_subagent_transcripts` 辅助函数

3. **`src/command/chat/storage/persist.rs`**
   - `SubAgentSnapshotPersist` 已有 `transcript_file` 字段，可用于定位 subagent transcript

### 边界情况处理

1. **没有时间戳的老数据**：`timestamp_ms` 默认为 0，排在最前面（这不影响新数据）
2. **损坏的 transcript 文件**：跳过无法解析的行
3. **空的 transcript 文件**：直接跳过
4. **SubAgent transcript 不存在**：从 `subagents.json` 读取列表时可能某些 transcript 文件已被删除

### 验证方案

1. 创建一个包含 Main Agent + Teammate + SubAgent 的对话
2. 确保消息有时间交错
3. 退出并重新加载会话
4. 验证消息顺序与实时运行时一致
