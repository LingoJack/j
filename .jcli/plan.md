# / 斜杠命令弹窗系统设计

## 目标

实现类似 `@` 弹窗的 `/` 斜杠命令系统，用于替代部分快捷键，减少 hint bar 的干扰。

## 核心需求

1. **触发条件**：只有在输入框为空时，输入 `/` 才能唤起弹窗
2. **交互方式**：与 `@` 弹窗一致（Up/Down 导航，Tab/Enter 确认，Esc 关闭）
3. **支持的命令**：
   - `/copy` - 复制最后一条 AI 回复
   - `/log` - 打开日志窗口
   - `/browse` - 进入消息浏览模式
   - `/config` - 打开配置界面
   - `/model` - 切换模型
4. **执行后行为**：清除输入框，执行对应操作

## 实现步骤

### Step 1: UIState 添加 slash 弹窗状态字段

**文件**: `src/command/chat/app/ui_state.rs`

添加字段：
```rust
/// / 斜杠命令弹窗是否激活
pub slash_popup_active: bool,
/// / 之后的过滤文本
pub slash_popup_filter: String,
/// / 在 input 中的字符索引（始终为 0）
pub slash_popup_start_pos: usize,
/// 弹窗中选中项索引
pub slash_popup_selected: usize,
```

### Step 2: autocomplete.rs 添加 slash 命令数据结构

**文件**: `src/command/chat/autocomplete.rs`

1. 定义 `SlashCommand` 枚举：
```rust
#[derive(Clone, Debug)]
pub enum SlashCommand {
    Copy,    // 复制最后一条 AI 回复
    Log,     // 打开日志窗口
    Browse,  // 浏览消息
    Config,  // 打开配置
    Model,   // 切换模型
}
```

2. 实现方法：
   - `display_label(&self) -> String`: 显示标签
   - `description(&self) -> String`: 命令描述
   - `get_filtered_slash_commands(filter: &str) -> Vec<SlashCommand>`: 根据过滤文本返回匹配命令

### Step 3: handler/chat.rs 添加 / 触发和弹窗处理逻辑

**文件**: `src/command/chat/handler/chat.rs`

1. 在 `handle_chat_mode` 函数开头添加 slash 弹窗拦截逻辑（类似 at_popup 拦截）
2. 处理 Up/Down/Tab/Enter/Esc/Backspace 按键
3. 在 `KeyCode::Char('/')` 处理中，检测输入框是否为空，若是则激活 slash 弹窗
4. 实现 `execute_slash_command(app: &mut ChatApp, cmd: &SlashCommand)` 函数执行命令

### Step 4: ui/chat.rs 添加 slash 弹窗绘制

**文件**: `src/command/chat/ui/chat.rs`

1. 在 `draw_chat_ui` 中添加 `draw_slash_popup` 调用
2. 实现 `draw_slash_popup` 函数（复用 `draw_popup_list` 通用函数）

### Step 5: 简化 hint bar 中的快捷键提示

**文件**: `src/command/chat/ui/chat.rs`

修改 `draw_hint_bar` 函数中 `ChatMode::Chat` 的 hints：
- 移除 `Ctrl+Y`, `Ctrl+G`, `Ctrl+B`, `Ctrl+E`, `Ctrl+T` 的提示
- 添加 `/` 命令提示

### Step 6: 在 ChatApp::new 中初始化新字段

**文件**: `src/command/chat/app/chat_app.rs`

在 `UIState` 初始化中添加：
```rust
slash_popup_active: false,
slash_popup_filter: String::new(),
slash_popup_start_pos: 0,
slash_popup_selected: 0,
```

## 文件修改清单

| 文件 | 修改内容 |
|------|----------|
| `src/command/chat/app/ui_state.rs` | 添加 slash 弹窗状态字段 |
| `src/command/chat/autocomplete.rs` | 添加 SlashCommand 枚举和过滤函数 |
| `src/command/chat/handler/chat.rs` | 添加 / 触发和弹窗处理逻辑 |
| `src/command/chat/ui/chat.rs` | 添加弹窗绘制，简化 hint bar |
| `src/command/chat/app/chat_app.rs` | 初始化新字段 |

## 命令映射表

| 命令 | 对应 Action | 描述 |
|------|-------------|------|
| `/copy` | `Action::CopyLastAiReply` | 复制最后一条 AI 回复 |
| `/log` | `Action::OpenLogWindows` | 打开日志窗口 |
| `/browse` | `Action::EnterMode(ChatMode::Browse)` | 进入消息浏览模式 |
| `/config` | `Action::EnterMode(ChatMode::Config)` | 打开配置界面 |
| `/model` | `Action::EnterMode(ChatMode::SelectModel)` | 切换模型 |

## 交互流程

```
用户输入 / (输入框为空)
    ↓
激活 slash_popup，显示命令列表
    ↓
用户输入过滤文本（如 "co"）
    ↓
列表过滤为 /copy, /config
    ↓
用户按 Tab/Enter
    ↓
执行选中命令，关闭弹窗，清空输入框
```

## 注意事项

1. **触发条件严格**：只有输入框完全为空时，输入 `/` 才触发弹窗
2. **与其他弹窗互斥**：激活 slash 弹窗时，关闭其他弹窗
3. **Esc 关闭**：按 Esc 关闭弹窗，保留 `/` 字符在输入框中
4. **Backspace 处理**：当 filter 为空时按 Backspace，关闭弹窗并删除 `/` 字符
