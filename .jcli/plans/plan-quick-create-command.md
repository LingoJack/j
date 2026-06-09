# Plan: quick-create-command

## 目标

在 `src/command/chat/ui/config/commands.rs` 相关的 Commands 配置页中，增加“快捷创建 command”的交互能力。重点先把 TUI 交互设计清楚，再实施代码改动。

## 当前行为梳理

- Commands tab 普通模式：
  - `c` 触发 `ConfigCreateCommandSelectSource`。
  - 当前代码在存在项目级 `.jcli/commands/` 时，进入 `CommandsMode::SelectSource` 选择用户级/项目级。
  - 当前代码在不存在项目级目录时，直接以用户级创建；这与“应该让用户选项目级还是用户级”的目标不一致，需要调整为始终进入选择界面，或至少提供可自动创建目录的项目级选项。
- 选择来源模式：
  - `j/k` 或 `↑/↓` 选择保存位置。
  - `Enter` 确认后设置 `pending_command_create = true`。
  - `Esc` 取消。
- `tui_loop` 检测 `pending_command_create` 后，退出 TUI raw 输入，打开 markdown editor，使用默认模板：
  - frontmatter: `name: my-command`, `description: 命令描述`
  - body: `# 命令内容 ...`
  - 保存后 `load_all_commands()` 刷新列表。

## 交互设计建议

### 方案 A：保留现有外部编辑器，增强“快捷创建”的入口与模板提示

这是最低风险方案：不引入新的输入状态机，只优化 Commands 页 UI，让用户明确知道 `c` 是快捷创建。

交互：
1. Commands tab header 始终展示操作提示：
   - `c 创建`、`Space/Enter 启用/禁用`、`a 全启用`、`d 全禁用`、`Esc 保存返回`
2. 空列表时继续展示“按 c 快速创建”，但非空列表也展示快捷创建提示。
3. 按 `c` 后：
   - 有项目目录：进入保存位置选择页。
   - 无项目目录：直接打开编辑器创建用户级命令。
4. 选择保存位置页继续保留当前 `j/k/Enter/Esc` 交互。
5. 编辑器内完成命令名、描述、正文编辑，保存即创建。

优点：
- 改动主要集中在 `commands.rs` 的渲染提示，少量检查 handler/update 是否需要调整。
- 不影响现有保存、解析、刷新逻辑。
- 用户学习成本低，符合“快捷创建 command”的描述。

不足：
- 真正的 name/description/body 仍在外部 markdown editor 中填写，不是 TUI 内联 wizard。

### 方案 B：TUI 内联快速向导 + 外部编辑器补正文

交互：
1. `c` 进入 QuickCreate 模式。
2. 第一步选择保存位置（若有项目级目录）。
3. 第二步在 TUI 内输入命令名。
4. 第三步输入描述。
5. `Enter` 后生成带 name/description 的模板并打开 markdown editor，只需补正文。

优点：
- 更“快捷”，减少编辑 frontmatter 的负担。

不足：
- 需要新增 UI 状态（如输入缓冲、步骤枚举），handler/update/tui_loop/渲染多处联动。
- 需处理输入法、退格、取消、校验命令名冲突等细节，风险明显更高。

### 方案 C：基于当前输入框内容快速创建

交互：
- 在 Commands tab 按 `c`，若当前 chat input 有文本，则尝试作为 command name 或 description 预填模板。

不推荐：
- Commands 配置页与 chat 输入语义混杂，容易误触或产生不可预期行为。

## 推荐实施方案

实施“先选择保存级别，再打开编辑器”的交互，并把它作为快捷创建 command 的核心流程：

1. 在 Commands tab 普通模式中新增/强化统一操作提示，明确：`c 快速创建`。
2. 用户按 `c` 后，进入保存级别选择界面，让用户选择：
   - `用户级`：保存到 `~/.jdata/agent/commands/`。
   - `项目级`：保存到当前项目 `.jcli/commands/`。
3. 选择界面交互：
   - `j/k` 或 `↑/↓` 移动选择。
   - `Enter` 确认。
   - `Esc` 取消并回到 Commands tab。
4. 确认保存级别后，再打开 Markdown 编辑器创建命令。
5. 编辑器保存成功后刷新 `loaded_commands`，并提示最终保存路径。
6. 如果当前项目还没有 `.jcli/commands/` 目录：
   - 仍显示“项目级”选项。
   - 用户确认项目级后，保存逻辑应自动创建 `.jcli/commands/` 目录。
   - 如果创建失败，显示错误 toast，不静默回退到用户级。
7. 尽量把改动集中在现有 Commands 创建链路：`commands.rs` 负责渲染选择 UI，`update_config.rs` 负责确认选择并设置 `command_create_source`，`tui_loop.rs` 负责按 source 保存。

## 实施文件范围预估

必改：
- `src/command/chat/ui/config/commands.rs`
  - 选择保存级别 UI 必须始终同时展示“用户级”和“项目级”。
  - 文案说明项目级会保存到 `.jcli/commands/`，若目录不存在则将在创建时自动建立。
  - 普通模式增加 `c 快速创建` 提示。
- `src/command/chat/app/chat_app/update_config.rs`
  - 修改 `update_config_command_select_source`：按 `c` 后不再因项目目录不存在而直接创建用户级，而是始终进入 `CommandsMode::SelectSource`。
  - `commands_source_idx` 默认 0（用户级），允许用户切换到 1（项目级）。
  - 确认选择逻辑保持：0 => `CommandSource::User`，1 => `CommandSource::Project`。

可能改：
- `src/command/chat/handler/tui_loop.rs`
  - 检查 `save_new_command(CommandSource::Project, ...)` 在 `.jcli/commands/` 不存在时是否会创建目录。
  - 如果不会创建，则补齐自动创建目录或在保存前创建目录。
- command infra 对应实现位置
  - 当前通过 `crate::command::chat::infra::command::save_new_command` 调用；若需要目录创建能力，应优先在该保存函数内部处理，保持调用方简单。

不做（本轮）：
- 不做 TUI 内联填写 name/description 的 wizard。
- 不复用 chat 输入框内容作为命令模板。

## 验证计划

- `cargo fmt`
- `cargo clippy -- -D warnings`
- 手动 TUI 验证：进入 Commands tab，检查普通模式提示、空列表提示、按 `c` 选择来源/打开编辑器、取消和保存后的刷新行为。

