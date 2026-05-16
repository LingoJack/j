# 代码块折行宽度修复方案

## 问题分析

当前折行引擎 (`WrapEngine`) 使用全局统一的折行宽度，不考虑代码块边框占用的空间。

**具体问题**：
- `editor.rs` 中设置折行宽度为 `content_width - line_num_width`
- 代码块内部每行渲染时需要额外 4 个字符的边框：`│ `（左）+ ` │`（右）
- 折行时没有减去这 4 个字符，导致长行折行后超出代码块右边框

**示例**（终端宽度 80，行号 6 字符）：
- 普通行折行宽度：`80 - 6 - 2（边框）= 72`
- 代码块内容宽度：`72 - 4（代码块边框）= 68`
- 当前问题：代码块内也用 72 折行，导致续行超出右边框 `│`

## 解决方案

让 `WrapEngine` 在重建缓存时接收代码块边界信息，对代码块内的行使用减去边框宽度的折行宽度。

### 代码块边框宽度

代码块内容行的边框占用固定为 4 个字符：
- `│` (1) + 空格 (1) = 左边框 2 字符
- 空格 (1) + `│` (1) = 右边框 2 字符
- 总计：`CODE_BLOCK_BORDER_WIDTH = 4`

### 修改文件清单

#### 1. `wrap_engine.rs` - 核心修改

**新增**：
- 常量 `CODE_BLOCK_BORDER_WIDTH: usize = 4`
- 方法 `rebuild_cache_with_code_blocks(lines, code_block_ranges)`
- 方法 `wrap_line_with_width(line, width)` - 按指定宽度折行

**修改**：
- `rebuild_cache()` 调用新方法（默认无代码块）
- `build_range()` 支持代码块内的行使用减量宽度
- `wrap_line()` 内部调用 `wrap_line_with_width`

**数据结构**：
- 新增 `code_block_ranges: Vec<Option<usize>>` 字段，存储每行所属代码块的边框宽度减量
  - `None` 表示普通行
  - `Some(4)` 表示代码块内容行

#### 2. `editor.rs` - 传入代码块边界

**修改**：
- `rebuild_wrap_cache()` 时从 `renderer.code_block_cache` 获取代码块范围
- 调用 `wrap.rebuild_cache_with_code_blocks(lines, ranges)`

#### 3. `renderer.rs` - 续行渲染对齐

**检查**：续行渲染已正确处理（renderer.rs 第 263-297 行），使用 `wrap_width` 计算填充宽度。

**无需修改**：续行渲染的填充宽度计算已基于 `wrap_width` 正确减去边框。

## 实现步骤

1. **wrap_engine.rs**: 添加 `CODE_BLOCK_BORDER_WIDTH` 常量和 `code_block_ranges` 字段
2. **wrap_engine.rs**: 新增 `rebuild_cache_with_code_blocks()` 方法
3. **wrap_engine.rs**: 修改 `compute_visual_line_count()` 和 `wrap_line()` 支持按行自定义宽度
4. **wrap_engine.rs**: 修改 `build_range()` 使用行级宽度
5. **editor.rs**: 调用新方法传入代码块范围
6. **测试验证**: 编译通过 + 手动测试

## 风险评估

- **低风险**：修改集中在折行引擎，不影响渲染逻辑
- **向后兼容**：`rebuild_cache()` 保持原有行为
- **性能影响**：额外存储 `Vec<Option<usize>>`，约 O(n) 空间，可接受