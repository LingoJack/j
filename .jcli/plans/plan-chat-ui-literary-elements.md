# Chat UI 文艺元素集成方案

## 目标

在 `src/command/chat/ui/` 的 TUI 界面中加入随机出现的文艺元素（诗句、文学名言等），为终端聊天体验增添温度和诗意。

## 可选展示位置分析

经过代码审阅，以下位置适合放置文艺元素：

| # | 位置 | 文件 | 优势 | 劣势 |
|---|------|------|------|------|
| A | **欢迎框问候语** | `components.rs:441` welcome_box() | 空会话时即展示，视觉焦点区域，有足够空间 | 只在无消息时可见 |
| B | **标题栏右侧** | `chat.rs:99` draw_title_bar() | 始终可见，可做副标题/座右铭 | 空间有限，需适配窄终端 |
| C | **输入框占位符** | `chat.rs:597` draw_input() | 用户注意焦点，idle 时可见 | 空间极有限，不能太长 |
| D | **帮助页底部** | `chat.rs:1096` draw_help() | 阅读停留时间长 | 只有按 ? 时才看到 |

## 推荐方案：仅 A（欢迎框诗句）

### 方案概述

1. **新建 `quotes.rs` 模块** - 存放文艺语录数据与随机选取逻辑
2. **改造欢迎框 (A)** - 移除原问候语和提示，仅展示一句随机诗句，极简设计

### 详细设计

#### 1. 新建 `assets/quotes.txt`

- 每行一句诗句，纯文本，UTF-8 编码
- 约 20 条精选中文诗句（混合古诗词、现代诗、散文金句）
- 空行自动跳过，便于排版
- 示例：
```
人生若只如初见
山有木兮木有枝
面朝大海，春暖花开
从前的日色变得慢
我见青山多妩媚
春风又绿江南岸
此心安处是吾乡
一蓑烟雨任平生
人间有味是清欢
浮生若梦，为欢几何
```

#### 2. `src/assets.rs` 新增便捷函数

遵循现有 `rust-embed` 模式，新增 `pub fn quotes_text() -> Cow<'static, str>` 加载 `assets/quotes.txt`。

#### 3. 新建 `src/command/chat/ui/quotes.rs`

- 提供 `fn get_quotes() -> Vec<&'static str>` 函数，解析 `quotes_text()` 按行拆分、过滤空行
- 提供 `fn random_quote(index: usize) -> &'static str` 函数，按索引取诗句（`index % len`）
- 首次调用时解析并缓存结果（使用 `std::sync::OnceLock`）

#### 4. 改造 `components.rs` 的 `welcome_box()`

当前 welcome_box 结构：
```
╭──────────────────────╮
│                      │
│  Hi! What can I...   │  <-- 问候语行（将移除）
│                      │
│  Type a message...   │  <-- 提示行（将移除）
│                      │
╰──────────────────────╯
```

改造后（仅保留诗句，极简设计）：
```
╭──────────────────────────╮
│                          │
│                          │
│  人生若只如初见          │  <-- 随机诗句（居中，暗色）
│                          │
│                          │
╰──────────────────────────╯
```

- **移除两行原文**：问候语 "Hi! What can I help you?" 和提示 "Type a message, press Enter"
- **不显示作者**，仅展示诗句正文
- 框体高度保持不变（利用原有空行作为视觉呼吸空间）
- 诗句居中显示，使用新颜色 `welcome_quote`
- 超长自动截断到 inner 宽度
- `welcome_box()` 函数签名新增 `quote_index: usize` 参数

### 语录数据设计

数据存放在 `assets/quotes.txt`，每行一句，约 20 条精选中文诗句，8-20 字之间。运行时通过 `rust-embed` 加载解析。

### 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `assets/quotes.txt` | 新建 | 诗句纯文本数据，每行一句 |
| `src/assets.rs` | 修改 | 新增 `quotes_text()` 便捷函数 |
| `src/command/chat/ui/quotes.rs` | 新建 | 解析 + 随机选取函数 |
| `src/command/chat/ui/mod.rs` | 修改 | 新增 `pub mod quotes;` |
| `src/command/chat/ui/components.rs` | 修改 | `welcome_box()` 移除原文，展示诗句 |
| `src/command/chat/theme.rs` | 修改 | Theme 新增 `welcome_quote` 颜色字段 |

### 实施步骤

1. 创建 `assets/quotes.txt`，写入精选诗句
2. 修改 `src/assets.rs`，新增 `quotes_text()` 函数
3. 创建 `quotes.rs` 模块，实现解析和选取函数
4. 修改 `theme.rs`，新增 `welcome_quote` 颜色字段（带默认值）
5. 修改 `components.rs` 的 `welcome_box()`，移除问候语/提示，集成诗句展示
6. 更新 `mod.rs` 导出新模块
7. 编译验证
