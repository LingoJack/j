# Context Compaction 对比报告：j-cli vs Claude Code

## 背景

当 AI agent 在探索项目时，会产生大量 tool call 结果（读文件、搜索、执行命令等）。随着对话变长，上下文窗口会被撑满，需要压缩旧内容。核心问题是：**压缩后模型会不会"看了后面忘了前面"？**

本报告对比 j-cli 和 Claude Code 两个项目在 context compaction 上的实现差异，并评估 j-cli 可借鉴的改进方向。

---

## 1. j-cli 的实现（当前状态，含 2026-04-15 改进）

### 架构：双层压缩 + Skill 保护

#### micro_compact（`src/command/chat/agent/compact.rs`）

- **触发**：每次 agent 循环时检查，当 tool result 数量超过阈值
- **策略**：保留最近 `keep_recent` 个 tool result，其余替换为占位符
- **替换内容**：`[Previous: used {tool_name}]`
- **豁免工具**：`LoadSkill`、`Task*`、`Todo*`、`Agent`、`Plan`、`Ask`、`SendMessage`、`CreateTeammate` 不压缩
- **问题**：只保留工具名，模型完全不知道之前读了什么内容（如文件路径、搜索关键词等）

#### auto_compact（LLM 总结 + Skill 重注入）

- **触发**：token 数超过上下文窗口阈值
- **策略**：调用 LLM 对历史对话做 9 段结构化总结，替换旧消息
- **已实现（2026-04-15）**：
  - ✅ 9 段结构化总结模板（Primary Request / Key Concepts / Files and Code / Errors and Fixes / Problem Solving / Active Skills/Workflows / Pending Tasks / Current Work / Next Step）
  - ✅ InvokedSkillsMap 追踪已加载的 skill，auto_compact 后重注入（25K 总预算，每个 skill 5K）
  - ✅ 保存完整 transcript 到 `~/.jdata/agent/data/transcripts/`，总结中引用路径
- **仍缺失**：
  - ❌ 压缩后文件重新注入（Claude Code 的 50K 预算重读最近 5 个文件）
  - ❌ micro_compact 替换内容增强（只保留工具名，无文件路径等摘要信息）
  - ❌ Session Memory 机制（后台持续提取关键发现到持久化文件）

### 缺失能力（更新）

| 能力 | j-cli | 备注 |
|---|---|---|
| 持久化 Session Memory | ❌ | Claude Code 核心防遗忘机制 |
| 压缩后文件重新注入 | ❌ | 无额外 API 开销 |
| micro_compact 保留摘要信息 | ❌ | 几乎零成本改进 |
| 结构化总结模板 | ✅ | 已在 2026-04-15 实现 |
| Skill 重注入 | ✅ | 已在 2026-04-15 实现 |
| Hook 与 Compact 集成 | ❌ | 无 PreCompact/PostCompact 事件 |

---

## 2. Claude Code 的实现

### 架构：三层递进式压缩 + 双记忆系统

#### Tier 1: Microcompaction（连续、非阻塞）

两种路径：

**A. Cached Microcompaction（1P 模型专用）**
- 使用 `cache_edits` API 在服务端直接删除旧 tool result，不修改本地消息
- 本地维护 `CachedMCState` 追踪工具调用顺序和已删除的引用
- 保留最近 ~10-15 个 tool result（GrowthBook 可配置）

**B. Time-Based Microcompaction（冷缓存检测）**
- 当距上次 assistant 消息超过阈值（60-120 分钟），判定服务端缓存已过期
- 直接在本地消息中替换：`[Old tool result content cleared]`
- 重置 `CachedMCState`

#### Tier 2: Session Memory（关键差异 — j-cli 没有的机制）

**这是 Claude Code 最重要的防遗忘机制。**

##### 触发机制

注册为 **post-sampling hook**（每次 LLM API 调用完成后检查），条件判断逻辑：

```
shouldExtractMemory =
  (tokenGrowth ≥ 5000 AND toolCalls ≥ 3)    // 活跃工作期间
  OR
  (tokenGrowth ≥ 5000 AND noToolCallsInLastTurn)  // 自然对话断点
```

同时需要先满足初始化阈值：`currentTokens ≥ 10000`

用 `sequential()` 包装，确保同时只有一个提取任务运行。

##### 执行方式

- 调用 `runForkedAgent()` 创建隔离的 ToolUseContext
- **不是后台线程**，而是事件驱动的异步任务（fire-and-forget，不阻塞主对话）
- 本质是一次独立的 LLM API 调用（会消耗额外 tokens）
- Forked agent 共享父对话的 prompt cache（`CacheSafeParams`），降低额外开销
- 只允许 Edit 操作 session memory 文件（`createMemoryFileCanUseTool()`）

