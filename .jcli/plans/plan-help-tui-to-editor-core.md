# UI 组件公共化方案报告

## 一、现状评估

| 文件                    | 定位                           | 依赖 Theme          | 主要内容                                                  |
| ----------------------- | ------------------------------ | ------------------- | --------------------------------------------------------- |
| `chat/ui/components.rs` | **事实上的组件库**（已模块化） | ✅ 完整             | 24+ 原子组件：pointer/label/toggle/tab_bar/welcome_box 等 |
| `help/ui.rs`            | help TUI 渲染                  | ✅ 完整             | 顶层 draw 函数 + 带滚动 tab_bar + 标题栏 + 内容区         |
| `todo/ui.rs`            | todo TUI 渲染                  | ⚠️ 部分硬编码 Color | draw_ui + 编辑光标折行 + 命令面板 + 状态栏输入框          |

**核心发现**：chat 模块已经做对了一半（components.rs 就是组件库雏形），但只在 chat 内部消费；help 与 todo 各自重复造了轮子。

---

## 二、重复 / 可复用点清单

**A. 硬常量重复**
- INDENT（`"  "`/`"   "`）、POINTER（`" ❯ "`）、checkbox、分隔符在三处均有出现。

**B. 同名/近似函数**
- `pointer_span`：chat 与 todo 各有一版（chat 用 theme、todo 硬编码 Yellow）。
- 行内光标：chat 的 `cursor_spans`（单行）vs todo 的 `build_cursor_wrapped_lines`（支持折行+占位符）。功能呈超集关系，可合并。
- tab 栏：chat 的 `tab_bar`（简单）vs help 的 `draw_tab_bar`（带 ◀▶ 滚动）。后者是前者超集。
- 帮助快捷键：chat 有 `help_key_row`，todo 用硬编码 `Line::from(vec![Span::styled(...)])`。
- 分隔线：chat 有 `separator_line`，help/todo 用 `"─".repeat(n)` 硬编码。

**C. 可直接迁移的 chat 组件（与 chat 领域无关）**
`separator_line`、`section_header`、`pointer_span`、`label_span`、`desc_span`、`help_key_row`、`hint_spans`、`tab_bar`、`toggle_row`、`selectable_row`、`toggle_list_item`、`cursor_spans`、`ItemList`。

**D. 应留在 chat 的（业务耦合）**
`welcome_box`（依赖 `palette::get_gradient`、`quotes::get_quote`）、`global_*_row`（Global tab 三列布局）、`secret_field_row`（API Key 遮罩）。

---

## 三、目标目录设计

项目已有 `src/tui/`（共享 editor widget）——建议复用并扩展，而不是新建 `src/ui/`：

```
src/tui/
├── mod.rs
├── editor/               # 已有
└── components/           # 新增
    ├── mod.rs            # pub use 重导出
    ├── consts.rs         # INDENT / POINTER_* / TOGGLE_* / SEPARATOR_V / LABEL_WIDTH
    ├── pointer.rs        # pointer_span
    ├── label.rs          # label_span / desc_span
    ├── separator.rs      # separator_line / section_header
    ├── tab_bar.rs        # 合并后的 scrollable tab_bar（兼容简单用法）
    ├── hint.rs           # hint_spans / help_key_row / hint_bar
    ├── cursor.rs         # 合并后的 cursor 渲染（单行/折行两入口）
    ├── list.rs           # ItemList / selectable_row / toggle_list_item
    ├── row.rs            # toggle_row / text_field_row（通用版本）
    └── input_bar.rs      # render_input_status_bar（todo 抽过来）
```

chat 专属组件保留在 `chat/ui/components.rs`，改为薄封装，调用 `tui::components`。

---

## 四、关键挑战

**① Theme 位置耦合**
Theme 当前在 `src/command/chat/render/theme.rs`。组件上移到 `src/tui/` 后出现反向依赖（tui → command/chat）。

**建议**：把 Theme 上移到 `src/tui/theme.rs` 或 `src/theme.rs`，chat 通过 `pub use` 兼容。这是本次重构的**前置条件**。

**② todo 的颜色硬编码**
todo 目前大量使用 `Color::Yellow/Cyan/Green/Red`。迁移时必须同时接入 Theme，否则组件 API 不一致。

**建议**：为 todo 引入 theme 字段（它已在命令面板用了 `app.theme`，扩展面更小）。

**③ API 兼容性**
chat 的 `tab_bar` 接受 `&[(&str, bool)]`，help 的 `draw_tab_bar` 直接读 `HelpApp`。合并时需统一为"纯数据参数 + 可选滚动配置"。

---

## 五、分阶段执行顺序

**Phase 0｜前置（必须先做）**
1. 把 `Theme` 从 `command/chat/render/theme.rs` 上移到 `src/theme.rs`（chat 内部 re-export 保持兼容）。
2. todo 模块引入 Theme，替换硬编码 Color。

**Phase 1｜最小抽取（零行为变更）**
3. 新建 `src/tui/components/`，先迁入**纯函数+常量**（consts / pointer / label / separator / hint / help_key_row）。
4. chat/help/todo 切换 import 路径，删除本地重复实现（todo 的 pointer_span、help 硬编码 `─` 等）。

**Phase 2｜组件合并**
5. 合并 tab_bar：设计为 `tab_bar(items, active, scroll_opts, theme)`，help 和 chat 都用它。
6. 合并 cursor 渲染：提供 `cursor_inline`（单行）和 `cursor_wrapped`（折行+占位符）两个入口，共享底层实现。
7. 抽出 `input_bar`（todo 的 `render_input_status_bar`），未来 chat 的 config 编辑栏也可复用。

**Phase 3｜行/列表组件**
8. 迁入 `ItemList`、`toggle_row`、`selectable_row`、`toggle_list_item`。
9. chat 的 `global_*_row`、`secret_field_row` 保留原位，改为组合调用公共组件。

---

## 六、风险与建议

- **风险**：Theme 上移涉及大量 import 路径变更，建议单独一个 commit，工具：`cargo fmt && cargo check` 逐步验证。
- **风险**：todo 模块的视觉风格（边框+Title block）与 chat 不同，迁移时**不要顺手统一样式**，保持行为等价。
- **建议**：每个 Phase 单独 PR，Phase 0 不做则后续全阻塞。
- **建议**：保留 chat/ui/components.rs 作为 chat 专属 facade 文件，降低跨模块耦合感。
- **不建议**：一次性把 todo/help 的 `draw_ui` 顶层函数也合并——它们的布局差异过大，强行抽象会得到参数爆炸的"万能 draw"。

---

**结论**：**适合抽取，且收益明确**，但必须先完成 Theme 解耦与 todo 的 Theme 接入两项前置工作，否则组件 API 会被迫在"接收 Theme"和"硬编码颜色"之间分裂。建议按 Phase 0→3 分 4 个 PR 推进。