# 实验指标数据记录方案

## 一、核心设计理念

用户提出的核心问题：**单纯记录指标数值没有上下文，不知道"发生了什么"、在"什么时机"记录的**。

解决思路：**指标作为事件记录而非数值统计**，每条记录包含：
1. **事件类型**（event_type）：发生了什么事情
2. **触发时机**（trigger）：在什么时机触发
3. **关联上下文**（context）：当时的状态是什么
4. **量化指标**（metrics）：具体数值

这样 metrics.jsonl 就成为"事件流"，可以和 transcript.jsonl（对话流）、ops.jsonl（操作流）对应分析。

---

## 二、现有数据流分析

系统已有三个层次的数据记录：

| 文件 | 记录内容 | 事件粒度 | 缺失的指标 |
|------|----------|----------|-----------|
| transcript.jsonl | LLM 对话消息 | 每条消息 | Token 消耗、上下文长度估算 |
| ops.jsonl | Edit/Write/Bash 操作 | 每个操作 | 执行耗时、用户决策 |
| info.log | 运行时日志 | 不规则 | 需解析文本，无结构化关联 |

**关键发现**：transcript.jsonl 和 ops.jsonl 都有 `timestamp_ms`，但没有"事件类型"字段区分不同场景，也没有关联到具体的"业务事件"（如 compact 触发、Skill 加载）。

---

## 三、推荐方案：事件驱动的 metrics.jsonl

### 设计原则

1. **事件语义化**：每条记录描述"发生了什么事件"，而非"当前数值是多少"
2. **上下文完备**：带上触发时机、业务状态，便于事后分析
3. **关联已有文件**：通过 `round_idx`、`timestamp_ms` 与 transcript.jsonl 对应
4. **结构化优先**：JSONL 格式，便于程序化分析

### 数据结构设计

