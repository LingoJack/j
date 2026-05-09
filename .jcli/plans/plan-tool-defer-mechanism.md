# Tool Defer 机制实现计划

## 需求概述

引入 Tool 的 defer 机制：
- 提供一个 `LoadTool` 工具，用于在运行时加载其他 Tool
- 未设置为 defer 的 Tool：和当前一样拼到 system prompt 里
- 设置为 defer 的 Tool：在 `LoadTool` 加载之后才拼到 prompt 里
- 是否 defer 可以在 `src/command/chat/ui/config/tools.rs` 设置，默认都不是 defer

## 核心设计

### 1. 数据模型变更

#### `AgentConfig` (`src/command/chat/storage/config.rs`)
- 新增 `deferred_tools: Vec<String>` 字段（默认空）
- 序列化/反序列化兼容（`#[serde(default)]`）

#### `ToolRegistry` (`src/command/chat/tools/definition.rs`)
- 新增 `deferred_tools: Vec<String>` 字段
- 新增方法：
  - `set_deferred_tools(&mut self, tools: Vec<String>)` — 设置哪些工具是 defer 的
  - `build_tools_summary_split(&self, disabled: &[String]) -> (String, String)` — 返回 (立即加载的工具摘要, defer 的工具摘要)
  - `to_llm_tools_filtered_split(&self, disabled: &[String]) -> (Vec<ToolDefinition>, Vec<ToolDefinition>)` — 返回 (立即加载的工具, defer 的工具)

#### `AgentLoopSharedState` (`src/command/chat/agent/config.rs`)
- 新增 `deferred_tools: Vec<String>` 字段（或复用现有机制）

### 2. System Prompt 构建变更

#### `SystemPromptBuilder` (`src/command/chat/app/system_prompt.rs`)
- 新增 `deferred_tools: Vec<String>` 字段
- `build()` 方法中：
  - 调用 `build_tools_summary_split()` 获取两部分工具摘要
  - 只将非 defer 的工具摘要拼入 system prompt
  - defer 的工具摘要不拼入（等 LoadTool 加载后再提供）
- `build_with_agent_md()` 同样处理

### 3. Agent Loop 变更

#### `run_main_agent_loop` (`src/command/chat/agent/agent_loop.rs`)
- 每轮获取工具时，默认只提供非 defer 的工具给 LLM
- 当 LLM 调用 `LoadTool` 时，后续轮次将该工具从 defer 列表中移除，使其可用

### 4. LoadTool 实现

#### 新建 `src/command/chat/tools/load_tool.rs`
- 工具名：`LoadTool`
- 参数：`{ "name": "工具名" }`
- 执行逻辑：
  - 检查指定工具是否存在于注册表中
  - 检查该工具是否在 defer 列表中
  - 如果在 defer 列表中，将其移出（即"加载"）
  - 返回加载结果
- 需要访问 `AgentConfig` 或共享状态来修改 defer 列表

**关键问题**：`Tool::execute()` 签名目前只接受 `arguments` 和 `cancelled`，无法访问共享状态。

**解决方案**：
- 方案 A：扩展 `Tool` trait 的 `execute` 方法（破坏性变更，影响所有工具）
- 方案 B：`LoadTool` 持有 `Arc<Mutex<Vec<String>>>` 引用 defer 列表（类似其他工具持有所需状态的模式）
- 方案 C：通过 `ToolRegistry` 的 `execute` 方法在调用前/后处理 defer 逻辑

**选择方案 B**：`LoadTool` 持有 `Arc<Mutex<Vec<String>>>` 引用 deferred_tools 列表，执行时修改该列表。这是最小侵入性的方案，与现有模式一致（如 `ShellTool` 持有 `BackgroundManager`）。

### 5. UI 配置变更

#### 交互设计（层级导航模式）

Tools tab 列表中所有工具名竖直排列，当前选中的工具下方展开显示两个选项（启用、defer）竖直排列。同时只能看到选中工具的选项。

```
  Shell
▸ Browser          ← 当前选中
    [启用 ✓]       ← 选项1：启用/禁用
    [defer  ]      ← 选项2：defer 开关
  PowerShell
  Read
```

**导航逻辑**：
- **工具列表层级**（默认）：`↑`/`↓` 切换选中的工具，`Tab` 进入选中工具的选项区
- **选项层级**：`↑`/`↓` 在"启用"和"defer"选项之间切换焦点，`Tab` 返回工具列表
- **Toggle**：`Enter`/`Space` 切换当前焦点选项的状态
- 默认所有工具都不是 defer 的
- 禁用的工具 defer 选项置灰（无意义）

**UI 状态**：
- `UiState` 新增 `tools_in_options: bool` 标记是否在选项层级
- `UiState` 新增 `tools_option_idx: usize` 标记当前焦点选项（0=启用, 1=defer）
- 进入选项层级时默认焦点在"启用"选项

#### `src/command/chat/ui/config/tools.rs`
- `draw_tab_tools_list`：
  - 非选中工具：只显示工具名
  - 选中工具：显示工具名 + 下方缩进显示两个选项（启用/defer）
  - 选项层级中焦点选项用不同样式高亮

