# 重构 config.rs：按 Tab 拆分子模块

## 目标
将 `src/command/chat/ui/config.rs`（1435 行）拆分为 `config/` 子目录，每个 Tab 独立一个文件。

## 拆分方案

### 文件结构
```
src/command/chat/ui/config.rs      → config/mod.rs (保留分发器 + 公共逻辑)
src/command/chat/ui/config/
  ├── mod.rs          # draw_config_screen, draw_tab_bar_line, 公共 import
  ├── model.rs        # draw_tab_model_header, draw_tab_model_list, adjust_provider_scroll_offset
  ├── global.rs       # draw_tab_global_lines
  ├── tools.rs        # draw_tab_tools_header, draw_tab_tools_list
  ├── skills.rs       # draw_tab_skills_header, draw_tab_skills_list
  ├── hooks.rs        # draw_tab_hooks_lines
  ├── commands.rs     # draw_tab_commands_header, draw_tab_commands_list
  ├── teammates.rs    # draw_tab_teammates_header, draw_tab_teammates_list
  ├── session.rs      # draw_tab_session_header, draw_tab_session_list, format_timestamp, days_to_ymd, is_leap
  └── archive.rs      # draw_tab_archive_header, draw_tab_archive_list
```

### 各文件内容

#### `config/mod.rs`（~200 行）
- 保留公共 `use` 语句
- 保留 `draw_tab_bar_line` 函数
- 保留 `draw_config_screen` 函数（主入口分发器）
- 从各子模块 `use` 各 `draw_tab_*` 函数
- 声明子模块 `mod model; mod global; ...`

#### `config/model.rs`（~250 行）
- `adjust_provider_scroll_offset`
- `draw_tab_model_header`
- `draw_tab_model_list`

#### `config/global.rs`（~140 行）
- `draw_tab_global_lines`

#### `config/tools.rs`（~60 行）
- `draw_tab_tools_header`
- `draw_tab_tools_list`

#### `config/skills.rs`（~50 行）
- `draw_tab_skills_header`
- `draw_tab_skills_list`

#### `config/hooks.rs`（~80 行）
- `draw_tab_hooks_lines`

#### `config/commands.rs`（~60 行）
- `draw_tab_commands_header`
- `draw_tab_commands_list`

#### `config/teammates.rs`（~170 行）
- `draw_tab_teammates_header`
- `draw_tab_teammates_list`

#### `config/session.rs`（~80 行）
- `draw_tab_session_header`
- `draw_tab_session_list`
- `format_timestamp`（私有辅助函数）
- `days_to_ymd`（私有辅助函数）
- `is_leap`（私有辅助函数）

#### `config/archive.rs`（~50 行）
- `draw_tab_archive_header`
- `draw_tab_archive_list`

### 实施步骤
1. 创建 `config/` 目录
2. 将 `config.rs` 重命名为 `config/mod.rs`
3. 将各 Tab 的函数剪切到对应子文件
4. 在 `mod.rs` 中声明子模块并导入公共函数
5. 各子文件添加必要的 `use` 语句
6. 所有函数保持 `pub(super)` 可见性（仅供 mod.rs 使用）
7. 运行 `cargo check` 验证编译通过
8. 运行 `cargo clippy` 检查无警告
9. 运行 `cargo fmt` 格式化

### 设计原则
- 函数签名和行为完全不变
- 无逻辑改动，纯文件移动
- 子模块函数使用 `pub(super)` 可见性
- `mod.rs` 作为唯一的对外接口层
