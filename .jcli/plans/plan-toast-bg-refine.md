# Toast 背景色优化方案

## 问题
Toast 通知弹窗使用独立的、带明显色调的背景色（如暗绿色、暗红色），与整体界面的 `bg_primary` 基调不协调，显得突兀。

## 对比参考
补全弹窗（`@`、`/`、文件等）通过 `draw_popup_list` 使用 `t.bg_primary` 作为背景，仅靠边框和文字颜色传递语义信息，视觉上非常自然。

## 方案
**Toast 改为使用 `bg_primary` 作为背景色**，仅通过边框颜色和文字颜色区分成功/错误状态。

### 修改点

1. **`src/command/chat/ui/chat.rs` - `draw_toast` 函数**
   - 将 Toast 的背景色从 `toast_success_bg` / `toast_error_bg` 改为 `bg_primary`
   - 边框和文字颜色保持不变（仍用 `toast_success_border` / `toast_error_border` / `toast_success_text` / `toast_error_text`）

2. **`src/command/chat/theme.rs` - 所有主题**
   - 移除 `toast_success_bg` 和 `toast_error_bg` 两个字段（或保留但不再使用）
   - 由于这两个字段在 `archive.rs` 和 `render_cache.rs` 中也有引用，需要一并处理

3. **其他引用点**
   - `src/command/chat/ui/archive.rs` - 检查 `toast_error_border` 的使用
   - `src/command/chat/render_cache.rs` - 检查 `toast_error_border` 的使用
   - `src/command/chat/tools/classification.rs` - 检查 `toast_error_border` 的使用

### 最小改动方案（推荐）
仅修改 `draw_toast` 函数中的背景色逻辑，不删除主题字段（避免大范围改动）：

```rust
// draw_toast 中：将两处背景色改为 bg_primary
let clear = Block::default().style(Style::default().bg(t.bg_primary));
// ...
.style(Style::default().bg(t.bg_primary))
```

这样只改两行代码，Toast 瞬间变得自然，与弹窗风格统一。
