# Plan: YAML 语法高亮改进

## 问题分析

当前 YAML 文件高亮效果差的根本原因：

1. **关键字字典过简**：`YAML_KEYWORDS` 只有 7 个词（`true`, `false`, `null`, `yes`, `no`, `on`, `off`），无法覆盖 YAML 丰富的语法结构
2. **缺少 YAML 特有语法处理**：现有 `highlight.rs` 只处理通用字符串和注释，没有针对 YAML 的特殊语法：
   - 键名（key）：如 `name:`, `version:` 等冒号前的标识符
   - 列表指示符 `-`
   - 文档分隔符 `---` / `...`
   - 锚点 `&anchor` / 别名 `*alias` / 合并键 `<<:`
   - 块标量指示符 `|` / `>`
   - 类型标签 `!!str`, `!!int` 等

## 改进方案

### 方案一：扩展 YAML_KEYWORDS + 复用现有颜色

在 `dicts.rs` 扩展关键字列表，并在 `highlight.rs` 添加 YAML 专用处理函数，复用现有 `EditorTheme` 颜色字段：

| YAML 元素 | 复用颜色字段 | 视觉效果 |
|-----------|-------------|----------|
| 键名 | `code_type` | 青/黄色系 |
| 键名后的冒号 `:` | `code_default` | 默认色 |
| 文档分隔符 `---` / `...` | `code_keyword` | 橙/红色系 |
| 列表指示符 `-` | `code_keyword` | 橙/红色系 |
| 锚点 `&` / 别名 `*` / 合并 `<<` | `code_attribute` | 紫色系 |
| 标量指示符 `|` / `>` | `code_macro` | 紫色系 |
| 类型标签 `!!xxx` | `code_comment`（斜体） | 灰色斜体 |

**优点**：不修改 `EditorTheme` 结构，改动范围小
**缺点**：颜色语义不够精确（如用 `code_attribute` 表示锚点）

### 方案二：新增 YAML 专用颜色字段

在 `EditorTheme` 添加：
- `code_yaml_key`: 键名颜色
- `code_yaml_anchor`: 锚点/别名颜色
- `code_yaml_indicator`: 列表指示符/块标量指示符颜色

**优点**：语义清晰，颜色可独立定制
**缺点**：需要修改多处（`EditorTheme` 定义、`Theme` 转换、渲染调用等）

---

**推荐方案一**：改动最小，效果明显。后续如有需求可升级到方案二。

## 实施步骤

### Step 1: 扩展 YAML_KEYWORDS（`src/markdown/highlight/dicts.rs`）

添加常见 YAML 配置项关键字作为"高亮候选"（主要供 `classify_word` 使用）：
```rust
pub const YAML_KEYWORDS: &[&str] = &[
    // 布尔/空值
    "true", "false", "null", "yes", "no", "on", "off",
    // 常见配置键名（仅作参考，主要靠 handle_yaml_key 高亮）
    "name", "version", "id", "type", "kind", "status", "state",
    "enabled", "disabled", "active", "inactive",
    "title", "description", "summary", "content", "text",
    "path", "file", "dir", "directory", "url", "uri", "href", "link",
    "host", "port", "address", "endpoint", "server", "client",
    "user", "username", "password", "token", "key", "secret", "credential",
    "timeout", "interval", "duration", "delay", "retry", "count",
    "max", "min", "limit", "size", "width", "height", "length",
    "default", "optional", "required", "readonly", "hidden",
    "include", "exclude", "filter", "pattern", "regex", "match",
    "input", "output", "source", "target", "destination", "result",
    "start", "end", "begin", "finish", "create", "update", "delete",
    "get", "post", "put", "patch", "head", "options",
    "depends", "depends_on", "dependencies", "requires", "provides",
    "env", "environment", "config", "setting", "option", "param", "parameter",
    "value", "values", "data", "items", "list", "array", "map", "dict", "object",
    // GitHub Actions / CI 特有
    "runs-on", "uses", "with", "runs", "steps", "jobs", "workflow",
    "on", "push", "pull_request", "branch", "branches", "tag", "tags",
    "checkout", "setup", "cache", "restore", "save", "upload", "download",
    // Kubernetes 特有
    "apiVersion", "kind", "metadata", "spec", "selector", "template",
    "containers", "image", "resources", "limits", "requests",
    "ports", "volumes", "volumeMounts", "env", "envFrom",
    "replicas", "strategy", "rollback", "revisionHistoryLimit",
];
```