```json
// ========== 核心事件类型 ==========

// 事件 1：LLM 调用完成（每轮结束时记录）
{
  "event": "llm_round_complete",
  "timestamp_ms": 1234567890000,
  "round_idx": 3,
  "trigger": "stream_done",  // stream_done | stream_error | retry_exhausted
  "metrics": {
    "input_tokens": 1234,      // API response.usage（如果可用）
    "output_tokens": 567,
    "estimated_context_tokens": 15000,  // 本轮开始时的估算值
    "messages_count": 25,      // 本轮发送的消息数
    "tool_calls_count": 2      // 本轮 LLM 返回的 tool_calls 数量
  },
  "context": {
    "model": "claude-3-5-sonnet",
    "tools_available": ["Read", "Edit", "Bash", "Write"],
    "has_pending_user_messages": false
  }
}

// 事件 2：工具调用完成
{
  "event": "tool_call_complete",
  "timestamp_ms": 1234567890100,
  "round_idx": 3,
  "trigger": "tool_executed",  // tool_executed | tool_rejected | tool_modified
  "metrics": {
    "exec_duration_ms": 150   // 工具执行耗时
  },
  "context": {
    "tool_call_id": "call_abc123",
    "tool_name": "Edit",
    "tool_args_path": "src/foo.rs",
    "is_error": false,
    "result_length": 200,     // 工具返回内容长度
    "user_decision": "approved"  // approved | rejected | modified（仅 ToolConfirm 模式）
  }
}

// 事件 3：compact 触发
{
  "event": "compact_triggered",
  "timestamp_ms": 1234567890200,
  "round_idx": 4,  // 触发时的轮次（compact 发生在下一轮开始前）
  "trigger": "token_threshold_exceeded",  // token_threshold_exceeded | manual
  "metrics": {
    "tokens_before": 120000,
    "tokens_after": 35000,
    "messages_before": 48,
    "messages_after": 15,
    "summary_length": 800,    // LLM 生成的摘要长度
    "micro_compact_count": 5   // 本次 compact 中被替换的 tool result 数
  },
  "context": {
    "compact_type": "auto",   // auto | micro
    "transcript_saved": ".transcripts/transcript_1234567890.jsonl",
    "invoked_skills_preserved": ["webapp-gen", "sql-to-go"],
    "threshold_config": 100000
  }
}

// 事件 4：Skill 加载
{
  "event": "skill_loaded",
  "timestamp_ms": 1234567890300,
  "round_idx": 2,
  "trigger": "loadskill_tool_call",  // loadskill_tool_call | auto_compact_recovery
  "metrics": {
    "content_length_chars": 5000,
    "attachment_injected": true
  },
  "context": {
    "skill_name": "webapp-gen",
    "skill_dir": "~/.jdata/agent/skills/webapp-gen",
    "arguments": "博客系统",
    "load_reason": "user_explicit_call"  // user_explicit_call | compact_recovery
  }
}

// 事件 5：SubAgent/Teammate 创建
{
  "event": "agent_spawned",
  "timestamp_ms": 1234567890400,
  "round_idx": 3,
  "trigger": "agent_tool_call",
  "metrics": {
    "spawn_count": 1          // 本次创建的 agent 数
  },
  "context": {
    "agent_type": "subagent", // subagent | teammate
    "agent_name": "search",
    "task_prompt": "搜索 foo 函数的定义",
    "parent_tool_call_id": "call_abc123",
    "worktree_enabled": false
  }
}

// 事件 6：用户干预
{
  "event": "user_intervention",
  "timestamp_ms": 1234567890500,
  "round_idx": 3,
  "trigger": "tool_confirm_decision",
  "metrics": {},
  "context": {
    "tool_call_id": "call_def456",
    "tool_name": "Bash",
    "tool_args_command": "rm -rf /",
    "user_action": "rejected",  // rejected | modified
    "modified_args": null      // 如果 modified，记录修改后的参数
  }
}

// 事件 7：任务边界（可选，用于实验标记）
{
  "event": "task_boundary",
  "timestamp_ms": 1234567890000,
  "round_idx": 0,
  "trigger": "experiment_start",
  "metrics": {},
  "context": {
    "boundary_type": "task_start",  // task_start | task_end
    "experiment_id": "E1",
    "baseline_mode": "A",
    "task_description": "实现用户注册功能"
  }
}
```

### 关键设计点

1. **round_idx 作为关联键**：所有事件都记录当前轮次，便于和 transcript.jsonl 对应分析
2. **trigger 区分触发来源**：知道"为什么"触发这个事件
3. **metrics + context 分离**：量化指标与业务上下文分开，便于不同维度的分析
4. **timestamp_ms 完备**：便于绘制时序图

---

## 四、与现有文件的关联关系

```
sessions/<id>/
├── transcript.jsonl     ← 对话消息流（role/content/tool_calls）
├── ops.jsonl            ← 操作审计流（Edit/Write/Bash）
├── metrics.jsonl        ← 事件指标流（新增，事件驱动）
├── .transcripts/        ← compact 前快照
└── session.json         ← 元信息

关联分析示例：
- 第 3 轮的 transcript 消息 → metrics 中 round_idx=3 的 llm_round_complete
- 第 3 轮的 tool_calls → metrics 中 round_idx=3 的 tool_call_complete
- compact 触发 → metrics 中 compact_triggered + .transcripts/ 快照
```

---

## 五、实现方案

### 方案 A：扩展 SessionPaths + 独立写入器

在 `storage/session.rs` 中新增：

```rust
impl SessionPaths {
    /// 指标事件文件：sessions/<id>/metrics.jsonl
    pub fn metrics_file(&self) -> PathBuf {
        self.dir.join("metrics.jsonl")
    }
}
```

新增 `storage/metrics.rs` 模块：

```rust
/// 指标事件类型
#[derive(Serialize)]
pub enum MetricEvent {
    LlmRoundComplete { ... },
    ToolCallComplete { ... },
    CompactTriggered { ... },
    SkillLoaded { ... },
    AgentSpawned { ... },
    UserIntervention { ... },
    TaskBoundary { ... },
}

/// 写入指标事件（追加到 metrics.jsonl）
pub fn write_metric_event(session_id: &str, event: MetricEvent) {
    let paths = SessionPaths::new(session_id);
    let line = serde_json::to_string(&event).unwrap();
    // append to file...
}
```

