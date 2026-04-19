# Plan: Deferred Tool Loading（延迟工具加载）

## 概述

对齐 Claude Code 的 Deferred Tool Loading 机制：当工具数量超过阈值时，核心工具全量发送 schema，低频工具只发送名称（标记 defer_loading），通过 ToolSearch 工具让模型按需发现并加载完整定义。减少首次 API 调用的 token 开销。

---

## Claude Code 机制分析

### 核心概念

| 概念 | 说明 |
|------|------|
| `ToolSearchTool` | 特殊工具，让模型搜索延迟工具并获取完整 schema |
| `shouldDefer` | 工具属性，标记该工具是否应该延迟加载 |
| `defer_loading` | API 字段，表示只发送工具名不发送 schema |
| `isDeferredTool()` | 判断函数：MCP 工具默认延迟，或 shouldDefer=true |
| `alwaysLoad` | 工具属性，强制不延迟（优先级最高） |

### 工作流程

1. **请求构建阶段**：
   ```typescript
   // 1. 判断是否启用 ToolSearch（工具数量超阈值）
   const useToolSearch = isToolSearchEnabledOptimistic()

   // 2. 分离核心工具和延迟工具
   const deferredToolNames = new Set(tools.filter(isDeferredTool).map(t => t.name))

   // 3. 构建 schema，延迟工具添加 defer_loading: true
   const toolSchemas = filteredTools.map(tool =>
     toolToAPISchema(tool, {
       deferLoading: useToolSearch && deferredToolNames.has(tool.name)
     })
   )
   ```

2. **模型发现延迟工具**：
   - 通过 `<available-deferred-tools>` 或 system-reminder 消息看到延迟工具名称列表
   - 调用 `ToolSearch` 工具搜索：
     - `select:Read,Edit,Grep` — 精确选择
     - `notebook jupyter` — 关键词搜索
     - `+slack send` — 必须词 + 可选词

3. **ToolSearch 返回格式**：
   ```json
   {
     "matches": ["ReadFile", "EditFile"],
     "query": "select:Read,Edit",
     "total_deferred_tools": 15
   }
   ```
   返回的匹配工具完整 schema 会在下一轮请求中自动加入。

---

## j-cli 现状分析

### 当前工具发送机制

```rust
// tools/definition.rs:285
pub fn to_openai_tools_filtered(&self, disabled: &[String]) -> Vec<ChatCompletionTools> {
    self.tools.iter()
        .filter(|t| !disabled.iter().any(|d| d == t.name()))
        .map(|t| ChatCompletionTools::Function(ChatCompletionTool {
            function: FunctionObject {
                name: t.name().to_string(),
                description: Some(t.description().trim().to_string()),
                parameters: Some(t.parameters_schema()),
                strict: None,
            },
        }))
        .collect()
}
```

**问题**：所有工具 schema 全量发送，无延迟加载机制。

### 工具清单（约 20+）

| 工具 | 类型 | 建议 |
|------|------|------|
| Bash, Read, Write, Edit, Glob, Grep | 核心 | 全量发送 |
| WebFetch, WebSearch | 核心 | 全量发送 |
| Ask | 核心 | 全量发送 |
| Task, TaskOutput | 低频 | 可延迟 |
| TodoWrite, TodoRead | 低频 | 可延迟 |
| Compact | 低频 | 可延迟 |
| RegisterHook | 低频 | 可延迟 |
| EnterPlanMode, ExitPlanMode | 低频 | 可延迟 |
| EnterWorktree, ExitWorktree | 低频 | 可延迟 |
| LoadSkill | 低频 | 可延迟 |
| Agent, AgentTeam, CreateTeammate | 低频 | 可延迟 |
| ComputerUse | 低频 | 可延迟 |

---

## 改进方案

### Phase 1: 基础架构

#### Step 1.1: 扩展 Tool Trait

**文件**: `src/command/chat/tools/definition.rs`

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult;

    // 新增方法
    fn should_defer(&self) -> bool { false }  // 默认不延迟
    fn always_load(&self) -> bool { false }   // 强制全量发送（优先级最高）
    fn search_hint(&self) -> Option<&str> { None }  // 搜索提示词
}
```

#### Step 1.2: 工具分类常量

**新文件**: `src/command/chat/tools/classification.rs`（已存在，扩展）

```rust
/// 工具延迟加载阈值
pub const DEFER_THRESHOLD: usize = 15;

/// 核心工具列表（永不延迟）
pub const CORE_TOOL_NAMES: &[&str] = &[
    "Bash", "Read", "Write", "Edit", "Glob", "Grep",
    "WebFetch", "WebSearch", "Ask",
];

