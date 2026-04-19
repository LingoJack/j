# Plan: Deferred Tool Loading（延迟工具加载）— 用户自主掌控方案

## 概述

对齐 Claude Code 的 Deferred Tool Loading 机制，但核心思路改变：**不通过代码硬编码工具分类，而是通过 AgentConfig 配置 + Config Panel UI 面板让用户自主掌控哪些工具延迟加载**。

核心设计：
1. `AgentConfig` 新增 `deferred_tools: Vec<String>` 字段，持久化到 session config
2. Config Panel 的 Tools Tab 增加三态切换：`启用` / `禁用` / `延迟`
3. 新增 `ToolSearch` 工具，让模型按需发现并获取延迟工具的完整 schema
4. 请求构建时根据 `deferred_tools` 列表决定发送全量还是简化 schema

---

## 改动清单

### Step 1: AgentConfig 新增 `deferred_tools` 字段

**文件**: `src/command/chat/storage.rs`

在 `AgentConfig` 中新增字段（位于 `disabled_tools` 之后）：

```rust
pub struct AgentConfig {
    // ... 现有字段 ...
    pub disabled_tools: Vec<String>,
    /// 新增：延迟加载的工具名称列表
    /// 列表中的工具不发送完整 schema，通过 ToolSearch 按需发现
    #[serde(default)]
    pub deferred_tools: Vec<String>,
    // ... 后续字段 ...
}
```

**初始化位置**: `src/command/chat/handler/tui_loop.rs:242`

```rust
// 在 agent_config 初始化处添加
deferred_tools: Vec::new(),
```

**理由**: 与 `disabled_tools` 并列，用户通过同一配置面板管理。`serde(default)` 保证向后兼容旧配置文件。

---

### Step 2: Config Panel Tools Tab 三态 UI

**目标**: 在 Tools Tab 中，每个工具项从 `ON/OFF` 二态变为 `ON/DEFER/OFF` 三态。

#### 2.1 新增 Action

**文件**: `src/command/chat/app/types.rs`（或 Action enum 所在位置）

```rust
// 在现有 Action 枚举中添加
Action::ToggleMenuToggleDefer,  // 将选中工具标记为"延迟"状态
```

#### 2.2 工具状态判断函数

**文件**: `src/command/chat/app/chat_app.rs`（或独立为 `tools_state.rs`）

```rust
/// 工具在 Config Panel 中的三态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolState {
    /// 启用：全量发送 schema
    Enabled,
    /// 禁用：不发送
    Disabled,
    /// 延迟：只发名称，通过 ToolSearch 发现
    Deferred,
}

impl ToolState {
    pub fn from_config(name: &str, disabled: &[String], deferred: &[String]) -> Self {
        if disabled.iter().any(|d| d == name) {
            Self::Disabled
        } else if deferred.iter().any(|d| d == name) {
            Self::Deferred
        } else {
            Self::Enabled
        }
    }
}
```

#### 2.3 更新 ToggleMenuToggle 处理（现有 Enter 键）

**文件**: `src/command/chat/app/chat_app.rs`

修改 `Action::ToggleMenuToggle` 的处理逻辑（约 1453 行）：

```rust
// 原逻辑：在 disabled 和 enabled 之间切换
// 新逻辑：循环三态 Enabled → Disabled → Deferred → Enabled
Action::ToggleMenuToggle => {
    if self.ui.config_tab == ConfigTab::Tools {
        let tool_names = self.tool_registry.tool_names();
        if let Some(name) = tool_names.get(self.ui.config_field_idx) {
            let name = name.to_string();
            let state = ToolState::from_config(
                &name,
                &self.state.agent_config.disabled_tools,
                &self.state.agent_config.deferred_tools,
            );
            match state {
                ToolState::Enabled => {
                    // Enabled → Disabled
                    self.state.agent_config.disabled_tools.push(name);
                }
                ToolState::Disabled => {
                    // Disabled → Deferred
                    if let Some(pos) = self.state.agent_config.disabled_tools.iter().position(|d| d == &name) {
                        self.state.agent_config.disabled_tools.remove(pos);
                    }
                    self.state.agent_config.deferred_tools.push(name);
                }
                ToolState::Deferred => {
                    // Deferred → Enabled
                    if let Some(pos) = self.state.agent_config.deferred_tools.iter().position(|d| d == &name) {
                        self.state.agent_config.deferred_tools.remove(pos);
                    }
                }
            }
        }
    }
    // ... Skills 和 Commands 部分保持不变 ...
}
```

