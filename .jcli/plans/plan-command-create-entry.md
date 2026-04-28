# Commands 面板创建命令入口实现计划

## 需求背景

在配置面板的 Commands Tab 中，当前只显示已加载的命令列表和启用/禁用开关。用户希望添加一个快速入口，允许用户在 TUI 中直接创建新的自定义命令，并可以选择保存到用户级（`~/.jdata/agent/commands/`）或项目级（`.jcli/commands/`）。

## 实现方案

参考现有的 `pending_system_prompt_edit`、`pending_agent_md_edit` 的实现模式，通过打开全屏 Markdown 编辑器来创建新命令。

在编辑之前，提供一个简单的选择界面，让用户选择保存到哪个级别。

### 涉及文件修改

#### 1. `src/command/chat/app/ui_state.rs`

添加新字段：

```rust
/// 配置界面：是否有待处理的命令创建
pub pending_command_create: bool,
/// 配置界面：命令创建的目标级别（User 或 Project）
pub command_create_source: CommandSource,
```

需要引入 `CommandSource` 类型。

#### 2. `src/command/chat/app/action.rs`

添加新的 Action 枚举：

```rust
ConfigCreateCommand,
ConfigCreateCommandSelectSource,  // 打开级别选择
ConfigCreateCommandConfirmSource, // 确认级别选择
```

#### 3. `src/command/chat/handler/config.rs`

在 `handle_config_key` 函数中，为 Commands Tab 添加按键 'c' 处理：

```rust
(KeyCode::Char('c'), ConfigTab::Commands) => {
    Action::ConfigCreateCommandSelectSource
}
```

添加选择模式下的按键处理：

```rust
// 选择模式下
(KeyCode::Char('j') | KeyCode::Down, ...) => Action::ConfigCreateCommandNavigateSource(CursorDirection::Down),
(KeyCode::Char('k') | KeyCode::Up, ...) => Action::ConfigCreateCommandNavigateSource(CursorDirection::Up),
(KeyCode::Enter, ...) => Action::ConfigCreateCommandConfirmSource,
(KeyCode::Esc, ...) => Action::ConfigCancelCreate,
```

并在底部提示中添加按键说明。

#### 4. `src/command/chat/app/ui_state.rs`

添加模式枚举：

```rust
/// Commands Tab 的模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommandsMode {
    #[default]
    Normal,       // 正常列表浏览
    SelectSource, // 选择保存级别
}
```

在 `ChatUiState` 中添加：

```rust
pub commands_mode: CommandsMode,
pub commands_source_idx: usize,  // 0=用户级, 1=项目级
```

#### 5. `src/command/chat/app/action.rs`

添加 Action：

```rust
ConfigCreateCommandNavigateSource(CursorDirection),
ConfigCancelCreate,
```

#### 6. `src/command/chat/app/chat_app/update.rs`

添加 Action 分发：

```rust
Action::ConfigCreateCommandSelectSource => self.update_config_command_select_source(),
Action::ConfigCreateCommandNavigateSource(dir) => self.update_config_command_navigate_source(dir),
Action::ConfigCreateCommandConfirmSource => self.update_config_command_confirm_source(),
Action::ConfigCancelCreate => self.update_config_cancel_create(),
```

#### 7. `src/command/chat/app/chat_app/update_config.rs`

实现方法：

```rust
pub(super) fn update_config_command_select_source(&mut self) {
    // 检查是否有项目级目录，没有则只能选用户级
    if crate::command::chat::infra::command::project_commands_dir().is_some() {
        self.ui.commands_mode = CommandsMode::SelectSource;
        self.ui.commands_source_idx = 0;
    } else {
        // 没有项目级目录，直接使用用户级
        self.ui.command_create_source = CommandSource::User;
        self.ui.pending_command_create = true;
    }
}

pub(super) fn update_config_command_navigate_source(&mut self, dir: CursorDirection) {
    match dir {
        CursorDirection::Up => {
            if self.ui.commands_source_idx > 0 {
                self.ui.commands_source_idx -= 1;
            }
        }
        CursorDirection::Down => {
            if self.ui.commands_source_idx < 1 {
                self.ui.commands_source_idx += 1;
            }
        }
    }
}

pub(super) fn update_config_command_confirm_source(&mut self) {
    self.ui.command_create_source = if self.ui.commands_source_idx == 0 {
        CommandSource::User
    } else {
        CommandSource::Project
    };
    self.ui.commands_mode = CommandsMode::Normal;
    self.ui.pending_command_create = true;
}

pub(super) fn update_config_cancel_create(&mut self) {
    self.ui.commands_mode = CommandsMode::Normal;
}
```

#### 8. `src/command/chat/handler/tui_loop.rs`