/// 低频工具默认延迟（当启用 ToolSearch 时）
pub const DEFERRED_BY_DEFAULT: &[&str] = &[
    "Task", "TaskOutput",
    "TodoWrite", "TodoRead",
    "Compact", "RegisterHook",
    "EnterPlanMode", "ExitPlanMode",
    "EnterWorktree", "ExitWorktree",
    "LoadSkill", "Agent", "AgentTeam", "CreateTeammate",
    "ComputerUse",
];

/// 判断工具是否应该延迟
pub fn should_defer_tool(tool: &dyn Tool, tool_search_enabled: bool) -> bool {
    // 1. always_load=true → 不延迟
    if tool.always_load() {
        return false;
    }
    // 2. 核心工具 → 不延迟
    if CORE_TOOL_NAMES.contains(&tool.name()) {
        return false;
    }
    // 3. ToolSearch 本身 → 不延迟
    if tool.name() == "ToolSearch" {
        return false;
    }
    // 4. 未启用 ToolSearch → 全量发送
    if !tool_search_enabled {
        return false;
    }
    // 5. 工具自身标记 或 默认延迟列表
    tool.should_defer() || DEFERRED_BY_DEFAULT.contains(&tool.name())
}
```

---

### Phase 2: ToolSearch 工具

#### Step 2.1: ToolSearch 工具实现

**新文件**: `src/command/chat/tools/tool_search.rs`

```rust
use crate::command::chat::tools::{Tool, ToolResult, schema_to_tool_params, parse_tool_args};
use async_openai::types::chat::{ChatCompletionTools, ChatCompletionTool, FunctionObject};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, atomic::AtomicBool};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToolSearchInput {
    /// 搜索查询："select:ToolA,ToolB" 精确选择，或关键词搜索
    query: String,
    /// 最大返回数量（默认 5）
    #[serde(default = "default_max_results")]
    max_results: usize,
}

fn default_max_results() -> usize { 5 }

#[derive(Debug, Serialize)]
pub struct ToolSearchOutput {
    /// 匹配的工具名称列表
    matches: Vec<String>,
    /// 原始查询
    query: String,
    /// 延迟工具总数
    total_deferred_tools: usize,
    /// 匹配工具的完整 schema 定义
    schemas: Vec<Value>,
}

pub struct ToolSearchTool {
    /// 对 ToolRegistry 的引用（用于获取工具列表和 schema）
    registry: Arc<std::sync::Mutex<ToolRegistryRef>>,
}

/// ToolRegistry 的轻量引用（只包含工具列表）
pub struct ToolRegistryRef {
    tools: Vec<Box<dyn Tool>>,
    deferred_tool_names: Vec<String>,
}

impl Tool for ToolSearchTool {
    fn name(&self) -> &str { "ToolSearch" }
    fn always_load(&self) -> bool { true }  // 永不延迟

    fn description(&self) -> &str {
        "搜索延迟加载的工具并获取完整 schema 定义。\
         支持两种查询方式：\
         - 'select:Read,Edit' — 精确选择指定工具\
         - 'file read' — 关键词搜索匹配工具"
    }

    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<ToolSearchInput>()
    }

    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        if cancelled.load(std::sync::atomic::Ordering::Relaxed) {
            return ToolResult {
                output: "已取消".to_string(),
                is_error: true,
                images: vec![],
                plan_decision: PlanDecision::None,
            };
        }

        let input: ToolSearchInput = match parse_tool_args(arguments) {
            Ok(i) => i,
            Err(e) => return e,
        };

        let registry = self.registry.lock().unwrap();
        let matches = search_tools(&input.query, &registry, input.max_results);

        // 构建匹配工具的完整 schema
        let schemas: Vec<Value> = matches
            .iter()
            .filter_map(|name| {
                registry.tools.iter()
                    .find(|t| t.name() == name)
                    .map(|t| build_tool_schema_json(t))
            })
            .collect();

        let output = ToolSearchOutput {
            matches: matches.clone(),
            query: input.query,
            total_deferred_tools: registry.deferred_tool_names.len(),
            schemas,
        };

        ToolResult {
            output: serde_json::to_string_pretty(&output).unwrap_or_default(),
            is_error: false,
            images: vec![],
            plan_decision: PlanDecision::None,
        }
    }
}

/// 构建单个工具的 JSON schema 表示
fn build_tool_schema_json(tool: &dyn Tool) -> Value {
    serde_json::json!({
        "name": tool.name(),
        "description": tool.description().trim(),
        "parameters": tool.parameters_schema()
    })
}

