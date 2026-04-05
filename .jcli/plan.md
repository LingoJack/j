# Plan: 顶部状态栏优化方案

## 目标
- **信息展示**：新增 Context Usage（替代模型名称）
- **视觉风格**：极简单行 + 底部分割线
- **实现策略**：纯估算方案，不依赖 API token 用量

---

## 一、现状分析

### 1. 当前状态栏结构 (`draw_title_bar` in `chat.rs`)

```
┌─────────────────────────────────────────────────────────────────┐
│ 🦞 Sprite  │  💫 model_name  │  📬 N 条消息  [加载状态] [远程] │
└─────────────────────────────────────────────────────────────────┘
```
- 高度：3 行（含圆角边框）
- 元素从左到右线性排列

### 2. 上下文长度估算

`compact.rs` 中已有估算函数：
```rust
pub fn estimate_tokens(messages: &[ChatMessage]) -> usize {
    serde_json::to_string(messages).unwrap_or_default().len() / 4
}
```
- 基于字符数粗略估算（~4 chars/token）
- 可以直接复用

---

## 二、优化方案：极简单行 + 底部分割线

```
🦞 Sprite │ 💫 Context: 12K │ 💬 42 │ ⏳ ...
──────────────────────────────────────────────────────────────────
```

**高度**：2 行（1 行内容 + 1 行分割线）

**信息布局**（从左到右）：
| 位置 | 内容 | 示例 |
|------|------|------|
| 品牌 | 图标 + 应用名 | `🦞 Sprite` |
| 上下文 | 💫 + Context + 估算值 | `💫 Context: 12K` |
| 消息 | 会话消息数 | `📬 42` |
| 状态 | 动态状态 | `⏳ ...` 或 `🔗 remote` |

**视觉效果**：
- 移除上边框和左右边框
- 仅保留底部分割线（`─` 字符横线）
- 使用 `│` 分隔各信息块
- 保持现有 emoji 风格

---

## 三、技术实现

### Step 1: 新增上下文格式化工具函数

```rust
/// 格式化上下文估算值
fn format_context_tokens(tokens: usize) -> String {
    if tokens >= 1000 {
        format!("{}K", tokens / 1000)
    } else {
        tokens.to_string()
    }
}
```

### Step 2: 重写 `draw_title_bar`

改为单行内容 + 底部分割线：
```rust
fn draw_title_bar(&self, area: Rect, buf: &mut Buffer) {
    // 估算上下文
    let estimated = compact::estimate_tokens(&self.state.session.messages);
    let ctx_str = format_context_tokens(estimated);
    
    // 第一行：状态信息
    let content_line = Line::default()
        .spans(vec![
            Span::styled("🦞 Sprite ", style_icon),
            Span::styled("│ ", style_sep),
            Span::styled("💫 Context: ", style_context_icon),
            Span::styled(ctx_str, style_context),
            Span::styled(" │ ", style_sep),
            Span::styled(format!("📬 {}", msg_count), style_count),
            // 动态状态
            status_span,
        ]);
    
    // 第二行：底部分割线
    let separator = Line::styled("─".repeat(area.width as usize), style_dim);
    
    content_line.render(area, buf);
    separator.render(Rect::new(area.x, area.y + 1, area.width, 1), buf);
}
```

### Step 3: 调整 `draw_chat` 中的布局

状态栏高度从 3 行改为 2 行：
```rust
let title_height = 2; // 内容行 + 分割线
let msg_area = Rect::new(
    area.x, 
    area.y + title_height, 
    area.width, 
    area.height - title_height - input_height
);
```

---

## 四、文件变更清单

| 文件 | 变更内容 |
|------|----------|
| `src/command/chat/ui/chat.rs` | 重写 `draw_title_bar`，移除模型名称，添加底部分割线 |
| `src/command/chat/compact.rs` | 确保 `estimate_tokens` 为 pub |

---

## 五、用户体验

### 正常状态
```
🦞 Sprite │ 💫 Context: 12K │ 📬 42
──────────────────────────────────────────────────────────────────
```

### 加载中
```
🦞 Sprite │ 💫 Context: 12K │ 📬 43 │ ⏳ thinking...
──────────────────────────────────────────────────────────────────
```

### 远程连接
```
🦞 Sprite │ 💫 Context: 12K │ 📬 42 │ 🔗 remote
──────────────────────────────────────────────────────────────────
```

---

## 六、实现步骤

1. 新增 `format_context_tokens` 工具函数
2. 重写 `draw_title_bar`：移除模型名称、改为单行 + 底部分割线
3. 调整 `draw_chat` 布局计算（title_height = 2）
4. 测试各模式下的显示效果