#### 2.4 更新 ToggleMenuEnableAll / ToggleMenuDisableAll

**文件**: `src/command/chat/app/chat_app.rs`

```rust
Action::ToggleMenuEnableAll => {
    if self.ui.config_tab == ConfigTab::Tools {
        self.state.agent_config.disabled_tools.clear();
        self.state.agent_config.deferred_tools.clear();  // 新增：清除延迟列表
        self.show_toast("已启用全部工具", false);
    }
    // ...
}

Action::ToggleMenuDisableAll => {
    if self.ui.config_tab == ConfigTab::Tools {
        self.state.agent_config.disabled_tools = self
            .tool_registry
            .tool_names()
            .iter()
            .map(|n| n.to_string())
            .collect();
        self.state.agent_config.deferred_tools.clear();  // 新增：禁用时清除延迟
        self.show_toast("已禁用全部工具", false);
    }
    // ...
}
```

#### 2.5 更新 Tools Tab 渲染

**文件**: `src/command/chat/ui/config.rs`

修改 `render_tools_tab` 函数中每个工具项的渲染：

```rust
// 现有代码（约 705 行）：
// let is_enabled = !app.state.agent_config.disabled_tools.iter().any(|d| d == *name);

// 改为三态：
let state = ToolState::from_config(
    name,
    &app.state.agent_config.disabled_tools,
    &app.state.agent_config.deferred_tools,
);

// 根据三态渲染不同的标记
let (toggle_text, toggle_style) = match state {
    ToolState::Enabled => (
        TOGGLE_ON,
        Style::default().fg(t.config_toggle_on).add_modifier(Modifier::BOLD),
    ),
    ToolState::Deferred => (
        "◐",  // 半亮标记，表示延迟
        Style::default().fg(t.title_loading).add_modifier(Modifier::BOLD),  // 黄/橙色
    ),
    ToolState::Disabled => (
        TOGGLE_OFF,
        Style::default().fg(t.config_toggle_off),
    ),
};
```

同时更新底部快捷键提示：

```rust
// 现有提示
// "Enter/Space: 启用/禁用  a: 全部启用  d: 全部禁用  t: 总开关  Esc: 保存"

// 改为
// "Enter: 切换(启用→禁用→延迟)  a: 全部启用  d: 全部禁用  t: 总开关  Esc: 保存"
```

#### 2.6 更新 handler/config.rs 保存时机

**文件**: `src/command/chat/handler/config.rs`

Tools Tab 的 Esc 键保存已有，不需要额外改动。`save_agent_config` 会自动序列化 `deferred_tools`。

---

### Step 3: ToolSearch 工具

#### 3.1 新建 `src/command/chat/tools/tool_search.rs`

