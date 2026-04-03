# Plan: 优化 tool call 和 tool result 的 UI 展示

## 目标

针对用户反馈的两个具体问题：

1. **JSON key 高亮**：折叠模式下的参数预览中，JSON key 应该高亮显示
2. **对象预览格式修正**：折叠模式下 `{...}` 应该显示为闭合格式，当前 `{...}` 不够清晰

---

## 当前实现分析

### 折叠模式参数预览（`render_cache.rs:1108-1133`）

```rust
// 折叠模式：图标 + 工具名 + 参数预览
let args_preview: String = tc.arguments.chars().take(60).collect();
let suffix = if tc.arguments.chars().count() > 60 {
    "…"
} else {
    ""
};

lines.push(Line::from(vec![
    Span::styled("  ", Style::default()),
    Span::styled(icon, Style::default().fg(tool_color)),
    Span::styled(" ", Style::default()),
    Span::styled(
        tc.name.clone(),
        Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
    ),
    if !args_preview.is_empty() {
        Span::styled(
            format!(" {}{}", args_preview, suffix),
            Style::default().fg(theme.text_dim),
        )
    } else {
        Span::raw("")
    },
]));
```

问题：
1. 参数预览整体用 `text_dim` 颜色，没有区分 key/value
2. 对象截断时显示 `{"key": "value"...`，没有闭合括号

---

## 实施方案

### 改动 1：JSON key 高亮

在折叠模式下解析 JSON，为 key 添加高亮颜色（使用 `theme.text_normal` 或 `theme.label_ai`）。

**伪代码**：
```rust
// 折叠模式：解析 JSON 并高亮 key
if let Ok(json) = serde_json::from_str::<serde_json::Value>(&tc.arguments) {
    render_json_preview_with_highlight(&json, tool_color, theme, lines);
} else {
    // 非 JSON 参数，保持原逻辑
}
```

### 改动 2：对象预览格式修正

截断时确保括号闭合：
- `{"key": "value"...}` → `{"key": "value", ...}`
- 对象：`{key: val, ...}`
- 数组：`[item1, item2, ...]`

---

## 涉及文件

| 文件 | 改动 |
|------|------|
| `src/command/chat/render_cache.rs` | 修改 `render_tool_call_request_msg` 函数的折叠模式逻辑 |

---

## 具体代码改动位置

`render_cache.rs:1108-1133` 折叠模式部分

需要：
1. 添加 `render_json_preview_with_highlight()` 函数
2. 解析 JSON 并生成带样式的 Span 列表
3. key 用高亮颜色，value 用 dim 颜色
4. 截断时添加闭合括号
