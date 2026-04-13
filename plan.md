# j-cli Chat 模块优化方案

基于 claude-code (TypeScript) 的 agent loop、tool 体系、提示词和 reminder 处理，对比 j-cli chat 模块的改进方向。

---

## 一、Agent Loop 优化

### 1.1 结构化 Loop State（优先级：高）

**claude-code 做法**：query.ts 使用 `State` struct 统一管理跨迭代可变状态（messages、toolUseContext、autoCompactTracking、turnCount、transition 等），每次 `continue` 时整体替换 state，而非散落 9 个独立变量赋值。

**j-cli 现状**：`agent.rs` 的 `run_agent_loop` 用 `for _round in 0..max_tool_rounds` 循环，所有状态（messages、finish_reason、assistant_text、raw_tool_calls）都是循环内局部变量，没有跨轮持久化的 tracking state。

**改进**：
- 引入 `AgentLoopState` 结构体，包含 `turn_count`、`auto_compact_tracking`（连续失败计数）、`max_output_tokens_recovery_count`、`transition`（上一次 continue 的原因）等
- 每轮循环开始时解构 state，continue 时整体更新
- 好处：可追踪连续 compact 失败、max_tokens 恢复重试等场景

### 1.2 流式 Tool 执行（StreamingToolExecutor）（优先级：中）

**claude-code 做法**：`StreamingToolExecutor` 在流式响应中 **收到 tool_use 就立即启动执行**，不等所有 tool_use 块收完。工具结果通过 `getCompletedResults()` 在流式循环中实时 yield。

**j-cli 现状**：必须等整个流式响应结束（`finish_reason = ToolCalls`），才批量发送 `StreamMsg::ToolCallRequest` 到主线程，然后等所有 tool results 回来。

**改进**：
- 在 agent loop 的 stream 循环中，收到完整 tool_call（name+arguments 都到齐）时，立即通过 channel 发送到主线程执行
- 好处：多个 tool call 时可并行执行，减少用户等待

### 1.3 Model Fallback（优先级：低）

**claude-code 做法**：当主力模型出错（FallbackTriggeredError），自动切换到 fallback model 重试，并清除已产生的 assistant messages（避免 signature 不匹配）。

**j-cli 现状**：没有 model fallback 机制。

**改进**：在 `AgentConfig` 中添加 `fallback_model` 字段，API 错误时自动降级。

### 1.4 Max Output Tokens Recovery（优先级：中）

**claude-code 做法**：当响应因 max_output_tokens 截断时：
1. 先尝试提升 max_output_tokens（从默认 8k→64k）
2. 最多重试 3 次，注入 recovery message 让模型从中断处继续
3. 恢复消息明确指示"no apology, no recap, pick up mid-thought"

**j-cli 现状**：没有 max_output_tokens 恢复机制，截断就是截断。

**改进**：在 agent loop 中检测 `finish_reason = Length`，注入恢复消息并 continue 循环。

---

## 二、Tool 体系优化

### 2.1 Tool trait 丰富化（优先级：高）

**claude-code 做法**：`Tool` 接口极其丰富（~50 个方法），核心包括：
- `isReadOnly(input)` — 只读判断
- `isConcurrencySafe(input)` — 并发安全判断
- `isDestructive(input)` — 破坏性判断
- `interruptBehavior()` — 被中断时是 cancel 还是 block
- `checkPermissions(input, ctx)` — 工具级权限检查
- `validateInput(input, ctx)` — 输入验证
- `maxResultSizeChars` — 结果大小限制
- `shouldDefer` / `alwaysLoad` — 延迟加载/始终加载
- `prompt()` — 每个工具贡献自己的提示词片段
- `buildTool()` 工厂函数提供安全默认值

**j-cli 现状**：`Tool` trait 只有 5 个方法（name, description, parameters_schema, execute, requires_confirmation），粒度太粗。