```rust
use super::definition::{Tool, ToolResult, parse_tool_args, schema_to_tool_params, PlanDecision};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToolSearchInput {
    /// 搜索查询。支持两种模式：
    /// - "select:ToolA,ToolB" — 精确选择指定工具
    /// - "file read" — 关键词搜索匹配工具名和描述
    query: String,
}

pub const NAME: &str = "ToolSearch";

pub struct ToolSearchTool {
    /// 持有 ToolRegistry 的引用，用于查找延迟工具
    registry: Arc<Mutex<ToolRegistrySnapshot>>,
}

/// ToolRegistry 的只读快照（由 ToolRegistry 在构建请求时更新）
pub struct ToolRegistrySnapshot {
    pub deferred_tool_names: Vec<String>,
    /// 延迟工具的完整 schema 缓存
    pub deferred_tool_schemas: Vec<(String, String, Value)>,  // (name, description, parameters)
}

impl ToolSearchTool {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Mutex::new(ToolRegistrySnapshot {
                deferred_tool_names: Vec::new(),
                deferred_tool_schemas: Vec::new(),
            })),
        }
    }

    pub fn snapshot_arc(&self) -> Arc<Mutex<ToolRegistrySnapshot>> {
        Arc::clone(&self.registry)
    }
}

impl Tool for ToolSearchTool {
    fn name(&self) -> &str { NAME }
    fn description(&self) -> &str {
        "搜索延迟加载的工具并获取完整定义。延迟工具只发送了名称，调用此工具获取完整参数定义。\n\
         查询模式：\n\
         - 'select:ToolA,ToolB' — 精确选择指定工具\n\
         - 'task todo' — 关键词搜索匹配工具"
    }
    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<ToolSearchInput>()
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        if cancelled.load(Ordering::Relaxed) {
            return ToolResult {
                output: "已取消".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        let input = match parse_tool_args::<ToolSearchInput>(arguments) {
            Ok(i) => i,
            Err(e) => return e,
        };

        let snapshot = self.registry.lock().unwrap();
        let result = search_deferred_tools(&input.query, &snapshot);

        ToolResult {
            output: result,
            is_error: false,
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }
}

/// 搜索延迟工具并返回格式化结果
fn search_deferred_tools(query: &str, snapshot: &ToolRegistrySnapshot) -> String {
    let query_lower = query.to_lowercase();
    let matched_schemas: Vec<&(String, String, Value)>;

    // 精确选择模式：select:ToolA,ToolB
    if let Some(names) = query_lower.strip_prefix("select:") {
        let requested: Vec<&str> = names.split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        matched_schemas = snapshot.deferred_tool_schemas.iter()
            .filter(|(name, _, _)| {
                requested.iter().any(|r| r.eq_ignore_ascii_case(name))
            })
            .collect();
    } else {
        // 关键词搜索
        let keywords: Vec<&str> = query_lower.split_whitespace().collect();
        matched_schemas = snapshot.deferred_tool_schemas.iter()
            .filter(|(name, desc, _)| {
                let search_text = format!("{} {}", name.to_lowercase(), desc.to_lowercase());
                keywords.iter().all(|kw| search_text.contains(kw))
            })
            .collect();
    }

    if matched_schemas.is_empty() {
        return format!(
            "未找到匹配的延迟工具。当前可用的延迟工具：\n{}",
            snapshot.deferred_tool_names.join(", ")
        );
    }

    let mut output = String::new();
    output.push_str(&format!(
        "找到 {} 个延迟工具（共 {} 个）：\n\n",
        matched_schemas.len(),
        snapshot.deferred_tool_names.len()
    ));

    for (name, desc, params) in &matched_schemas {
        output.push_str(&format!("## {}\n{}\n\n参数定义：\n", name, desc.trim()));
        output.push_str(&serde_json::to_string_pretty(params).unwrap_or_default());
        output.push_str("\n\n");
    }

    output
}
```

#### 3.2 在 tools.rs 中导出

**文件**: `src/command/chat/tools.rs`

```rust
pub mod tool_search;  // 新增
```

#### 3.3 在 tool_names 中添加常量

**文件**: `src/command/chat/tools.rs`

```rust
pub mod tool_names {
    // ... 现有常量 ...
    pub const TOOL_SEARCH: &str = super::tool_search::NAME;  // 新增
}
```

#### 3.4 在 ToolRegistry::new 中注册 ToolSearchTool

**文件**: `src/command/chat/tools/definition.rs`

```rust
// 在 ToolRegistry::new 的 tools vec 中添加
Box::new(super::tool_search::ToolSearchTool::new()),
```

注意：ToolSearchTool 的 snapshot 需要在构建请求前更新。ToolRegistry 需要持有 snapshot 的引用。

---

### Step 4: ToolRegistry 集成 snapshot 更新

**文件**: `src/command/chat/tools/definition.rs`

#### 4.1 ToolRegistry 新增 snapshot 字段

