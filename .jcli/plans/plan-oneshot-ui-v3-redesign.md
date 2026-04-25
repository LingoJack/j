# Oneshot UI V3 重构计划

## 核心设计

- 左侧统一 2 空格 padding
- AI 回复前显示 `Sprite` 标签（与 TUI 对齐）
- 思考动画：复用 `ThinkingStyle` Braille 帧动画 + 呼吸灯
- 交互组件：工具确认用 `┌──┐` 直角边框，Ask 用 `╭──╮` 圆角边框
- 工具调用/结果行保持简洁

---

## 1. 整体布局

```
$ j ai 读取 oneshot.rs 文件并统计行数

  ⠹ 思考中...                              ← 帧动画 + 呼吸灯颜色

  ⚙ R1 · 2 工具
  📄 Read  src/command/chat/oneshot.rs
  🔧 Read ✓ 956 行 0.8s

  ⚙ R2 · 1 工具
  ⚡ Bash  wc -l oneshot.rs
  🔧 Bash ✓ 1 行输出 0.3s

  Sprite                                    ← AI 标签（label_ai 色，BOLD）
  ┌──────────────────────────────────────┐
  │  markdown 渲染的回复内容...            │  ← render_md 输出
  └──────────────────────────────────────┘

  会话 ID: abc123
```

**不每行加 `Sprite >` 前缀**，只在以下位置出现标签：
- 思考动画行（用主题色呼吸灯）
- AI 回复内容前（`Sprite` 标签行）

工具调用/结果行保持纯 2 空格缩进，不加标签前缀。

## 2. 思考动画

**对标 TUI**: `ThinkingStyle::frame(tick)` + `thinking_pulse_color()`

```
  ⠋ 思考中...
  (100ms)
  ⠙ 思考中...
  (100ms)
  ⠹ 思考中...
```

实现：
- 使用 `ThinkingStyle::Braille`（从 config 中读取用户偏好）
- 启动独立线程，`Arc<AtomicBool>` 控制停止
- `\r` + crossterm `Clear(CURRENT_LINE)` 回到行首重绘
- 帧间隔 100ms
- 颜色使用 `thinking_pulse_color()` 呼吸灯（正弦波，周期 1.5s，最低亮度 30%）
- 第一条文本 chunk 到来时停止动画

## 3. 工具确认框

**对标 TUI**: `render_tool_confirm_area` — `┌──┐` + 金黄色

```
  ┌─ ⚡ Bash 需要确认 ────────────────────────┐
  │  $ cargo build                             │
  │                                            │
  │  ❯ 允许执行                                │
  │    拒绝                                    │
  │    始终允许 (allow: Bash.command=...)       │
  │                                            │
  │  • ↑↓ 移动  Enter 确认                     │
  └────────────────────────────────────────────┘
```

- 左侧 2 空格缩进
- 直角边框 `┌─...─┐` / `│` / `└─...┘`
- 标题: 工具图标 + 工具名 + "需要确认"
- 参数: Bash 用 `$` 前缀显示命令，其他工具显示截断预览
- 选中项: `❯` + cyan BOLD
- 未选中: dimmed
- 宽度: `min(term_width - 4, 56)`

## 4. Ask 交互框

**对标 TUI**: `selector_block` — 圆角 `╭─╮`

**单选：**
```
  ╭─ 测试确认 ────────────────────────────────╮
  │  这是一条测试消息，Ask 工具是否正常工作？    │
  │                                            │
  │  ❯ 功能正常                                │
  │    Ask 工具运行正常，无需进一步测试          │
  │                                            │
  │    需要调试                                 │
  │    Ask 工具存在问题，需要排查               │
  │                                            │
  │  • ↑↓ 移动  Enter 确认                     │
  ╰────────────────────────────────────────────╯
```

**多选：**
```
  ╭─ 多选测试 ────────────────────────────────╮
  │  请选择以下选项                             │
  │                                            │
  │  ❯ ◉ 选项A                                │
  │    选项A描述                                │
  │                                            │
  │    ○ 选项B                                 │
  │    选项B描述                                │
  │                                            │
  │  • ↑↓ 移动  Space 切换  Enter 确认         │
  ╰────────────────────────────────────────────╯
```

- 圆角边框 `╭─...─╮` / `│` / `╰─...╯`
- 宽度: `min(term_width - 4, 56)`
- 每个选项占 2 行（label + description），之间空 1 行
- 选中/未选 checkbox: `◉`/`○`

## 5. 工具调用/结果行

保持简洁（不加标签前缀），与 TUI 折叠模式对齐：

```
  ⚙ R1 · 2 工具
  📄 Read  src/command/chat/oneshot.rs
  🔧 Read ✓ 956 行 1.2s
  ⚡ Bash  cargo build
  🔧 Bash ✓ 12 行输出 45.3s
```

颜色映射（通过 `ToolCategory::color()` + `Theme::terminal()`）：
- File 类: `label_user` 蓝
- Execute 类: `title_loading` 金黄
- Other: `text_dim` 灰
- 成功 `✓`: 绿
- 失败 `✗`: 红

## 6. AI 回复内容

在 markdown 渲染前显示标签行：

```
  Sprite                                     ← label_ai 色 BOLD
  (render_md 输出)
```

## 7. 其他元素

- 重试: `⟳ 重试中 (1/3, 2000ms) — 错误信息` （黄色）
- 压缩: `📦 压缩上下文中...` （dimmed）
- 中断: `⏹ 已中断` （dimmed）
- 会话 ID: `会话 ID: xxx` （dimmed）

---

## 实施步骤

1. **思考动画**: 从 config 导入 `ThinkingStyle`，添加动画线程
2. **工具确认框**: 重写 `interactive_confirm()` 使用 `┌──┐` 边框
3. **Ask 交互框**: 重写 Ask 线程使用 `╭──╮` 圆角边框
4. **工具行颜色**: 接入 `Theme::terminal()` + `ToolCategory::color()`
5. **AI 标签行**: 在 markdown 渲染前输出 `Sprite` 标签
6. **整体清理**: 统一缩进，清理冗余代码

---

## 技术要点

- 动画线程: `Arc<AtomicBool>` 控制停止，100ms 间隔
- 交互框光标: `MoveUp(total_lines)` + `Clear(FromCursorDown)` 全量重绘
- 宽度: `min(term_width().saturating_sub(4), 56)` 
- 颜色: `Theme::terminal()` 获取主题色 → `ratatui_color_to_colored()` 转换
- 呼吸灯: 复用 `thinking_pulse_color` 公式，应用到终端 ANSI 颜色
