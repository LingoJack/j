# SubAgent Tool 特化渲染方案

## 需求分析

Agent（SubAgent）工具的 tool call request 展示需要特化：
1. **prompt 要有边框**，类似 tool result 的 `render_agent_result_nested` 样式（`┌──┐` 边框）
2. **background run 要有标识**（如 `[background]` 标签），**折叠模式下也要显示**
3. **description 也要显示**

## 现状

### tool call request（`render_tool_call_request_msg`）

**折叠模式**：
- Agent 不在 `extract_tool_description_from_args` 的 match 分支中 → 返回 None
- 回退到 raw arguments JSON preview：`🤖 Agent {"prompt": "...", "description": "..."}`
- 效果：一大坨 JSON，可读性差

**展开模式**：
- 走 `render_json_params_enhanced`，逐字段列出
- 效果：所有参数平铺，prompt 全文裸露显示，无边框

### tool result（`render_tool_result_msg`）

- Agent result 使用 `render_agent_result_nested`，有 `┌──┐` 边框包裹
- 效果良好

## 实现方案

### 1. 新增 `AgentCallArgs` 结构体和提取函数

**文件**: `src/command/chat/render/cache.rs`（在 `BashArgs` 结构体附近）

```rust
/// Agent 工具参数结构（用于渲染）
struct AgentCallArgs {
    prompt: String,
    description: Option<String>,
    run_in_background: bool,
}

/// 从 Agent 工具的 arguments JSON 中提取参数
fn extract_agent_args(arguments: &str) -> Option<AgentCallArgs> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;
    Some(AgentCallArgs {
        prompt: parsed.get("prompt")?.as_str()?.to_string(),
        description: parsed.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
        run_in_background: parsed.get("run_in_background").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}
```

### 2. 新增常量

**文件**: `src/command/chat/constants.rs`

```rust
/// Agent tool call request 展开模式下 prompt 最大显示行数
pub const AGENT_CALL_PROMPT_MAX_LINES: usize = 15;
```

### 3. 修改 `render_tool_call_request_msg` 函数 — 折叠模式

在折叠模式的 `else` 分支中（约 L1625），在 `extract_tool_description_from_args` 调用之前，先检查是否为 Agent 工具，如果是则走专用逻辑：

```rust
} else {
    // 折叠模式
    // Agent 工具专用折叠渲染：显示 description + [background] 标识
    if matches!(tc.name.as_str(), "Agent" | "AgentTeam") {
        if let Some(agent_args) = extract_agent_args(&tc.arguments) {
            let mut desc_parts: Vec<String> = Vec::new();
            if agent_args.run_in_background {
                desc_parts.push("[background]".to_string());
            }
            if let Some(ref desc) = agent_args.description {
                desc_parts.push(desc.clone());
            }
            if desc_parts.is_empty() {
                // 只有 prompt，截取第一行作为预览
                let first_line = agent_args.prompt.lines().next().unwrap_or("");
                let preview = if first_line.chars().count() > TOOL_ARG_PREVIEW_MAX_CHARS {
                    format!("{}...", &first_line[..TOOL_ARG_PREVIEW_MAX_CHARS])
                } else {
                    first_line.to_string()
                };
                desc_parts.push(preview);
            }
            let desc_text = desc_parts.join("  ");
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(icon, Style::default().fg(tool_color)),
                Span::styled(" ", Style::default()),
                Span::styled(
                    tc.name.clone(),
                    Style::default().fg(tool_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {}", desc_text), Style::default().fg(theme.text_dim)),
            ]));
            continue; // 或 skip 到下一个 tool_call
        }
    }

    // 原有的 tool_desc / raw arguments 逻辑保持不变
    let tool_desc = extract_tool_description_from_args(&tc.name, &tc.arguments);
    ...
}
```

### 4. 修改 `render_tool_call_request_msg` 函数 — 展开模式

在展开模式的参数详情渲染中（约 L1597），在 `matches!(tc.name.as_str(), "Bash" | "Shell")` 之后添加 Agent 分支：