在 Phase 5 (Side-effects) 区域添加处理逻辑：

```rust
if app.ui.pending_command_create {
    app.ui.pending_command_create = false;
    input_thread.pause();
    input_thread.drain();
    
    let source = app.ui.command_create_source;
    let title = match source {
        CommandSource::User => "创建命令 (用户级)",
        CommandSource::Project => "创建命令 (项目级)",
    };
    
    // 提供模板内容
    let template = r#"---
name: my-command
description: 命令描述
---

# 命令内容

在这里编写命令的提示词正文..."#;
    
    match crate::tui::editor_markdown::open_markdown_editor_on_terminal(
        &mut terminal,
        title,
        template,
        &app.ui.theme,
    ) {
        Ok((Some(new_text), _)) => {
            // 解析 frontmatter 获取 name
            // 根据选择的 source 保存到对应目录
            // 成功后重新加载 commands
            app.state.loaded_commands = load_all_commands();
            app.update(Action::ShowToast("命令已创建", false));
        }
        Ok((None, _)) => {}
        Err(e) => {
            app.update(Action::ShowToast(format!("编辑器错误: {}", e), true));
        }
    }
    
    input_thread.drain();
    input_thread.resume();
    needs_redraw = true;
}
```

#### 9. `src/command/chat/ui/config/commands.rs`

修改 `draw_tab_commands_header`，添加创建入口提示：

- 当列表为空时，显示 "按 c 创建命令"
- 当列表非空时，在底部添加提示行

添加选择级别的渲染函数：

```rust
pub(super) fn draw_tab_commands_select_source<'a>(app: &ChatApp) -> Vec<Line<'a>> {
    // 渲染选择界面
    // [ ] 用户级 (~/.jdata/agent/commands/)
    // [ ] 项目级 (.jcli/commands/) -- 仅当项目级目录存在时显示
}
```

#### 10. `src/command/chat/ui/config.rs`

在 `draw_config_screen` 中，当 `CommandsMode::SelectSource` 时渲染选择界面。

#### 11. `src/command/chat/infra/command.rs`

添加创建命令的辅助函数：

```rust
/// 保存新命令到指定目录
pub fn save_new_command(source: CommandSource, content: &str) -> std::io::Result<PathBuf> {
    // 解析 frontmatter 获取 name
    // 根据来源确定目录
    // 保存文件
}

/// 返回指定来源的 commands 目录路径
pub fn commands_dir_for_source(source: CommandSource) -> PathBuf {
    match source {
        CommandSource::User => commands_dir(),
        CommandSource::Project => {
            JcliConfig::find_config_dir()
                .map(|d| d.join("commands"))
                .unwrap_or_else(|| commands_dir()) // 回退到用户级
        }
    }
}
```

### 实现步骤

1. **第一步：添加 UI 状态字段、模式和 Action**
   - 修改 `ui_state.rs` 添加 `CommandsMode`、`pending_command_create`、`command_create_source`、`commands_source_idx`
   - 修改 `action.rs` 添加相关 Action 枚举

2. **第二步：添加按键处理**
   - 修改 `handler/config.rs` 添加按键 'c' 和选择模式的处理

3. **第三步：添加 Action 处理逻辑**
   - 修改 `update.rs` 添加分发
   - 修改 `update_config.rs` 实现方法

4. **第四步：实现选择界面渲染**
   - 修改 `commands.rs` 添加选择级别渲染
   - 修改 `config.rs` 集成选择界面

5. **第五步：实现编辑器调用**
   - 修改 `tui_loop.rs` Phase 5 添加编辑器调用逻辑
   - 修改 `infra/command.rs` 添加保存函数

6. **第六步：更新 UI 提示**
   - 修改 `commands.rs` 添加创建入口提示

7. **第七步：测试验证**
   - 运行 `cargo clippy -- -D warnings` 确保无警告
   - 运行 `cargo fmt` 格式化代码

### 设计细节

#### 选择级别界面

当按下 'c' 时：
- 如果项目级目录不存在（非 jcli 项目），直接打开编辑器，保存到用户级
- 如果项目级目录存在，显示选择界面：
  ```
  选择保存位置：
    > 用户级 (~/.jdata/agent/commands/)
      项目级 (.jcli/commands/)
  
  j/k 或 ↑/↓ 选择，Enter 确认，Esc 取消
  ```

#### 命令模板

创建新命令时，提供包含 frontmatter 的模板：

```markdown
---
name: my-command
description: 命令描述
---

# 命令内容

在这里编写命令的提示词正文...
```

#### 错误处理

- 如果 frontmatter 格式不正确或缺少 `name` 字段，显示错误提示
- 如果同名命令已存在，提示用户修改 name
