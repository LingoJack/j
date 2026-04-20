# Tools Tab UI 重设计计划 — 按功能分列网格布局

## 问题分析

当前 Tools Tab 使用简单垂直列表（每个工具独占一行），列表过长，横向空间浪费。

## 设计方案：按功能分列，无分割线

将工具按 `ToolCategory` 分为多列（组），每列是一组功能相关的工具，紧凑排列：
- 无分割线，无分组标题
- 左右键切换列（组）
- 上下键在同列内移动
- 某列到底后自动跳到下一列头部

### 视觉效果

```
  总开关: ● 开启 (18/21)                              (t 切换)

  ❯ ● Read        ● Bash       ● WebFetch    ● EnterPlanMode   ● Agent     ● Compact   ● Ask
    ● Write       ● Task       ● WebSearch   ● ExitPlanMode                 ● TodoWrite
    ● Edit        ● TaskOutput ● Browser                                    ● TodoRead
    ● Glob                                                                  ● RegisterHook
```

**选中项** `❯ ● Read` 加粗高亮。按右键跳到 `● Bash` 列，按左键回到 `● Read` 列。

### 实际渲染：分组纵向排列

考虑到终端宽度限制（通常 80-120 列），每列需要 16-18 字符，7 列太挤。
调整为 **多行多列网格**，每个工具项独立占一行，但按列优先（column-major）布局：

```
  总开关: ● 开启 (18/21)                              (t 切换)

  列1          列2          列3          列4
  ❯ ● Read     ● Bash       ● WebFetch   ● EnterPlanMode
    ● Write     ● Task       ● WebSearch  ● ExitPlanMode
    ● Edit      ● TaskOutput ○ Browser    ● Agent
    ● Glob
```

**简化方案**：根据终端宽度自动计算 `cols_per_row`（一行放几列），列内纵向排列工具：

```
  总开关: ● 开启 (18/21)                              (t 切换)

  ❯ ● Read      ● Bash       ● WebFetch   ● EnterPlanMode   ● Agent      ● Compact    ● Ask
    ● Write      ● Task       ● WebSearch  ● ExitPlanMode                              ● TodoWrite
    ● Edit       ● TaskOutput ○ Browser                                                ● TodoRead
    ● Glob                                                                           ● RegisterHook
```

导航逻辑：
- **上/下**：在同列内移动（如 Read → Write → Edit → Glob）
- **左/右**：在列间跳转（如 Read 列 → Bash 列）
- **到底再下**：跳到下一列头部

## 核心实现思路

### 1. 数据结构：二维网格

将工具按 `ToolCategory` 分组，每组形成一"列"：

```
columns: [
  [Read, Write, Edit, Glob],          // 列0: 文件
  [Bash, Task, TaskOutput],           // 列1: 执行
  [WebFetch, WebSearch, Browser],     // 列2: 网络
  [EnterPlanMode, ExitPlanMode],      // 列3: 计划
  [Agent],                            // 列4: 代理
  [Compact],                          // 列5: 压缩
  [Ask, TodoWrite, TodoRead, RegisterHook, ...], // 列6: 其他
]
```

### 2. 索引映射

在 `ChatApp.ui` 中新增两个字段追踪二维位置：
```rust
pub tool_col_idx: usize,   // 当前列索引
pub tool_row_idx: usize,   // 当前列内行索引
```

同时维护一个 `config_field_idx` 的兼容映射函数，确保 `ToggleMenuToggle` 等操作通过 `tool_col_idx + tool_row_idx` 找到正确的工具名。

### 3. 渲染逻辑

所有列并排显示在同一个可视区域内。每列固定宽度（如 18 字符）。
渲染时按行遍历（每行包含各列的同位工具）：

```
行0: [列0的Read]  [列1的Bash]  [列2的WebFetch]  ...
行1: [列0的Write] [列1的Task]  [列2的WebSearch] ...
行2: [列0的Edit]  [列1的TaskOutput] [列2的Browser] ...
行3: [列0的Glob]
```

如果一行内放不下所有列，分成多"大行"：
- 终端宽度 80 → 每大行放 4 列
- 终端宽度 120 → 每大行放 6 列

### 4. 导航逻辑修改

修改 `handler/config.rs` 中 Tools tab 的按键处理：

