# Ask 弹窗自由输入 UX 优化方案

## 现状分析

当前 Ask 弹窗的"自由输入"交互存在以下体验问题：

1. **需要 Enter 激活**：用上下键导航到"自由输入"行后，还需要按 Enter 才能进入输入模式
2. **暂存丢失**：在自由输入模式下按 Esc 或上下键离开后，已输入的内容被 `clear()` 清空
3. **上键冲突**：在自由输入模式中按上键无法回到选项列表（只能 Esc 退出并丢失内容）

> 注：当前已有"直接打字自动跳到自由输入"的快捷路径（`handle_ask_mode` 中 `KeyCode::Char(c)` 分支），但该路径同样会 `clear()` 已有内容。

## 优化目标

1. **导航即输入**：光标初始位置仍在第一个选项（不变）；但当光标**导航到**自由输入行时，直接就是可输入状态（`tool_interact_typing = true`），不需要按 Enter 激活
2. **暂存内容**：在自由输入行和选项行之间切换时，保留已输入的草稿内容
3. **上键返回**：在自由输入模式中按上键（光标已在行首位置时）退回选项列表，并保留草稿

## 修改计划

### 1. UIState 新增字段（`ui_state.rs`）

```rust
/// ask 自由输入草稿缓存（每个问题的草稿独立存储）
pub tool_ask_drafts: Vec<String>,
```

每个问题对应一个 `String`，用于暂存自由输入的草稿内容。

### 2. 初始化逻辑（`stream_poll.rs` - ask 弹窗触发入口）

- 初始化 `tool_ask_drafts` 为 `vec!["".to_string(); questions.len()]`
- `init_ask_question_state` 不变：cursor 仍初始化在第一个选项（index 0）

### 3. 选项间导航逻辑（`update_tool_interact.rs` - `update_ask_option_navigate`）

**核心改动**：进入/离开自由输入行时处理草稿保存与恢复。

- **离开自由输入行**时：
  - 保存当前 `tool_interact_input` 到 `tool_ask_drafts[current_idx]`
  - 不 clear input
  - 设置 `tool_interact_typing = false`
- **进入自由输入行**时：
  - 从 `tool_ask_drafts[current_idx]` 恢复草稿到 `tool_interact_input`
  - 光标放在末尾
  - 设置 `tool_interact_typing = true`（**直接可输入，无需 Enter**）

### 4. 键盘处理（`tool_confirm.rs` - `handle_ask_mode`）

**自由输入模式（`tool_interact_typing == true`）中的改动：**

- **上键（Up）**：当光标在行首（`cursor == 0`）且上方有选项时：
  - 保存草稿到 `tool_ask_drafts`
  - `tool_interact_typing = false`
  - `tool_ask_cursor` 上移一位（到上一个选项）
- **Esc**：保存草稿到 `tool_ask_drafts`，退出输入模式但不清空内容，光标保持在自由输入行

**选项模式（`tool_interact_typing == false`）中的改动：**

- **直接打字（Char）**：从 `tool_ask_drafts` 恢复已有草稿后追加字符（而非 clear 后追加）
- **Enter 在自由输入行**：如果不在 typing 状态且光标在自由输入行，应直接恢复草稿进入 typing 状态（不再需要 Enter 激活）

### 5. 提交答案后清理（`update_tool_interact.rs`）

- 提交自由输入后，清空当前问题的草稿 `tool_ask_drafts[current_idx]`

### 6. 取消 Ask 时清理（`update_ask_cancel`）

- 清空 `tool_ask_drafts`

### 7. 渲染微调（`confirm_render.rs`）

- 非输入模式下的"自由输入"行：如果有暂存草稿，显示草稿预览文本而非 "✏ 自由输入..."
  - 例：`✏ 已输入: xxx...` 或直接显示暂存内容（灰色/暗色）

## 涉及文件

| 文件 | 修改内容 |
|------|---------|
| `src/command/chat/app/ui_state.rs` | 新增 `tool_ask_drafts` 字段 |
| `src/command/chat/app/stream_poll.rs` | 初始化 drafts |
| `src/command/chat/app/chat_app/update_tool_interact.rs` | 导航时保存/恢复草稿；提交后清草稿 |
| `src/command/chat/handler/tool_confirm.rs` | 上键退出输入模式、Esc 暂存、打字恢复草稿 |
| `src/command/chat/render/cache/confirm_render.rs` | 显示草稿预览 |

## 交互流程对比

### 优化前
1. 弹窗出现 -> 光标在第一个选项
2. 按 N 次 Down 到自由输入行
3. **按 Enter 激活输入**  <-- 多余步骤
4. 开始打字
5. 按 Esc -> 内容丢失

### 优化后
1. 弹窗出现 -> 光标在第一个选项
2. 按 N 次 Down 到自由输入行 -> **直接就是输入状态，可打字**
3. 如果想选选项 -> 按上键回到选项列表（草稿暂存）
4. 按 Down 回到自由输入 -> 草稿恢复，继续编辑
5. 也可直接打字 -> 自动跳到自由输入行（保留已有草稿）
6. 按 Esc -> 草稿暂存
