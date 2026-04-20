# 抽象 List Component 方案

## 目标
将 config.rs 中各 tab 列表的手动 `lines.push(line) + lines.push(spacer)` 模式，抽象为一个 `ItemList` 组件，统一管理 item 之间的间距。

## 当前问题
- 每个 `draw_tab_*_list` 函数都在循环末尾手动 push 一行 spacer
- 间距逻辑散落在 9 处，无法统一调整
- 最后一个 item 后面也会多一个 spacer

## 设计

### `ItemList` 结构体（放在 components.rs 中）

```rust
/// 配置面板列表组件
///
/// 自动在 item 之间插入空行间距，最后一个 item 后不插入。
/// 同时维护 field_line_indices，用于滚动定位。
pub struct ItemList<'a> {
    lines: Vec<Line<'a>>,
    field_line_indices: Vec<usize>,
    theme: &'a Theme,
}

impl<'a> ItemList<'a> {
    pub fn new(theme: &'a Theme) -> Self { ... }

    /// 添加一个列表项，自动在非首个 item 前插入空行
    pub fn push(&mut self, line: Line<'a>) {
        if !self.lines.is_empty() {
            self.lines.push(Line::from(""));
        }
        self.field_line_indices.push(self.lines.len());
        self.lines.push(line);
    }

    /// 添加一行非 item 内容（如分组标题、分隔线），不触发间距逻辑
    pub fn push_raw(&mut self, line: Line<'a>) {
        self.lines.push(line);
    }

    /// 消费 self，返回 (lines, field_line_indices)
    pub fn into_parts(self) -> (Vec<Line<'a>>, Vec<usize>) { ... }
}
```

### 改动范围

1. **components.rs**: 
   - 删除 `compact_spacer_line` 函数
   - 新增 `ItemList` 结构体及实现

2. **config.rs**: 
   - 更新 import：移除 `compact_spacer_line`，新增 `ItemList`
   - 改造 9 个 `draw_tab_*_list` 函数，改为返回 `ItemList`
   - `draw_config_screen` 调用处相应适配

### 各函数改造方式

- **简单列表**（Tools/Skills/Commands/Archive/Session）：函数内创建 `ItemList`，for 循环中用 `push`，返回 list
- **Model tab**：同上
- **Global tab**：组内 item 用 `push`，组间分隔线用 `push_raw`
- **Teammates tab**：Teammate items 用 `push`，SubAgent 分组标题用 `push_raw`，SubAgent items 也用 `push`
- **compact_exempt 子列表**：也用 `ItemList` 的 `push`

### 调用方改造（draw_config_screen）

```rust
// 之前：
draw_tab_tools_list(&mut list_lines, &mut field_line_indices, app);

// 之后：
let list = draw_tab_tools_list(app);
let (item_lines, item_indices) = list.into_parts();
list_lines.extend(item_lines);
field_line_indices.extend(item_indices);
```
