# 主题感知欢迎框诗句渐变色

## 问题

`welcome_box` 函数中的诗句渐变色（`GRADIENT_TRIPLES`）是硬编码的 16 组 RGB 三元组，
为深色背景（Midnight 主题）设计。当切换到 Light、Anthropic Light、Nord 等主题时，
渐变色与背景不协调：
- 浅色背景上深色渐变对比度差
- 各主题色温（冷/暖/极地冰蓝）与渐变色风格不匹配

## 方案

### 修改 Theme 结构体

在 Theme 中新增 3 个 Color 字段，定义每个主题的"欢迎框诗句渐变基调"：

```rust
// 欢迎框诗句渐变色
pub welcome_gradient_start: Color, // 起始色
pub welcome_gradient_mid: Color,   // 中间色（对比性中间调）
pub welcome_gradient_end: Color,   // 结束色
```

### 各主题配色

| 主题 | start | mid | end | 风格描述 |
|------|-------|-----|-----|---------|
| Midnight | (212,175,55) 古金 | (220,80,90) 胭脂 | (255,230,140) 淡金 | 古典金色（保持原有第一组） |
| Dark | (200,200,80) 淡金 | (180,100,120) 暮红 | (200,220,160) 浅绿 | 低饱和度柔和 |
| Light | (25,80,180) 蓝 | (200,140,30) 琥珀 | (34,139,80) 绿 | 清新明快 |
| Nord | (136,192,208) 冰蓝 | (163,190,140) 嫩芽 | (143,188,187) 青瓷 | 极地冰蓝 |
| Monokai | (230,219,116) 黄 | (249,38,114) 粉红 | (166,226,46) 绿 | 经典高对比 |
| Anthropic Light | (204,120,92) 赭陶 | (74,122,80) 绿 | (160,120,48) 琥珀 | 暖色赭陶 |
| Anthropic Dark | (130,170,255) 蓝 | (192,153,255) 紫 | (195,232,141) 绿 | 月蓝幽彩 |
| Terminal | DarkGray | Gray | DarkGray | 低调中性 |

### 修改 welcome_box 函数

将硬编码的 `GRADIENT_TRIPLES[quote_idx]` 替换为从 Theme 读取渐变色，
并根据 `quote_idx` 做色相偏移以产生视觉变化（保留"每次不同"的效果）。

具体实现：
1. 从 `theme.welcome_gradient_start/mid/end` 读取主题三色
2. 基于 `quote_idx` 对三色的 RGB 通道做 ±15% 的周期性偏移
3. 偏移公式：`offset = sin(quote_idx * phase) * amplitude`，不同通道用不同相位
4. 这样产生 16 种微变体，视觉上协调但每次略有不同

### 需修改的文件

1. **src/command/chat/theme.rs** - Theme 结构体新增 3 字段，7 个主题构造函数各加配色
2. **src/command/chat/ui/components.rs** - `welcome_box` 函数使用 theme 渐变色替代硬编码
