# Plan: subagent-teammate-system-prompt-and-tool-fold

## 背景

### 消息流分析

当前架构中，Teammate/SubAgent 的消息流：

1. **广播消息注入**：
   - 其他 agent 通过 `SendMessage` 广播消息
   - 格式：`<AgentName> @Target message` 或 `<AgentName> message`
   - Teammate/SubAgent 通过 `pending_user_messages` 接收，drain 到本地 `messages` 列表

2. **发送给 LLM**：
   - `call_llm_non_stream` → `build_request_with_tools` → `sanitize_messages` → `to_openai_messages`
   - 所有消息（包括来自其他 agent 的 tool call 广播）完整发送给 LLM

3. **问题**：
   - 如果 Frontend teammate 执行了 20 次 tool call，Backend teammate 会收到 20 条 `<Frontend> [调用工具 xxx]` 消息
   - 这些消息占用大量上下文 token
   - 导致 LLM API 成本增加、响应变慢

### 需求

用户需要：
1. **SubAgent 和 Teammate 有独立 system prompt**（身份明确）
2. **其他 agent 的 tool call 消息压缩折叠**：
   - 发送给 LLM 时，来自其他 agent 的 tool call 消息需要压缩
   - 前一定数量保留完整，超过阈值后压缩为摘要
   - 格式：`[AgentName call xxx, yyy, zzz tools, qqq time]`

---

## 技术方案

### Part 1: SubAgent 独立 System Prompt

#### 1.1 创建 SubAgent System Prompt 模板

新增文件 `assets/subagent_system_prompt.md`：

```markdown
# SubAgent System Prompt

你是一个专门的子代理，负责执行特定任务。

## 基本信息
- 名称: {{.name}}
- 任务描述: {{.description}}

## 基础能力
{{.base_prompt}}

## 工作原则
1. 自主决策，减少交互
2. 遇到错误时尝试分析并解决
3. 任务完成后简洁汇报结果

## 限制
- 不能使用 Agent 工具（防止递归创建子代理）
- 不能使用 SendMessage 工具（无团队通信权限）
```

#### 1.2 修改 SubAgent 代码

修改 `src/command/chat/tools/sub_agent.rs`：
- 加载模板并替换变量
- 当前代码直接使用 `params.system_prompt`（来自主 agent），需要改为构建独立 prompt

修改 `src/assets.rs`：
- 添加 `SUBAGENT_SYSTEM_PROMPT_TEMPLATE` 常量和加载函数

---

### Part 2: 其他 Agent Tool Call 消息压缩

#### 2.1 消息识别与分组

在 `sanitize_messages` 或新增函数中：

1. **识别来源**：
   - 广播消息格式：`<AgentName> ...`
   - Tool call 广播：`<AgentName> [调用工具 ToolName]`
   - 使用正则提取 agent 来源和工具名

2. **分组策略**：
   - 按 `agent_source` 分组
   - 同一 agent 的连续 tool call 消息合并
   - 区分「自己调用」和「旁听其他 agent」

#### 2.2 压缩阈值

配置项（新增到 `AgentConfig` 或 `DerivedAgentShared`）：
```rust
/// 其他 agent tool call 保留完整数量阈值
pub other_agent_toolcall_threshold: usize,  // 默认 5
```

#### 2.3 压缩算法

新增模块 `src/command/chat/agent/message_compression.rs`：

```rust
/// 压缩来自其他 agent 的 tool call 消息
///
/// 策略：保留最近的 threshold 条完整消息，较早的压缩为摘要
pub fn compress_other_agent_toolcalls(
    messages: &[ChatMessage],
    self_agent_name: &str,
    threshold: usize,
) -> Vec<ChatMessage> {
    // 1. 识别消息来源（从 <AgentName> 格式提取）
    // 2. 按 agent_source 分组
    // 3. 对每组：最近的 threshold 条保留完整
    // 4. 较早的消息合并为摘要（放在消息列表开头位置）
}
```

压缩格式设计：
```
单条保留（最近）：<Frontend> [调用工具 Read]
多条压缩（较早）：<Frontend> [早期工具调用摘要: Read×5, Edit×8, Bash×3, 共 16 次]
```

**压缩位置**：摘要消息放在该 agent 最早出现的位置，后续保留最近的消息。

#### 2.4 集成点

在 `call_llm_non_stream` 调用前，对 `messages` 应用压缩：

修改 `src/command/chat/teammate/teammate_loop.rs`：
```rust
// 原代码：
let response_choice = match call_llm_non_stream(
    &rt, &client, &provider, &messages, &tools, Some(&system_prompt), None,
) { ... }

// 改为：
let compressed_messages = compress_other_agent_toolcalls(
    &messages,
    &name,  // teammate 自己的名字
    OTHER_AGENT_TOOLCALL_THRESHOLD,
);
let response_choice = match call_llm_non_stream(
    &rt, &client, &provider, &compressed_messages, &tools, Some(&system_prompt), None,
) { ... }
```