##### Session Memory 文件结构

10 个结构化段落（`prompts.ts`）：

1. Session Title — 会话标题
2. Current State — 当前状态和发现
3. Task specification — 任务规格
4. Files and Functions — 文件列表和函数
5. Workflow — 工作流程
6. Errors & Corrections — 错误和修正
7. Codebase and System Documentation — 代码库文档
8. Learnings — 学到的经验
9. Key results — 关键结果
10. Worklog — 工作日志

每段约 2000 tokens 上限，总计约 12000 tokens。可自定义 template 和 prompt。

##### 与 Compaction 的集成

在 `autoCompact.ts` 中，compaction 触发时**先尝试 Session Memory compaction**：
1. 等待进行中的提取完成（`waitForSessionMemoryExtraction()`，15s 超时，60s 过期判定）
2. 读取 session memory 文件内容
3. 用 `lastSummarizedMessageId` 划分已总结/未总结的消息边界
4. 删除已总结的旧消息，保留 session memory + 边界后的新消息
5. 截断超长段落（`truncateSessionMemoryForCompact()`）

**关键**：如果 session memory 为空或无效，回退到传统 API 总结。

#### Tier 2.5: Extract Memories（跨 session 持久记忆）

这是与 Session Memory **独立**的第二套记忆系统，用于跨 session 持久化知识。

- **触发**：`handleStopHooks`（每个 turn 结束后 fire-and-forget），有节流控制
- **执行**：`runForkedAgent()` + 只允许读写 `~/.claude/projects/<path>/memory/` 目录
- **输出**：user / feedback / project / reference 四类记忆文件 + `MEMORY.md` 索引
- **与 j-cli 的关系**：j-cli 已有类似机制（`~/.codebuddy/projects/.../memory/`），但 Claude Code 的实现更自动化（LLM 自动提取而非用户手动保存）

#### Tier 3: Full Compaction（最后手段）

**触发**：token 接近上下文窗口上限（200K 窗口时约 187K 触发）

总结 prompt 结构化为 9 个部分：
1. Primary Request and Intent（用户的主要请求）
2. Key Technical Concepts（框架、模式）
3. Files and Code Sections（具体文件、代码片段、重要原因）
4. Errors and Fixes（详细错误描述和解决方案）
5. Problem Solving（已找到的解决方案）
6. All User Messages（关键用户反馈）
7. Pending Tasks（未完成任务）
8. Current Work（当前工作状态）
9. Optional Next Step（最近对话的直接引用）

压缩后重新注入：
- 最近 5 个读过的文件（每个最多 5K tokens，总预算 50K tokens）
- Skill 内容（每个最多 5K tokens，总预算 25K tokens）
- Plan 文件（如果在 plan mode）
- Hook 结果

Compaction 前后还会触发 `PreCompact` / `PostCompact` hook 事件。

---

## 3. Claude Code Hook 系统全景

j-cli 的 hook 系统有 8 个事件 + 2 种类型（Shell/Builtin），Claude Code 则有 27 个事件 + 5 种类型。以下是与 compaction 相关的关键差异：

### 事件对比

| 事件类别 | j-cli | Claude Code |
|---|---|---|
| 消息生命周期 | PreSendMessage, PostSendMessage | UserPromptSubmit, Stop, StopFailure |
| LLM 请求 | PreLlmRequest, PostLlmResponse | （内部 post-sampling hook，不暴露） |
| 工具执行 | PreToolExecution, PostToolExecution | PreToolUse, PostToolUse, PostToolUseFailure |
| 会话生命周期 | SessionStart, SessionEnd | SessionStart, SessionEnd |
| **Compaction** | ❌ 无 | ✅ PreCompact, PostCompact |
| 子 agent | ❌ 无 | ✅ SubagentStart, SubagentStop |
| 通知 | ❌ 无 | ✅ Notification |
| 权限 | ❌ 无 | ✅ PermissionRequest, PermissionDenied |
| 配置变更 | ❌ 无 | ✅ ConfigChange, CwdChanged, FileChanged |

### Hook 类型对比