```rust
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
    pub todo_manager: Arc<super::todo::TodoManager>,
    pub plan_mode_state: Arc<super::plan::PlanModeState>,
    pub worktree_state: Arc<super::worktree::WorktreeState>,
    pub permission_queue: Option<Arc<PermissionQueue>>,
    pub plan_approval_queue: Option<Arc<super::plan::PlanApprovalQueue>>,
    /// 新增：ToolSearch 工具的快照引用
    tool_search_snapshot: Option<Arc<Mutex<super::tool_search::ToolRegistrySnapshot>>>,
}
```

#### 4.2 ToolRegistry::new 中获取 snapshot

```rust
// 在构建 ToolSearchTool 时保存 snapshot 引用
let tool_search = super::tool_search::ToolSearchTool::new();
let tool_search_snapshot = tool_search.snapshot_arc();
// ...
tools.push(Box::new(tool_search));

let mut registry = Self {
    // ...
    tool_search_snapshot: Some(tool_search_snapshot),
};
```

#### 4.3 新增 `to_openai_tools_with_defer` 方法

```rust
impl ToolRegistry {
    /// 构建带延迟加载的工具 schema，同时更新 ToolSearch snapshot
    pub fn to_openai_tools_with_defer(
        &self,
        disabled: &[String],
        deferred: &[String],
    ) -> Vec<ChatCompletionTools> {
        // 1. 更新 ToolSearch snapshot
        if let Some(ref snapshot_arc) = self.tool_search_snapshot {
            let deferred_schemas: Vec<(String, String, Value)> = self.tools.iter()
                .filter(|t| deferred.iter().any(|d| d == t.name()))
                .map(|t| (t.name().to_string(), t.description().to_string(), t.parameters_schema()))
                .collect();
            if let Ok(mut snapshot) = snapshot_arc.lock() {
                snapshot.deferred_tool_names = deferred.to_vec();
                snapshot.deferred_tool_schemas = deferred_schemas;
            }
        }

        // 2. 构建 tools schema
        self.tools.iter()
            .filter(|t| !disabled.iter().any(|d| d == t.name()))
            .map(|t| {
                let is_deferred = deferred.iter().any(|d| d == &t.name());

                if is_deferred {
                    ChatCompletionTools::Function(ChatCompletionTool {
                        function: FunctionObject {
                            name: t.name().to_string(),
                            description: Some(format!(
                                "(延迟加载) 使用 ToolSearch 'select:{}' 获取完整参数定义。",
                                t.name()
                            )),
                            parameters: Some(serde_json::json!({
                                "type": "object",
                                "properties": {}
                            })),
                            strict: None,
                        },
                    })
                } else {
                    ChatCompletionTools::Function(ChatCompletionTool {
                        function: FunctionObject {
                            name: t.name().to_string(),
                            description: Some(t.description().trim().to_string()),
                            parameters: Some(t.parameters_schema()),
                            strict: None,
                        },
                    })
                }
            })
            .collect()
    }
}
```

#### 4.4 新增 `build_tools_summary_with_defer` 方法

类似 `build_tools_summary`，但延迟工具只输出名称和提示：

```rust
pub fn build_tools_summary_with_defer(&self, disabled: &[String], deferred: &[String]) -> String {
    let mut md = String::new();
    for t in self.tools.iter()
        .filter(|t| !disabled.iter().any(|d| d == t.name()))
    {
        let name = t.name();
        if deferred.iter().any(|d| d == name) {
            // 延迟工具：只输出名称提示
            md.push_str(&format!("<{}>\n(延迟加载，使用 ToolSearch 'select:{}' 获取完整定义)\n<{}/>\n\n", name, name, name));
        } else {
            md.push_str(&format!("<{}>\n", name));
            md.push_str(&format!("description:\n{}\n", t.description().trim()));
            md.push_str(&json_schema_to_xml_params(&t.parameters_schema()));
            md.push_str(&format!("<{}/>\n\n", name));
        }
    }
    md.trim_end().to_string()
}
```

---

### Step 5: 请求构建调用更新