### Step 2: 添加 YAML 专用处理函数（`src/markdown/highlight.rs`）

在主解析循环中，针对 `yaml` / `yml` 语言，添加以下处理函数：

```rust
/// 处理 YAML 键名（冒号前的标识符）
/// 模式：`key:` 或 `key :`（冒号后可有空格）
fn handle_yaml_key(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") || !ch.is_alphanumeric() && ch != '_' {
        return false;
    }
    // 向前查看是否后面有冒号（允许中间有更多字符）
    let rest: String = ctx.chars.clone().collect();
    // 检查模式：word 后面紧跟 `:` 或 `: `（冒号后有空格）
    // 但要排除字符串内的冒号（已有引号处理）
    if let Some(colon_pos) = rest.find(':') {
        let before_colon = &rest[..colon_pos];
        // 确保冒号前只有合法的键名字符
        if before_colon.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.') {
            // 检查冒号后是否是值开始（空格、换行、或结束）
            let after_colon = &rest[colon_pos + 1..];
            let is_key = after_colon.is_empty() 
                || after_colon.starts_with(' ') 
                || after_colon.starts_with('\n')
                || after_colon.starts_with('\t');
            if is_key {
                flush_buf(ctx);
                let mut key = String::new();
                // 读取键名部分
                while let Some(&c) = ctx.chars.peek() {
                    if c == ':' {
                        key.push(c);
                        ctx.chars.next();
                        break;
                    }
                    key.push(c);
                    ctx.chars.next();
                }
                // 键名用 type 风格，冒号用 default
                let key_part = key.trim_end_matches(':');
                ctx.spans.push(Span::styled(key_part.to_string(), ctx.styles.type_style));
                ctx.spans.push(Span::styled(":", ctx.styles.default_style));
                return true;
            }
        }
    }
    false
}

/// 处理 YAML 文档分隔符 `---` 和 `...`
fn handle_yaml_document_marker(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") {
        return false;
    }
    let rest: String = ctx.chars.clone().collect();
    if rest.starts_with("---") || rest.starts_with("...") {
        flush_buf(ctx);
        let marker = if rest.starts_with("---") { "---" } else { "..." };
        for _ in 0..marker.len() {
            ctx.chars.next();
        }
        ctx.spans.push(Span::styled(marker.to_string(), ctx.styles.kw_style));
        return true;
    }
    false
}

/// 处理 YAML 锚点 `&anchor`、别名 `*alias`、合并键 `<<:`
fn handle_yaml_anchor(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") {
        return false;
    }
    // 处理 `<<:` 合并键
    let rest: String = ctx.chars.clone().collect();
    if rest.starts_with("<<:") {
        flush_buf(ctx);
        ctx.chars.next(); ctx.chars.next(); // <<
        ctx.chars.next(); // :
        let anchor_style = Style::default().fg(ctx.theme.code_attribute);
        ctx.spans.push(Span::styled("<<", anchor_style));
        ctx.spans.push(Span::styled(":", ctx.styles.default_style));
        return true;
    }
    // 处理 `&anchor` 或 `*alias`
    if ch != '&' && ch != '*' {
        return false;
    }
    flush_buf(ctx);
    let mut anchor = String::new();
    anchor.push(ch);
    ctx.chars.next();
    while let Some(&c) = ctx.chars.peek() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            anchor.push(c);
            ctx.chars.next();
        } else {
            break;
        }
    }
    let anchor_style = Style::default().fg(ctx.theme.code_attribute);
    ctx.spans.push(Span::styled(anchor, anchor_style));
    true
}

/// 处理 YAML 块标量指示符 `|` 或 `>` 及其修饰符
fn handle_yaml_block_scalar(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") || (ch != '|' && ch != '>') {
        return false;
    }
    flush_buf(ctx);
    let mut indicator = String::new();
    indicator.push(ch);
    ctx.chars.next();
    // 可选修饰符：`-`（strip）、`+`（keep）、数字（缩进）
    while let Some(&c) = ctx.chars.peek() {
        if c == '-' || c == '+' || c.is_ascii_digit() {
            indicator.push(c);
            ctx.chars.next();
        } else {
            break;
        }
    }
    let block_style = Style::default().fg(ctx.theme.code_macro);
    ctx.spans.push(Span::styled(indicator, block_style));
    true
}

/// 处理 YAML 类型标签 `!!str`, `!!int`, `!!bool` 等
fn handle_yaml_tag(ch: char, ctx: &mut ParseContext<'_, '_>) -> bool {
    if !matches!(ctx.lang, "yaml" | "yml") || ch != '!' {
        return false;
    }
    let rest: String = ctx.chars.clone().collect();
    if !rest.starts_with("!!") {
        return false;
    }
    flush_buf(ctx);
    let mut tag = String::new();
    tag.push('!');
    ctx.chars.next();
    tag.push('!');
    ctx.chars.next();
    while let Some(&c) = ctx.chars.peek() {
        if c.is_alphanumeric() {
            tag.push(c);
            ctx.chars.next();
        } else {
            break;
        }
    }
    let tag_style = Style::default().fg(ctx.theme.code_comment).add_modifier(Modifier::ITALIC);
    ctx.spans.push(Span::styled(tag, tag_style));
    true
}
```

