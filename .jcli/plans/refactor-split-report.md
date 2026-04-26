# 大文件拆分报告

> 基于 `src/` 目录全量扫描，按文件行数降序排列，仅列出 >= 500 行的文件。
> 生成时间: 2025-07-14

---

## 1. `command/chat/render/cache.rs` — 2981 行

### 当前结构

| 名称 | 类型 | 行范围 | 职责 |
|---|---|---|---|
| `THINKING_FOLDED_MAX_LINES` 等 5 个常量 | const | L29-39 | 渲染参数 |
| `RenderContext` / `ContentContext` | struct | L44-57 | 渲染上下文 |
| `find_stable_boundary` | pub fn | L60-89 | 增量渲染边界查找 |
| `build_message_lines_incremental` | pub fn | L97-485 | 增量缓存引擎（390 行） |
| `wrap_md_line_in_bubble` / `_with_margin` | pub fn / fn | L488-571 | 气泡包装 |
| `render_user_msg` | pub fn | L574-658 | 用户消息气泡 |
| `parse_agent_prefix` / `agent_name_color` | fn | L664-693 | Agent 名称解析配色 |
| `render_thinking_block` | fn | L697-746 | 思考内容折叠 |
| `render_assistant_msg` | pub fn | L755-863 | AI 消息气泡 |
| `bordered_line` | fn | L870-923 | 边框行渲染 |
| `render_tool_confirm_area` | fn | L926-976 | 工具确认框入口 |
| `render_ask_questions` | fn | L979-1336 | Ask 问答渲染（358 行） |
| `render_tool_confirm_content` | fn | L1339-1574 | 工具确认内容（236 行） |
| `render_agent_perm_confirm_area` | fn | L1576-1667 | 子 Agent 权限确认 |
| `render_plan_approval_confirm_area` | fn | L1670-1779 | Plan 审批确认 |
| `render_tool_call_request_msg` | pub fn | L1781-2039 | 工具调用请求渲染 |
| `render_json_params_enhanced` | fn | L2042-2077 | JSON 参数渲染 |
| `render_tool_result_msg` | pub fn | L2083-2247 | 工具结果渲染入口 |
| `render_diff_content` | fn | L2250-2294 | Diff 着色 |
| `render_agent_result_nested` | fn | L2297-2354 | Agent 嵌套结果 |
| `render_bash_result` | fn | L2357-2406 | Bash 结果高亮 |
| `render_todo_result` | fn | L2410-2492 | Todo 结果样式 |
| `thinking_pulse_color` / `comet_gradient_line` | fn | L2525-2610 | 动画效果 |
| `copy_to_clipboard` | pub fn | L2612-2643 | 剪贴板操作 |
| `TeammateCallArgs` + `extract_*` + `render_*_expanded` | struct + fn | L2649-2935 | 各类工具展开渲染 |
| `render_exit_plan_mode_request` / `render_bash_call_request_expanded` | fn | L2899-2981 | Plan 模式/Bash 展开渲染 |

### 拆分方案

转为目录模块 `cache/`：

| 新文件 | 包含内容 | 预估行数 |
|---|---|---|
| `cache.rs` (入口 re-export) | pub mod 声明 + re-export | ~30 |
| `cache/incremental.rs` | `RenderContext`, `ContentContext`, `find_stable_boundary`, `build_message_lines_incremental` | ~430 |
| `cache/bubble.rs` | `wrap_md_line_in_bubble`, `_with_margin`, `bordered_line` | ~100 |
| `cache/msg_render.rs` | `render_user_msg`, `render_assistant_msg`, `render_thinking_block`, `parse_agent_prefix`, `agent_name_color` | ~250 |
| `cache/confirm_render.rs` | 所有确认区域渲染（`render_tool_confirm_area`, `render_ask_questions`, `render_tool_confirm_content`, `render_agent_perm_confirm_area`, `render_plan_approval_confirm_area`） | ~850 |
| `cache/tool_call_render.rs` | 工具调用请求渲染 + 各类展开渲染 + 参数提取结构体 | ~340 |
| `cache/tool_result_render.rs` | 工具结果渲染 + `render_diff_content` + `render_bash_result` + `render_todo_result` 等 | ~460 |
| `cache/animation.rs` | `current_tick`, `thinking_pulse_color`, `comet_gradient_line` | ~100 |
| `cache/clipboard.rs` | `copy_to_clipboard` | ~40 |

