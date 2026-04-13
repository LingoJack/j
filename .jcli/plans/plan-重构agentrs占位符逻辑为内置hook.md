# 重构 Agent 占位符逻辑为内置 Hook 系统

## 问题分析

### 1. 当前占位符拼接问题 (chat_app.rs:1950-1989)

`system_prompt_fn` 中存在大量硬编码的占位符替换：

```rust
let resolved = template
    .replace("{{.current_dir}}", &current_dir)
    .replace("{{.skills}}", &skills_summary)
    .replace("{{.skill_dir}}", &skill_dir)
    .replace("{{.project_skill_dir}}", &project_skill_dir)
    .replace("{{.tools}}", &tools_summary)
    .replace("{{.style}}", &style_text)
    .replace("{{.memory}}", &memory_text)
    .replace("{{.soul}}", &soul_text)
    .replace("{{.teammates}}", &teammates_summary)
    .replace("{{.tasks}}", &tasks_summary)
    .replace("{{.background_tasks}}", &background_summary)
    .replace("{{.session_state}}", &session_state_summary);
```

**问题**：
- 逻辑分散在 chat_app.rs 的 `system_prompt_fn` 闭包中，难以维护
- 新增动态内容需要修改多处代码
- 没有统一的扩展机制

### 2. Reminder 硬编码问题 (agent.rs:91-124)

```rust
// 后台任务完成通知 - 硬编码
let notifications = background_manager.drain_notifications();
for notif in notifications {
    let body = format!("<background_task_completed>...");
    push_system_reminder(&mut messages, body);
}

// Todo nag reminder - 硬编码  
if todo_manager.has_todos() && todo_manager.turns_since_last_call() >= TODO_NAG_INTERVAL_ROUNDS {
    let todos_summary = todo_manager.format_todos_summary();
    let body = format!("<todo_reminder>...");
    push_system_reminder(&mut messages, body);
}
```

**问题**：
- Reminder 逻辑硬编码在 agent.rs 中
- 无法动态启用/禁用特定 reminder
- 用户无法自定义 reminder 行为
- 没有利用已有的 hook 系统

### 3. Hook 系统现状 (hook.rs)

**已有的 HookEvent**:
- PreSendMessage, PostSendMessage
- PreLlmRequest, PostLlmResponse  ← 可用于注入动态内容
- PreToolExecution, PostToolExecution
- SessionStart, SessionEnd

**HookManager 三级架构**:
- user_hooks: `~/.jdata/agent/hooks.yaml`
- project_hooks: `.jcli/hooks.yaml`
- session_hooks: 运行时动态注册

**HookResult 支持**:
- `inject_messages`: 向消息列表追加消息
- `system_prompt`: 替换系统提示词

---

## 改进方案

### 方案概述：引入"内置 Hook"概念

将当前的占位符逻辑和 reminder 逻辑重构为"内置 Hook"，作为系统默认行为：
- 内置 Hook 在用户 Hook 之前执行
- 可通过配置文件禁用特定内置 Hook
- 用户 Hook 可以感知/修改内置 Hook 的结果

### 架构设计

```
Hook 执行顺序：
[内置 Hook] → [用户级 Hook] → [项目级 Hook] → [Session级 Hook]

PreLlmRequest 事件点：
┌─────────────────────────────────────────────────────────────┐
│  1. BackgroundTasksInjector (内置)                          │
│     - 检查 running tasks，注入到 system_prompt 或 messages   │
│  2. TodoReminderInjector (内置)                             │
│     - 检查 nag 条件，注入 reminder message                   │
│  3. TasksSummaryInjector (内置)                             │
│     - 构建任务摘要，注入 system_prompt 占位符                │
│  4. SessionStateInjector (内置)                             │
│     - 构建会话状态摘要                                       │
│  5. 用户自定义 Hook (可选)                                  │
│     - 可感知/修改上述注入的内容                              │
└─────────────────────────────────────────────────────────────┘
```

### 具体改动

#### 1. 扩展 HookEvent（推荐方案）

**分析现有 HookEvent 的问题**：
- `PreLlmRequest` 发生在 system_prompt 构建之后、API 请求之前
- 但占位符替换（如 `{{.background_tasks}}`）在 `system_prompt_fn` 中完成，早于 Hook 执行
- 这导致内置 Hook 无法优雅地处理占位符替换

**推荐方案：新增两个事件**

```rust
pub enum HookEvent {
    // ... 现有事件
    PreSystemPromptBuild,   // 系统提示词构建前（用于占位符数据准备）
    PostSystemPromptBuild,  // 系统提示词构建后（用于 inject_messages）
}
```

