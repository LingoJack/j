# Plan: 将消息气泡背景色做成可配置选项

## 概述

在 `AgentConfig` 中新增一个布尔字段 `flat_bubble`（默认 `false`），当用户开启后，气泡背景色会被替换为 `bg_primary`（即与一般背景色相同），实现"扁平"无边框的视觉效果。

## 需要修改的文件

### 1. 恢复主题 JSON 原始值（7 个文件）

之前已将部分主题 JSON 的 `bubble_ai` / `bubble_user` 改为与 `bg_primary` 相同，现在需要恢复到原始设计值，以便"默认"模式保持原有的差异化气泡：

| 主题 | `bubble_ai` 恢复为 | `bubble_user` 恢复为 |
|---|---|---|
| midnight | `#1c1c26` | 已是 `#284678`（无需恢复） |
| anthropic_light | 已是 `#faf6f1`（需恢复为 `#f5f0ea`） | 需恢复为 `#e8f0f8` |
| anthropic_dark | 已是 `#222436`（需恢复为 `#1e2030`） | 需恢复为 `#2d3f76` |
| monokai | 已是 `#272822`（需恢复为 `#2b2c26`） | 已是 `#37415a`（无需恢复） |
| nord | 已是 `#323844`（无需恢复） | 需恢复为 `#344b6e` |
| light | 已是 `#ffffff`（无需恢复） | 已是 `#ffffff`（需恢复为 `#dcebff`） |
| dark | 已是 `#222222`（无需恢复） | 需恢复为 `#26416e` |

### 2. `src/command/chat/storage/config.rs`

- 在 `AgentConfig` 结构体中添加：
  ```rust
  /// 气泡背景色与主背景色一致（扁平效果）
  #[serde(default)]
  pub flat_bubble: bool,
  ```

### 3. `src/constants.rs`

- 在 `CONFIG_GLOBAL_FIELDS_TAB` 中添加 `"flat_bubble"` 字段（建议在 `"thinking_style"` 后面）

### 4. `src/command/chat/render/helpers.rs`

- `config_field_label_global`: 添加 `"flat_bubble" => "扁平气泡"`
- `config_field_desc_global`: 添加 `"flat_bubble" => "开启后气泡背景色与主背景色一致"`
- `config_field_value_global`: 添加 `"flat_bubble"` 的显示值（开启/关闭）
- `config_field_raw_value_global`: 添加 `"flat_bubble"` 的原始值（true/false）
- `config_field_set_global`: 添加 `"flat_bubble"` 的设置逻辑

### 5. `src/command/chat/app/chat_app/update_config.rs`

- 在 `update_config_enter` 的 Global tab 分支中添加 `flat_bubble` 的 toggle 逻辑（类似 `auto_restore_session`）
- toggle 后同时清空 `msg_lines_cache` 以触发重新渲染

### 6. `src/command/chat/render/cache/msg_render.rs`

- 在 `build_user_message_lines` 和 `build_assistant_message_lines` 中，当 `flat_bubble` 为 true 时，将 `bubble_bg` 替换为 `theme.bg_primary`
- 需要传入 `flat_bubble` 参数（通过函数参数或在调用处判断后替换颜色）

### 7. `src/command/chat/render/cache.rs`

- `build_message_lines_incremental` 调用处需要传递 `flat_bubble` 参数

## 实现策略

采用**最小侵入**方案：在 `msg_render.rs` 的两个函数中增加一个 `flat_bubble: bool` 参数。当 `flat_bubble` 为 true 时，将 `bubble_bg` 覆盖为 `theme.bg_primary`，其余渲染逻辑不变。调用链上逐层传递该参数。

具体改动：
- `build_user_message_lines(..., flat_bubble: bool)` - 加参数
- `build_assistant_message_lines(..., flat_bubble: bool)` - 加参数
- `build_message_lines_incremental` 中读取配置并传入
