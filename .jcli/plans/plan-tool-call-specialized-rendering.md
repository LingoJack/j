# Tool Call 特化渲染计划

## 目标

为所有当前使用通用 JSON 渲染（`render_json_params_enhanced` / 截断预览 `{json...}`）的 tool call 请求和 tool call 结果，添加语义化的专用渲染。

## 现状分析

### 已有专用渲染（无需改动）
- **Tool Call 请求**: Bash, Agent, Teammate, ExitPlanMode
- **Tool Call 结果**: Bash, Agent/Teammate/Compact/LoadSkill/Plan(嵌套边框), Todo, Diff

### 需要添加专用渲染的工具（按优先级分组）

---

## 实现方案

### 修改文件范围
- `src/command/chat/render/cache/tool_call_render.rs` — 添加 tool call 请求的专用渲染
- `src/command/chat/render/cache/tool_result_render.rs` — 添加 tool call 结果的专用渲染（如需要）

### 优先级 P0：高频工具（核心工作流）

#### 1. Task（任务管理）
- **请求**: 显示 `action` 标签 + `title` / `description` 预览
- **折叠描述**: `extract_task_args` → `"create: 实现用户登录功能"` / `"list"` / `"update #42: 已完成"`

#### 2. TaskOutput（获取后台任务输出）
- **请求**: 显示 `task_id` + `block/wait` 状态
- **折叠描述**: `"检查任务 abc123 状态"`

#### 3. WebSearch（网络搜索）
- **请求**: 显示搜索关键词 `query` + 搜索类型
- **折叠描述**: `"搜索: Rust async best practices"`

#### 4. WebFetch（网页抓取）
- **请求**: 显示目标 URL + 模式
- **折叠描述**: `"抓取: https://example.com"`

#### 5. Ask（用户提问）
- **请求**: 显示问题文本 + 选项预览
- **折叠描述**: `"提问: 是否继续？"`

### 优先级 P1：中频工具

#### 6. TodoWrite / TodoRead（待办管理）
- **请求 TodoWrite**: 显示操作项数量 + 第一项预览
- **请求 TodoRead**: 显示 `"读取待办列表"`
- **折叠描述**: `"更新 3 项待办"` / `"读取待办"`

#### 7. Compact（对话压缩）
- **请求**: 显示 focus 内容预览
- **折叠描述**: `"压缩对话 (focus: 架构设计)"`

#### 8. EnterPlanMode（进入计划模式）
- **请求**: 显示描述字段
- **折叠描述**: `"进入计划模式: add-auth"`

#### 9. LoadSkill（加载技能）
- **请求**: 显示技能名称 + 参数预览
- **折叠描述**: `"加载技能: sql-to-go"`

### 优先级 P2：低频/辅助工具

#### 10. RegisterHook（注册钩子）
- **请求**: 显示 action + event
- **折叠描述**: `"注册钩子: pre_commit"`

#### 11. SendMessage（发送消息）
- **请求**: 显示目标 + 消息预览
- **折叠描述**: `"发送消息 → @Backend"`

#### 12. EnterWorktree / ExitWorktree（工作树管理）
- **请求**: 显示操作类型 + 名称
- **折叠描述**: `"进入工作树: feature-x"` / `"退出工作树"`

#### 13. WorkDone（工作完成声明）
- **请求**: 显示摘要
- **折叠描述**: `"完成: 已实现登录功能"`

#### 14. IgnoreMessage / ComputerUse / Browser
- **请求**: 简短语义描述
- **折叠描述**: 相应简短文本

---

## 实现步骤

### Step 1：在 `tool_call_render.rs` 中添加参数解析函数

为每个工具添加 `extract_xxx_args` 函数，解析 JSON arguments 并提取关键字段。遵循现有的 `extract_bash_args` / `extract_agent_args` 模式。

```rust
// 示例：Task 参数解析
struct TaskCallArgs {
    action: String,
    title: Option<String>,
    description_preview: Option<String>,
}

fn extract_task_args(args: &str) -> Option<TaskCallArgs> {
    let v: Value = serde_json::from_str(args).ok()?;
    Some(TaskCallArgs {
        action: v.get("action")?.as_str()?.to_string(),
        title: v.get("title").and_then(|t| t.as_str()).map(String::from),
        description_preview: v.get("description")
            .and_then(|d| d.as_str())
            .map(|s| truncate(s, 60)),
    })
}
```

### Step 2：在 `render_tool_call_request_msg` 的 match 中添加新分支

为每个工具添加专用的展开渲染函数，类似 `render_bash_call_request_expanded` 的模式。

### Step 3：更新 `extract_tool_description_from_args` 添加新工具的折叠描述

当前该函数只覆盖 Bash/Read/Write/Edit/Glob/Grep/Agent/Teammate，需要扩展覆盖所有工具。

### Step 4：验证

- `cargo fmt`
- `cargo clippy -- -D warnings`
- 手动运行确认渲染效果

---

## 工作量评估

| 步骤 | 工作量 |
|---|---|
| Step 1: 参数解析函数 (14 个工具) | 中 |
| Step 2: 展开模式渲染 (14 个工具) | 高 |
| Step 3: 折叠描述提取扩展 | 低 |
| Step 4: 验证 | 低 |

预计改动集中在 `tool_call_render.rs` 一个文件，约新增 300-400 行代码。