### 方案 B：复用现有 info.log + 增强结构化

如果不想新增文件，可增强现有 info.log 的结构化程度：

```rust
// 增强版日志：带 JSON payload
write_info_log_with_payload("llm_round_complete", json!{
    "round_idx": 3,
    "input_tokens": 1234,
    "output_tokens": 567,
    "estimated_context_tokens": 15000
});
```

日志格式变为：
```
[2025-01-01 10:00:00] [llm_round_complete] {"round_idx":3,"input_tokens":1234,...}
```

便于后处理脚本解析。

**推荐方案 A**：独立的 metrics.jsonl 文件，语义更清晰，便于实验分析工具直接读取。

---

## 六、数据采集点

| 事件 | 采集位置 | 携带数据 |
|------|----------|----------|
| llm_round_complete | agent_loop.rs（stream 结束时） | round_idx, usage, estimated_tokens |
| tool_call_complete | tool_processor.rs | round_idx, tool_name, duration, is_error |
| compact_triggered | compact.rs | round_idx, tokens_before/after, messages_before/after |
| skill_loaded | compact.rs（record_skill_invocation） | skill_name, content_length |
| agent_spawned | AgentTool/TeammateTool | agent_type, agent_name, task_prompt |
| user_intervention | stream_poll.rs（ToolConfirm 处理） | tool_name, user_action |

---

## 七、实验分析示例

有了 metrics.jsonl，实验分析变得简单：

```python
# E1: 分析 compact 效果
import json

def analyze_compact_effect(metrics_path):
    events = [json.loads(line) for line in open(metrics_path)]
    
    # 找出所有 compact 事件
    compacts = [e for e in events if e['event'] == 'compact_triggered']
    
    # 计算平均削减比例
    for c in compacts:
        reduction_ratio = c['metrics']['tokens_after'] / c['metrics']['tokens_before']
        print(f"Compact at round {c['round_idx']}: {reduction_ratio:.2%} retained")
    
    # 绘制上下文长度时序图
    rounds = [e for e in events if e['event'] == 'llm_round_complete']
    tokens = [r['metrics']['estimated_context_tokens'] for r in rounds]
    plot_tokens_over_rounds(tokens)

# E4: 分析 Edit 工具可靠性
def analyze_edit_reliability(metrics_path, ops_path):
    events = [json.loads(line) for line in open(metrics_path)]
    ops = [json.loads(line) for line in open(ops_path)]
    
    # 筛选 Edit 相关
    edit_metrics = [e for e in events 
                    if e['event'] == 'tool_call_complete' 
                    and e['context']['tool_name'] == 'Edit']
    edit_ops = [o for o in ops if o['op']['kind'] == 'edit']
    
    # 关联分析
    total = len(edit_metrics)
    failures = len([e for e in edit_metrics if e['context']['is_error']])
    user_rejected = len([e for e in edit_metrics if e['context']['user_decision'] == 'rejected'])
    
    print(f"Edit calls: {total}, Failures: {failures}, User rejected: {user_rejected}")
```

---

## 八、总结

| 问题 | 解决方案 |
|------|----------|
| 单纯数值无上下文 | 事件驱动设计，每条记录描述"发生了什么" |
| 不知道触发时机 | trigger 字段区分触发来源 |
| 无法关联业务 | round_idx + timestamp_ms 关联 transcript.jsonl |
| 无法分析 | 结构化 JSONL，便于程序化分析 |

**核心改动**：
1. 新增 `metrics.jsonl` 文件（事件驱动结构）
2. 新增 `storage/metrics.rs` 模块（事件写入）
3. 在关键采集点调用写入函数（agent_loop/tool_processor/compact/stream_poll）