**优先级**: 高。确认区域渲染（850 行）和工具结果渲染（460 行）逻辑最独立，拆分风险最低。

---

## 2. `command/chat/infra/hook.rs` — 2952 行

### 当前结构

| 名称 | 类型 | 行范围 | 职责 |
|---|---|---|---|
| `HookEvent` + impl | enum | L50-138 | Hook 事件类型 |
| `OnError` | enum | L141-149 | 错误处理策略 |
| `HookFilter` + impl | struct | L155-204 | 过滤条件 |
| `HookType` + impl | enum | L207-224 | Hook 类型 |
| `HookDef` / `HookKind` / `ShellHook` / `LlmHook` / `BuiltinHook` | struct/enum | L247-390 | Hook 定义 |
| `HookDirDef` + 加载函数 | struct + fn | L399-559 | 从目录加载 |
| `BuiltinHookFn` + `BuiltinHook` | type + struct | L562-579 | 内置 Hook |
| `HookContext` + Default | struct | L614-691 | 执行上下文 |
| `HookAction` / `HookResult` / `HookOutcome` | enum/struct | L713-795 | 执行结果 |
| `HookMetrics` / `HookManager` / `HookEntry` | struct | L801-871 | 管理器 |
| impl `HookManager` | impl | L892-1480 | 管理器方法（680 行） |
| `execute_hook_with_provider` / `execute_llm_hook` / `execute_shell_hook` | fn | L1485-1805 | 执行引擎（320 行） |
| 辅助访问函数 x9 | fn | L1810-1911 | Hook 属性访问 |
| 测试 | mod tests | L1916-2952 | 36 个测试（1036 行） |

### 拆分方案

转为目录模块 `hook/`：

| 新文件 | 包含内容 | 预估行数 |
|---|---|---|
| `hook.rs` (入口 re-export) | pub mod + re-export | ~30 |
| `hook/types.rs` | `HookEvent`, `OnError`, `HookFilter`, `HookType`, `HookContext`, `HookAction`, `HookResult`, `HookOutcome` | ~430 |
| `hook/definition.rs` | `HookKind`, `ShellHook`, `LlmHook`, `BuiltinHook`, `HookDef`, `HookDirDef`, 加载函数 | ~290 |
| `hook/manager.rs` | `HookManager`, `HookMetrics`, `HookEntry`, `hook_unique_id` | ~700 |
| `hook/executor.rs` | 执行引擎 + 辅助访问函数 | ~430 |
| `hook/tests.rs` | 36 个测试 | ~1036 |

**优先级**: 高。类型定义（430 行）和执行引擎（430 行）完全无副作用，独立性强。

---

## 3. `command/chat/app/chat_app.rs` — 2361 行

### 当前结构

| 名称 | 类型 | 行范围 | 职责 |
|---|---|---|---|
| `ChatApp` | struct | L49-121 | 应用状态（73 字段） |
| `config_tab_field_count` | pub fn | L125-146 | 配置面板字段数 |
| `ChatApp::new()` | pub fn | L150-621 | 构造函数（470 行，含内置 hook 注册） |
| `ChatApp::update()` | pub fn | L635-2170 | Reducer（**1535 行巨型函数**） |
| 其余 15 个小方法 | pub fn | L2173-2361 | 工具方法 |

`update()` 内部按 Action 枚举分支组织：