/// 搜索工具（精确选择或关键词匹配）
fn search_tools(query: &str, registry: &ToolRegistryRef, max_results: usize) -> Vec<String> {
    let query_lower = query.to_lowercase();

    // 1. 精确选择：select:ToolA,ToolB
    if let Some(names) = query_lower.strip_prefix("select:") {
        let requested: Vec<&str> = names.split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        return requested.iter()
            .filter_map(|name| {
                // 先在延迟工具中找
                if registry.deferred_tool_names.iter().any(|n| n.to_lowercase() == *name) {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .take(max_results)
            .collect();
    }

    // 2. 关键词搜索
    let keywords: Vec<&str> = query_lower.split_whitespace().collect();
    registry.deferred_tool_names.iter()
        .filter(|name| {
            let name_lower = name.to_lowercase();
            // CamelCase 分词匹配
            let parts: Vec<&str> = name_lower
                .replace('_', " ")
                .split_whitespace()
                .collect();
            keywords.iter().all(|kw| {
                parts.iter().any(|p| p.contains(kw)) || name_lower.contains(kw)
            })
        })
        .take(max_results)
        .cloned()
        .collect()
}
```

#### Step 2.2: 注册 ToolSearch 工具

**文件**: `src/command/chat/tools/definition.rs`（ToolRegistry::new）

```rust
// 在 ToolRegistry::new 中添加
Box::new(super::tool_search::ToolSearchTool {
    registry: Arc::new(Mutex::new(ToolRegistryRef {
        tools: vec![],  // 初始为空，稍后填充
        deferred_tool_names: vec![],
    })),
}),
```

---

### Phase 3: 请求构建改造

#### Step 3.1: 新增延迟加载 schema 构建方法

**文件**: `src/command/chat/tools/definition.rs`

```rust
/// 延迟加载配置
pub struct DeferredConfig {
    /// 是否启用延迟加载（工具数量超阈值）
    pub enabled: bool,
    /// 延迟工具名称列表
    pub deferred_names: Vec<String>,
}

impl ToolRegistry {
    /// 判断是否应启用延迟加载
    pub fn should_enable_defer(&self) -> bool {
        self.tools.len() > DEFER_THRESHOLD
    }

    /// 获取延迟加载配置
    pub fn get_deferred_config(&self) -> DeferredConfig {
        let enabled = self.should_enable_defer();
        let deferred_names = if enabled {
            self.tools.iter()
                .filter(|t| should_defer_tool(t.as_ref(), true))
                .map(|t| t.name().to_string())
                .collect()
        } else {
            vec![]
        };
        DeferredConfig { enabled, deferred_names }
    }

    /// 构建带延迟标记的工具 schema
    pub fn to_openai_tools_with_defer(
        &self,
        disabled: &[String],
        defer_config: &DeferredConfig,
    ) -> Vec<ChatCompletionTools> {
        self.tools.iter()
            .filter(|t| !disabled.iter().any(|d| d == t.name()))
            .map(|t| {
                let is_deferred = defer_config.enabled
                    && defer_config.deferred_names.contains(&t.name().to_string());

                if is_deferred {
                    // 延迟工具：只发送名称，带 defer_loading 标记
                    // 注意：OpenAI API 暂不支持 defer_loading 字段，
                    // 这里用简化描述 + 空 parameters 表示延迟状态
                    ChatCompletionTools::Function(ChatCompletionTool {
                        function: FunctionObject {
                            name: t.name().to_string(),
                            // 延迟工具用简短描述
                            description: Some(format!(
                                "延迟加载工具。使用 ToolSearch 'select:{}' 获取完整定义。",
                                t.name()
                            )),
                            // 延迟工具不发送 parameters
                            parameters: Some(serde_json::json!({"type": "object", "properties": {})),
                            strict: None,
                        },
                    })
                } else {
                    // 核心工具：全量发送
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

#### Step 3.2: 更新 agent loop 调用

**文件**: `src/command/chat/agent/agent_loop.rs`（run_main_agent_loop）

```rust
// 修改 tools 参数传递
let defer_config = tool_registry.get_deferred_config();
let tools = tool_registry.to_openai_tools_with_defer(&disabled_tools, &defer_config);

// 如果有延迟工具，添加 system-reminder 提示
let deferred_tools_hint = if defer_config.enabled && !defer_config.deferred_names.is_empty() {
    format!(
        "\n\n<available-deferred-tools>\n{}\n</available-deferred-tools>",
        defer_config.deferred_names.join("\n")
    )
} else {
    String::new()
};
// 将 hint 合入 system_prompt
```

#### Step 3.3: 更新 chat_app.rs 调用点

**文件**: `src/command/chat/app/chat_app.rs`

修改两处调用：
- `send_message` 方法（约 2408 行）
- `wake_from_inbox` 方法（约 3152 行）

```rust
// 原代码
let tools = if tools_enabled {
    self.tool_registry.to_openai_tools_filtered(&self.state.agent_config.disabled_tools)
} else { vec![] };

// 改为
let defer_config = self.tool_registry.get_deferred_config();
let tools = if tools_enabled {
    self.tool_registry.to_openai_tools_with_defer(
        &self.state.agent_config.disabled_tools,
        &defer_config
    )
} else { vec![] };
```

---

### Phase 4: System Prompt 集成

#### Step 4.1: 延迟工具列表提示

**文件**: `src/command/chat/agent_md.rs` 或 system prompt 构建处

在 system prompt 中添加延迟工具提示块：

```markdown
<available-deferred-tools>
Task
TodoWrite
Compact
EnterPlanMode
LoadSkill
Agent
</available-deferred-tools>

这些工具已延迟加载，只显示名称。使用 ToolSearch 工具搜索并获取完整定义：
- ToolSearch query="select:Task" — 精确选择
- ToolSearch query="todo task" — 关键词搜索
```

#### Step 4.2: 动态注入延迟工具列表

**文件**: `src/command/chat/app/chat_app.rs`（system_prompt_fn）

```rust
let system_prompt_fn: Arc<dyn Fn() -> Option<String> + Send + Sync> = Arc::new(move || {
    // ... 原有 system prompt 构建 ...

    // 添加延迟工具提示
    if defer_config.enabled {
        prompt.push_str(&format!(
            "\n\n<available-deferred-tools>\n{}\n</available-deferred-tools>\n\
            使用 ToolSearch 工具搜索并获取完整定义。",
            defer_config.deferred_names.join("\n")
        ));
    }

    Some(prompt)
});
```

---

### Phase 5: 工具分类标记

#### Step 5.1: 为低频工具添加 should_defer 标记

为每个低频工具实现 `should_defer() -> true`：

**示例**: `src/command/chat/tools/task.rs`

```rust
impl Tool for TaskTool {
    fn name(&self) -> &str { "Task" }
    fn should_defer(&self) -> bool { true }  // 新增
    // ...
}
```

需要修改的工具列表：
- `task.rs` - TaskTool
- `todo.rs` - TodoWriteTool, TodoReadTool
- `compact.rs` - CompactTool
- `hook.rs` - RegisterHookTool
- `plan.rs` - EnterPlanModeTool, ExitPlanModeTool
- `worktree.rs` - EnterWorktreeTool, ExitWorktreeTool
- `skill.rs` - LoadSkillTool
- `sub_agent.rs` - AgentTool
- `create_teammate.rs` - CreateTeammateTool, AgentTeamTool
- `computer_use.rs` - ComputerUseTool

---

## 文件改动清单

| 文件 | 改动类型 | 改动内容 |
|------|----------|----------|
| `tools/definition.rs` | 扩展 | Tool Trait 新增方法，DeferredConfig，to_openai_tools_with_defer |
| `tools/classification.rs` | 扩展 | DEFER_THRESHOLD, CORE_TOOL_NAMES, should_defer_tool |
| `tools/tool_search.rs` | 新建 | ToolSearchTool 实现 |
| `tools.rs` | 修改 | 添加 tool_search 模块导出 |
| `agent/agent_loop.rs` | 修改 | 使用 defer_config，注入延迟工具提示 |
| `agent/api.rs` | 可选修改 | 支持 defer_loading 字段（若 API 支持） |
| `app/chat_app.rs` | 修改 | 更新两处 to_openai_tools 调用 |
| 多个工具文件 | 扩展 | 实现 should_defer() 方法 |

---

## 测试计划

1. **单元测试**：
   - `should_defer_tool` 分类逻辑
   - ToolSearch 搜索匹配算法

2. **集成测试**：
   - 工具数量 < 阈值：全量发送
   - 工具数量 > 阈值：核心工具全量，低频工具延迟
   - ToolSearch 调用返回完整 schema

3. **边界测试**：
   - Plan Mode 下 ToolSearch 可用性
   - disabled_tools 与延迟加载的交互
   - 子 Agent 的工具传递

---

## 风险与兼容性

1. **API 兼容性**：
   - OpenAI API 暂不支持 `defer_loading` 字段
   - 方案：用简化描述 + 空 parameters 表示延迟状态
   - Claude API 可能支持，需要验证

2. **模型兼容性**：
   - 模型需要理解延迟工具提示格式
   - ToolSearch 返回 schema 格式需模型可解析

3. **向后兼容**：
   - 工具数量 < 阈值时行为不变
   - 可通过配置禁用延迟加载

---

## 配置项

**文件**: `~/.jcli/config.toml`

```toml
[agent]
# 延迟加载阈值（工具数量超过此值启用）
defer_threshold = 15

# 强制禁用延迟加载
disable_defer_loading = false
```

---

## 实施优先级

1. **Phase 1** (高)：基础架构 - Tool Trait 扩展，分类常量
2. **Phase 2** (高)：ToolSearch 工具实现
3. **Phase 3** (高)：请求构建改造
4. **Phase 4** (中)：System Prompt 集成
5. **Phase 5** (低)：工具分类标记（可渐进完成）