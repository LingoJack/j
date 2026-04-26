# 拆分 chat.rs 方案

## 现状分析
- 文件：`src/command/chat/ui/chat.rs`，430 行
- 包含 3 个函数：
  - `draw_chat_ui` (80 行) — 主界面入口，调用各子模块
  - `get_line_at` (23 行) — 行定位辅助
  - `draw_messages` (302 行) — 消息区渲染，是最大的函数

`draw_messages` 内部可明显分为 4 个逻辑段：
1. 空消息欢迎界面 (L128-148)
2. 缓存检查与增量构建 (L150-224)
3. 文字渲染 pass (L271-314)
4. 图片渲染 pass (L316-429) — 约 113 行，最重

## 拆分方案

**从 `draw_messages` 中提取 2 个独立私有函数**：

### 1. `render_text_pass` — 文字渲染 pass
- 从 `draw_messages` 中提取 L271-314 的文字渲染循环
- 职责：遍历可见行，渲染文字行，同时收集图片标记
- 签名：`fn render_text_pass(f, inner, cached, start, end, history_total, msg_area_bg) -> Vec<(usize, u16, String)>`
- 返回收集到的 `img_markers`

### 2. `render_image_pass` — 图片渲染 pass
- 从 `draw_messages` 中提取 L316-429 的图片渲染循环
- 职责：处理图片加载状态（Ready/Failed/Loading/Pending），启动异步加载线程
- 签名：`fn render_image_pass(f, inner, cached, img_markers, start, visible_height, bubble_max_width, app)`

### 不变部分
- `draw_chat_ui` 保持不变（已经很简洁）
- `get_line_at` 保持不变（已是独立小函数）
- `draw_messages` 简化为调用 `render_text_pass` + `render_image_pass` 的编排函数

## 预期效果
- `draw_messages` 从 ~300 行缩减为 ~80 行（缓存逻辑 + 调度）
- `render_text_pass` ~40 行
- `render_image_pass` ~110 行
- 总行数基本不变，但每个函数职责清晰、可独立阅读
- 无需新建文件，所有函数仍在 `chat.rs` 内（规模适中）