#### `src/command/chat/app/chat_app/update_config.rs`
- `update_toggle_menu_toggle`：根据当前层级和焦点选项 toggle 对应状态
- `update_config_navigate`：根据层级决定上下键行为
  - 工具列表层级：上下切换工具
  - 选项层级：上下切换启用/defer
- `Tab` 键：在工具列表层级和选项层级之间切换

### 6. 持久化

#### `src/command/chat/storage/config.rs`
- `save_agent_config`：需要保存 `deferred_tools` 字段
- `load_agent_config`：需要加载 `deferred_tools` 字段

### 7. 子 Agent 传递

#### `DerivedAgentShared` (`src/command/chat/tools/derived_shared.rs`)
- 新增 `deferred_tools: Arc<Mutex<Vec<String>>>` 字段
- `build_child_registry`：将 deferred_tools 传递给子注册表

## 实现步骤

### Phase 1: 数据模型
1. `AgentConfig` 添加 `deferred_tools` 字段
2. `ToolRegistry` 添加 `deferred_tools` 和相关方法
3. `AgentLoopSharedState` 添加 `deferred_tools`
4. `DerivedAgentShared` 添加 `deferred_tools`

### Phase 2: System Prompt
1. `SystemPromptBuilder` 支持 defer 过滤
2. `ChatApp` 初始化时传递 deferred_tools

### Phase 3: Agent Loop
1. 每轮工具过滤时排除 defer 的工具
2. 实现 `LoadTool`

### Phase 4: UI
1. `UiState` 新增 `tools_in_options` 和 `tools_option_idx` 字段
2. Tools tab 渲染：选中工具展开显示启用/defer 选项
3. 键盘交互：Tab 切换层级，上下键根据层级切换
4. `update_config` 中处理层级切换和 defer toggle
5. `config_tab_field_count` 需要考虑展开选项对列表高度的影响

### Phase 5: 持久化与传递
1. 配置保存/加载包含 deferred_tools
2. 子 Agent 继承 deferred_tools

## 关键代码变更点

### `ToolRegistry` 新增方法

```rust
/// 设置哪些工具是 defer 的
pub fn set_deferred_tools(&mut self, tools: Vec<String>) {
    self.deferred_tools = tools;
}

/// 将工具按 defer 状态分组
fn partition_tools<'a>(&'a self, disabled: &'a [String]) -> (Vec<&'a dyn Tool>, Vec<&'a dyn Tool>) {
    self.tools
        .iter()
        .filter(|t| !disabled.iter().any(|d| d == t.name()))
        .filter(|t| t.is_available())
        .partition(|t| !self.deferred_tools.iter().any(|d| d == t.name()))
}

/// 构建非 defer 工具的摘要（用于 system prompt）
pub fn build_tools_summary_non_deferred(&self, disabled: &[String]) -> String {
    let (immediate, _) = self.partition_tools(disabled);
    // ... 构建 immediate 工具的摘要
}

/// 获取非 defer 的 LLM 工具定义
pub fn to_llm_tools_non_deferred(&self, disabled: &[String]) -> Vec<ToolDefinition> {
    let (immediate, _) = self.partition_tools(disabled);
    // ... 转换 immediate 工具
}
```

### `LoadTool` 结构

```rust
pub struct LoadTool {
    deferred_tools: Arc<Mutex<Vec<String>>>,
    tool_registry: Arc<ToolRegistry>,
}

impl Tool for LoadTool {
    fn name(&self) -> &str { "LoadTool" }
    
    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        // 解析参数获取工具名
        // 从 deferred_tools 中移除该工具名
        // 返回成功/失败信息
    }
}
```

### UI 交互示例

在 Tools tab 中，列表竖直排列，选中工具展开选项：
```
  Shell
▸ Browser              ← 当前选中工具
    [启用 ✓]           ← 选项1焦点：启用
    [defer  ]          ← 选项2：defer（未开启）
  PowerShell
  Read
```

按键映射：
- `↑`/`↓`：工具列表层级切换工具 / 选项层级切换启用/defer
- `Tab`：进入选中工具的选项区 / 从选项区返回工具列表
- `Enter`/`Space`：toggle 当前焦点选项的状态
- `e`/`a`：全局启用/禁用全部工具（现有功能保持）
- `E`/`A`：全局设置/取消全部 defer（新增）

## 风险与注意事项

1. **向后兼容**：`deferred_tools` 字段需要 `#[serde(default)]` 保证旧配置兼容
2. **禁用与 defer 的交互**：禁用的工具不应该出现在 defer 列表中（或 defer 状态对禁用工具无意义）
3. **子 Agent**：需要确保子 Agent 正确继承父 Agent 的 defer 设置
4. **LoadTool 自身**：LoadTool 本身不能是 defer 的（否则无法加载其他工具）
5. **TUI 输出规范**：所有后台线程日志走 `write_info_log` / `write_error_log`
