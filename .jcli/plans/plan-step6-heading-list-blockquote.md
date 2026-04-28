# Step 6: 迁 heading / list / blockquote

## 分析结论

**不需要迁移**。

### 理由

1. **内联语法已走共享层**：`render_inline()` 使用 `parse_inline_text()` + `render_inlines()` 走共享层，heading/list/blockquote 的内联元素（bold/code/link 等）已正确渲染

2. **共享层 block 渲染不适合 editor**：
   - Heading H1/H2：共享层输出内容行 + 分隔线，editor 只需内容行（更紧凑）
   - BlockQuote：共享层输出前后空行 + `| ` prefix，editor 无空行 + `▎` prefix（更紧凑）
   - List：共享层渲染整个 List block，editor 逐行渲染单 item（更灵活）

3. **强制迁移会破坏 editor 的紧凑风格**

### 现状验证

Editor 渲染路径：
```
render_single_line_with_number()
  ├── heading H1/H2/H3/H4 → render_inline() → 共享层 render_inlines()
  ├── list (-, *, 1.) → render_inline() → 共享层 render_inlines()
  ├── blockquote (>) → render_inline() → 共享层 render_inlines()
  ├── task list (- [ ], - [x]) → render_inline() → 共享层 render_inlines()
  ├── rule (---) → 独立渲染
  ├── table → parse_table_from_source() + 共享层 render_table() ✓ Step 5
  └── code_block → 共享层 render_code_block() ✓ Step 3
```

### 最终状态

**Editor markdown 渲染已全部迁移共享层**：

| Block 类型 | 迁移状态 | 共享层 API |
|-----------|---------|-----------|
| Paragraph | ✓ | `render_inlines()` |
| Heading | ✓ | `render_inlines()` |
| List | ✓ | `render_inlines()` |
| BlockQuote | ✓ | `render_inlines()` |
| Task List | ✓ | `render_inlines()` |
| Rule | ✓ | 无需迁移（简单样式一致）|
| Table | ✓ Step 5 | `parse_table_from_source()` + `render_table()` |
| CodeBlock | ✓ Step 3 | `render_code_block()` |
| Inline (bold/code/link) | ✓ | `parse_inline_text()` + `render_inlines()` |

## 下一步

**Step 7：清理废弃代码**
- 移除 `markdown_cache.rs` 中的 `#[allow(dead_code)]`
- 确认 `markdown_cache.rs` 是否需要保留（目前未使用）
- 清理其他未使用的代码

## 备注

`markdown_cache.rs` 设计用于全文缓存 + 按需渲染，但当前 editor 采用逐行渲染模式。
如果未来需要 editor 预览模式或全文渲染优化，可以启用该缓存。