| Action 分组 | 行范围 | 行数 | 说明 |
|---|---|---|---|
| Chat 输入/文本编辑 | L637-660 | ~25 | InsertChar, DeleteChar, ClearInput |
| 弹窗交互 | L662-733 | ~75 | At/File/Skill/Command 弹窗 |
| 流式生命周期 | L735-822 | ~90 | StreamChunk, StreamDone, StreamError |
| 工具执行 | L825-836 | ~12 | ExecutePendingTool, RejectPendingTool |
| Ask 工具交互 | L838-991 | ~155 | AskNavigate, AskSubmitAnswer 等 |
| 工具交互区 | L993-1047 | ~55 | ToolInteractNavigate 等 |
| 模式切换/浏览导航 | L1049-1203 | ~155 | EnterMode, BrowseNavigate 等 |
| **配置编辑** | L1205-1672 | **~470** | ConfigNavigate, ConfigEdit*, ConfigSwitch*, ToggleMenu* |
| 模型/主题选择 | L1674-1743 | ~70 | ModelSelect*, ThemeSelect* |
| **归档管理** | L1745-2037 | **~290** | Archive*, RestoreArchive |
| **会话管理** | L1793-1977 | **~185** | SwitchSession, NewSession, LoadSessionList |
| 流式控制 | L2047-2068 | ~22 | CancelStream, CancelToolsOnly |
| UI 管理 | L2070-2130 | ~60 | ShowToast, TickToast, SaveConfig |
| 应用控制 | L2132-2169 | ~40 | Quit, ToggleExpandTools |

### 拆分方案

转为目录模块 `chat_app/`：

| 新文件 | 包含内容 | 预估行数 |
|---|---|---|
| `chat_app.rs` (入口) | `ChatApp` 结构体 + `new()` + `config_tab_field_count` + 小方法 + re-export | ~700 |
| `chat_app/update_config.rs` | 配置编辑 Action 处理（提取为 `ChatApp` 的私有方法） | ~470 |
| `chat_app/update_session.rs` | 会话/归档管理 Action 处理 | ~300 |
| `chat_app/update_tool_interact.rs` | Ask/工具交互 Action 处理 | ~210 |
| `chat_app/update_misc.rs` | 其余 Action 处理（弹窗/流式/浏览/UI） | ~540 |

**优先级**: 高。`update()` 1535 行是全项目最大单函数，配置编辑分支（470 行）和会话管理分支（295 行）最值得独立。

---

## 4. `tui/editor_core/renderer.rs` — 1559 行

### 当前结构

| 名称 | 类型 | 行范围 | 职责 |
|---|---|---|---|
| `EditorRenderer` | struct | - | 渲染器状态 |
| impl `EditorRenderer` | impl | 全文件 | 渲染逻辑 |

具体方法：

- `new()` — 构造
- `render()` / `render_main_area()` — 主渲染入口
- `render_line_number_gutter()` — 行号栏
- `render_highlight_spans()` — 语法高亮
- `render_selection()` — 选区渲染
- `render_cursor()` — 光标
- `render_status_bar()` — 状态栏
- `render_command_bar()` — 命令栏
- `render_minimap()` — 小地图
- `render_search_popup()` / `render_command_popup()` — 弹窗
- `compute_visible_range()` — 可见区域计算
- `wrap_line()` — 折行
- `apply_syntax_highlight()` — 语法高亮

### 拆分方案

转为目录模块 `renderer/`：

| 新文件 | 包含内容 |
|---|---|
| `renderer.rs` (入口) | 结构体定义 + `new()` + `render()` 主入口 |
| `renderer/main_area.rs` | 主区域渲染（行号/高亮/选区/光标） |
| `renderer/overlays.rs` | 状态栏/命令栏/小地图/弹窗 |
| `renderer/syntax.rs` | 语法高亮 + 折行 |
| `renderer/visible_range.rs` | 可见区域计算 |

**优先级**: 中。

---

## 5. `command/chat/tools/browser.rs` — 1522 行

### 当前结构

Browser 工具实现，结构较单一：

- `BrowserTool` struct + impl
- DOM 查询/点击/输入/截图等方法
- `impl Tool for BrowserTool` trait 实现

### 拆分方案

转为目录模块 `browser/`：

