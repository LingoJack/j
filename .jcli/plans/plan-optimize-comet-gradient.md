# Plan: 优化彗星 (Comet) 思考动画渐变效果

## 现状分析

当前 Comet 动画存在以下问题：

### 1. 单色渲染（最大问题）
`cache.rs:290` 渲染彗星时，整个帧用统一的 `pulse_color`（单一 RGB 颜色）：
```rust
let indicator_line = Line::from(Span::styled(frame, Style::default().fg(pulse_color)));
```
这意味着 `▓▒░` 拖尾的每个字符颜色完全相同，失去了密度渐变带来的视觉层次感。

### 2. 已有渐变基础设施未被利用
`palette.rs` 已有 8 组主题调色板（每组 16 个三色渐变元组），`welcome_box` 已成功使用它实现逐字符 RGB 插值。但 Comet 动画完全没有利用这套系统。

### 3. 帧数据不够精细
当前帧轨道宽度 9，拖尾仅 `▓▒░` 三个字符，视觉层次有限。

## 优化方案

### Step 1: 增强 Comet 帧数据
**文件**: `src/command/chat/storage/config.rs`

- 扩展轨道宽度为 12（原 9），拖尾增加为 `██▓▒░·` 6 个字符（原 3 个），提供更丰富的密度梯度
- 增加 ping-pong 帧数（14 帧，原 12 帧），运动更平滑
- `frame()` 返回类型保持 `&'static str`，不变更签名

### Step 2: 新增 Comet 专用渲染逻辑
**文件**: `src/command/chat/render/cache.rs`

- 在 `if streaming_text == "◍"` 分支中，针对 `ThinkingStyle::Comet` 添加特殊处理
- 使用 `palette::get_gradient()` 获取当前主题的渐变三元组
- 逐字符插值着色：
  - `██`（彗星头部）→ 亮色（end_c 或 start_c）
  - `▓▒░·`（拖尾）→ 逐步衰减至暗色
  - 空格 → 保持背景色
- 拖尾颜色随 tick 做色相偏移，产生流动感
- 非 Comet 风格保持原有逻辑不变

### Step 3: 利用已有调色板系统
**文件**: `src/command/chat/render/cache.rs`

- 从 `app.ui.theme.welcome_palette` 读取调色板索引
- 使用 `palette::get_gradient(palette, tick_based_idx)` 获取渐变三元组
- 渐变方向随彗星运动方向翻转（左移 vs 右移），头亮尾暗

## 涉及文件

| 文件 | 改动 |
|------|------|
| `src/command/chat/storage/config.rs` | 优化 Comet 帧数据（更长轨道、更丰富拖尾） |
| `src/command/chat/render/cache.rs` | Comet 逐字符渐变渲染逻辑 |

## 不变更

- `palette.rs` — 完全复用，不修改
- `ThinkingStyle` 的枚举定义和接口签名
- 其他 ThinkingStyle（Braille/Classic/Pulse/Wave/Blink）的渲染逻辑
- `constants.rs` 中的 THINKING_PULSE 常量
