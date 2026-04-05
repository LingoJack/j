# Plan: Typora 风格实时 Markdown 编辑器 + Editor 组件拆分调研

## 调研结论

---

### 一、当前 Editor 组件使用情况分析

#### 1. 调用点统计

| 调用位置 | 函数 | 用途 | 内容类型 |
|---------|------|------|----------|
| `src/command/chat/handler/tui_loop.rs:447` | `open_editor_on_terminal` | 编辑 System Prompt | Markdown 文本 |
| `src/command/chat/handler/tui_loop.rs:476` | `open_editor_on_terminal` | 编辑 Style | 纯文本 |
| `src/command/report.rs:175` | `open_multiline_editor_with_content` | 编辑日报 | Markdown 文本 |
| `src/command/report.rs:630` | `open_multiline_editor_with_content` | 编辑日报文件 | Markdown 文本 |
| `src/command/script.rs:44` | `open_multiline_editor_with_content` | 创建脚本 | Shell 脚本 |
| `src/command/script.rs:81` | `open_multiline_editor_with_content` | 编辑脚本 | Shell 脚本 |

#### 2. 分析结论

当前所有场景共用同一套编辑器实现，**暂不需要拆分**。

---

### 二、已有 Markdown 渲染能力

```
src/command/chat/markdown/
├── parser.rs        # Markdown → Ratatui Line 渲染（核心）
├── highlight.rs     # 代码语法高亮
├── image_cache.rs   # 图片缓存
└── image_loader.rs  # 图片加载
```

**已支持**：标题、加粗、斜体、删除线、链接、代码块、列表、表格、引用块等。

---

### 三、选定方案：行级渲染切换（接近 Typora）

#### 效果示意

```
┌─────────────────────────────────────────────────────────────┐
│  ◆ 标题一                    ← 渲染显示（蓝色加粗 + 下划线）   │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━ ← 分隔线                        │
│  |**粗体** 文字              ← 当前编辑行（显示源码 + 光标）   │
│  • 列表项                    ← 渲染显示                       │
│  • 另一个列表                ← 渲染显示                       │
│  ┌─ code ──────────────────┐ ← 代码块渲染                    │
│  │ fn main() {             │                                 │
│  │     println!("Hello");  │                                 │
│  └─────────────────────────┘                                 │
└─────────────────────────────────────────────────────────────┘
```

#### 核心原理

1. **当前编辑行**：显示原始 Markdown 源码，可编辑
2. **非编辑行**：显示渲染后的效果
3. **行切换**：光标移动时动态切换行的显示模式

---

### 四、技术实现方案

#### 1. 核心数据结构

```rust
struct MarkdownEditor<'a> {
    // 源码存储
    lines: Vec<String>,
    // 光标位置
    cursor_line: usize,
    cursor_col: usize,
    // 滚动偏移
    scroll_offset: usize,
    // 渲染缓存（行号 -> 渲染结果）
    rendered_cache: HashMap<usize, Vec<Line<'a>>>,
    // 主题
    theme: Theme,
    // 编辑模式
    mode: EditMode, // Normal, Insert, Visual, Command
}
```

#### 2. 渲染流程

```
每一帧渲染:
1. 遍历可见行（viewport 范围）
2. 判断每行是否为当前编辑行
   - 是：显示源码 + 光标
   - 否：调用 markdown_to_lines() 渲染
3. 处理滚动同步
```

#### 3. 关键难点与解决方案

| 难点 | 解决方案 |
|------|----------|
| 渲染行数 != 源码行数 | 维护 `源码行号 -> 渲染行号` 映射表 |
| 代码块渲染为多行 | 代码块作为一个整体单元处理 |
| 光标位置计算 | 编辑行使用源码位置，非编辑行跳转到对应源码行 |
| 性能优化 | 只渲染可见区域 + 缓存 |

#### 4. 行类型定义

```rust
enum LineType {
    // 普通行：一行源码 = 一行渲染
    Normal,
    // 标题：渲染后可能有前缀和分隔线
    Heading { level: u8 },
    // 代码块：多行源码 = 多行渲染
    CodeBlock { 
        start_line: usize, 
        end_line: usize,
        lang: String,
    },
    // 列表项：可能有嵌套
    ListItem { depth: usize },
    // 表格：多行源码 = 多行渲染
    Table { start_line: usize, end_line: usize },
}
```

---

### 五、实现步骤

#### Phase 1：基础框架（2 天）

- [ ] 创建 `src/tui/editor_markdown.rs`
- [ ] 实现基础编辑功能（插入、删除、换行）
- [ ] 实现行级渲染切换（简单文本）
- [ ] 集成 `markdown_to_lines()`

#### Phase 2：复杂块处理（2 天）

- [ ] 代码块的多行渲染处理
- [ ] 表格的多行渲染处理
- [ ] 光标在块内移动的逻辑

#### Phase 3：Vim 模式 + 搜索（2 天）

- [ ] 移植现有 Vim 模式逻辑
- [ ] 搜索时显示源码（搜索结果高亮）
- [ ] 撤销/重做

#### Phase 4：细节优化（1 天）

- [ ] 性能优化：渲染缓存
- [ ] 状态栏：行号、列号、修改标记
- [ ] 快捷键帮助

---

### 六、文件结构

```
src/tui/
├── editor.rs              # 通用编辑器（保持现有，用于脚本）
├── editor_markdown.rs     # Markdown 编辑器（新增）
│   ├── MarkdownEditor
│   ├── LineType
│   └── RenderCache
└── mod.rs

src/command/chat/markdown/
├── parser.rs              # 复用现有渲染逻辑
└── ...
```

---

### 七、调用点修改

| 文件 | 函数 | 修改 |
|------|------|------|
| `tui_loop.rs` | System Prompt 编辑 | 使用 `MarkdownEditor` |
| `tui_loop.rs` | Style 编辑 | 保持通用编辑器 |
| `report.rs` | 日报编辑 | 使用 `MarkdownEditor` |
| `script.rs` | 脚本编辑 | 保持通用编辑器 |

---

## 决策点确认

1. **方案选择**：行级渲染切换 ✓
2. **Vim 模式**：保留现有功能 ✓
3. **代码块处理**：作为整体单元渲染 ✓
