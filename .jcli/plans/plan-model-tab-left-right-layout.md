# Model Tab 左右布局改造计划

## 背景

用户希望将 Model tab 的交互方式改造成类似 Tools tab 的左右布局模式，使用 Tab 键进入编辑区域。

## 当前实现

### Model tab（现状）
- 顶部有 Provider sub-tabs（水平滚动）
- 使用 `config_provider_idx` 标记当前选中的 Provider
- 使用 `config_field_idx` 标记选中的配置字段
- Tab 键用于切换 Provider
- Enter 进入字段编辑

### Tools tab（参考实现）
- 左右分栏布局：左侧工具列表，右侧详情面板
- `tools_in_options` 标记是否在选项层级
- `tools_option_idx` 标记选项焦点索引
- Tab 键切换层级（列表 ↔ 详情）
- 上下键在当前层级导航

## 改造方案

### 1. UI 状态扩展 (`ui_state.rs`)

新增两个字段：

```rust
/// Model tab：是否在配置字段编辑层级（右侧面板）
pub model_in_fields: bool,
/// Model tab：配置字段焦点索引（当 model_in_fields 为 true 时使用）
pub model_field_idx: usize,
```

### 2. 渲染层改造 (`ui/config/model.rs`)

#### 2.1 `draw_tab_model_header`
- 简化为仅显示当前 Provider 名称和活跃状态提示
- 不再显示水平滚动的 sub-tabs

#### 2.2 `draw_tab_model_list` → `draw_tab_model_providers`
- 渲染左侧 Provider 列表
- 显示 Provider 名称 + 活跃标记
- 使用 `config_provider_idx` 作为选中索引

#### 2.3 新增 `draw_tab_model_detail`
- 渲染右侧配置字段详情（无边框）
- 显示当前 Provider 的所有配置字段
- 使用 `model_field_idx` 标记选中字段
- Tab 进入编辑模式时字段高亮

### 3. 主渲染入口改造 (`ui/config.rs`)

仿照 Tools tab 的左右分栏逻辑，但右侧不画边框（无边框平铺）：

```rust
if is_model_split {
    let left_w = (area.width as usize * 35 / 100).max(20) as u16;
    let right_w = area.width.saturating_sub(left_w);

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(left_w), Constraint::Min(right_w)])
        .split(chunks[1]);

    // 左侧：Provider 列表（无边框）
    // 右侧：配置字段详情（无边框，与左侧用竖线分隔即可）
}
```

### 4. 按键处理改造 (`handler/config.rs`)

#### Model tab 新按键绑定

| 按键 | 行为 |
|------|------|
| Tab | 切换层级（Provider 列表 ↔ 配置字段） |
| Shift+Tab | 反向切换层级 |
| 上/下 (Provider 层级) | 切换 Provider |
| 上/下 (字段层级) | 切换配置字段 |
| Enter (字段层级) | 进入编辑 |
| a | 新增 Provider |
| d | 删除当前 Provider |
| s | 设为活跃 Provider |

### 5. Action 和状态更新

#### 5.1 新增 Action (`action.rs`)

```rust
Action::ModelToggleLevel,  // Tab 切换层级
```

#### 5.2 状态更新函数 (`update_config.rs`)

```rust
pub(super) fn update_model_toggle_level(&mut self) {
    if self.ui.model_in_fields {
        // 从字段层级返回 Provider 列表
        self.ui.model_in_fields = false;
    } else {
        // 进入选中 Provider 的字段编辑
        self.ui.model_in_fields = true;
        self.ui.model_field_idx = 0; // 默认焦点在第一个字段
    }
}

pub(super) fn update_model_navigate(&mut self, dir: CursorDirection) {
    if self.ui.model_in_fields {
        // 字段层级导航
        let total = CONFIG_FIELDS.len();
        match dir {
            CursorDirection::Up => {
                if self.ui.model_field_idx > 0 {
                    self.ui.model_field_idx -= 1;
                }
            }
            CursorDirection::Down => {
                if self.ui.model_field_idx < total - 1 {
                    self.ui.model_field_idx += 1;
                }
            }
        }
    } else {
        // Provider 列表层级导航
        let total = self.state.agent_config.providers.len();
        if total == 0 { return; }
        match dir {
            CursorDirection::Up => {
                if self.ui.config_provider_idx == 0 {
                    self.ui.config_provider_idx = total - 1;
                } else {
                    self.ui.config_provider_idx -= 1;
                }
            }
            CursorDirection::Down => {
                self.ui.config_provider_idx = (self.ui.config_provider_idx + 1) % total;
            }
        }
    }
}
```

### 6. 编辑状态兼容

当 `model_in_fields = true` 且 `config_editing = true` 时，编辑逻辑使用 `model_field_idx` 替代 `config_field_idx`。

### 7. 初始化 (`chat_app.rs`)

在 UIState 初始化中添加：

```rust
model_in_fields: false,
model_field_idx: 0,
```

## 实施步骤

1. **ui_state.rs**: 添加 `model_in_fields` 和 `model_field_idx` 字段
2. **chat_app.rs**: 初始化新字段
3. **action.rs**: 添加 `ModelToggleLevel` Action
4. **update.rs**: 添加 Action 分发
5. **update_config.rs**: 实现状态更新函数
6. **model.rs**: 重构渲染函数（header → provider list → detail）
7. **config.rs**: 添加 Model tab 左右分栏渲染逻辑
8. **handler/config.rs**: 修改 Model tab 按键绑定

## 预期效果

```
┌──────────────────────────────────────────────────────────────┐
│ ⚙ 模型配置                                                    │
│                                                              │
│  ● Provider-1  │ name: Provider-1                            │
│    Provider-2  │ api_base: https://api.xxx.com/v1            │
│                │ api_key: sk-xxxxx                           │
│                │ model: gpt-4                                │
│                │ supports_vision: ○                          │
│                │                                             │
│                │ (Tab 编辑, s 设为活跃)                       │
└──────────────────────────────────────────────────────────────┘
```

- 左侧：Provider 列表（无边框，用竖线 `│` 分隔）
- 右侧：选中 Provider 的配置字段（无边框平铺）
- Tab 键切换焦点
- 在字段层级按 Enter 进入编辑
