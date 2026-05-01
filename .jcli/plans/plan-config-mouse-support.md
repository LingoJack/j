# 配置界面鼠标操作支持

## 目标
为配置界面 (Config mode) 添加鼠标点击支持，让用户可以通过鼠标点击 Tab 栏切换 Tab、点击列表项选中字段、点击 Toggle 开关等。

## 当前状态分析

### 已有鼠标支持
- `tui_loop.rs` 中 `dispatch_event` 已处理 `Event::Mouse`：滚轮上下滚动（`mouse_scroll_action` 已支持 Config 模式）、左键拖拽选区、右键上下文菜单（仅 Chat 模式的消息区）。
- 但鼠标**点击**操作仅在 Chat 消息区处理（依赖 `msg_area_inner` + `screen_to_text_pos`），Config 模式下的点击完全被忽略。

### 配置界面布局结构
`draw_config_screen` 将面板拆分为两层：
1. **固定头部** (`chunks[0]`)：顶部边框 + 标题 + Tab 栏 + Tab 专属头部
2. **可滚动列表** (`chunks[1]`)：带底部边框的字段列表

Tab 栏在固定头部中的位置（相对 chunks[0]）：
- 第 0 行：空行
- 第 1 行：Tab 栏（由 `draw_tab_bar_line` 生成）
- 第 2 行：空行
- 第 3 行：分隔线
- 第 4 行起：tab_header_lines

列表区域 (`chunks[1]`) 的内部布局：
- 减去 1 行底部 border
- 有 `config_scroll_offset` 偏移
- `field_line_indices` 记录每个可交互项的行号

## 实现方案

### 1. 在 UIState 中新增 config 屏幕区域缓存字段

```rust
// ui_state.rs
pub struct UIState {
    // ...existing fields...
    /// 配置面板整体区域（config_screen 的 area 参数）
    pub config_screen_area: Option<Rect>,
    /// 配置面板固定头部区域（chunks[0]）
    pub config_header_area: Option<Rect>,
    /// 配置面板列表区域（chunks[1]）
    pub config_list_area: Option<Rect>,
    /// 配置面板当前 Tab 栏行的 Y 坐标（全局屏幕行号）
    pub config_tab_bar_y: Option<u16>,
    /// 配置面板可交互项的行号列表（与 field_line_indices 同步，每帧更新）
    pub config_field_lines: Vec<usize>,
    /// 配置面板可交互项总数
    pub config_field_count: usize,
}
```

### 2. 在 draw_config_screen 中记录布局信息

在渲染函数中，将 chunks[0]、chunks[1] 的坐标、tab 栏位置、field_line_indices 等信息写入 `app.ui` 对应字段，供鼠标事件分发使用。

### 3. 在 tui_loop.rs 中处理 Config 模式的鼠标点击

在 `dispatch_event` 的 `Event::Mouse` 分支中，当 `app.ui.mode == ChatMode::Config` 时添加以下处理：

#### 3a. 左键点击 Tab 栏 → 切换 Tab
- 检查点击行是否等于 `config_tab_bar_y`
- 根据点击列号计算点击了哪个 Tab（通过 Tab 文本的列位置范围）
- 触发 `ConfigSwitchTab` 切换到对应 Tab

#### 3b. 左键点击列表项 → 选中该项
- 检查点击位置是否在 `config_list_area` 内
- 将点击的屏幕行号转换为列表内的行号（减去 area.y + 1 border + config_scroll_offset）
- 在 `config_field_lines` 中二分查找最近的项
- 更新对应的选中索引（config_field_idx / session_list_index / archive_list_index 等）

#### 3c. 左键双击/再次点击已选中项 → 触发 Enter 操作
- 如果点击的项已经是当前选中项，视为"确认"操作
- 触发对应的 Enter/ConfigEnter/ToggleMenuToggle 等动作

### 4. 需要修改的文件

| 文件 | 修改内容 |
|------|----------|
| `src/command/chat/app/ui_state.rs` | 新增 config 布局缓存字段 |
| `src/command/chat/ui/config.rs` | 渲染时记录布局信息到 app.ui |
| `src/command/chat/handler/tui_loop.rs` | 添加 Config 模式鼠标点击处理 |
| `src/command/chat/app/mod.rs` | 新字段初始化（如需要） |

### 5. 技术细节

#### Tab 点击检测
Tab 栏由 `tab_bar()` 组件渲染，每个 Tab 格式为 `" {label} "`，Tab 之间用 ` {SEPARATOR_V} ` 分隔。
需要在 draw 阶段计算每个 Tab 的列范围并缓存到 UIState：

```rust
pub struct ConfigTabHitBox {
    pub tab: ConfigTab,
    pub start_col: u16,
    pub end_col: u16,
}
pub config_tab_hitboxes: Vec<ConfigTabHitBox>,
```

#### 列表项点击检测
列表区域内部坐标映射：
```
list_inner_y = click_row - list_area.y - 1  // 减去顶部 border（列表区域只有底部 border）
list_content_y = list_inner_y + config_scroll_offset
```
然后在 `config_field_lines` 中查找 `list_content_y` 落在哪个项的范围内。

#### 各 Tab 的选中索引映射
不同 Tab 使用不同的选中索引字段：
- Model/Global/Tools/Skills/Hooks/Commands → `config_field_idx`
- Session → `session_list_index`
- Archive → `archive_list_index`
- Teammates → `teammate_list_index`
- Global (compact_exempt_sublist) → `compact_exempt_idx`

这与 `mouse_scroll_action` 中的映射逻辑一致。

### 6. 交互设计

- **单击 Tab**：切换到对应 Tab
- **单击列表项**：选中该项
- **单击已选中项**：触发 Enter 操作（开始编辑/切换开关）
- **滚轮**：已支持，无需修改