#### 5.1 chat_app.rs — send_message 方法（约 2408 行）

```rust
// 原代码
// let tools = if tools_enabled {
//     self.tool_registry.to_openai_tools_filtered(&self.state.agent_config.disabled_tools)
// } else { vec![] };

// 改为
let disabled_tools = self.state.agent_config.disabled_tools.clone();
let deferred_tools = self.state.agent_config.deferred_tools.clone();
let tools = if tools_enabled {
    self.tool_registry.to_openai_tools_with_defer(&disabled_tools, &deferred_tools)
} else { vec![] };
```

#### 5.2 chat_app.rs — system_prompt_fn（约 2424 行）

```rust
// 在 system_prompt_fn 闭包中
// 原代码
// let tools_summary = tool_registry.build_tools_summary(&disabled_tools);

// 改为
let tools_summary = tool_registry.build_tools_summary_with_defer(&disabled_tools, &deferred_tools);

// 并在 prompt 末尾添加延迟工具列表提示
if !deferred_tools.is_empty() {
    prompt.push_str(&format!(
        "\n\n<available-deferred-tools>\n{}\n</available-deferred-tools>",
        deferred_tools.join("\n")
    ));
}
```

#### 5.3 chat_app.rs — wake_from_inbox 方法（约 3152 行）

同 5.1 和 5.2，修改 tools 构建和 system prompt。

#### 5.4 oneshot.rs — oneshot 模式（约 419 行）

```rust
// 原代码
// let openai_tools = tool_registry.to_openai_tools_filtered(&agent_config.disabled_tools);

// 改为
let openai_tools = tool_registry.to_openai_tools_with_defer(
    &agent_config.disabled_tools,
    &agent_config.deferred_tools,
);
```

同时更新 `resolve_oneshot_system_prompt` 使用 `build_tools_summary_with_defer`。

#### 5.5 子 Agent 工具构建

**文件**: `src/command/chat/tools/sub_agent.rs`（约 203 行）

```rust
// 原代码
// let tools = child_registry.to_openai_tools_filtered(&disabled);

// 改为
let deferred = self.shared.deferred_tools.as_ref().clone();  // 需要在 DerivedAgentShared 中添加
let tools = child_registry.to_openai_tools_with_defer(&disabled, &deferred);
```

**文件**: `src/command/chat/tools/derived_shared.rs`

```rust
pub struct DerivedAgentShared {
    // ... 现有字段 ...
    pub disabled_tools: Arc<Vec<String>>,
    pub deferred_tools: Arc<Vec<String>>,  // 新增
}
```

**文件**: `src/command/chat/app/chat_app.rs`（DerivedAgentShared 构建处）

```rust
let deferred_tools_arc = Arc::new(agent_config.deferred_tools.clone());
let derived_agent_shared = DerivedAgentShared {
    // ... 现有字段 ...
    disabled_tools: Arc::clone(&disabled_tools_arc),
    deferred_tools: Arc::clone(&deferred_tools_arc),  // 新增
};
```

---

### Step 6: Plan Mode 兼容性

**文件**: `src/command/chat/tools/plan.rs`

确保 ToolSearch 在 plan mode 下可用（它是只读工具）：

```rust
// 在 plan.rs 的 is_allowed_in_plan_mode 函数中添加 ToolSearch
pub fn is_allowed_in_plan_mode(name: &str) -> bool {
    matches!(name,
        // ... 现有允许列表 ...
        | tool_names::TOOL_SEARCH  // 新增
    )
}
```

---

## 文件改动清单

