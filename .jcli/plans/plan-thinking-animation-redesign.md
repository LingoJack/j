# Thinking 动画重新设计方案（Config Panel 可配置）

## 现状分析

当前 thinking 指示器的实现：

1. **位置**：消息区内，AI 气泡内部，"Sprite" 标签下方
2. **内容**：一个静态字符 `◍`，配合正弦波呼吸灯颜色脉冲
3. **动画机制**：
   - `thinking_pulse_color()` 基于 `SystemTime` 正弦波计算颜色（周期 1500ms，亮度 0.3~1.0）
   - 对 `label_ai` 颜色 RGB 分量乘以因子实现明暗渐变
4. **刷新频率**：loading 状态下 TUI 每 100ms 刷新一次
5. **触发条件**：`streaming_content` 为空时显示 `◍`

### 现有不足
- 单个 `◍` 字符视觉冲击力弱，缺乏动感
- 不可配置，用户无法选择自己喜欢的风格

---

## 方案总览

提供 **5 种动画风格**（含原版），在 Config Panel 全局配置页中可切换：

| 枚举值 | 名称 | 预览 | 说明 |
|--------|------|------|------|
| `Braille` | 旋转点阵 | ⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ | 经典 braille spinner，辨识度最高（默认） |
| `Classic` | 经典圆点 | ◍ | 原版：静态 ◍ + 颜色脉冲呼吸 |
| `Pulse` | 呼吸圆点 | · ◦ ○ ◔ ◕ ● ◕ ◔ ○ ◦ · | 渐变圆环呼吸 |
| `Wave` | 波浪三连 | ●··  ·●·  ··● | 三点波浪起伏 |
| `Blink` | 闪烁光标 | █ _ | 极简终端风 |

默认值为 `Braille`，所有风格均叠加颜色脉冲效果。

---

## 实现计划（7 步）

### Step 1：新增 `ThinkingStyle` 枚举

**文件**：`src/command/chat/storage/config.rs`

```rust
/// 思考指示器动画风格
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingStyle {
    /// Braille 点阵旋转（默认）
    #[default]
    Braille,
    /// 经典圆点（原版 ◍ + 颜色脉冲）
    Classic,
    /// 圆环呼吸（渐变大小）
    Pulse,
    /// 三点波浪
    Wave,
    /// 光标闪烁
    Blink,
}

impl ThinkingStyle {
    /// 所有可能值，用于 config panel 循环切换
    pub const ALL: &[ThinkingStyle] = &[
        ThinkingStyle::Braille,
        ThinkingStyle::Classic,
        ThinkingStyle::Pulse,
        ThinkingStyle::Wave,
        ThinkingStyle::Blink,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Braille => "旋转点阵",
            Self::Classic => "经典圆点",
            Self::Pulse => "呼吸圆点",
            Self::Wave => "波浪三连",
            Self::Blink => "闪烁光标",
        }
    }

    pub fn next(&self) -> Self {
        let idx = Self::ALL.iter().position(|s| s == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    /// 当前帧对应的显示字符
    pub fn frame(&self, tick: u64) -> &'static str {
        match self {
            Self::Braille => {
                const FRAMES: &[&str] = &[
                    "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏",
                ];
                FRAMES[(tick as usize) % FRAMES.len()]
            }
            Self::Classic => "◍",
            Self::Pulse => {
                const FRAMES: &[&str] = &[
                    "·", "◦", "○", "◔", "◕", "●", "◕", "◔", "○", "◦",
                ];
                FRAMES[(tick as usize) % FRAMES.len()]
            }
            Self::Wave => {
                const FRAMES: &[&str] = &["● · ·", "· ● ·", "· · ●", "· ● ·"];
                FRAMES[(tick as usize) % FRAMES.len()]
            }
            Self::Blink => {
                const FRAMES: &[&str] = &["█", " "];
                FRAMES[(tick as usize / 5) % FRAMES.len()] // 500ms 闪烁
            }
        }
    }
}
```

### Step 2：`AgentConfig` 新增 `thinking_style` 字段

**文件**：`src/command/chat/storage/config.rs`

在 `AgentConfig` struct 中添加：

```rust
/// 思考指示器动画风格
#[serde(default)]
pub thinking_style: ThinkingStyle,
```

由于 `ThinkingStyle` 实现了 `Default`（默认 `Braille`），旧配置文件自动兼容。

### Step 3：注册为 Config Panel 全局字段

**文件**：`src/constants.rs`

在 `CONFIG_GLOBAL_FIELDS_TAB` 的 `"auto_restore_session"` 后面插入 `"thinking_style"`：

