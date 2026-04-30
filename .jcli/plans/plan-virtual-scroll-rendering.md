# 虚拟滚动渲染优化方案

## 问题分析

### 当前架构存在的性能瓶颈

当消息数量达到 1500 条时，打字/按键不灵敏（吞输入、吞按键），主要原因是：

1. **每帧遍历所有消息**：即使缓存命中，`build_message_lines_incremental` 仍需遍历 1500 条消息构建 `msg_start_lines` 和检查 `per_msg_cache`（O(n)）

2. **缓存失效条件过于激进**：`is_loading=true` 时历史消息缓存也会失效（见 `chat.rs` 第 611-624 行），导致每帧都要完整重建

3. **渲染定位效率低**：`get_line_at` 虽然只渲染可见行，但定位每行时仍需遍历 `per_msg_lines` 计算偏移（线性搜索）

4. **主循环阻塞**：渲染帧 + 处理输入串行执行，当渲染耗时超过 33ms，输入事件堆积

### 关键代码路径

```
tui_loop.rs:
  run_tui_loop() →
    terminal.draw() → draw_chat_ui() → draw_messages() →
      build_message_lines_incremental() ← 遍历 1500 条消息
      render_text_pass() ← 定位每行时遍历 per_msg_lines
    处理输入事件（批量消费，但可能已堆积）
```

---

## 优化方案

### 核心思路

**只处理可见区域的消息**：用户只能看到约 20-30 条消息，不需要遍历/渲染全部 1500 条。

### 实施步骤

#### Step 1: 新增消息范围定位函数

**目标**：根据 `scroll_offset` 和 `visible_height`，快速定位可见消息的索引范围 `[first_visible_idx, last_visible_idx]`

**改动文件**：`src/command/chat/ui/chat.rs`

**新增函数**：
```rust
/// 根据 scroll_offset 和 visible_height，定位可见消息索引范围
/// 返回 (first_visible_idx, last_visible_idx, first_msg_local_start, last_msg_local_end)
fn find_visible_msg_range(
    cached: &MsgLinesCache,
    scroll_offset: usize,
    visible_height: usize,
) -> (usize, usize, usize, usize) {
    // 使用二分查找快速定位第一条可见消息
    let first_idx = find_msg_at_line(cached, scroll_offset);
    
    // 计算最后可见行号
    let last_visible_line = scroll_offset + visible_height;
    
    // 使用二分查找定位最后一条可见消息
    let last_idx = find_msg_at_line(cached, last_visible_line);
    
    // 计算首尾消息内的局部行偏移
    let first_msg_start = cached.msg_start_lines[first_idx].1;
    let first_local_start = scroll_offset - first_msg_start;
    
    let last_msg_start = cached.msg_start_lines[last_idx].1;
    let last_local_end = (last_visible_line - last_msg_start)
        .min(cached.per_msg_lines[last_idx].lines.len());
    
    (first_idx, last_idx, first_local_start, last_local_end)
}

/// 二分查找：给定全局行号，返回所属消息索引
fn find_msg_at_line(cached: &MsgLinesCache, line: usize) -> usize {
    let msg_starts = &cached.msg_start_lines;
    let mut lo = 0usize;
    let mut hi = msg_starts.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let (_, start) = msg_starts[mid];
        if start <= line {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo.saturating_sub(1).min(msg_starts.len().saturating_sub(1))
}
```

#### Step 2: 优化 `build_message_lines_incremental` 的缓存命中逻辑

**目标**：历史消息缓存应完全复用，即使 `is_loading=true` 也只重渲染流式部分

**改动文件**：`src/command/chat/render/cache.rs`

**核心改动**：
```rust
// 改进缓存复用逻辑：
// 1. 历史消息（per_msg_lines）只在消息数量或内容变化时重建
// 2. 流式内容（streaming_lines）每次都重渲染，但复用 stable_lines
// 3. msg_start_lines 只在消息数量变化时重建

pub fn build_message_lines_incremental(
    app: &ChatApp,
    inner_width: usize,
    bubble_max_width: usize,
    old_cache: Option<&MsgLinesCache>,
) -> (
    Vec<(usize, usize)>,
    Vec<PerMsgCache>,
    Vec<Line<'static>>,
    Arc<Vec<Line<'static>>,
    usize,
) {
    // === 分离历史缓存和流式缓存 ===
    
    // 1. 检查历史消息是否需要重建
    let history_cache_valid = old_cache
        .map(|c| {
            c.bubble_max_width == bubble_max_width
            && c.expand_tools == app.ui.expand_tools
            && c.msg_count == display_msgs.len()
            && c.per_msg_lines.len() == display_msgs.len()
            && // 检查每条消息内容长度是否一致
            c.per_msg_lines.iter().all(|p| {
                display_msgs.get(p.msg_index)
                    .map(|m| m.content.len() == p.content_len)
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    
    // 2. 历史缓存有效时直接复用
    let (msg_start_lines, per_msg_cache) = if history_cache_valid {
        // 直接 clone 旧缓存（避免遍历 1500 条消息）
        let old = old_cache.unwrap();
        (old.msg_start_lines.clone(), old.per_msg_lines.clone())
    } else {
        // 需要重建：只遍历变化部分
        build_history_cache_incremental(...)
    };
    
    // 3. 流式内容每次重渲染（复用 stable_lines）
    let streaming_lines = build_streaming_lines(app, bubble_max_width, old_cache);
    
    // ...
}
```

#### Step 3: 新增增量历史缓存构建函数