**改进**：
```rust
pub trait Tool: Send + Sync {
    // 已有
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult;

    // 新增
    fn is_read_only(&self, _arguments: &str) -> bool { false }
    fn is_concurrency_safe(&self, _arguments: &str) -> bool { false }
    fn is_destructive(&self, _arguments: &str) -> bool { false }
    fn interrupt_behavior(&self) -> InterruptBehavior { InterruptBehavior::Block }
    fn check_permissions(&self, arguments: &str, ctx: &PermissionContext) -> PermissionResult {
        PermissionResult::Allow
    }
    fn max_result_size_chars(&self) -> usize { 50000 }
}
```

好处：
- `is_read_only` → 可跳过权限确认（Read/Grep/Glob 不需要确认）
- `is_concurrency_safe` → 多个 tool call 可并行执行
- `is_destructive` → UI 显示红色警告
- `interrupt_behavior` → 控制用户取消时的行为
- `check_permissions` → 工具级细粒度权限（替代当前的 `requires_confirmation` 二元判断）

### 2.2 工具级提示词（Tool Prompt）（优先级：中）

**claude-code 做法**：每个 Tool 实现 `prompt()` 方法，动态生成该工具在系统提示词中的使用指引。例如 BashTool 的 prompt 会根据当前权限模式给出不同的使用说明。

**j-cli 现状**：工具描述是静态的 `description()` 返回值，没有根据运行环境动态调整的能力。

**改进**：添加 `fn prompt(&self, ctx: &ToolPromptContext) -> String` 方法，让工具可以在系统提示词中注入更详细的使用指引。

### 2.3 Tool Result 大小控制（优先级：中）

**claude-code 做法**：`maxResultSizeChars` + `applyToolResultBudget()` — 工具结果超过阈值时持久化到文件，只给模型一个文件路径预览。还有 `contentReplacementState` 跟踪跨轮的内容替换。

**j-cli 现状**：tool result 原样注入 messages，没有大小控制。长输出（如大文件 grep 结果）会快速耗尽 context window。

**改进**：
- `Tool::max_result_size_chars()` 默认 50000
- 超过阈值时截断 + 提示"结果过长，已截断至前 N 字符"
- 或写入临时文件，给模型文件路径

### 2.4 Deferred Tool Loading（优先级：低）

**claude-code 做法**：`ToolSearchTool` + `shouldDefer` 机制 — 工具数量多时，只发送核心工具 schema，其余通过 ToolSearch 延迟发现。减少首次 API 调用的 token 开销。

**j-cli 现状**：所有工具 schema 全量发送。

**改进**：当工具数量 > 阈值时，核心工具全量发送，低频工具标记为 defer，添加 ToolSearch 工具让模型按需发现。

---

## 三、提示词（System Prompt）优化

### 3.1 分层系统提示词架构（优先级：高）

**claude-code 做法**：系统提示词分三层：
1. **defaultSystemPrompt** — 核心能力描述（工具使用规则、安全约束、格式要求）
2. **userContext** — 用户级上下文（`prependUserContext`，如 cwd、os、timestamp）
3. **systemContext** — 系统级上下文（`appendSystemContext`，如项目配置）

每层独立缓存，组合后通过 `asSystemPrompt()` 统一类型。

**j-cli 现状**：单个 `system_prompt_fn()` 返回一个 Option<String>，用模板占位符 `{{.tools}}`、`{{.background_tasks}}` 等替换。

**改进**：
- 拆分为 `core_prompt` + `user_context` + `system_context` 三部分
- 核心提示词包含：工具使用规则、安全约束、格式要求
- user_context 包含：当前目录、操作系统、时间戳
- system_context 包含：项目级配置、权限规则
- 好处：各层可独立更新，减少核心提示词的 token 开销

### 3.2 动态提示词注入（Priority: Medium）

**claude-code 做法**：
- Memory 系统：`loadMemoryPrompt()` 注入记忆系统指引
- Attachment 系统：`getAttachmentMessages()` 动态注入文件变更、队列命令
- Skill discovery：预取 skill 发现结果，注入到下一轮
- 每轮重新构建 system prompt（从磁盘读取最新配置）

**j-cli 现状**：system_prompt 每轮重建，但缺少动态注入能力。background notification 和 todo nag 已经用 `push_system_reminder()` 注入，但方式较原始。

**改进**：
- 添加 `PromptInjector` trait，统一管理动态提示词注入
- 在每轮 API 调用前收集所有 injectors 的输出，统一追加

