# Fix: bypass 模式换行后光标位置计算错误

## 问题分析

`draw_input` 中存在两个硬编码问题：

### 1. prompt_width 硬编码

```rust
let (prompt, prompt_style, prompt_width) = if ... {
    (" bypass + ", ..., 10)  // 硬编码 10
} else if ... {
    (" + ", ..., 3)          // 硬编码 3
} ...
```

prompt 字符串与 width 分开定义，若 prompt 变化需手动同步 width，易出错且不灵活。

### 2. 续行缩进硬编码为 3

```rust
} else {
    spans.push(Span::styled("   ", Style::default()));  // 硬编码 3 空格
}
```

光标 X 坐标统一按 `prompt_width` 偏移计算：
```rust
let cursor_x = area.x + prompt_width as u16 + cursor_col_in_line;
```

**正常模式**：prompt_width=3，续行缩进=3 → 正确  
**bypass 模式**：prompt_width=10，续行缩进=3 → 光标偏移 7 字符

此外，`wrap_width` 按 `usable_width - prompt_width` 计算，续行缩进仅 3，折行点与显示不匹配。

## 修复方案

**核心原则**：prompt 宽度由字符串动态计算，续行缩进自动适应。

### 1. 动态计算 prompt_width

```rust
// 先确定 prompt 字符串和样式
let (prompt, prompt_style) = if app.state.is_loading && app.ui.auto_approve {
    (" bypass + ", Style::default().fg(t.config_toggle_on))
} else if app.state.is_loading {
    (" + ", Style::default().fg(t.input_prompt_loading))
} else if app.ui.auto_approve {
    (" bypass > ", Style::default().fg(t.config_toggle_on))
} else {
    (" > ", Style::default().fg(t.input_prompt))
};

// 动态计算宽度（prompt 只含 ASCII 字符，用 len() 即可）
let prompt_width = prompt.len();
```

### 2. 续行缩进动态生成

```rust
// 原代码
} else {
    spans.push(Span::styled("   ", Style::default()));
}

// 改为
} else {
    spans.push(Span::styled(
        " ".repeat(prompt_width),
        Style::default(),
    ));
}
```

## 效果

- prompt 字符串任意变化，宽度自动计算
- 续行缩进自动匹配 prompt 宽度
- 光标计算对所有模式都正确
- 折行宽度与显示宽度一致

## 涉及文件

- `src/command/chat/ui/input.rs` — `draw_input` 函数