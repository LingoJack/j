# Context Compaction 对比报告：j-cli vs Claude Code

## 背景

当 AI agent 在探索项目时，会产生大量 tool call 结果（读文件、搜索、执行命令等）。随着对话变长，上下文窗口会被撑满，需要压缩旧内容。核心问题是：**压缩后模型会不会"看了后面忘了前面"？**

本报告对比 j-cli 和 Claude Code 两个项目在 context compaction 上的实现差异。

---

## 1. j-cli 的实现（当前）

### 架构：双层压缩

#### micro_compact（`src/command/chat/agent/compact.rs` L172-241）

- **触发**：每次 agent 循环时检查，当 tool result 数量超过阈值
- **策略**：保留最近 `keep_recent` 个 tool result，其余替换为占位符
- **替换内容**：`[Previous: used {tool_name}]`
- **豁免工具**：`LoadSkill`、`Task*`、`Todo*`、`Agent`、`Plan` 等不压缩
- **问题**：只保留工具名，模型完全不知道之前读了什么内容

#### auto_compact（LLM 总结）

- **触发**：token 数超过上下文窗口阈值
- **策略**：调用 LLM 对历史对话做总结，替换旧消息
- **问题**：总结质量依赖 prompt，没有结构化模板；没有文件重新注入机制

### 缺失的能力

| 能力 | j-cli |
|---|---|
| 持久化记忆（Session Memory） | ❌ |
| 压缩后文件重新注入 | ❌ |
| 结构化总结模板 | ❌ |
| 后台异步提取关键信息 | ❌ |

---

## 2. Claude Code 的实现

### 架构：三层递进式压缩

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

触发机制：
- 注册为 post-sampling hook，每次 LLM API 调用完成后检查
- 条件：上下文增长 ≥ 5000 tokens，且工具调用 ≥ 3 次（或无工具调用的自然对话断点）
- 用 `sequential()` 包装，确保同时只有一个提取任务运行

执行方式：
- 调用 `runForkedAgent()` 创建隔离的 ToolUseContext
- **不是后台线程**，而是事件驱动的异步任务（fire-and-forget，不阻塞主对话）
- 本质是一次独立的 LLM API 调用（会消耗额外 tokens）

提取内容写入 session memory markdown 文件：
- 当前状态和发现
- 关键决策和模式
- 文件列表和最近修改
- 错误和解决方案
- 任务状态

在 compact 时，该文件内容会被重新注入到压缩后的上下文中，作为"记忆"保留。

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

---

## 3. 核心差异对比

| 维度 | j-cli | Claude Code |
|---|---|---|
| **压缩层数** | 2 层 | 3 层 |
| **Micro compact 替换内容** | `[Previous: used Read]` | `[Old tool result content cleared]` |
| **Session Memory** | ❌ 无 | ✅ 后台异步 LLM 提取关键信息到文件 |
| **总结结构化程度** | 普通 prompt | 9 段结构化模板 |
| **压缩后文件重新注入** | ❌ 无 | ✅ 最近 5 个文件，50K 预算 |
| **Skill/Plan 重新注入** | ❌ 无 | ✅ Skill 25K + Plan |
| **跨 session 记忆** | ❌ 无 | ✅ Session Memory 文件持久化 |
| **额外 API 开销** | 无 | 有（Session Memory 提取消耗额外 tokens） |

---

## 4. "探索项目"场景的具体影响

假设 agent 依次读取 10 个文件来理解项目结构：

### j-cli 的情况

```
Read src/main.rs      → [Previous: used Read]     ← 完全丢失
Read src/cli.rs       → [Previous: used Read]     ← 完全丢失
Read src/config.rs    → [Previous: used Read]     ← 完全丢失
...（中间 5 个文件全部丢失）
Read src/handler.rs   → 完整内容 ✅（最近 keep_recent 个）
Read src/agent.rs     → 完整内容 ✅
```

模型只能依赖 assistant 之前的回复来"回忆"，但 assistant 的回复通常不会逐行复述文件内容。

### Claude Code 的情况

```
Read src/main.rs      → [Old tool result content cleared]
Read src/cli.rs       → [Old tool result content cleared]
...

但同时：
- Session Memory 文件记录了："main.rs 是入口，路由到 REPL 或 clap"
- Session Memory 文件记录了："cli.rs 定义了 SubCmd 枚举"
- Full compact 后，最近 5 个文件会被重新注入
- 总结包含 "Files and Code Sections" 段落
```

模型在 compact 后仍然有：结构化总结 + 最近文件内容 + Session Memory 关键发现。

---

## 5. j-cli 可借鉴的改进方向

### 优先级 1：micro_compact 替换内容增强（低成本，无额外 API 调用）

当前：
```
[Previous: used Read]
```

改进为：
```
[Previous: Read src/main.rs — 入口文件，路由到 REPL 或 clap 解析，约 80 行]
```

实现思路：在 micro_compact 替换时，保留文件路径，并从 assistant 之前对该 tool result 的回复中提取简要摘要，或者简单保留 tool input 参数（文件路径、搜索关键词等）。

### 优先级 2：auto_compact 后文件重新注入（中等成本，无额外 API 调用）

在 auto_compact 完成后，把最近读过的文件重新 attach 到上下文中：
- 维护一个 `recent_files: Vec<(PathBuf, Timestamp)>` 追踪最近读过的文件
- Compact 后重新读取最近 N 个文件，注入到消息序列中
- 设置 token 预算上限（如 50K tokens）

### 优先级 3：结构化总结模板（低成本，无额外 API 调用）

参考 Claude Code 的 9 段结构，改进 auto_compact 的总结 prompt，确保关键信息不丢失：
- 用户的主要请求和意图
- 具体文件路径和关键代码片段
- 错误和修复记录
- 当前工作状态和待办任务

### 优先级 4：Session Memory 机制（高成本，高收益，需要额外 API 调用）

实现后台"笔记"系统：
- 每隔 N 次 tool call 或 N tokens 增长后触发
- 调用 LLM 提取关键发现，写入 `~/.jdata/agent/session_memory.md`
- Compact 时将该文件内容注入上下文
- 额外 API 开销需要考虑，但对长对话场景收益显著

---

## 6. 结论

j-cli 和 Claude Code 在 microcompaction 层面的做法相似（都是丢弃旧 tool result），但 Claude Code 通过 **Session Memory + 文件重新注入 + 结构化总结** 三个机制大幅缓解了信息丢失问题。

对于 j-cli 来说，投入产出比最高的改进路线：

1. **micro_compact 保留更多上下文**（文件路径 + 简要摘要）— 几乎零成本
2. **compact 后重新注入最近文件** — 不需要额外 API 调用
3. **结构化总结模板** — 不需要额外 API 调用，只改 prompt
4. **Session Memory** — 需要额外 API 开销，但对长探索任务收益最大