同样修改 `src/command/chat/tools/sub_agent.rs`。

#### 2.5 消息来源标记（可选）

为了更精确识别来源，可在 `ChatMessage` 中添加字段：

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub agent_source: Option<String>,  // 消息来源 agent 名
```

但当前广播消息格式 `<AgentName> ...` 已经包含来源信息，可通过正则提取，无需新字段。

---

## 实施步骤

### Step 1: SubAgent System Prompt 模板
1. 创建 `assets/subagent_system_prompt.md`
2. 修改 `src/assets.rs` 添加加载函数
3. 修改 `sub_agent.rs` 使用模板构建 prompt

### Step 2: 消息压缩模块
1. 新增 `src/command/chat/agent/message_compression.rs`
2. 实现 `extract_agent_source` 函数（从 `<AgentName>` 格式提取来源）
3. 实现 `compress_other_agent_toolcalls` 函数

### Step 3: 配置扩展
1. 在 `DerivedAgentShared` 或新配置结构中添加 `other_agent_toolcall_threshold`
2. 默认值 5（保留前 5 条完整）

### Step 4: 集成到 Teammate Loop
1. 修改 `teammate_loop.rs`：在 `call_llm_non_stream` 前调用压缩
2. 传入 `self_agent_name`（teammate 名字）和阈值

### Step 5: 集成到 SubAgent Loop
1. 修改 `sub_agent.rs`：同样在调用 LLM 前压缩
2. 传入 `self_agent_name`（subagent 的 description/sanitize_name）

### Step 6: 测试验证
1. 单元测试：消息来源提取
2. 单元测试：压缩算法正确性
3. 集成测试：运行 teammate 场景验证上下文减少

---

## 文件变更清单

| 文件路径 | 变更类型 | 说明 |
|---------|---------|------|
| `assets/subagent_system_prompt.md` | 新增 | SubAgent system prompt 模板 |
| `src/assets.rs` | 修改 | 添加 SubAgent 模板加载 |
| `src/command/chat/tools/sub_agent.rs` | 修改 | 使用独立 prompt + 消息压缩 |
| `src/command/chat/agent/message_compression.rs` | 新增 | 消息压缩模块 |
| `src/command/chat/tools/derived_shared.rs` | 修改 | 添加压缩阈值配置 |
| `src/command/chat/teammate/teammate_loop.rs` | 修改 | 调用消息压缩 |

---

## 压缩示例

### 原始消息序列（teammate Backend 收到的广播）：
```
<Frontend> [调用工具 Read]       # 第 1 条（最早）
<Frontend> [调用工具 Edit]
<Frontend> [调用工具 Bash]
<Frontend> [调用工具 Read]
<Frontend> [调用工具 Edit]
<Frontend> [调用工具 Bash]
...
<Frontend> [调用工具 Edit]       # 第 15 条
<Frontend> [调用工具 Read]       # 第 16 条（较近）
<Frontend> [调用工具 Bash]       # 第 17 条
<Frontend> [调用工具 Edit]       # 第 18 条
<Frontend> [调用工具 Bash]       # 第 19 条
<Frontend> [调用工具 Edit]       # 第 20 条（最近）
```

### 压缩后（threshold=5）发送给 Backend 的 LLM：
```
<Frontend> [早期工具调用摘要: Read×5, Edit×6, Bash×4, 共 15 次]  # 压缩前 15 条
<Frontend> [调用工具 Read]       # 第 16 条（保留，最近）
<Frontend> [调用工具 Bash]       # 第 17 条（保留）
<Frontend> [调用工具 Edit]       # 第 18 条（保留）
<Frontend> [调用工具 Bash]       # 第 19 条（保留）
<Frontend> [调用工具 Edit]       # 第 20 条（保留，最近）
```

**压缩逻辑**：
- 来自 Frontend 的 tool call 共 20 条
- threshold=5，保留最近 5 条（第 16-20 条）
- 较早的 15 条压缩为一条摘要，放在该 agent 最早出现的位置

---

## Notes

1. **只压缩「旁听」消息**：
   - 自己的 tool call 消息不压缩（完整保留）
   - 只压缩来自其他 agent 的 `<OtherAgent> [调用工具...]` 消息

2. **时间戳可选**：
   - 当前广播消息不含时间戳
   - 可通过消息顺序推断大致时间
   - 若需要精确时间，需扩展广播消息格式

3. **保留关键信息**：
   - 压缩摘要需包含工具名统计
   - 便于 LLM 理解其他 agent 的工作进展

4. **向后兼容**：
   - 压缩只影响发送给 LLM 的消息
   - 不影响 UI 显示和 session 存储
   - transcript 文件仍保留完整消息