**事件触发时机**：

```
[chat_app.rs:1950 system_prompt_fn 闭包]
         ↓
1. 调用所有 PreSystemPromptBuild 内置 Hook
   - 收集占位符数据（background_tasks, todos 等）
   - 返回 Map<placeholder, value>
         ↓
2. 执行占位符替换（使用 Hook 返回的数据）
         ↓
3. 调用所有 PostSystemPromptBuild 内置 Hook
   - 返回 inject_messages（reminder 类内容）
         ↓
4. 继续执行 PreLlmRequest Hook（用户级）
```

**为什么需要两个事件？**

| 事件 | 用途 | 输出 |
|------|------|------|
| PreSystemPromptBuild | 准备占位符数据 | `placeholder_values: HashMap<String, String>` |
| PostSystemPromptBuild | 注入 reminder 消息 | `inject_messages: Vec<ChatMessage>` |

**分离的好处**：
1. **职责清晰**：占位符处理和消息注入是两种不同类型的操作
2. **执行顺序明确**：先准备数据，再替换，再注入消息
3. **用户 Hook 可感知**：用户可以在 PreLlmRequest 看到最终的 system_prompt 和 inject_messages
4. **可扩展性**：未来新增占位符只需在 PreSystemPromptBuild 添加内置 Hook

#### 2. 新增 BuiltinHook trait 和实现

```rust
// src/command/chat/builtin_hook.rs

/// 内置 Hook：系统自动执行的 Hook
pub trait BuiltinHook: Send + Sync {
    /// Hook 名称（用于配置禁用）
    fn name(&self) -> &'static str;
    
    /// 关联的事件
    fn event(&self) -> HookEvent;
    
    /// 是否启用（可被配置覆盖）
    fn is_enabled(&self, config: &BuiltinHookConfig) -> bool;
    
    /// 执行逻辑：返回要注入的内容
    fn execute(&self, context: &mut HookContext) -> Option<BuiltinHookResult>;
}

pub struct BuiltinHookResult {
    /// 要追加到 system_prompt 的内容（用于占位符场景）
    pub system_prompt_append: Option<String>,
    /// 要注入的消息（用于 reminder 场景）
    pub inject_messages: Vec<ChatMessage>,
}
```

#### 3. 具体内置 Hook 实现

**BackgroundTasksInjector**:
```rust
pub struct BackgroundTasksInjector {
    manager: Arc<BackgroundManager>,
}

impl BuiltinHook for BackgroundTasksInjector {
    fn name(&self) -> &'static str { "background_tasks" }
    fn event(&self) -> HookEvent { HookEvent::PreLlmRequest }
    
    fn execute(&self, ctx: &mut HookContext) -> Option<BuiltinHookResult> {
        let running = self.manager.list_running();
        if running.is_empty() { return None; }
        
        let summary = build_running_summary_from_list(&running);
        Some(BuiltinHookResult {
            inject_messages: vec![build_system_reminder_msg(summary)],
            ..Default::default()
        })
    }
}
```

**TodoReminderInjector**:
```rust
pub struct TodoReminderInjector {
    manager: Arc<TodoManager>,
}

impl BuiltinHook for TodoReminderInjector {
    fn name(&self) -> &'static str { "todo_reminder" }
    fn event(&self) -> HookEvent { HookEvent::PreLlmRequest }
    
    fn execute(&self, ctx: &mut HookContext) -> Option<BuiltinHookResult> {
        if !self.manager.has_todos() { return None; }
        if self.manager.turns_since_last_call() < TODO_NAG_INTERVAL_ROUNDS { return None; }
        
        let summary = self.manager.format_todos_summary();
        let msg = build_todo_reminder_msg(summary);
        Some(BuiltinHookResult {
            inject_messages: vec![msg],
            ..Default::default()
        })
    }
}
```

**BackgroundNotificationInjector** (完成通知):
```rust
pub struct BackgroundNotificationInjector {
    manager: Arc<BackgroundManager>,
}

impl BuiltinHook for BackgroundNotificationInjector {
    fn name(&self) -> &'static str { "background_notification" }
    fn event(&self) -> HookEvent { HookEvent::PreLlmRequest }
    
    fn execute(&self, ctx: &mut HookContext) -> Option<BuiltinHookResult> {
        let notifications = self.manager.drain_notifications();
        if notifications.is_empty() { return None; }
        
        let msgs = notifications.into_iter()
            .map(|n| build_background_notification_msg(n))
            .collect();
        
        Some(BuiltinHookResult {
            inject_messages: msgs,
            ..Default::default()
        })
    }
}
```