### 3.3 Prompt Caching 感知（优先级：低）

**claude-code 做法**：严格排序工具（`localeCompare` 排序），确保 MCP 工具在 built-in 之后，以最大化 prompt cache 命中率。还有 `skipCacheWrite`、`cache_deleted_input_tokens` 追踪等。

**j-cli 现状**：工具顺序固定按注册顺序，不考虑 prompt cache。

**改进**：如果使用的 API 支持 prompt caching（如 Anthropic），需要对工具列表排序以保持稳定缓存键。

---

## 四、Reminder 处理优化

### 4.1 结构化 Reminder 系统（优先级：高）

**claude-code 做法**：reminder 通过 `<system-reminder>` 标签注入，有明确的分类：
- `tool_hint` — 工具使用提醒（如"优先用专用工具而非 bash"）
- `content_policy` — 内容安全策略
- `context_warning` — 上下文窗口警告
- `memory` — 记忆系统注入
- `attachment` — 文件变更/队列命令等

每种 reminder 有不同的注入时机和优先级。

**j-cli 现状**：`push_system_reminder()` 是唯一入口，所有 reminder 都以相同格式注入 user 消息。只有两种场景使用：background notification 和 todo nag。

**改进**：
- 定义 `ReminderType` 枚举，区分不同类型的 reminder
- 定义 `ReminderPriority`，控制注入顺序
- 添加更多 reminder 场景：
  - **tool_hint**：当模型用 Bash 执行本应用专用工具的操作时（如 `cat file` → 提示用 Read 工具）
  - **context_warning**：当 token 接近阈值时警告
  - **subagent_result**：子 agent 完成时注入结果摘要

### 4.2 Tool Hint Reminder（优先级：高）

**claude-code 做法**：系统提示词中包含大量工具使用指引，例如：
- "ALWAYS prefer using specialized tools over bash commands"
- "Use Read tool instead of cat/head/tail"
- "Use Grep tool instead of grep command"
- "Use Edit tool instead of sed/awk"

**j-cli 现状**：系统提示词中只有工具列表和简短描述，没有使用指引。

**改进**：在系统提示词中添加工具使用指引：
```
工具使用规则：
1. 优先使用专用工具而非 bash 命令：Read > cat/head/tail, Grep > grep, Edit > sed/awk, Glob > find
2. 文件编辑优先用 Edit 工具（精确替换），Write 仅用于创建新文件或完全重写
3. 搜索代码用 Grep 工具，搜索文件名用 Glob 工具
4. 避免在 bash 中运行交互式命令（vim, top, python REPL 等）
```

---

## 五、Compaction 优化

### 5.1 多层 Compaction 架构（优先级：高）

**claude-code 做法**：5 层 compaction 策略：
1. **Tool Result Budget** — 跨轮大小控制
2. **Snip** — 剪切中间消息（保留头尾）
3. **Microcompact** — 缓存编辑式替换旧 tool results
4. **Context Collapse** — 渐进式折叠（保留摘要）
5. **Auto Compact** — LLM 摘要压缩

每层独立触发，组合使用。还有 `consecutiveFailures` 熔断计数，防止无限重试。

**j-cli 现状**：2 层 compaction：
1. `micro_compact` — 替换旧 tool result 为占位符
2. `auto_compact` — LLM 摘要压缩

**改进**：
- 添加 **consecutiveFailures** 追踪，auto_compact 连续失败 N 次后停止重试
- 添加 **Tool Result Budget** 层：在 API 调用前统一检查所有 tool results 的总大小，超限则截断最旧的
- `auto_compact` 的 summary prompt 改进：claude-code 的 compact 会保留关键决策、当前状态、进度，j-cli 当前只说"Be concise but preserve critical details"

### 5.2 Compact Boundary 消息（优先级：低）

**claude-code 做法**：compaction 后 yield `compact_boundary` 系统消息，包含 `preservedSegment`（保留的消息 UUID 范围）。SDK 消费者可据此知道哪些历史消息已被摘要替代。

**j-cli 现状**：compact 后简单替换 messages，没有 boundary 信号。

