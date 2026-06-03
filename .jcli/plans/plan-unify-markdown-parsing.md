# 统一 Markdown 解析：前端删除 parser.ts，复用 Rust 端 pulldown_cmark

## 核心问题

前端 `parser.ts` 用 `@lezer/markdown` 自己解析 markdown，bug 一堆（链接/图片/加粗/代码等都不对）。
Rust 端已有完善的 `pulldown_cmark` 解析器（`src/markdown/parser.rs`），IR 类型全部带 `#[derive(Serialize)]`，
且 `/api/file` 已经返回 `RenderedDoc.payload = ParsedDocument JSON`。

## 方案

**前端不解析 markdown，全部走 Rust 后端解析。**

### 当前流程（有问题）：
```
打开文件 → /api/file → RenderedDoc { source, payload }
  → 前端忽略 payload
  → 前端自己用 parser.ts (Lezer) 解析 source → Block[] → 渲染 DOM
编辑时 → 前端自己解析 → bug 一堆
```

### 新流程：
```
打开文件 → /api/file → RenderedDoc { source, payload }
  → 前端直接用 payload (ParsedDocument JSON) → 渲染 DOM
编辑时 → source 变化 → POST /api/parse { source } → ParsedDocument JSON → 增量更新 DOM
```

## 改动清单

### 1. Rust 端：新增 `/api/parse` 接口

文件：`src/command/read/server.rs`

```rust
#[derive(Deserialize)]
struct ParseReq { source: String }

async fn api_parse(Json(req): Json<ParseReq>) -> Result<Json<serde_json::Value>, ApiError> {
    tokio::task::spawn_blocking(move || {
        let doc = crate::markdown::parser::parse_markdown(&req.source, 120);
        serde_json::to_value(&doc)
            .map_err(|e| ApiError::internal(format!("解析失败：{e}")))
    }).await...
}
```

路由注册：`.route("/api/parse", post(api_parse))`

### 2. 前端：删除 `parser.ts`

整个文件删除。前端不再解析 markdown。

### 3. 前端：修改 `MarkdownEditor.tsx`

- 删除 `import { parseMarkdown, parseInline } from './parser'`
- 初始化时从 props 接收 `ParsedDocument`（而不是 source 自己解析）
- 编辑后调用 `/api/parse` 获取新 IR
- Debounce `/api/parse` 调用（300ms）

### 4. 前端：修改 `Reader.tsx`

- 从 `/api/file` 的 `payload` 字段直接提取 `ParsedDocument`
- 传给 `MarkdownEditor` 组件

### 5. 前端：删除 `@lezer/markdown` 依赖

`package.json` 移除 `@lezer/markdown`、`@lezer/common`、`@lezer/highlight`。

### 6. 保留的文件

- `inline-renderer.ts` — 将 `Inline[]` 渲染为 DOM 节点（纯渲染，不涉及解析）
- `cache.ts` — DOM 缓存（纯缓存，不涉及解析）
- `editor.css` — 编辑器样式
- `MarkdownEditor.tsx` — 编辑器组件（改为消费后端 IR）

### 7. 删除的文件

- `parser.ts` — 前端 Lezer 解析器（bug 源头）

## 接口变更

### MarkdownEditor Props

```typescript
// Before:
{ initialSource, onChange, onParsed, onSave, path, baseDir }

// After:
{ initialDoc: ParsedDocument, initialSource, onChange, onParsed, onSave, path, baseDir }
```

新增 `initialDoc`：来自 `/api/file` 的 `payload`，避免重复解析。

## 编辑时的增量解析

用户编辑 → `syncFromDom()` → 提取 source → debounce 300ms → `POST /api/parse` → 新的 `ParsedDocument` → 增量更新 DOM

缓存：相同的 source 不重复请求（基于 source hash）。