| 新文件 | 包含内容 |
|---|---|
| `browser.rs` (入口) | struct 定义 + `Tool` trait 实现 |
| `browser/actions.rs` | DOM 操作方法（click/type/scroll/screenshot） |
| `browser/dom.rs` | DOM 查询和元素定位 |

**优先级**: 中。

---

## 6. `command/chat/markdown/parser.rs` — 1518 行

### 当前结构

| 名称 | 类型 | 行范围 | 行数 |
|---|---|---|---|
| `markdown_to_lines` | pub fn | L11-858 | **847 行巨型函数** |
| `cell_to_pieces` | fn | L862-886 | 表格单元格拆分 |
| `wrap_cell_styled` | fn | L890-942 | 表格单元格折行 |
| `display_width_cell` | fn | L945-965 | 单元格宽度 |
| `split_text_with_urls` | fn | L968-1026 | URL 高亮 |
| mod tests | 测试 | L1028-1518 | 490 行测试 |

### 拆分方案

| 新文件 | 包含内容 | 行数 |
|---|---|---|
| `parser.rs` (入口) | `markdown_to_lines` 主入口 + re-export | ~100 |
| `parser/parser_state.rs` | `ParserState` struct，各事件处理方法（heading/code_block/table/blockquote/text） | ~750 |
| `parser/table.rs` | `cell_to_pieces` + `wrap_cell_styled` + `display_width_cell` | ~100 |
| `parser/url.rs` | `split_text_with_urls` | ~60 |

**优先级**: 高。`markdown_to_lines` 847 行是全项目第二大单函数，各事件处理共享状态适合封装为 struct。

---

## 7. `command/notebook/app.rs` — 1440 行

### 当前结构

| 逻辑分组 | 行范围 | 行数 |
|---|---|---|
| 数据结构（`NoteItem`, `ExpandedDirs`, `FlatEntry`） | L20-132 | ~110 |
| 文件 IO（路径/加载/保存/编辑/剪贴板/Finder） | L134-378 | ~244 |
| `NotebookApp` struct + impl（new/reload/build_flat_entries） | L397-801 | ~400 |
| 按键处理（7 个 `handle_*_mode` 函数） | L806-1440 | **~634** |

### 拆分方案

| 新文件 | 包含内容 | 行数 |
|---|---|---|
| `app.rs` (入口) | struct 定义 + `NotebookApp::new()` + `reload()` + `build_flat_entries()` | ~500 |
| `app/handler.rs` | 7 个 `handle_*_mode` 按键处理函数 | ~634 |
| `app/storage.rs` | 文件 IO 操作 | ~244 |

**优先级**: 中。

---

## 8. `command/chat/oneshot.rs` — 1381 行

### 当前结构

| 逻辑分组 | 行范围 | 行数 |
|---|---|---|
| UI 辅助（终端宽度/颜色/动画/打印） | L36-260 | ~220 |
| 入口函数 `handle_chat` | L293-367 | ~75 |
| `run_oneshot_no_tools` | L370-444 | ~75 |
| 交互式确认 UI | L466-603 | ~140 |
| 流式重绘 | L447-633 | ~50 |
| `run_oneshot_agent` | L635-1263 | **~628 行巨型函数** |
| `handle_tool_call` + 辅助 | L1266-1381 | ~115 |

### 拆分方案

| 新文件 | 包含内容 | 行数 |
|---|---|---|
| `oneshot.rs` (入口) | 入口函数 + `run_oneshot_agent` + `run_oneshot_no_tools` | ~800 |
| `oneshot/ui.rs` | UI 辅助（颜色/动画/终端宽度/打印） | ~220 |
| `oneshot/interactive.rs` | `interactive_confirm` + Ask 弹窗处理 | ~380 |

**优先级**: 中。

---

## 9. `command/chat/agent/agent_loop.rs` — 1267 行

### 当前结构

| 名称 | 行范围 | 行数 |
|---|---|---|
| `push_compact_tool_messages` | L40-88 | ~50 |
| `StreamingToolCallPart` struct | L91-95 | ~5 |
| `run_main_agent_loop` | L98-1267 | **~1170 行巨型函数** |