**改进**：添加 compact boundary 消息，供 UI 显示"上下文已压缩"提示，以及 session 持久化时区分原始消息和压缩摘要。

---

## 六、权限系统优化

### 6.1 工具级权限检查（优先级：中）

**claude-code 做法**：`Tool::checkPermissions()` + `canUseTool()` 回调，三层权限：
1. 工具自身的 `checkPermissions()` — 工具级逻辑
2. `validateInput()` — 输入验证
3. 通用权限系统 — `alwaysAllowRules` / `alwaysDenyRules` / `alwaysAskRules`，支持 per-tool pattern matching

还有 `PermissionMode`（default / plan / bypassPermissions）和 `ToolPermissionContext` 上下文。

**j-cli 现状**：`JcliConfig::is_allowed()` 做简单规则匹配，`Tool::requires_confirmation()` 做二元判断。

**改进**：
- 在 Tool trait 中添加 `check_permissions()` 方法
- 权限检查流程：工具级 `check_permissions()` → 通用规则匹配 → UI 确认
- 添加 `PermissionMode`：default（需要确认）/ auto（信任工具判断）/ plan（只读模式）

### 6.2 Permission Mode（优先级：中）

**claude-code 做法**：
- `default` — 每次危险操作都需要确认
- `plan` — 只允许只读工具 + 写入 plan 文件
- `bypassPermissions` — 跳过所有确认

**j-cli 现状**：只有 `allow_all`（跳过确认）和逐条规则两种模式，没有 plan mode 的权限控制。

**改进**：与已有的 PlanModeState 联动，plan mode 下自动限制为只读工具。

---

## 七、错误处理与恢复

### 7.1 API 错误重试与恢复（优先级：中）

**claude-code 做法**：
- Prompt-too-long → reactive compact → 重试
- Max output tokens → escalate → recovery message → 重试
- Model fallback → 切换模型 → 重试
- Stop hooks → 可能阻止继续或注入错误 → 重试
- 所有恢复路径都有次数限制，防止死循环

**j-cli 现状**：API 错误直接 `tx.send(StreamMsg::Error)` 然后 return，没有恢复机制。

**改进**：
- 添加 `max_retries` 配置
- 区分可恢复错误（rate limit、timeout）和不可恢复错误（auth、invalid model）
- 可恢复错误自动重试，带指数退避

### 7.2 Stop Hooks（优先级：低）

**claude-code 做法**：`handleStopHooks()` — 在 LLM 完成一轮后、进入下一轮前执行 stop hooks，可以：
- 阻止继续（preventContinuation）
- 注入 blocking errors（强制模型修正）
- 收集日志

**j-cli 现状**：只有 `PreLlmRequest` 和 `PostToolExecution` hook，没有 stop hook。

**改进**：添加 `PostAssistantMessage` hook，在每轮 assistant 回复后执行，可用于安全审查、质量检查等。

---

## 实施优先级排序

| 优先级 | 改进项 | 预估改动量 |
|--------|--------|-----------|
| P0 | 2.1 Tool trait 丰富化 | 中（改 trait + 所有 tool 实现） |
| P0 | 4.1 结构化 Reminder 系统 | 小 |
| P0 | 4.2 Tool Hint Reminder | 小（改 system prompt 模板） |
| P1 | 1.1 结构化 Loop State | 中 |
| P1 | 3.1 分层系统提示词 | 中 |
| P1 | 5.1 多层 Compaction 改进 | 中 |
| P1 | 2.3 Tool Result 大小控制 | 小 |
| P1 | 6.1 工具级权限检查 | 中 |
| P2 | 1.4 Max Output Tokens Recovery | 小 |
| P2 | 1.2 流式 Tool 执行 | 大 |
| P2 | 2.2 工具级提示词 | 中 |
| P2 | 3.2 动态提示词注入 | 中 |
| P2 | 6.2 Permission Mode | 小 |
| P2 | 7.1 API 错误重试与恢复 | 中 |
| P3 | 1.3 Model Fallback | 小 |
| P3 | 2.4 Deferred Tool Loading | 大 |
| P3 | 5.2 Compact Boundary 消息 | 小 |
| P3 | 7.2 Stop Hooks | 中 |