### Step 3: 调整主解析循环顺序（`highlight_code_line`）

在 `highlight_code_line` 的主循环中，按优先级插入 YAML 处理：

```rust
while let Some(&ch) = ctx.chars.peek() {
    // YAML 专用处理（优先级最高）
    if handle_yaml_document_marker(ch, &mut ctx) {
        continue;
    }
    if handle_yaml_tag(ch, &mut ctx) {
        continue;
    }
    if handle_yaml_anchor(ch, &mut ctx) {
        continue;
    }
    if handle_yaml_block_scalar(ch, &mut ctx) {
        continue;
    }
    if handle_yaml_key(ch, &mut ctx) {
        continue;
    }
    // 通用处理
    if handle_double_quote(ch, &mut ctx) {
        continue;
    }
    // ... 其他现有处理
}
```

### Step 4: 处理 YAML 列表指示符 `-`

列表指示符 `-` 需要在行首检测。在 `highlight_code_line` 开头添加：

```rust
// YAML 列表行检测：行首 `- ` 或 `-`
if matches!(lang_str, "yaml" | "yml") && trimmed.starts_with("- ") {
    let mut result = Vec::new();
    // 计算缩进
    let indent_len = line.len() - trimmed.len();
    result.push(Span::styled(
        " ".repeat(indent_len),
        ctx.styles.default_style,
    ));
    result.push(Span::styled("-", ctx.styles.kw_style));
    result.push(Span::styled(" ", ctx.styles.default_style));
    // 处理剩余内容（列表项值）
    let rest = &trimmed[2..];
    // ... 递归或继续解析
}
```

### Step 5: 测试验证

创建测试 YAML 文件验证高亮效果：
```yaml
---
name: example
version: "1.0"
enabled: true
items:
  - &anchor first
  - *anchor
  - <<: *anchor
description: |
  This is a literal
  block scalar
summary: >
  Folded block
  scalar
!!str custom: value
...
```

## 改动文件清单

| 文件 | 改动类型 | 说明 |
|------|---------|------|
| `src/markdown/highlight/dicts.rs` | 扩展 | 扩展 YAML_KEYWORDS |
| `src/markdown/highlight.rs` | 新增函数 | 添加 5 个 YAML 专用处理函数 |
| `src/markdown/highlight.rs` | 修改 | 调整主解析循环 |

## 预期效果

改进后 YAML 文件将呈现：
- 键名高亮为青/黄色（`code_type`）
- 文档分隔符 `---` / `...` 高亮为橙/红色（`code_keyword`）
- 列表指示符 `-` 高亮为橙/红色（`code_keyword`）
- 锚点 `&` / 别名 `*` / 合并 `<<` 高亮为紫色（`code_attribute`）
- 块标量指示符 `|` / `>` 高亮为紫色（`code_macro`）
- 类型标签 `!!xxx` 高亮为灰色斜体（`code_comment`）

## 风险与回退

- **解析冲突**：`handle_yaml_key` 可能误判普通标识符。通过严格检查冒号后上下文降低风险。
- **性能**：新增前瞻检查可能略微增加解析时间。YAML 文件通常不大，影响有限。
- **回退策略**：每个 YAML 处理函数返回 `false` 时自动进入通用处理，保持原有行为。