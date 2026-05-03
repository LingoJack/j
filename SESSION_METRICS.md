# j-cli Session 统计数据说明

## 存储位置

所有 session 数据存储在 `~/.jdata/agent/data/sessions/` 目录下，每个会话对应一个子目录：

```
~/.jdata/agent/data/sessions/
├── <session_id_1>/
│   ├── transcript.jsonl    # 对话记录（JSONL 格式，append-only）
│   ├── display.jsonl       # UI 显示消息记录
│   ├── session.json        # 会话元数据
│   ├── metrics.json        # 性能统计指标（会话结束时写入）
│   ├── ops.jsonl           # 操作审计记录（Edit/Write/Bash）
│   ├── tasks.json          # Task 状态
│   ├── todos.json          # Todo 状态
│   ├── plan.json           # Plan 状态
│   ├── skills.json         # 已加载 Skill 状态
│   ├── hooks.json          # Hook 状态
│   ├── sandbox.json        # Sandbox 状态
│   ├── teammates.json      # Teammate 状态
│   ├── teammates/          # Teammate 独立目录
│   │   └── <sanitized_name>/
│   │       ├── transcript.jsonl
│   │       └── todos.json
│   └── subagents/          # SubAgent 独立目录
│       └── <sub_id>/
│           ├── transcript.jsonl
│           └── todos.json
└── <session_id_2>/
    └── ...
```

---

## metrics.json 字段说明

`metrics.json` 在会话正常结束时写入，记录本次会话的性能和质量指标。

| 字段 | 类型 | 含义 |
|---|---|---|
| `total_llm_calls` | `u32` | LLM API 调用次数（每轮对话算一次） |
| `total_tool_calls` | `u32` | 工具调用次数（所有 LLM 返回的 `tool_calls` 数组元素总数） |
| `total_input_tokens` | `u64` | 累计输入 token 数（来自 API 响应的 `usage.prompt_tokens`，未获取到则为 0） |
| `total_output_tokens` | `u64` | 累计输出 token 数（来自 API 响应的 `usage.completion_tokens`，未获取到则为 0） |
| `estimated_context_tokens_peak` | `usize` | 上下文 token 峰值（各轮对话中估算上下文 token 数的最大值） |
| `auto_compact_count` | `u32` | 自动压缩触发次数（含 `CompactTool` 手动触发） |
| `micro_compact_count` | `u32` | 微压缩（`micro_compact`）触发次数 |
| `skill_loads` | `Vec<String>` | 本次会话加载过的 Skill 名称列表 |
| `ttft_ms_per_call` | `Vec<u64>` | 每次 LLM 调用的首字延迟（TTFT, Time To First Token），单位毫秒。流式路径下是精确值，非流式 fallback 路径下为整个调用耗时 |
| `total_llm_elapsed_ms` | `u64` | LLM 调用总耗时（毫秒）—— 仅计算 LLM API 等待时间（含流式读取），不含工具执行时间 |
| `total_tool_elapsed_ms` | `u64` | 工具执行总耗时（毫秒）—— 仅计算工具调用执行时间 |
| `session_start_ms` | `u64` | 会话开始时间（epoch 毫秒时间戳） |
| `session_end_ms` | `u64` | 会话结束时间（epoch 毫秒时间戳） |

### 衍生指标

基于上述字段可计算：

- **总耗时**：`session_end_ms - session_start_ms`（毫秒）
- **平均 TTFT**：`ttft_ms_per_call` 的平均值
- **Token 消耗**：`total_input_tokens + total_output_tokens`
- **工具调用占比**：`total_tool_calls / total_llm_calls`（每轮平均工具调用数）

---

## session.json 字段说明

会话元数据文件，用于会话列表展示和快速加载。

| 字段 | 类型 | 含义 |
|---|---|---|
| `id` | `String` | 会话 ID（格式：`{timestamp_hex}-{pid_hex}`） |
| `title` | `String` | 会话标题（首条 user 消息截断） |
| `message_count` | `usize` | 消息计数 |
| `created_at` | `u64` | 创建时间戳（epoch seconds） |
| `updated_at` | `u64` | 最后更新时间戳（epoch seconds） |
| `model` | `Option<String>` | 使用的模型名称 |
| `auto_approve` | `bool` | 是否自动批准所有操作（bypass 模式） |

---

## transcript.jsonl 格式说明

对话记录文件，采用 JSONL（每行一个 JSON 对象）格式，append-only 追加写入。

每行是一个 `SessionEvent`，支持以下事件类型：

### 1. Msg 事件（消息）

```json
{
  "type": "msg",
  "role": "user|assistant|tool|system",
  "content": "消息内容",
  "tool_calls": [...],        // 仅 assistant 有工具调用时存在
  "tool_call_id": "...",      // 仅 tool 角色存在
  "reasoning_content": "...", // thinking mode 的思考内容
  "timestamp_ms": 1234567890
}
```

### 2. Clear 事件（清空对话）

```json
{ "type": "clear" }
```

### 3. Restore 事件（还原快照）

```json
{
  "type": "restore",
  "messages": [...]
}
```

### 4. Metrics 事件（性能指标）

```json
{
  "type": "metrics",
  "metrics": { ... }
}
```

---

## ops.jsonl 格式说明

操作审计记录，记录 Edit/Write/Bash 工具的调用，用于安全审计。

每行是一个 `SessionOp`：

```json
{
  "op": { "kind": "edit", "path": "/path/to/file" },
  "timestamp_ms": 1234567890,
  "is_error": false
}
```

```json
{
  "op": { "kind": "write", "path": "/path/to/file" },
  "timestamp_ms": 1234567890,
  "is_error": false
}
```

```json
{
  "op": { "kind": "bash", "command": "cargo build" },
  "timestamp_ms": 1234567890,
  "is_error": true
}
```

---

## display.jsonl 格式说明

UI 显示消息记录，结构与 `transcript.jsonl` 相同，但仅用于 TUI 渲染层，不参与 agent 逻辑。

---

## 其他文件说明

| 文件 | 用途 |
|---|---|
| `tasks.json` | Task 系统状态（多步骤任务管理） |
| `todos.json` | Todo 列表状态（会话级待办） |
| `plan.json` | Plan 模式状态（实现计划） |
| `skills.json` | 已加载 Skill 状态 |
| `hooks.json` | Session Hook 状态 |
| `sandbox.json` | Sandbox 状态（沙盒隔离） |
| `teammates.json` | Teammate 状态（多 agent 协作） |
| `subagents.json` | SubAgent 状态（子 agent 快照） |
| `.transcripts/` | Compact 前的快照备份目录 |

---

## 示例：metrics.json

```json
{
  "total_llm_calls": 5,
  "total_tool_calls": 12,
  "total_input_tokens": 8234,
  "total_output_tokens": 1567,
  "estimated_context_tokens_peak": 12456,
  "auto_compact_count": 1,
  "micro_compact_count": 0,
  "skill_loads": ["webapp-gen"],
  "ttft_ms_per_call": [234, 189, 312, 156, 278],
  "total_llm_elapsed_ms": 45230,
  "total_tool_elapsed_ms": 128760,
  "session_start_ms": 1704067200000,
  "session_end_ms": 1704067350000
}
```

解读：
- 会话持续 150 秒（2.5 分钟）
- LLM 等待耗时 45 秒，工具执行耗时 129 秒
- 工具执行占总耗时的 74%
- 平均 TTFT 约 234ms
- 总 token 消耗 9801（输入 8234 + 输出 1567）
