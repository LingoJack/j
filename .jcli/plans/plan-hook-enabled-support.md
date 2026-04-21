# Hook Enabled/Disabled Support 实施计划

## 目标
让用户能够在配置界面中启用/禁用任意 Hook（包括 builtin、user、project、session 级别）。

## 现状分析

### 已有模式参考（Tools/Skills/Commands）
- `AgentConfig` 已有 `disabled_tools: Vec<String>`、`disabled_skills: Vec<String>`、`disabled_commands: Vec<String>`
- UI 使用 `toggle_list_item` 组件显示启用/禁用状态
- 通过 `Action::ToggleMenuToggle` 切换单项，`Action::ToggleMenuEnableAll/DisableAll` 批量操作

### Hooks 现状
- `HookEntry` 已有 `name`、`event`、`source`、`label`、`session_index` 等字段
- `HookManager.execute` 收集所有 hook（builtin → user → project → session）并执行
- UI (`hooks.rs`) 当前只展示信息，无交互功能
- `config_tab_field_count` 对 Hooks 返回 0（无可交互项）

### 存储架构差异
- **全局级（builtin/user/project）hook**：禁用状态应存于 `AgentConfig.disabled_hooks`（跨 session 持久化）
- **Session 级 hook**：禁用状态应存于 `SessionHookPersist` 中（session 状态文件 `hooks.json`）

当前 `SessionHookPersist` 结构：
```rust
pub struct SessionHookPersist {
    pub event: HookEvent,
    pub definition: HookDef,  // 无 enabled 字段
}
```

## 实施步骤

### Step 1: AgentConfig 添加 disabled_hooks 字段（全局级）

**文件**: `src/command/chat/storage/config.rs`

```rust
/// 被禁用的全局 hook 标识列表（格式: "{source}:{event}:{name}"）
/// 仅用于 builtin/user/project 级别，session 级别使用 SessionHookPersist.enabled
#[serde(default)]
pub disabled_hooks: Vec<String>,
```

### Step 2: SessionHookPersist 添加 enabled 字段（Session 级）

**文件**: `src/command/chat/storage/persist.rs`

```rust
pub struct SessionHookPersist {
    pub event: HookEvent,
    pub definition: HookDef,
    /// 是否启用（默认 true）
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool { true }
```

同步修改 `HookManager.session_hooks_snapshot()` 以保留 `enabled` 字段。

### Step 3: HookEntry 增加 enabled 字段 & 唯一标识方法

**文件**: `src/command/chat/infra/hook.rs`

1. 为 `HookEntry` 添加计算字段：
   ```rust
   /// 唯一标识符（用于 disabled_hooks 匹配，仅全局级使用）
   pub id: String,
   /// 是否启用
   pub enabled: bool,
   ```

2. 新增辅助函数生成全局级唯一标识：
   ```rust
   fn hook_unique_id(source: &str, event: HookEvent, kind: &HookKind) -> String {
       let name = hook_name(kind);
       match name {
           Some(n) => format!("{}:{}:{}", source, event.as_str(), n),
           None => format!("{}:{}:{}", source, event.as_str(), hook_label(kind)),
       }
   }
   ```

3. 在 `list_hooks` 方法中填充 `id` 和 `enabled`：
   - 全局级（builtin/user/project）：检查 `disabled_hooks` 列表
   - Session 级：从 `session_hooks` 中获取 `enabled` 状态（需修改 session_hooks 存储结构）

### Step 4: 修改 session_hooks 存储结构

**问题**: 当前 `session_hooks: HashMap<HookEvent, Vec<HookKind>>` 不存储 `enabled` 信息。

**方案**: 引入包装结构：
```rust
struct SessionHookEntry {
    kind: HookKind,
    enabled: bool,  // 默认 true
}

// 修改 HookManager
session_hooks: HashMap<HookEvent, Vec<SessionHookEntry>>,
```

同步修改：
- `register_session_hook()` → 默认 `enabled: true`
- `session_hooks_snapshot()` → 输出包含 `enabled` 字段
- `restore_session_hooks()` → 读取 `enabled` 字段

### Step 5: HookManager.execute 跳过 disabled hooks

**文件**: `src/command/chat/infra/hook.rs`