`run_main_agent_loop` 内部逻辑块：

| 阶段 | 行范围 | 行数 |
|---|---|---|
| 初始化 | L106-144 | ~40 |
| Compact 子管线 | L199-336 | ~140 |
| 请求构建 | L354-485 | ~130 |
| 流式读取 | L554-674 | ~120 |
| 错误恢复 | L676-788 | ~110 |
| 非流式 fallback | L819-1026 | ~210 |
| 工具调用处理 | L1042-1161 | ~120 |
| Stop hook + 终止 | L1162-1256 | ~95 |

### 拆分方案

将 `run_main_agent_loop` 按阶段提取为私有函数，共享状态封装为 `AgentLoopContext` struct。

| 新文件 | 包含内容 | 行数 |
|---|---|---|
| `agent_loop.rs` (入口) | `run_main_agent_loop` 主循环 + 调度 | ~300 |
| `agent_loop/compact.rs` | Compact 子管线 | ~200 |
| `agent_loop/request.rs` | 请求构建 | ~200 |
| `agent_loop/stream.rs` | 流式读取 + 错误恢复 | ~300 |
| `agent_loop/fallback.rs` | 非流式 fallback | ~250 |

**优先级**: 高。1170 行单函数是全项目第三大函数，但 async 函数拆分需谨慎处理借用。

---

## 10. `command/report.rs` — 1246 行

### 当前结构

| 逻辑分组 | 行范围 | 行数 |
|---|---|---|
| 入口 + 日报写入 | L23-436 | ~410 |
| Git 操作（set-url/push/pull/ensure_git/sync_remote） | L678-1058 | **~380** |
| 检查与搜索 | L1063-1246 | ~183 |
| 文件 IO 工具 | 散布 | ~100 |

### 拆分方案

转为目录模块 `report/`：

| 新文件 | 包含内容 | 行数 |
|---|---|---|
| `report.rs` (入口) | `handle_report` + 日报写入 + 配置管理 | ~500 |
| `report/git.rs` | Git 操作 | ~380 |
| `report/search.rs` | 检查/搜索 | ~183 |

**优先级**: 中。Git 操作完全独立。

---

## 11. `command/chat/handler/chat.rs` — 1104 行

### 当前结构

| 逻辑分组 | 行范围 | 行数 |
|---|---|---|
| `handle_chat_mode` | L17-688 | **671 行** |
| `check_and_activate_mention_popup` | L691-787 | ~100 |
| 弹窗拦截（slash/at/file/skill/command） | 嵌入 handle_chat_mode | ~430 |
| `execute_slash_command` | L799-883 | ~85 |
| Dump 功能（4 个函数） | L893-1104 | ~210 |

### 拆分方案

| 新文件 | 包含内容 | 行数 |
|---|---|---|
| `chat.rs` (入口) | `handle_chat_mode`（提取弹窗逻辑后约 230 行） | ~230 |
| `handler/popup.rs` | 弹窗拦截逻辑 | ~430 |
| `handler/dump.rs` | Dump 功能 | ~210 |
| `handler/slash_command.rs` | `execute_slash_command` | ~85 |

**优先级**: 中高。弹窗拦截和 Dump 功能逻辑独立。

---

## 12. `tui/editor_core/editor.rs` — 1076 行

### 当前结构

| 逻辑分组 | 行范围 | 行数 |
|---|---|---|
| `MarkdownEditor` struct | L37-72 | ~35 |
| 构造 + 光标移动 + 缓存 | L74-580 | ~500 |
| 渲染（render + 状态栏 + 命令栏 + 弹窗） | L594-982 | **~390** |
| 公共 API（open_*） | L999-1076 | ~80 |

### 拆分方案

| 新文件 | 包含内容 | 行数 |
|---|---|---|
| `editor.rs` (入口) | struct + 核心编辑逻辑 + 公共 API | ~700 |
| `editor/render.rs` | 渲染相关方法 | ~390 |

