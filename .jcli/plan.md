# Plan: editor_core 模块优化方案

## 概述

通过分析 `src/tui/editor_core/` 目录下的 8 个源文件，识别出以下主要优化空间，聚焦于**性能瓶颈**和**代码可维护性**。

---

## 一、性能优化 (高优先级)

### 1.1 History 模块 - 使用 VecDeque 替代 Vec

**现状问题:**
- `history.rs` 第 68-70 行使用 `self.stack.remove(0)` 移除最旧记录
- `Vec::remove(0)` 是 O(n) 操作，需要移动所有后续元素

**优化方案:**
```rust
use std::collections::VecDeque;

pub struct History {
    stack: VecDeque<Snapshot>,  // 替代 Vec<Snapshot>
    cursor: usize,
    max_size: usize,
}
```
- `push_front` / `pop_front` 均为 O(1)

**收益:** 队列头部删除从 O(n) 降为 O(1)

---

### 1.2 Renderer 模块 - 统一代码块检测逻辑

**现状问题:**
- `renderer.rs` 中存在 4 个功能重叠的方法重复遍历代码块:
  - `find_complete_code_block` (L496-519)
  - `is_line_in_complete_code_block` (L521-547)
  - `find_code_block_range` (L586-605)
  - `find_code_block_range_for_fence` (L608-639)
- 已有 `CodeBlockCache` 缓存结构，但部分方法仍回退到旧逻辑

**优化方案:**
1. 移除冗余方法，统一使用 `CodeBlockCache`
2. 删除旧的回退逻辑代码
3. 确保 `ensure_cache_valid` 在渲染前被调用

**收益:** 减少约 100 行重复代码，提升缓存命中率

---

### 1.3 WrapEngine 模块 - 统一折行计算逻辑

**现状问题:**
- `wrap_line` (L157-211) 和 `compute_visual_line_count` (L124-143) 有重复的字符遍历逻辑
- 两处代码各自维护宽度计算，存在不一致风险

**优化方案:**
```rust
impl WrapEngine {
    /// 统一折行计算，返回 (视觉行数, Vec<VisualLine>)
    fn compute_wrap(&self, line: &str, line_num: usize) -> (usize, Vec<VisualLine>) {
        // 单次遍历同时计算数量和内容
        if !self.enabled {
            let vl = VisualLine::from_line(line, line_num);
            return (1, vec![vl]);
        }
        // ... 统一逻辑
    }
}
```

**收益:** 保证逻辑一致性，减少重复代码

---

## 二、代码质量优化 (中优先级)

### 2.1 Renderer 模块拆分

**现状问题:**
- `renderer.rs` 文件超过 1400 行，职责过多

**优化方案:**
拆分为独立模块:
```
src/tui/editor_core/renderer/
├── mod.rs           # MarkdownRenderer 主结构和公共接口
├── code_block.rs    # 代码块检测和渲染
├── table.rs         # 表格解析和渲染
└── inline.rs        # 行内元素渲染 (粗体、斜体、链接等)
```

**收益:** 提高可维护性，各模块职责清晰

---

### 2.2 Vim 模块 - 提取命令处理

**现状问题:**
- `handle_normal_mode` 方法约 100 行，match 分支过多

**优化方案:**
提取光标移动命令到独立方法:
```rust
impl Vim {
    fn handle_normal_mode(&mut self, input: &Input, buffer: &mut TextBuffer) -> Transition {
        // 先处理特殊命令
        match input.key {
            Key::Char('i') => return Transition::Mode(Mode::Insert),
            Key::Char(':') => return Transition::Mode(Mode::Command(String::new())),
            // ...
            _ => {}
        }
        // 再处理光标移动
        self.handle_cursor_movement(input, buffer)
    }
    
    fn handle_cursor_movement(&mut self, input: &Input, buffer: &mut TextBuffer) -> Transition {
        // 光标移动相关逻辑
    }
}
```

**收益:** 提高可读性，便于后续添加新命令

---

### 2.3 Editor render 方法拆分

**现状问题:**
- `render` 方法 (L316-428) 过长，混合多种职责

**优化方案:**
```rust
impl MarkdownEditor {
    pub fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
        let ctx = self.prepare_render_context(area);
        self.render_content(f, area, &ctx);
        self.render_status_bar(f, area);
        self.render_command_bar(f, area);
    }
}
```

**收益:** 提高可读性和可测试性

---

## 三、实施计划

### Phase 1: 性能优化 (预计 0.5-1 天)
1. [ ] History: Vec -> VecDeque
2. [ ] Renderer: 移除冗余代码块检测方法
3. [ ] WrapEngine: 合并折行计算逻辑

### Phase 2: 代码重构 (预计 1-2 天)
1. [ ] Renderer 模块拆分
2. [ ] Vim 模块方法提取
3. [ ] Editor render 方法拆分

---

## 四、总结

| 模块 | 优化点 | 优先级 | 预计工作量 |
|------|--------|--------|-----------|
| history.rs | VecDeque 替换 | 高 | 0.5 天 |
| renderer.rs | 移除冗余方法 + 拆分 | 高 | 1.5 天 |
| wrap_engine.rs | 统一折行计算 | 中 | 0.5 天 |
| vim.rs | 方法提取 | 中 | 0.5 天 |
| editor.rs | render 拆分 | 中 | 0.5 天 |

**核心改动:** 约 200-300 行代码调整，无功能变化