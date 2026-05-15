# Help TUI 重构：对齐 Notebook 布局

## 目标

将 Help 页面从"横向 Tab"布局重构为"左右分栏"布局（类似 Notebook），支持目录层级结构。

## 当前架构

### Help 布局（src/command/help/）
- Tab Bar（1 行）- 横向 Tab 列表
- Title Bar（3 行）- 标题 + 分隔线
- Content（flex）- Markdown 内容渲染
- Hint Bar（1 行）- 快捷键提示

### Notebook 布局（src/command/notebook/）
- Title Bar（3 行）- 标题栏
- Main Area（flex）- 左右分栏：
  - 左侧：笔记列表（树形结构，支持子目录展开/折叠）
  - 右侧：Markdown 编辑器
- Status Bar（3 行）- 状态栏/输入区
- Hint Bar（1 行）- 快捷键提示

## 重构方案

### 1. 数据模型改造

**现有**：
```rust
struct HelpTab {
    name: String,    // 从 frontmatter 读取
    content: String, // Markdown 内容
}
```

**改造后**：
```rust
/// 帮助文档条目（树形结构）
struct HelpEntry {
    kind: HelpEntryKind,
    guide: String, // 树形缩进引导线
}

enum HelpEntryKind {
    Dir {
        dir_path: String,   // 如 "chat"
        name: String,       // 显示名
        file_count: usize,  // 子文件数
    },
    File {
        path: String,       // 如 "quickstart" 或 "chat/commands"
        name: String,       // 显示名
        content: String,    // Markdown 内容
    },
}
```

### 2. 文件组织约定

**assets/help/** 目录结构：
```
assets/help/
├── quickstart.md        # 快速开始
├── alias.md             # 别名管理
├── daily.md             # 日报/周报
├── note.md              # 笔记
├── chat/
│   ├── commands.md      # 对话命令
│   └── tools.md         # 工具列表
├── script/
│   ├── basics.md        # 脚本基础
│   └── examples.md      # 脚本示例
├── hook.md              # Hook 系统
├── lock.md              # 文件加密
└── install.md           # 安装说明
```

**命名规则**：
- 文件名作为显示名（可中划线转空格，下划线转空格）
- 目录名同理
- 无需 frontmatter，完全从文件系统结构推断

### 3. UI 布局改造

```
┌──────────────────────────────────────────────────────────────┐
│  📖 j help — 共 N 篇文档                            [1/10]    │
├───────────────────┬──────────────────────────────────────────┤
│ 📂 chat (2)       │                                          │
│   💬 commands     │   # 对话命令                              │
│   🔧 tools        │                                          │
│ 📂 script (2)     │   | 命令 | 说明 |                         │
│   📜 basics       │   |------|------|                         │
│   📝 examples     │   | `/help` | 显示帮助 |                  │
│ 📄 quickstart     │   | `/clear` | 清空对话 |                 │
│ 📄 alias          │                                          │
│ 📄 daily          │   ## 快捷键                               │
│ 📄 note           │                                          │
│                   │                                          │
├───────────────────┴──────────────────────────────────────────┤
│ ←→ 切换 │ ↑↓ 滚动 │ Enter 展开 │ / 命令 │ q 退出              │
└──────────────────────────────────────────────────────────────┘
```

### 4. 交互设计

**Normal 模式**：
- `↑↓` / `jk` - 上下移动选中
- `←→` / `hl` - 左右切换（折叠目录 / 切换焦点）
- `Enter` - 展开/折叠目录，或选中文件
- `1-9, 0` - 快速跳转到第 N 个条目
- `/` - 打开命令面板
- `q` - 退出

**命令面板**：
- `theme` - 切换主题
- `help` - 返回首页
- `quit` - 退出

### 5. 实现步骤

#### 阶段 1：数据模型重构
1. 新增 `HelpEntry` / `HelpEntryKind` 类型
2. 新增 `load_help_entries()` 函数，从 assets 读取文件并构建树形结构
3. 新增 `ExpandedDirs` 状态持久化（复用 Notebook 的模式）

#### 阶段 2：UI 布局重构
1. 修改 `HelpApp` 状态，增加 `flat_entries`、`expanded_dirs` 等
2. 重写 `draw_ui`，采用左右分栏布局
3. 左侧渲染目录树（复用 Notebook 的 `render_list` 逻辑）
4. 右侧渲染 Markdown 预览（复用现有的 `markdown_to_lines`）

#### 阶段 3：交互逻辑
1. 新增目录展开/折叠逻辑
2. 修改键盘事件处理
3. 支持鼠标点击展开目录

#### 阶段 4：清理
1. 移除旧的 Tab 相关代码
2. 移除 frontmatter 解析逻辑
3. 更新所有 help 文件（移除 frontmatter）

### 6. 文件变更清单

**新增**：
- 无（复用现有模块）

**修改**：
- `src/assets/help.rs` - 新增 `load_help_entries()` 函数
- `src/command/help/app.rs` - 重构状态模型
- `src/command/help/ui.rs` - 重构 UI 布局
- `src/command/help.rs` - 更新事件处理

**移除**：
- `HelpTab` 结构体
- `load_help_tabs()` 函数
- frontmatter 相关解析逻辑

**更新**：
- `assets/help/*.md` - 移除 frontmatter，重新组织目录结构

### 7. 风险与考虑

1. **向后兼容**：现有 help 文件有 frontmatter，需要迁移
2. **性能**：文件数量较少，无性能问题
3. **国际化**：显示名从文件名推断，支持中文文件名

## 备选方案

### 方案 A：保留 Tab + 增加目录

保留顶层 Tab，每个 Tab 内部支持目录层级。这样改动较小，但用户体验不如完全的树形结构直观。

### 方案 B：完全复用 Notebook 代码

将 Help 实现为只读的 Notebook，共用 90% 的代码。但 Help 不需要编辑、新建、删除等功能，复用可能带来不必要的复杂度。

## 推荐

采用主方案（完全重构为树形布局），理由：
1. 用户体验更好，支持任意层级目录
2. 代码结构更清晰，专用于 Help 场景
3. 复用 Notebook 的树形渲染逻辑，开发量可控