在 `execute` 方法中，执行每个 hook 前检查：

```rust
pub fn execute(
    &self,
    event: HookEvent,
    context: HookContext,
    disabled_hooks: &[String],  // 新增参数
) -> Option<HookResult> {
    // ...
    // builtin/user/project: 检查 disabled_hooks
    for hook in builtin_hooks.iter() {
        let id = hook_unique_id("builtin", event, hook);
        if disabled_hooks.contains(&id) { continue; }
        // ...
    }
    // session: 检查 SessionHookEntry.enabled
    for entry in session_hooks.iter() {
        if !entry.enabled { continue; }
        // ...
    }
}
```

### Step 6: UI hooks.rs 改造为交互式列表

**文件**: `src/command/chat/ui/config/hooks.rs`

1. 添加 header（显示启用/总数）：
   ```rust
   pub(super) fn draw_tab_hooks_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
       // 显示: "Hooks (n/m 启用)" + 操作提示
   }
   ```

2. 改造 body 为 `ItemList`（使用 `toggle_list_item`）：
   ```rust
   pub(super) fn draw_tab_hooks_list<'a>(app: &ChatApp) -> ItemList<'a> {
       // 每行显示: [source] [event] label (enabled/disabled)
       // session 级: "session:PreSendMessage:my-hook"
       // 全局级: "user:PostSendMessage:check-mod"
   }
   ```

### Step 7: UI config/mod.rs 调用 hooks 新方法

**文件**: `src/command/chat/ui/config/mod.rs`

修改 `ConfigTab::Hooks` 分支，调用新的 header 和 list 方法。

### Step 8: handler/config.rs 添加 Hooks 交互处理

**文件**: `src/command/chat/handler/config.rs`

在 `ConfigTab::Hooks` 分支添加键盘处理：
- `Space/Enter`: Toggle 当前选中 hook
- `e`: Enable all
- `d`: Disable all

### Step 9: chat_app.rs Action 处理逻辑

**文件**: `src/command/chat/app/chat_app.rs`

在 `Action::ToggleMenuToggle` 等处理中添加 Hooks 分支：

```rust
if app.ui.config_tab == ConfigTab::Hooks {
    // 获取当前选中 hook 的 source 和 id
    // 全局级: toggle AgentConfig.disabled_hooks
    // Session 级: toggle SessionHookEntry.enabled + save_hooks_state
}
```

### Step 10: 更新调用方传递 disabled_hooks

所有调用 `HookManager.execute` 的地方需传入 `disabled_hooks`：
- 搜索 `hook_manager.lock().execute(`
- 传入 `&app.state.agent_config.disabled_hooks`

## 存储策略总结

| Hook 来源 | 禁用存储位置 | 持久化文件 | 生命周期 |
|-----------|-------------|-----------|---------|
| builtin | `AgentConfig.disabled_hooks` | `agent_config.json` | 全局 |
| user | `AgentConfig.disabled_hooks` | `agent_config.json` | 全局 |
| project | `AgentConfig.disabled_hooks` | `agent_config.json` | 全局（项目切换后需更新） |
| session | `SessionHookPersist.enabled` | `hooks.json` | Session 内 |

## Hook 唯一标识格式

```
{source}:{event}:{name_or_label}
```

示例：
- `builtin:PreSendMessage:compact-pre-check`
- `user:PostSendMessage:my-hook`
- `session:PreToolCall:check-mod-rs`（但使用 `enabled` 字段而非 ID 匹配）

## 测试要点

1. 禁用 user/project hook → 重启后状态保持（`agent_config.json`）
2. 禁用 builtin hook → 执行时跳过
3. 禁用 session hook → 当前 session 内不执行，保存到 `hooks.json`
4. 全部启用/禁用 → 立即生效
5. Session 恢复时正确加载 `enabled` 状态

## 注意事项

- Hook 默认启用（`disabled_hooks` 初始为空，`enabled` 默认 true）
- Session hook 的 `session_index` 用于 remove 操作，`enabled` 用于临时禁用（不删除定义）
- UI 需区分显示全局级和 Session 级 hook（不同样式或分组）
- 需考虑 hook label 过长时的 UI 截断（已有 40 字符截断逻辑）