**优先级**: 低。

---

## 13. `command/chat/tools/computer_use/tool.rs` — 1046 行

### 当前结构

| 逻辑分组 | 行范围 | 行数 |
|---|---|---|
| 数据类型 | L16-81 | ~65 |
| Action 处理器（13 个 action + 分发） | L266-865 | **~600** |
| 辅助函数 | L867-945 | ~80 |
| `Tool` trait 实现 | L947-1046 | ~100 |

### 拆分方案

| 新文件 | 包含内容 | 行数 |
|---|---|---|
| `tool.rs` (入口) | struct + `Tool` trait 实现 + `execute_single_action` 分发 | ~200 |
| `computer_use/actions.rs` | 13 个 action 方法 | ~600 |
| `computer_use/helpers.rs` | 辅助函数 | ~80 |

**优先级**: 低。

---

## 500-1000 行文件（暂不需拆分）

| 文件 | 行数 | 说明 |
|---|---|---|
| `command/chat/markdown/highlight.rs` | 969 | 结构合理 |
| `tui/editor_core/text_buffer.rs` | 877 | 结构合理 |
| `command/update.rs` | 875 | 单一职责 |
| `command/todo/app.rs` | 867 | 与 notebook/app.rs 类似，可后续拆分 |
| `theme.rs` | 824 | 主题定义，结构合理 |
| `command/chat/handler/tui_loop.rs` | 765 | 单一职责 |
| `command/chat/tools/sub_agent.rs` | 716 | 单一职责 |
| `command/chat/error.rs` | 700 | 错误定义，结构合理 |
| `command/chat/context/window.rs` | 689 | 单一职责 |
| `command/chat/input/autocomplete.rs` | 683 | 单一职责 |
| `command/chat/storage/session.rs` | 664 | 单一职责 |
| `command/chat/teammate/teammate_loop.rs` | 661 | 单一职责 |
| `command/chat/tools/derived_shared.rs` | 657 | 单一职责 |
| `command/chat/app/stream_poll.rs` | 629 | 单一职责 |
| `command/chat/tools/worktree.rs` | 624 | 单一职责 |
| `command/chat/permission/rules.rs` | 616 | 单一职责 |

---

## 总览：拆分优先级排序

| 优先级 | 文件 | 行数 | 核心问题 | 建议子模块数 |
|---|---|---|---|---|
| **高** | `chat_app.rs` | 2361 | `update()` 1535 行 | 4-5 |
| **高** | `cache.rs` | 2981 | 确认/工具渲染 1300+ 行 | 6-8 |
| **高** | `agent_loop.rs` | 1267 | `run_main_agent_loop` 1170 行 | 4-5 |
| **高** | `parser.rs` | 1518 | `markdown_to_lines` 847 行 | 3-4 |
| **高** | `hook.rs` | 2952 | HookManager 680 行 + 测试 1036 行 | 4-5 |
| **中高** | `handler/chat.rs` | 1104 | 弹窗拦截 + Dump 独立 | 3 |
| **中** | `renderer.rs` | 1559 | 渲染方法多 | 4 |
| **中** | `browser.rs` | 1522 | action 方法多 | 3 |
| **中** | `notebook/app.rs` | 1440 | 按键处理 634 行 | 3 |
| **中** | `oneshot.rs` | 1381 | `run_oneshot_agent` 628 行 | 3 |
| **中** | `report.rs` | 1246 | Git 操作 380 行 | 3 |
| **中** | `tool.rs` (computer_use) | 1046 | action 方法 600 行 | 3 |
| **低** | `editor.rs` | 1076 | 渲染 390 行 | 2 |

### 建议执行顺序

1. `cache.rs` — 职责边界最清晰，拆分风险最低
2. `chat_app.rs` — 收益最大（update 缩减 70%）
3. `hook.rs` — 类型/执行/管理分离自然
4. `agent_loop.rs` — 需要引入状态 struct，复杂度稍高
5. `parser.rs` — 需要引入 ParserState，复杂度最高
6. 其余按需拆分