| 文件 | 改动类型 | 改动内容 |
|------|----------|----------|
| `storage.rs` | 扩展 | AgentConfig 新增 `deferred_tools` 字段 |
| `handler/tui_loop.rs` | 扩展 | 初始化 `deferred_tools: Vec::new()` |
| `tools/tool_search.rs` | **新建** | ToolSearchTool 实现 |
| `tools.rs` | 扩展 | 添加 `tool_search` 模块和 `TOOL_SEARCH` 常量 |
| `tools/definition.rs` | 扩展 | ToolRegistry 新增 snapshot 字段，`to_openai_tools_with_defer`，`build_tools_summary_with_defer` |
| `app/chat_app.rs` | 修改 | ToggleMenuToggle 三态逻辑，send_message/wake_from_inbox 调用更新，DerivedAgentShared 新增字段 |
| `app/types.rs` | 扩展 | ToolState 枚举 |
| `ui/config.rs` | 修改 | Tools Tab 三态渲染 |
| `handler/config.rs` | 修改 | Tools Tab 键盘提示文本 |
| `tools/derived_shared.rs` | 扩展 | 新增 `deferred_tools` 字段 |
| `tools/sub_agent.rs` | 修改 | 使用 `to_openai_tools_with_defer` |
| `tools/create_teammate.rs` | 修改 | 使用 `to_openai_tools_with_defer` |
| `oneshot.rs` | 修改 | 使用 `to_openai_tools_with_defer` 和 `build_tools_summary_with_defer` |
| `tools/plan.rs` | 修改 | `is_allowed_in_plan_mode` 添加 ToolSearch |

---

## 用户交互设计

### Config Panel — Tools Tab

```
╔══════════════════════════════════════════════════════╗
║  Model  │  Global  │ [Tools] │  Skills  │  Hooks    ║
╠══════════════════════════════════════════════════════╣
║                                                      ║
║  ✓ Bash               执行 shell 命令                ║
║  ✓ Read               读取文件                       ║
║  ✓ Write              写入文件                       ║
║  ✓ Edit               编辑文件                       ║
║  ✓ Glob               文件搜索                       ║
║  ✓ Grep               内容搜索                       ║
║  ✓ WebFetch           获取 URL 内容                  ║
║  ✓ WebSearch          网页搜索                       ║
║  ✓ Ask                向用户提问                     ║
║  ✓ ToolSearch         搜索延迟工具                   ║
║  ◐ Task               [延迟] 使用 ToolSearch 加载    ║
║  ◐ TodoWrite          [延迟] 使用 ToolSearch 加载    ║
║  ◐ TodoRead           [延迟] 使用 ToolSearch 加载    ║
║  ◐ Compact            [延迟] 使用 ToolSearch 加载    ║
║  ✗ RegisterHook       [禁用]                         ║
║  ✓ EnterPlanMode      进入计划模式                   ║
║  ...                                                 ║
║                                                      ║
║  Enter: 切换(启用→禁用→延迟)  a: 全部启用            ║
║  d: 全部禁用  t: 总开关  Esc: 保存返回               ║
╚══════════════════════════════════════════════════════╝
```

**三态循环**: Enter 键按下时 `启用(✓) → 禁用(✗) → 延迟(◐) → 启用(✓)`

**快捷键**:
- `Enter/Space`: 循环切换三态
- `a`: 全部启用（清除 disabled + deferred）
- `d`: 全部禁用（所有工具加入 disabled，清除 deferred）
- `t`: 工具调用总开关
- `Esc`: 保存并退出

---

## 测试计划

1. **配置持久化**: 设置 deferred_tools 后重启，验证配置恢复
2. **三态切换**: Enter 键正确在三个状态间循环
3. **ToolSearch 调用**: 模型调用 ToolSearch 返回正确 schema
4. **延迟工具请求**: deferred 工具发送简化 schema，核心工具发送全量
5. **子 Agent 传递**: 子 Agent 正确继承 deferred_tools 配置
6. **Plan Mode**: ToolSearch 在 plan mode 下可用
7. **向后兼容**: 无 deferred_tools 字段的旧配置正常加载

---

## 实施优先级

1. **Step 1** (基础设施): AgentConfig 新增字段 + 初始化
2. **Step 2** (UI): Config Panel 三态渲染
3. **Step 3** (核心): ToolSearch 工具实现
4. **Step 4** (集成): ToolRegistry snapshot + to_openai_tools_with_defer
5. **Step 5** (调用更新): 所有调用点切换到新方法
6. **Step 6** (兼容): Plan Mode + 子 Agent