| 类型 | j-cli | Claude Code |
|---|---|---|
| Shell 命令 | ✅ | ✅ Command（支持条件、async、once） |
| 内置闭包 | ✅ BuiltinHook | ✅ Function（仅 session 级） |
| LLM Prompt | ❌ | ✅ Prompt（用 LLM 评估 hook 逻辑） |
| HTTP 请求 | ❌ | ✅ HTTP（POST 到 URL） |
| Agent 验证 | ❌ | ✅ Agent（运行子 agent 验证） |

### Post-Sampling Hook（Claude Code 独有）

这是一个**不暴露给用户**的内部 hook 系统，在每次 LLM 采样完成后触发。Session Memory 提取就注册在这里。j-cli 没有对等机制。

---

## 4. 核心差异对比

| 维度 | j-cli | Claude Code |
|---|---|---|
| **压缩层数** | 2 层（micro + auto） | 3 层（micro + session memory + full） |
| **Micro compact 替换内容** | `[Previous: used Read]` | `[Old tool result content cleared]` |
| **Session Memory** | ❌ 无 | ✅ 后台异步 LLM 提取关键信息到文件 |
| **总结结构化程度** | ✅ 9 段模板 | ✅ 9 段模板 |
| **Skill 重注入** | ✅ 25K 预算 | ✅ 25K 预算 |
| **压缩后文件重新注入** | ❌ 无 | ✅ 最近 5 个文件，50K 预算 |
| **Plan 重注入** | ❌ 无 | ✅ Plan 文件 |
| **PreCompact/PostCompact hook** | ❌ 无 | ✅ 可自定义压缩前后行为 |
| **跨 session 持久记忆** | ✅ 手动 memory 系统 | ✅ 自动 extractMemories |
| **额外 API 开销** | 无 | 有（Session Memory 提取消耗额外 tokens） |
| **Forked Agent 机制** | ❌ 无 | ✅ 共享 prompt cache 的隔离子 agent |

---

## 5. "探索项目"场景的具体影响

假设 agent 依次读取 10 个文件来理解项目结构：

### j-cli 的情况

```
Read src/main.rs      → [Previous: used Read]     ← 完全丢失（无路径、无摘要）
Read src/cli.rs       → [Previous: used Read]     ← 完全丢失
Read src/config.rs    → [Previous: used Read]     ← 完全丢失
...（中间 5 个文件全部丢失）
Read src/handler.rs   → 完整内容 ✅（最近 keep_recent 个）
Read src/agent.rs     → 完整内容 ✅
```

micro_compact 后，模型只能依赖 assistant 之前的回复来"回忆"，但 assistant 的回复通常不会逐行复述文件内容。

当 auto_compact 触发时：
- ✅ 9 段结构化总结会包含 "Files and Code" 段落（依赖 LLM 总结质量）
- ✅ 已加载的 Skill 内容会被重注入
- ❌ 但总结中没有最近文件的完整内容，只有 LLM 选择性提取的片段
- ❌ 没有 PreCompact hook 可以自定义注入逻辑

### Claude Code 的情况

```
Read src/main.rs      → [Old tool result content cleared]
Read src/cli.rs       → [Old tool result content cleared]
...

但同时：
- Session Memory 文件持续更新："main.rs 是入口，路由到 REPL 或 clap"
- Session Memory 文件记录了："cli.rs 定义了 SubCmd 枚举"
- Full compact 后，最近 5 个文件会被重新注入（完整内容）
- 总结包含 "Files and Code Sections" 段落
- PreCompact hook 可注入自定义指令
- PostCompact hook 可执行清理/通知
```

模型在 compact 后仍然有：Session Memory 关键发现 + 最近文件完整内容 + 结构化总结 + Skill 内容。

---

## 6. Session Memory 触发策略分析

### Claude Code 的策略：Post-Sampling Hook

Claude Code 在**每次 LLM 回复后**检查是否需要提取 session memory：

```
每次 LLM 回复 → post-sampling hook → shouldExtractMemory()?
  → 条件: tokenGrowth ≥ 5000 AND (toolCalls ≥ 3 OR 无 tool call)
  → 是: runForkedAgent() 更新 session memory 文件
  → 否: 跳过
```

**问题**：
- 检查频率高（每轮都检查），但实际提取频率受阈值控制
- 每次提取都消耗额外 API tokens（即使共享 prompt cache）
- 最大的 API 开销来自长对话中多次提取

### 替代策略：基于 micro_compact 进度触发

**核心思路**：既然 micro_compact 已经在持续丢弃旧信息，session memory 的更新节奏应该与之同步——在信息即将丢失前提取，而不是盲目地在每个 turn 后检查。