```rust
} else if matches!(tc.name.as_str(), "Agent" | "AgentTeam") {
    // Agent 工具使用专用渲染：边框 + prompt + 元信息
    if let Some(agent_args) = extract_agent_args(&tc.arguments) {
        render_agent_call_request_expanded(
            &agent_args,
            bubble_max_width,
            lines,
            theme,
        );
    }
}
```

### 5. 新增 `render_agent_call_request_expanded` 函数

**文件**: `src/command/chat/render/cache.rs`

```rust
/// 渲染 Agent 工具调用请求的展开模式（边框 + prompt + 元信息）
fn render_agent_call_request_expanded(
    args: &AgentCallArgs,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let border_color = theme.text_dim;
    let content_w = bubble_max_width.saturating_sub(6);

    // 元信息行：[background] 标识
    if args.run_in_background {
        for wrapped in wrap_text("[background]", content_w) {
            lines.push(Line::from(vec![
                Span::styled("    ", Style::default()),
                Span::styled(wrapped, Style::default().fg(theme.text_dim)),
            ]));
        }
    }

    // Prompt 边框显示（复用 render_agent_result_nested 的边框风格）
    let top_border = format!("  ┌{}┐", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        top_border,
        Style::default().fg(border_color),
    )));

    let prompt_lines: Vec<&str> = args.prompt.lines().collect();
    let total = prompt_lines.len();
    let max_display = AGENT_CALL_PROMPT_MAX_LINES;
    let display_lines = &prompt_lines[..total.min(max_display)];

    for line in display_lines {
        for wrapped in wrap_text(line, content_w) {
            lines.push(bordered_line(
                vec![Span::styled(wrapped, Style::default().fg(theme.text_dim))],
                bubble_max_width,
                border_color,
                Color::default(),
            ));
        }
    }

    // 截断提示
    if total > max_display {
        lines.push(bordered_line(
            vec![Span::styled(
                format!("... (共 {} 行)", total),
                Style::default().fg(theme.text_dim),
            )],
            bubble_max_width,
            border_color,
            Color::default(),
        ));
    }

    let bottom_border = format!("  └{}┘", "─".repeat(bubble_max_width.saturating_sub(4)));
    lines.push(Line::from(Span::styled(
        bottom_border,
        Style::default().fg(border_color),
    )));
}
```

### 6. 在 `extract_tool_description_from_args` 中添加 Agent 分支（兜底）

**文件**: `src/command/chat/render/cache.rs`（约 L2272）

在 `_ => None` 之前添加：
```rust
"Agent" | "AgentTeam" => parsed
    .get("description")
    .and_then(|v| v.as_str())
    .map(|s| s.to_string()),
```

注意：由于折叠模式已经在第 3 步中做了 Agent 专用分支（会先匹配），这里的添加主要是作为兜底保障。

## 修改文件清单

1. **`src/command/chat/render/cache.rs`**:
   - 新增 `AgentCallArgs` 结构体 + `extract_agent_args` 函数
   - 修改 `render_tool_call_request_msg`：折叠模式添加 Agent 专用分支（含 `[background]` + description）
   - 修改 `render_tool_call_request_msg`：展开模式添加 Agent 分支调用 `render_agent_call_request_expanded`
   - 新增 `render_agent_call_request_expanded` 函数
   - `extract_tool_description_from_args`: 添加 `"Agent" | "AgentTeam"` 兜底分支

2. **`src/command/chat/constants.rs`**:
   - 新增 `AGENT_CALL_PROMPT_MAX_LINES` 常量

## 预期效果

### 折叠模式
```
  🤖 Agent  [background]  搜索配置模块
```
无 description 时取 prompt 第一行：
```
  🤖 Agent  [background]  请在项目中搜索与配置模块相关的代码...
```

### 展开模式
```
  🤖 Agent - 搜索配置模块
    [background]
  ┌──────────────────────────────────┐
  │ 请在项目中搜索与配置模块相关的代码 │
  │ 列出所有配置文件和结构体定义       │
  └──────────────────────────────────┘
```

对比之前：
- 折叠：`🤖 Agent {"prompt":"请在项目中搜索...","description":"搜索配置模块","run_in_background":true}`
- 展开：所有参数平铺，无边框，无 background 标识

可读性大幅提升。