#### 4. HookManager 扩展

```rust
pub struct HookManager {
    user_hooks: HashMap<HookEvent, Vec<HookDef>>,
    project_hooks: HashMap<HookEvent, Vec<HookDef>>,
    session_hooks: HashMap<HookEvent, Vec<HookDef>>,
    // 新增：内置 Hook
    builtin_hooks: HashMap<HookEvent, Vec<Arc<dyn BuiltinHook>>>,
    builtin_config: BuiltinHookConfig,
}

impl HookManager {
    /// 执行 Hook（内置 → 用户 → 项目 → Session）
    pub fn execute(&self, event: HookEvent, mut context: HookContext) -> Option<HookResult> {
        let mut result = HookResult::default();
        
        // 1. 先执行内置 Hook
        if let Some(builtins) = self.builtin_hooks.get(&event) {
            for hook in builtins {
                if hook.is_enabled(&self.builtin_config) {
                    if let Some(br) = hook.execute(&mut context) {
                        // 合并内置 Hook 结果
                        if let Some(append) = br.system_prompt_append {
                            // 追加到 system_prompt
                            let sp = context.system_prompt.get_or_insert_default();
                            sp.push_str(&append);
                        }
                        result.inject_messages.get_or_insert_default()
                            .extend(br.inject_messages);
                    }
                }
            }
        }
        
        // 2. 再执行用户/项目/Session Hook（可感知内置 Hook 的结果）
        // ... 现有逻辑
    }
}
```

#### 5. 配置支持

```yaml
# ~/.jdata/agent/config.yaml 或 .jcli/config.yaml
builtin_hooks:
  background_tasks: true      # 启用后台任务注入
  background_notification: true  # 启用后台完成通知
  todo_reminder: true         # 启用 Todo nag
  tasks_summary: true         # 启用任务摘要
  session_state: true         # 启用会话状态
```

#### 6. System Prompt 占位符简化

两种方案：

**方案 A：保留占位符，但由内置 Hook 解析**

```rust
// 内置 Hook 处理 {{.background_tasks}} 等占位符
// 而不是在 system_prompt_fn 中硬编码
```

**方案 B：移除占位符，改用 inject_messages**

```rust
// System prompt 只保留静态内容
// 动态内容全部通过 inject_messages 注入
// 好处：更统一，坏处：可能增加消息长度
```

---

## 实施步骤

### Phase 1: 基础架构 (预计 2-3 小时)

1. 创建 `src/command/chat/builtin_hook.rs`
2. 定义 `BuiltinHook` trait 和相关结构
3. 扩展 `HookManager` 支持内置 Hook

### Phase 2: Reminder 迁移 (预计 1-2 小时)

1. 实现 `BackgroundNotificationInjector`
2. 实现 `TodoReminderInjector`
3. 从 agent.rs 移除硬编码逻辑

### Phase 3: 占位符迁移 (预计 2-3 小时)

1. 实现 `BackgroundTasksInjector`（running tasks）
2. 实现 `TasksSummaryInjector`
3. 实现 `SessionStateInjector`
4. 简化 `system_prompt_fn` 中的占位符逻辑

### Phase 4: 配置支持 (预计 1 小时)

1. 添加 `BuiltinHookConfig` 结构
2. 支持配置文件禁用内置 Hook
3. 更新文档

---

## 预期收益

1. **代码整洁**：agent.rs 和 chat_app.rs 更简洁
2. **可扩展**：新增动态内容只需添加新的内置 Hook
3. **可配置**：用户可禁用特定内置 Hook
4. **Hook 优先级**：用户 Hook 可感知/修改内置 Hook 的结果
5. **统一机制**：占位符和 reminder 使用同一套机制

---

## 待讨论的问题

1. **占位符 vs inject_messages**：
   - 占位符适合注入到 system_prompt（如 tools, skills）
   - inject_messages 适合事件驱动的内容（如 background_notification）
   - 是否需要区分处理？

2. **内置 Hook 执行时机**：
   - 当前设计在 PreLlmRequest 执行
   - 是否需要更细粒度的事件（PreSystemPromptBuild）？

3. **用户 Hook 如何感知内置 Hook**：
   - 内置 Hook 先执行，结果写入 HookContext
   - 用户 Hook 通过 stdin JSON 看到内置 Hook 的结果
   - 是否需要明确区分"内置"和"用户"的来源？