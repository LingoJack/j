# Tool Result 样式升级计划

## 目标

tool_result 渲染中，除了已有专用渲染的工具（Bash/Read/Diff/Todo/Agent/Compact 等），其余工具的 result 仍然使用"朴素"的灰色文本渲染（`text_dim` + 4 空格缩进），缺乏视觉层次。

需要为以下工具的 result 添加专用渲染：

## 现状分析

### 已有专用渲染（不需改动）

| 工具 | 渲染方式 | 状态 |
|------|----------|------|
| Read | 行号 + 语法高亮 | 已完成 |
| Bash | `$ 命令` 高亮 + 灰色输出 | 已完成 |
| Diff | 红/绿/蓝着色 | 已完成 |
| TodoRead/TodoWrite | ●/○ 列表 + 统计 | 已完成 |
| Agent/Teammate/Compact/LoadSkill/PlanMode | 边框嵌套显示 | 已完成 |
| Error | 红色 Error: 前缀 | 已完成 |

### 需要升级的工具 Result

| 工具 | 当前渲染 | 问题 | 升级方案 |
|------|----------|------|----------|
| **Glob** | 纯灰色文本 | 文件列表没有层次感 | 树形缩进 + 文件/目录颜色区分 + 文件数统计 |
| **Grep** | 纯灰色文本 | 搜索结果无视觉层次 | 文件名高亮 + 行号着色 + 匹配文本加粗 + 结果数统计 |
| **Write** | 纯灰色文本 | 只显示 "1 file written" | 文件路径高亮 + 操作摘要 |
| **Edit** | 纯灰色文本 | 同上 | 文件路径高亮 + diff 预览（如有） |
| **WebSearch** | 纯灰色文本 | 搜索结果无结构 | 标题高亮 + URL 灰色 + 摘要文本 |
| **WebFetch** | 纯灰色文本 | 网页内容无结构 | 标题高亮 + 正文折行 |
| **Task** | 纯灰色文本 | JSON 列表无层次 | 状态图标 + 标题 + ID 列表 |
| **SendMessage** | 纯灰色文本 | 消息文本无层次 | 发送目标高亮 + 消息内容 |

### 通用 fallback 也需改进

当前 `else` 分支（"正常结果"）：
```rust
// 纯灰色文本 + 4 空格缩进 + 折行
```

改进：对纯文本内容也增加视觉层次：
- 第一行内容用 `text_normal` 而非 `text_dim`
- 内容有行号前缀时保持现有逻辑
- 添加行数/字符数统计信息

## 详细方案

### 1. Glob Result：树形文件列表

```
🔍 Glob  ✓  找到 23 个文件
    src/
      command/
        chat/
          ui/
            chat.rs
            help.rs
      markdown/
        render/
          code_block.rs
    ...
    ... (共 23 个文件)
```

- 解析 content（每行一个路径）
- 提取公共前缀作为根目录
- 按层级缩进（`  ` 每层 +2 空格）
- 文件名用 `text_normal`，目录名用 `config_title` + Bold
- 截断超长列表

### 2. Grep Result：结构化搜索结果

```
🔍 Grep  ✓  找到 5 处匹配
    src/command/chat/ui/chat.rs
      123│ let theme = Theme::load();
      456│ fn draw_help(t: &Theme) {
    src/markdown/render.rs
      78│ impl Theme {
    ... (共 5 处匹配，3 个文件)
```

- 第一遍扫描：提取文件名 + 行号 + 内容
- 文件名用 `config_title` + Bold
- 行号用 `text_dim`
- 匹配内容用 `text_normal`
- 汇总：匹配数 + 文件数

### 3. Write/Edit Result：操作确认

```
📄 Write  ✓  src/main.rs (写入成功)
```

- 从 tool_args 提取 file_path
- 第一行摘要已由 `get_result_summary_for_tool` 处理
- 展开模式下显示文件路径 + 操作结果

### 4. WebSearch Result：结构化搜索结果

```
🌐 WebSearch  ✓  找到 5 个结果
    1. Rust Programming Language
       https://www.rust-lang.org
       A language empowering everyone to build reliable software.
    
    2. The Rust Book
       https://doc.rust-lang.org/book/
       The Rust Programming Language book.
    ...
```

- 解析搜索结果（标题/URL/摘要）
- 序号 + 标题（Bold + `text_normal`）
- URL 用 `text_dim`
- 摘要用 `text_dim`

### 5. WebFetch Result：内容预览

```
🌐 WebFetch  ✓  12.5KB
    # Page Title
    
    Content preview with proper wrapping...
```

- 自动检测 markdown 内容（标题、列表等）
- 有 markdown 标记时用 `markdown_to_lines` 渲染
- 纯文本时保持折行显示 + 内容大小统计

### 6. Task Result：结构化任务列表

```
⚡ Task  ✓  3 项任务
    #1  [completed]  重构 Help 布局
    #2  [in_progress]  代码块闭合折行
    #3  [pending]  清理旧代码
```

- 解析 JSON 任务列表
- 状态图标（同 Todo: ●/○/◉）
- 任务 ID + 标题

### 7. SendMessage Result：发送确认

```
✉️ SendMessage  ✓  已发送给 @Frontend
    消息内容预览...
```

- 从 tool_args 提取 to 目标
- 高亮目标名
- 展开模式显示消息预览

### 8. 通用 fallback 改进

```
🔧 ToolName  ✓  3 行, 128 字符
    第一行内容（text_normal）
    第二行内容（text_dim）
    第三行内容（text_dim）
```

## 实施优先级

1. **Glob** — 频率高，改动直观
2. **Grep** — 频率高，改动直观
3. **WebSearch** — 频率高，视觉提升大
4. **WebFetch** — 频率中
5. **Task** — 频率中
6. **Write/Edit** — 频率高但改动小
7. **SendMessage** — 频率低
8. **通用 fallback** — 兜底改进

## 文件变更

| 文件 | 变更 |
|------|------|
| `src/command/chat/render/cache/tool_result_render.rs` | 新增各工具专用渲染函数 + 修改 `render_tool_result_msg` 分发 |