```
micro_compact 执行时 → 检查被压缩的 tool result 数量
  → 如果被压缩的内容超过阈值（如 3 个 tool result）
  → 触发 session memory 更新（提取刚被压缩的信息）
  → 信息"转移"：从上下文 → session memory 文件
```

**优势**：

| 维度 | Post-Sampling | 基于 micro_compact 进度 |
|---|---|---|
| **触发时机** | 每轮 LLM 回复后 | 信息实际被丢弃时 |
| **语义** | "对话进行了多少" | "有多少信息即将丢失" |
| **提取效率** | 可能提取时信息还在上下文中（冗余） | 精准提取即将丢失的信息 |
| **API 开销** | 较高（频繁检查 + 提取） | 较低（只在信息丢失前提取） |
| **信息覆盖** | 依赖 token 增长和 tool call 阈值 | 直接追踪被压缩的内容 |
| **与 micro_compact 的配合** | 独立运行，可能重复 | 有机结合，信息"接力" |

**具体设计**：

```
micro_compact(messages, keep_recent) {
  // 1. 找出即将被压缩的 tool result
  to_compact = tool_indices[..len - keep_recent]

  // 2. 如果被压缩的数量 ≥ 阈值（如 3 个），标记需要更新 session memory
  if to_compact.len() >= SESSION_MEMORY_UPDATE_THRESHOLD {
    pending_memory_update = Some(to_compact)
  }

  // 3. 执行正常压缩
  for idx in to_compact {
    messages[idx].content = "[Previous: used {tool_name}]"
  }
}

// 4. 在 agent loop 中，micro_compact 后检查 pending_memory_update
//    fire-and-forget 提取（不阻塞主循环）
if let Some(compacted) = pending_memory_update {
  spawn_thread(|| {
    session_memory.update_from_compacted(compacted)
  })
}
```

**额外考虑**：

- session memory 不应该只依赖 micro_compact 触发，还需要一个**兜底机制**：当 auto_compact 直接触发（没有经过足够的 micro_compact 轮次）时，也应该更新 session memory
- 可以在 auto_compact 前始终触发一次 session memory 更新（类似 Claude Code 的 `waitForSessionMemoryExtraction()`）

---

## 7. j-cli 可借鉴的改进方向（按优先级）

### 优先级 1：micro_compact 替换内容增强（几乎零成本，无额外 API）

当前：
```
[Previous: used Read]
```

改进为：
```
[Previous: Read src/main.rs — Entry point, routes to REPL or clap dispatch, ~80 lines]
[Previous: Grep "fn handle_" — Found 12 matches in src/command/chat/handler/]
[Previous: Bash "cargo test" — 3 failures in test_chat_module]
```

实现思路：在 micro_compact 替换时，从 tool_call 的 input 参数中提取关键信息：
- `Read` / `Write` / `Edit` → 文件路径 + 内容前 100 字符
- `Grep` → 搜索关键词 + 匹配数量
- `Bash` → 命令前 100 字符 + exit code
- `Glob` → 搜索模式 + 匹配数量

这需要同时查看 assistant 消息中的 `tool_calls` 数组（取 input）和 tool result（取内容长度/exit code），而不是只看 tool_name。

**成本**：零 API 开销，只增加字符串拼接逻辑。

### 优先级 2：auto_compact 后文件重新注入（中等成本，无额外 API）

在 auto_compact 完成后，把最近读过的文件重新 attach 到上下文中：
- 维护一个 `recent_files: Vec<(PathBuf, Timestamp)>` 追踪最近读过的文件
- Compact 后重新读取最近 N 个文件，注入到消息序列中
- 设置 token 预算上限（如 50K tokens）

实现思路：
- 在 `PostToolExecution` builtin hook 中记录 `Read` / `Edit` / `Write` 工具访问的文件路径
- 在 `auto_compact` 完成后，读取 `recent_files` 列表中最近的 5 个文件
- 作为 `<system-reminder>` 附件注入到 summary 消息中（与 skill 重注入模式一致）

**成本**：文件 I/O（读 5 个文件），零 API 开销。

### 优先级 3：新增 PreCompact / PostCompact hook 事件（低成本）

让 hook 系统介入 compaction 流程，使扩展更灵活：

- **`PreCompact`**：在 auto_compact 执行前触发
  - HookContext: `messages`, `cwd`
  - HookResult: `inject_messages`（可以注入自定义总结指令）
- **`PostCompact`**：在 compact 完成后触发
  - HookContext: `messages`（压缩后的）, `cwd`
  - HookResult: `inject_messages`（可以注入额外上下文）

