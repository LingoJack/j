# 状态栏精简设计方案

## 现状分析

当前顶部状态栏（第二行）内容：

```
 🦞 [BYPASS] Sprite  │  💫 Context: 12.5K  │  📬 Message: 42  │  ⏳ 思考中...  │  📱 远程已连接
```

**问题诊断：**

1. **分隔符过多** — `│` 分隔符 + 前后空格 = 5 字符/个，4 个段就有 20 字符浪费在分隔线上
2. **标签冗余** — `Context:`、`Message:` 这些标签文字在熟悉后没有信息增量，纯占位
3. **Emoji 堆叠** — 🦞💫📬📱 每个 emoji 占 2 列宽（CJK），视觉噪音大
4. **Bypass 标识突兀** — `[BYPASS]` 大写方括号与整体风格不协调
5. **信息密度不均** — 常驻信息（名字、消息数）和动态信息（loading、远程）混在一起，没有层次

## 设计方案

### 核心思路：左标识 + 右指标，去掉标签文字和冗余分隔符

Bypass 模式时，直接将龙虾图标 `🦞` 替换为 `⚡`，无需额外文字标识。

### 详细规则

| 元素 | 现状 | 改进 |
|------|------|------|
| 品牌（普通） | `🦞 Sprite` | 保持不变 |
| 品牌（bypass） | `🦞 [BYPASS] Sprite` | `⚡ Sprite`，直接换图标 |
| Context | `💫 Context: 12.5K` | `context(12.5K)` |
| Message | `📬 Message: 42` | `message(42)`，用 `·` 与 Context 连接 |
| 分隔符 | `│`（5字符） | 用 `·`（1字符）连接紧凑指标 |
| Loading | `│ ⏳ 思考中...` | 右对齐显示 |
| 远程 | `│ 📱 远程已连接` | 右对齐显示 |

### 各状态下的效果

**1. 普通状态（无 bypass、无 loading）**
```
 🦞 Sprite context(12.5K)·message(42)
```

**2. Bypass 模式开启**
```
 ⚡ Sprite context(12.5K)·message(42)
```
图标直接替换为 ⚡（红色/黄色加粗），一眼可辨

**3. Loading 中**
```
 🦞 Sprite context(12.5K)·message(42)              ⏳ 思考中...
```
loading 状态右对齐，与左侧信息形成视觉平衡

**4. Bypass + Loading**
```
 ⚡ Sprite context(12.5K)·message(42)               ⏳ 执行 Bash...
```

**5. 远程连接**
```
 🦞 Sprite context(12.5K)·message(42)               📱 远程
```

**6. 全部状态叠加**
```
 ⚡ Sprite context(12.5K)·message(42)               ⏳ 执行 Bash... 📱 远程
```

### 右对齐实现方式

使用 ratatui 的 `Line` 无法直接右对齐，但可以通过计算左侧宽度 + 填充空格实现：

```rust
let left_width = calculate_spans_width(&left_spans);
let right_width = calculate_spans_width(&right_spans);
let padding = area.width as usize - left_width - right_width;
// 插入 padding 个空格
```

## 改动清单

1. **`src/command/chat/ui/chat.rs`** — `draw_title_bar` 函数重构
   - Bypass 时图标从 `🦞` 替换为 `⚡`
   - 去掉 `[BYPASS]` 文字标识
   - `Context:` / `Message:` 改为 `context(值)·message(值)` 紧凑格式
   - 去掉 💫📬 emoji
   - Loading / 远程连接右对齐

2. **无需改动 Theme** — 复用现有 `title_icon`、`title_model`、`title_count`、`config_toggle_off` 等颜色

3. **无需改动 hint.rs** — 底部提示栏保持不变

## 风险评估

- **信息丢失**：去掉 `Context:` / `Message:` 标签后，新用户可能不理解 `12.5K·42` 的含义。但底部 hint 栏已有快捷键提示，且这两个指标含义可从上下文推断。
- **右对齐计算**：需要精确计算 span 宽度（含 CJK 字符），已有 `display_width` 工具函数可用。