**目标**：当历史缓存部分失效时，只重建变化的消息，而非遍历全部

**改动文件**：`src/command/chat/render/cache.rs`

**新增函数**：
```rust
/// 增量构建历史消息缓存（只处理变化部分）
fn build_history_cache_incremental(
    display_msgs: &[DisplayMessage],
    old_cache: Option<&MsgLinesCache>,
    bubble_max_width: usize,
    inner_width: usize,
    theme: &Theme,
    expand: bool,
    browse_mode: bool,
    browse_idx: usize,
) -> (Vec<(usize, usize)>, Vec<PerMsgCache>) {
    let msg_count = display_msgs.len();
    let mut msg_start_lines = Vec::with_capacity(msg_count);
    let mut per_msg_cache = Vec::with_capacity(msg_count);
    let mut current_line_offset: usize = 0;
    
    // 尝试复用旧缓存
    let can_reuse = old_cache
        .map(|c| c.bubble_max_width == bubble_max_width && c.expand_tools == expand)
        .unwrap_or(false);
    
    for (idx, m) in display_msgs.iter().enumerate() {
        msg_start_lines.push((idx, current_line_offset));
        
        // 尝试复用单条消息缓存
        if can_reuse
            && let Some(old_per) = old_cache.and_then(|c| c.per_msg_lines.get(idx))
            && old_per.msg_index == idx
            && old_per.content_len == m.content.len()
        {
            // 直接复用
            current_line_offset += old_per.lines.len();
            per_msg_cache.push(old_per.clone());
            continue;
        }
        
        // 重新渲染此消息
        let tmp_lines = render_single_message(m, idx, ...);
        current_line_offset += tmp_lines.len();
        per_msg_cache.push(PerMsgCache {
            content_len: m.content.len(),
            lines: tmp_lines,
            msg_index: idx,
            is_selected: browse_mode && idx == browse_idx,
        });
    }
    
    (msg_start_lines, per_msg_cache)
}
```

#### Step 4: 优化 `render_text_pass` 的行定位逻辑

**目标**：使用消息范围预计算，避免对每行进行线性搜索

**改动文件**：`src/command/chat/ui/chat.rs`

**改动函数**：`render_text_pass`

**核心改动**：
```rust
fn render_text_pass(
    f: &mut ratatui::Frame,
    params: &TextPassParams,
    selection: Option<&MouseSelection>,
) -> Vec<(usize, u16, String)> {
    let mut img_markers: Vec<(usize, u16, String)> = Vec::new();
    
    // ★ 预计算可见消息范围（一次二分查找）
    let (first_idx, last_idx, first_local_start, last_local_end) = 
        find_visible_msg_range(params.cached, params.start, params.end - params.start);
    
    // ★ 只遍历可见范围内的消息
    for msg_idx in first_idx..=last_idx {
        let msg_cache = &params.cached.per_msg_lines[msg_idx];
        let msg_global_start = params.cached.msg_start_lines[msg_idx].1;
        
        // 计算此消息内的可见行范围
        let local_start = if msg_idx == first_idx { first_local_start } else { 0 };
        let local_end = if msg_idx == last_idx { last_local_end } else { msg_cache.lines.len() };
        
        for local_line in local_start..local_end {
            let line_idx = msg_global_start + local_line;
            let line = &msg_cache.lines[local_line];
            
            // 渲染此行...
            let screen_y = params.inner.y + (line_idx - params.start) as u16;
            // ...
        }
    }
    
    // 流式内容单独处理...
    
    img_markers
}
```

#### Step 5: 改进缓存失效触发点

**目标**：减少不必要的 `msg_lines_cache = None`

**改动文件**：多处 handler 文件

**核心原则**：
- 消息数量变化 → 历史缓存失效
- 消息内容变化 → 仅该条消息缓存失效
- 流式内容变化 → 仅流式部分重建
- 窗口大小变化 → 气泡宽度变化 → 全部重建
- 展开/折叠变化 → 仅工具调用相关消息重建

**具体改动**：
- `stream_poll.rs`: 消息追加时不应直接清空缓存，应标记需要增量更新
- `tool_confirm.rs`: 确认状态变化时只重建交互区
- `browse.rs`: 选中消息变化时只更新 `is_selected` 标记

---

## 实施优先级

| 优先级 | 步骤 | 预期效果 | 改动量 |
|--------|------|----------|--------|
| P0 | Step 2: 缓存命中优化 | 历史消息遍历 O(1500) → O(1) | 中 |
| P1 | Step 4: render_text_pass 优化 | 行定位 O(n) → O(log n) | 中 |
| P2 | Step 3: 增量缓存构建 | 新消息时遍历 O(新增) 而非 O(全部) | 大 |
| P3 | Step 5: 失效触发优化 | 减少不必要的重建 | 大 |

---

## 预期效果

- 渲染时间：从 100-200ms 降低到 5-10ms
- 输入响应：无吞字/吞键现象
- 内存占用：基本不变（缓存已存在）

---

## 风险与边界情况

1. **浏览模式选中状态变化**：需要检测并更新 `is_selected`
2. **工具展开/折叠**：影响工具调用消息的渲染行数
3. **窗口大小变化**：气泡宽度变化需全部重建
4. **鼠标选区**：需要精确的行号映射

---

## 测试验证

1. 功能测试：
   - 1500 条消息滚动正常
   - 浏览模式选中/复制正常
   - 流式输出显示正常
   - 工具确认区交互正常

2. 性能测试：
   - 测量单帧渲染时间
   - 测量输入响应延迟