实现思路：
1. 在 `HookEvent` 枚举中新增两个变体
2. 在 `compact.rs` 的 `auto_compact()` 函数前后调用 `hook_manager.execute()`
3. 需要将 `hook_manager` 传入 `auto_compact()` 函数（当前未传入）

**成本**：极低，只是枚举扩展 + 2 次 hook 调用。

### 优先级 4：Session Memory 机制 — 基于 micro_compact 进度触发（中等成本，需要额外 API）

实现后台"笔记"系统，但**触发节奏与 micro_compact 同步**而非每轮 LLM 回复后检查：

**设计**：

```
SessionMemoryUpdateTrigger:
  1. micro_compact 压缩 ≥ 3 个 tool result 时 → fire-and-forget 提取
  2. auto_compact 触发前 → 同步等待提取完成（兜底）
  3. 手动触发（用户命令 /summary）

SessionMemory 文件 (~/.jdata/agent/session_memory.md):
  - 10 段结构化内容（同 Claude Code）
  - 每次 micro_compact 后增量更新
  - auto_compact 时作为"记忆"注入到总结消息中

提取方式（简化版 forked agent）:
  - 在独立线程中调用 openai API（不共享 prompt cache）
  - 只允许 Edit session_memory.md
  - 输入：刚被 micro_compact 压缩的 tool result + 当前 memory 内容
  - 输出：更新后的 memory 文件
```

**与 Claude Code 的关键区别**：

| 维度 | Claude Code | j-cli 提议 |
|---|---|---|
| 触发时机 | 每次 LLM 回复后（post-sampling） | micro_compact 压缩信息时 |
| 触发条件 | token 增长 ≥ 5000 + tool call ≥ 3 | 被压缩的 tool result ≥ 3 |
| 语义 | "对话进行了多少" | "有多少信息即将丢失" |
| 提取内容 | 全量对话 → memory 文件 | 刚被压缩的内容 → memory 文件 |
| Prompt cache | 共享（降低开销） | 不共享（简化实现） |
| 同步机制 | `waitForSessionMemoryExtraction()` | auto_compact 前同步等待 |

**成本**：每次提取约消耗 ~2-5K 额外 tokens（取决于 memory 文件大小和被压缩内容量）。

### 优先级 5：micro_compact 替换内容增强 → 反哺 Session Memory（可选优化）

如果同时实现了优先级 1 和优先级 4，可以形成一个**信息接力闭环**：

```
tool result（完整内容）
  → micro_compact 替换为摘要 → "[Previous: Read src/main.rs — entry point, ~80 lines]"
  → 同时将完整内容发送给 session memory 提取
  → session memory 记录："[Files] src/main.rs: Entry point, routes to REPL or clap"
  → 后续 auto_compact 时，session memory 被注入到总结消息中
```

这样即使 micro_compact 后的摘要不够详细，session memory 中仍保留了更完整的关键发现。

---

## 8. 实施路线图

```
Phase 1（零 API 开销）:
  ├─ 优先级 1: micro_compact 替换内容增强
  ├─ 优先级 2: recent_files 追踪 + auto_compact 后文件重注入
  └─ 优先级 3: PreCompact/PostCompact hook 事件

Phase 2（需要额外 API）:
  └─ 优先级 4: Session Memory 机制（基于 micro_compact 进度触发）

Phase 3（优化）:
  └─ 优先级 5: 信息接力闭环（micro_compact 摘要 + session memory 提取联动）
```

---

## 9. 结论

j-cli 经过 2026-04-15 的改进，已经在结构化总结和 Skill 重注入上与 Claude Code 持平。但仍有三个关键差距：

1. **micro_compact 信息丢失严重**：`[Previous: used Read]` 让模型完全不知道之前读了什么。这个改动几乎零成本但收益最大。

2. **缺乏文件重注入机制**：auto_compact 后模型失去了所有文件内容，只能依赖 LLM 总结中的片段。`recent_files` 追踪 + 重注入可以低成本解决。

3. **Session Memory 的触发策略**：Claude Code 的 post-sampling 方式虽然有效但 API 开销较大。基于 micro_compact 进度触发是更精确的策略——**在信息即将丢失时提取**，而非盲目地在每个 turn 后检查。这种"信息接力"模型更符合 compaction 的语义：micro_compact 丢弃信息 → session memory 接收信息 → auto_compact 使用信息。

最核心的设计原则：**Session Memory 不应该是独立于 compaction 的旁路系统，而应该是 compaction 管线的一环**——在信息被丢弃前拦截并保存关键内容。