```rust
pub const CONFIG_GLOBAL_FIELDS_TAB: &[&str] = &[
    // ... 原有字段 ...
    "auto_restore_session",
    "thinking_style",       // <-- 新增
    "compact_enabled",
    // ... 后续字段 ...
];
```

同时调整分组定义（groups）中第二组的 count 从 `2` 改为 `3`（涵盖 theme + auto_restore_session + thinking_style）。

### Step 4：Config Panel 显示与交互

**文件**：`src/command/chat/render/helpers.rs`

#### 4a. `config_field_label_global` 新增分支

```rust
"thinking_style" => "思考动画",
```

#### 4b. `config_field_desc_global` 新增分支

```rust
"thinking_style" => "AI 思考时的加载动画风格",
```

#### 4c. `config_field_value_global` 新增分支

```rust
"thinking_style" => app.state.agent_config.thinking_style.display_name().to_string(),
```

#### 4d. `config_field_raw_value_global` 新增分支

```rust
"thinking_style" => app.state.agent_config.thinking_style.to_str().to_string(),
```

（需在 `ThinkingStyle` 上实现 `to_str()` 用于序列化）

#### 4e. `config_field_set_global` 新增分支

```rust
"thinking_style" => {
    // Enter 切换到下一个风格
    app.state.agent_config.thinking_style = app.state.agent_config.thinking_style.next();
}
```

**文件**：`src/command/chat/ui/config/global.rs`

在 `draw_tab_global_lines` 中新增对 `"thinking_style"` 的处理——类似 `"theme"` 的 `global_theme_row` 样式：

```rust
} else if *field_name == "thinking_style" {
    let style_name = app.state.agent_config.thinking_style.display_name();
    global_theme_row(label, style_name, desc, is_selected, "Enter 切换", t)
}
```

### Step 5：渲染层接入 `thinking_style`

**文件**：`src/command/chat/render/cache.rs`

将思考指示器渲染逻辑从硬编码改为读取配置：

```rust
// 原代码（第 286-288 行）：
// if streaming_text == "◍" {
//     let pulse_color = thinking_pulse_color(t);
//     let indicator_line = Line::from(Span::styled("◍", Style::default().fg(pulse_color)));

// 新代码：
if streaming_text == "◍" {
    let pulse_color = thinking_pulse_color(t);
    let tick = current_tick(); // 基于 SystemTime 计算帧序号
    let frame = app.state.agent_config.thinking_style.frame(tick);
    let indicator_line = Line::from(Span::styled(frame, Style::default().fg(pulse_color)));
```

新增辅助函数：

```rust
/// 基于当前时间计算 tick（每 100ms 递增 1）
fn current_tick() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64 / 100
}
```

### Step 6：Enter 键处理

**文件**：`src/command/chat/app/chat_app.rs`

在 `handle_enter` 的 `ConfigTab::Global` 分支（约第 1209 行），新增 `"thinking_style"` 的 Enter 切换处理：

```rust
} else if field == "thinking_style" {
    self.state.agent_config.thinking_style =
        self.state.agent_config.thinking_style.next();
}
```

---

## 涉及文件总览

| 文件 | 改动 |
|------|------|
| `src/command/chat/storage/config.rs` | 新增 `ThinkingStyle` 枚举 + `AgentConfig` 新增字段 |
| `src/constants.rs` | `CONFIG_GLOBAL_FIELDS_TAB` 插入 `"thinking_style"` + 调整分组 |
| `src/command/chat/render/helpers.rs` | 5 个 helper 函数新增 `"thinking_style"` 分支 |
| `src/command/chat/ui/config/global.rs` | `draw_tab_global_lines` 新增 thinking_style 渲染分支 |
| `src/command/chat/app/chat_app.rs` | Enter 键切换 thinking_style |
| `src/command/chat/render/cache.rs` | 思考指示器渲染改为读取 `thinking_style.frame(tick)` + 新增 `current_tick()` |

## 兼容性

- 旧配置文件无 `thinking_style` 字段 → serde `#[serde(default)]` 自动填充 `Braille`
- 所有 4 种风格均叠加原有的颜色脉冲效果（`thinking_pulse_color` 保留不变）
- 不改变 TUI 刷新频率（100ms）

## 预览效果

Config Panel 中将新增一行：

```
  思考动画        旋转点阵              AI 思考时的加载动画风格
                  ^^^^^^^^
                  Enter 切换
```

按 Enter 可在 `旋转点阵 → 经典圆点 → 呼吸圆点 → 波浪三连 → 闪烁光标` 之间循环切换。
