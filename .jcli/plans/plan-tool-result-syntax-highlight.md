# Tool Result 语法高亮计划

## 目标

Read 工具的输出内容（代码文件）在 tool result 渲染时支持语法高亮，而非全灰色 `text_dim`。

## 现状分析

### 数据流

1. **tool_call 消息**：包含 `tool_name="Read"` 和 `tool_args='{"file_path":"...","offset":...}'`
2. **tool_result 消息**：包含 `label="Read"` 和 `content="...文件内容..."`（带行号前缀）
3. **渲染入口**：`render_tool_result_msg(params, lines)` 在 `tool_result_render.rs:34`
4. **Read 输出渲染**：第 174-216 行"正常结果"分支，处理带行号文本

### 已有资源

- `highlight_code_line(line, lang, theme)` — `src/markdown/highlight.rs:42`
- `EditorTheme` — 包含 code_keyword/code_string/code_number 等配色
- `Theme` — Chat UI 主题，需映射到 `EditorTheme`

### 问题

1. `tool_args` 在 `render_tool_result_msg` 中可用，是 JSON 字符串
2. 需要解析 JSON 提取 `file_path`，再根据扩展名推断语言
3. `highlight_code_line` 需要 `EditorTheme`，但当前只有 `Theme`

## 方案

### 方案 A：在 tool_result_render 中直接高亮

修改 `render_tool_result_msg` 的"正常结果"分支：

```rust
else if tool_name == "Read" {
    // 解析 tool_args 提取 file_path
    let lang = tool_args
        .and_then(|args| parse_file_path_from_json(args))
        .and_then(|path| infer_lang_from_path(&path))
        .unwrap_or("");

    // 高亮渲染
    for line in all_lines {
        // 移除行号前缀，提取纯代码内容
        let (line_num, code_content) = split_line_number_prefix(line);
        let highlighted = highlight_code_line(&code_content, lang, &editor_theme);
        // 重新组合：行号前缀 + 高亮代码
        let mut spans = vec![Span::styled(line_num, Style::default().fg(theme.text_dim))];
        spans.extend(highlighted);
        lines.push(Line::from(spans));
    }
}
```

**优点**：改动集中，不影响其他渲染流程
**缺点**：需要解析 JSON + 主题映射

### 方案 B：复用现有代码块渲染

Read 工具输出已经带行号前缀（如 `     1│ fn main() {`）。可以直接用 IR 渲染器的代码块渲染逻辑，但需要调整行号格式。

**缺点**：行号格式与 IR 渲染器不兼容，改动较大

### 推荐：方案 A

## 实施步骤

1. **新增辅助函数**：
   - `parse_file_path_from_json(json: &str) -> Option<String>` — 从 tool_args JSON 提取 file_path
   - `infer_lang_from_path(path: &str) -> Option<&'static str>` — 根据扩展名推断语言
   - `split_line_number_prefix(line: &str) -> (String, String)` — 分离行号前缀和代码内容

2. **主题映射**：
   - `Theme` → `EditorTheme` 的颜色映射（或直接使用 theme 的字段）

3. **修改 render_tool_result_msg**：
   - 当 `tool_name == "Read"` 时走高亮分支
   - 否则保持原有逻辑

4. **cargo check + clippy**

## 文件变更

| 文件 | 变更 |
|------|------|
| `src/command/chat/render/cache/tool_result_render.rs` | 新增辅助函数 + Read 高亮分支 |
| `src/command/chat/render/theme.rs` | 可能需要新增 `EditorTheme` 映射 |

## 语言推断规则

| 扩展名 | 语言 |
|--------|------|
| `.rs` | rust |
| `.go` | go |
| `.py` | python |
| `.js`, `.ts`, `.tsx` | javascript |
| `.java` | java |
| `.c`, `.cpp`, `.h` | cpp |
| `.sh`, `.bash` | bash |
| `.json` | json |
| `.yaml`, `.yml` | yaml |
| `.md` | markdown |
| `.toml` | toml |
| `.sql` | sql |
| 其他 | 无高亮 |