```rust
ConfigTab::Tools => match key.code {
    KeyCode::Up => {
        // 当前列内上移，已在顶部则不变
        if app.ui.tool_row_idx > 0 {
            app.ui.tool_row_idx -= 1;
        }
    }
    KeyCode::Down => {
        // 当前列内下移，到底则跳到下一列头部
        let col_count = columns[app.ui.tool_col_idx].len();
        if app.ui.tool_row_idx + 1 < col_count {
            app.ui.tool_row_idx += 1;
        } else {
            // 跳到下一列头部
            app.ui.tool_col_idx = (app.ui.tool_col_idx + 1) % columns.len();
            app.ui.tool_row_idx = 0;
        }
    }
    KeyCode::Left => {
        if app.ui.tool_col_idx > 0 {
            app.ui.tool_col_idx -= 1;
            // 限制行索引不超过新列的长度
            app.ui.tool_row_idx = app.ui.tool_row_idx.min(columns[app.ui.tool_col_idx].len() - 1);
        }
    }
    KeyCode::Right => {
        app.ui.tool_col_idx = (app.ui.tool_col_idx + 1) % columns.len();
        app.ui.tool_row_idx = app.ui.tool_row_idx.min(columns[app.ui.tool_col_idx].len() - 1);
    }
    // ...
}
```

### 5. Toggle 操作兼容

`ToggleMenuToggle` 当前通过 `config_field_idx` 查找工具名。
新增辅助函数从二维索引映射到工具名：

```rust
fn current_tool_name(app: &ChatApp) -> Option<String> {
    let groups = group_tools_by_category(&app.tool_registry.tool_names());
    groups.get(app.ui.tool_col_idx)
        .and_then(|(_, tools)| tools.get(app.ui.tool_row_idx))
        .map(|s| s.to_string())
}
```

修改 `ToggleMenuToggle` 和 `ToggleMenuNavigate` 的 Tools 分支使用此函数，
或者维护一个从 `(col, row)` → `config_field_idx` 的映射。

**更简方案**：维护一个 `flat_index_from_grid(col, row)` 函数，
遍历分组累加偏移，返回线性索引。这样 `ToggleMenuToggle` 无需修改。

```rust
fn flat_index_from_grid(groups: &[(ToolCategory, Vec<&str>)], col: usize, row: usize) -> usize {
    groups[..col].iter().map(|(_, t)| t.len()).sum::<usize>() + row
}
```

## 需要修改的文件

| 文件 | 修改内容 |
|------|---------|
| `src/command/chat/ui/config/tools.rs` | 重写 `draw_tab_tools_list` 为多列网格渲染 |
| `src/command/chat/ui/config/mod.rs` | 传递 `width` 参数给 tools 绘制函数 |
| `src/command/chat/app/chat_app.rs` | 在 `AppUi` 中新增 `tool_col_idx`/`tool_row_idx` 字段；修改 `ToggleMenuNavigate` 的 Tools 分支支持二维导航；修改 `ToggleMenuToggle`/`EnableAll`/`DisableAll` 使用二维索引 |
| `src/command/chat/handler/config.rs` | 修改 Tools tab 的 Up/Down/Left/Right 按键处理 |

**可选修改**：
| `src/command/chat/tools/classification.rs` | 可添加 `display_name()` 用于 tooltip |

## 实施步骤

1. [ ] 在 `AppUi` 中新增 `tool_col_idx: usize` 和 `tool_row_idx: usize` 字段
2. [ ] 在 `tools.rs` 中新增 `group_tools_by_category` 和 `flat_index_from_grid` 函数
3. [ ] 重写 `draw_tab_tools_list` 为多列网格渲染（每列固定宽度，行优先遍历）
4. [ ] 修改 `handler/config.rs` 中 Tools tab 的按键处理：上下在列内移动，左右在列间移动
5. [ ] 修改 `chat_app.rs` 中 `ToggleMenuNavigate` 的 Tools 分支（或移除，由 handler 直接处理）
6. [ ] 修改 `ToggleMenuToggle`/`EnableAll`/`DisableAll` 使用二维索引映射到工具名
7. [ ] 调整 `mod.rs` 传递 `width` 参数给 tools 绘制函数
8. [ ] 编译验证 (`cargo build`)
9. [ ] 格式检查 (`cargo fmt && cargo clippy`)
