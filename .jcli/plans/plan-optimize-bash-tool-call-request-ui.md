# 优化 Bash Tool Call Request 的 UI

## 问题分析

当前 `render_tool_call_request_msg` 在**展开模式**下对 Bash 工具的渲染存在以下不足：

1. **不显示完整 command**：展开模式下 `command` 字段和其他参数混在一起以 JSON 形式展示，不够直观
2. **不显示是否后台运行**：`run_in_background` 信息未突出展示
3. **参数展示不直观**：使用通用 JSON 参数渲染，缺少差异化处理

**折叠模式**保持现状不变，只显示 description。

## 优化方案

### 核心改动：`render_tool_call_request_msg` 函数 (cache.rs)

针对 Bash/Shell 工具，只在**展开模式**下优化渲染逻辑：

#### 展开模式（优化后）
- **第一行（标题行）**：保持现状 `⚡ Bash - <description>  ⏳`
- **command 行**：以 `$` 前缀高亮显示完整命令（支持多行命令折行），无额外 emoji
- **附加信息行**（如有）：
  - 如果 `run_in_background: true`：显示 `[background]`
  - 如果 `timeout` 有值且非 120 默认值：显示 `timeout: <value>s`
  - 如果 `cwd` 有值：显示 `cwd: <path>`
- `description` 参数不再在 JSON 参数中重复展示（已在标题行显示）

#### 折叠模式
保持现状不变。

### 涉及文件

1. **`src/command/chat/render/cache.rs`** — 主要修改
   - `render_tool_call_request_msg`：对 Bash 工具的展开模式添加特殊渲染分支
   - 新增 `render_bash_call_request_expanded`：展开模式下的 Bash 渲染
   - 新增辅助函数 `extract_bash_args`：从 arguments JSON 中提取所有 Bash 参数字段

### 不涉及的文件
- `classification.rs` — 工具分类逻辑不变
- `ui/chat.rs` — 标题栏的 loading 提示已使用 `tc.tool_description`，无需改动
- `tools/shell.rs` — 工具定义不变
- `constants.rs` — 不新增常量

### 渲染效果示例

#### 折叠模式（保持现状）
```
  ⚡ Bash  执行测试命令
```

#### 展开模式（当前）
```
  ⚡ Bash - 执行测试命令  ⏳
    command: "cargo test --lib"
    description: "执行测试命令"
    timeout: 300
    run_in_background: true
```

#### 展开模式（优化后）
```
  ⚡ Bash - 执行测试命令  ⏳
    $ cargo test --lib
    [background]  timeout: 300s  cwd: /path/to/project
```
