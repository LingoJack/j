# Compact Exempt Tools Toggle List 方案

## 目标
在 Global tab 的 "豁免压缩工具" 行上按 Enter，进入一个工具 toggle 列表，展示所有已知工具，每个工具可以 toggle 是否豁免 micro_compact 压缩。Esc 返回 Global tab。

## UI 交互
```
Global tab:
  ...
  上下文压缩          [开启]        ← Enter toggle
  保留最近轮数        10            ← Enter 编辑数字
  豁免压缩工具        (13/20)       ← Enter 进入子列表

  ────────────────────── 豁免压缩工具 (Enter 确认, Esc 返回) ────────────

  [✓] LoadSkill        ← 内置（不可取消）
  [✓] Task             ← 内置（不可取消）
  [✓] Read             ← 用户已添加
  [ ] Grep             ← 未豁免
  [ ] Glob             ← 未豁免
  ...
```

## 改动文件

### 1. `src/command/chat/app/ui_state.rs`
- UIState 新增字段:
  - `compact_exempt_sublist: bool` — 是否在豁免工具子列表中
  - `compact_exempt_idx: usize` — 子列表选中索引

### 2. `src/command/chat/agent/compact.rs`
- 已有 `BUILTIN_EXEMPT_TOOLS` 常量，无需改动

### 3. `src/command/chat/ui/config.rs`
- `draw_tab_global_content()`:
  - 当 `compact_exempt_sublist` 为 true 时，调用新函数 `draw_compact_exempt_sublist()` 渲染工具 toggle 列表
  - 当 `compact_exempt_sublist` 为 false 时，`compact_exempt_tools` 行的 value 显示 `(n/m)` 计数格式
- 新增 `draw_compact_exempt_sublist()`:
  - 标题行: "豁免压缩工具 (Enter 切换, Esc 返回)"
  - 列出 `tool_registry.tool_names()` 所有工具
  - 内置工具标记为 `[✓]`（灰色，不可取消）
  - 用户自定义豁免工具标记为 `[✓]`（可取消）
  - 未豁免工具标记为 `[ ]`（可添加）

### 4. `src/command/chat/app/chat_app.rs`
- `Action::ConfigEnter` 的 Global tab 分支:
  - 当 `compact_exempt_tools` 上按 Enter: 设置 `compact_exempt_sublist = true`，`compact_exempt_idx = 0`
  - 当在子列表中按 Enter: toggle 对应工具的豁免状态
    - 如果是内置工具 (`BUILTIN_EXEMPT_TOOLS`)，忽略不处理
    - 否则在 `micro_compact_exempt_tools` 中添加/移除
- `Action::ConfigEsc`:
  - 如果 `compact_exempt_sublist == true`，退出子列表返回 Global tab
- `Action::ConfigUp` / `Action::ConfigDown`:
  - 如果 `compact_exempt_sublist == true`，移动 `compact_exempt_idx`

### 5. `src/command/chat/render/helpers.rs`
- `config_field_value_global` 的 `compact_exempt_tools` 分支: 改为 `(n/m)` 格式显示已豁免数量/总数

## 实现细节

### 工具来源
- 所有工具名来自 `app.tool_registry.tool_names()`，与 Tools tab 共享同一个列表
- 内置豁免工具 (`BUILTIN_EXEMPT_TOOLS`) 始终勾选且不可取消
- 用户自定义豁免 (`micro_compact_exempt_tools`) 可以 toggle

### 数据存储
- `micro_compact_exempt_tools: Vec<String>` 保持不变
- 内置工具始终豁免，不需要存储
- 保存时仍然只存用户额外新增的部分
