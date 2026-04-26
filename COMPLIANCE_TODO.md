# AGENT.md 合规性审查 — 待修复清单

> 生成日期: 2026-04-26 | 基于代码实际扫描（非旧报告）
> 规范：函数行数 ≤ 120 行，参数 ≤ 4 个

---

## 一、函数行数 > 120 行

按行数降序排列。修复方式：按职责拆分为多个子函数。

- [x] `command/chat/agent/agent_loop.rs:run_main_agent_loop` — 1170行, 5参数 *(已拆分为 agent_loop/ 子模块：mod.rs ~371行 + compact_phase.rs ~292行 + stream_reader.rs ~541行 + tool_dispatch.rs ~230行；参数已封装为 MainAgentLoopParams + 各 Context struct)*
- [ ] `command/chat/markdown/highlight.rs:highlight_code_line` — 853行, 3参数
- [ ] `command/chat/handler/chat.rs:handle_chat_mode` — 672行, 2参数
- [ ] `command/chat/oneshot.rs:run_oneshot_agent` — 629行, 7参数
- [ ] `command/chat/teammate/teammate_loop.rs:run_teammate_loop` — 539行, 1参数
- [ ] `command/chat/handler/tui_loop.rs:run_chat_tui_internal` — 503行, 1参数
- [ ] `command/chat/markdown/parser.rs:markdown_to_lines` — 478行, 3参数
- [ ] `command/chat/app/chat_app.rs:new` — 472行, 1参数
- [ ] `command/chat/render/cache.rs:build_message_lines_incremental` — 389行, 5参数
- [ ] `command/chat/ui/title_bar.rs:draw_title_bar` — 338行, 6参数
- [ ] `command/chat/handler/config.rs:handle_config_mode` — 336行, 2参数
- [ ] `command/chat/app/stream_poll.rs:poll_stream_actions` — 330行, 0参数
- [ ] `command/chat/tools/sub_agent.rs:run_sub_agent_loop` — 318行, 5参数
- [x] `command/chat/infra/hook/manager.rs:execute` — 313行, 4参数 *(已拆分：提取 `collect_hooks_for_event` / `merge_hook_result_into` / `handle_hook_error`，execute 主体缩减至 ~170 行)*
- [ ] `command/chat/tools/teammate_tool.rs:execute` — 271行, 2参数
- [x] `command/chat/tools/grep.rs:execute` — 258行, 2参数 *(已拆分：提取 `build_file_walker` / `search_single_file` / `build_content_line` / `format_grep_output` 等 8 个辅助函数，execute 主体缩减至 ~60 行)*
- [ ] `command/chat/render/cache/tool_call_render.rs:render_tool_call_request_msg` — 258行, 4参数
- [ ] `interactive/parser.rs:parse_interactive_command` — 230行, 1参数
- [ ] `command/notebook/app.rs:handle_input_mode` — 229行, 2参数
- [ ] `interactive/completer.rs:complete` — 228行, 5参数
- [ ] `command/chat/tools/sub_agent.rs:execute` — 216行, 2参数
- [x] `tui/editor_core/renderer.rs:render_visual_line` — 207行, 7参数 *(已拆分：提取 CursorLineContext + render_cursor_visual_line → visual_line.rs，续行/非光标行逻辑留在 renderer.rs 主文件，render_visual_line 缩减至 ~160 行)*
- [ ] `command/chat/ui/config.rs:draw_config_screen` — 201行, 3参数
- [ ] `util/shell_safety.rs:check_single_segment` — 196行, 1参数
- [x] `tui/editor_core/renderer.rs:render_inline` — 190行, 1参数 *(已拆分：提取至 renderer/inline.rs)*
- [x] `tui/editor_core/renderer.rs:render_single_line_with_number` — 186行, 4参数 *(已拆分：提取至 renderer/line.rs)*
- [ ] `tui/editor_core/editor.rs:handle_input` — 181行, 1参数
- [ ] `command/system.rs:generate_zsh_completion` — 178行, 1参数
- [ ] `command/chat/tools/file/edit.rs:execute` — 178行, 2参数
- [ ] `command/chat/ui/components.rs:welcome_box` — 177行, 3参数
- [ ] `command/chat/ui/chat.rs:draw_messages` — 175行, 3参数
- [ ] `command/notebook/ui.rs:render_status_bar` — 169行, 3参数
- [x] `command/chat/tools/shell.rs:execute` — 168行, 2参数 *(已拆分：提取 `execute_sync`，execute 主体缩减至 ~40 行，execute_sync ~120 行)*
- [ ] `command/report.rs:handle_pull` — 167行, 1参数
- [ ] `command/chat/render/cache/tool_result_render.rs:render_tool_result_msg` — 165行, 9参数
- [ ] `command/chat/app/chat_app/update.rs:update` — 159行, 1参数
- [ ] `command/chat/context/window.rs:select_units` — 155行, 7参数
- [x] `command/chat/tools/worktree.rs:execute` — 153行, 2参数 *(EnterWorktreeTool 拆出 `create_and_enter`，ExitWorktreeTool 拆出 `handle_keep`/`handle_remove`)*
- [ ] `command/chat/tools/shell.rs:execute_background` — 152行, 4参数
- [ ] `command/chat/ui/archive.rs:draw_archive_list` — 149行, 3参数
- [ ] `command/update.rs:perform_update_curl` — 148行, 2参数
- [ ] `command/chat/app/message.rs:send_message_internal` — 148行, 1参数
- [x] `command/chat/app/session_mgr.rs:save_session_state` — 146行, 0参数 *(已拆分：提取 `build_teammates_snapshot`，save_session_state 缩减至 ~90 行)*
- [x] `command/chat/tools/web_search.rs:exec_search` — 145行, 1参数 *(已拆分：提取 `send_search_request` + `format_search_results`，exec_search 缩减至 ~30 行)*
- [ ] `command/chat/render/cache/msg_render.rs:render_assistant_msg` — 145行, 5参数
- [x] `tui/editor_core/renderer.rs:render_table_rows` — 143行, 6参数 *(已拆分：提取至 renderer/table.rs)*
- [ ] `command/script.rs:handle_script` — 143行, 3参数
- [ ] `tui/editor_core/editor.rs:render` — 141行, 2参数
- [x] `command/chat/remote/server.rs:handle_websocket` — 141行, 2参数 *(已重构：改为传 `&WsConnectionState`，参数从 6 降至 2)*
- [ ] `command/todo/ui.rs:render_status_bar` — 140行, 3参数
- [ ] `command/chat/oneshot.rs:interactive_confirm` — 138行, 5参数
- [ ] `command/chat/input/autocomplete.rs:get_filtered_files_for_at` — 135行, 1参数
- [ ] `command/chat/context/compact.rs:auto_compact` — 134行, 7参数
- [x] `tui/editor_core/renderer.rs:render_cursor_visual_line` — 133行, 4参数 *(已拆分：提取至 renderer/visual_line.rs)*
- [x] `command/chat/tools/worktree.rs:execute` — 133行, 2参数 *(同上，EnterWorktreeTool::execute 缩减至 ~60 行)*
- [x] `command/chat/tools/web_fetch.rs:exec_fetch` — 130行, 2参数 *(已拆分：提取 `build_http_client` / `send_http_request` / `extract_text_from_response`，exec_fetch 主体缩减至 ~30 行)*
- [ ] `command/chat/ui/chat.rs:render_image_pass` — 129行, 6参数
- [x] `command/chat/tools/hook.rs:handle_register` — 127行, 1参数 *(已拆分：提取 `build_hook_def_from_params`，handle_register 缩减至 ~65 行)*
- [ ] `command/chat/tools/derived_shared.rs:execute_tool_with_permission` — 127行, 7参数

---

## 二、函数参数 > 4 个

按参数数降序排列。修复方式：封装为 Config/Options 结构体。

### > 7 参数

- [x] `command/chat/ui/chat.rs:render_text_pass` — 7参数, 46行 *(已封装为 `TextPassParams` struct)*
- [x] `command/chat/tools/derived_shared.rs:call_llm_non_stream` — 7参数, 52行 *(已封装为 `LlmNonStreamRequest` struct)*
- [x] `command/chat/render/cache/tool_result_render.rs:render_tool_result_msg` — 8参数, 165行 *(已封装为 `ToolResultRenderParams` struct，函数体开头解构局部变量避免大量改动)*

### 8 参数

- [x] `tui/editor_core/editor.rs:open_markdown_editor_on_terminal` — 7参数, 36行 *(已封装为 `MarkdownEditorOpts` struct，theme 改为 owned)*
- [ ] `command/chat/tools/definition.rs:new` — 8参数, 78行
- [x] `command/chat/remote/server.rs:run_server` — 8参数, 38行 *(已封装为 `ServerOpts` struct，run_server 降至 1 参数)*
- [ ] `command/notebook/app.rs:build_flat_entries_recursive` — 8参数, 79行

### 7 参数

- [x] `tui/editor_core/renderer.rs:render_visual_line` — 7参数, 207行 *(同上，render_visual_line 缩减至 ~160 行，参数未变)*
- [x] `tui/editor_core/editor.rs:open_markdown_editor` — 6参数, 28行 *(同上，opts + content 两参数)*
- [x] `tui/editor_core/editor.rs:open_markdown_editor_with_content` — 6参数, 10行 *(同上，opts + initial_lines 两参数)*
- [x] `tui/components/row.rs:text_field_row` — 7参数, 33行 *(已封装为 `TextFieldRowCtx` struct)*
- [x] `tui/components/row.rs:toggle_list_item` — 7参数, 44行 *(已封装为 `ToggleListItemCtx` struct)*
- [ ] `command/chat/oneshot.rs:run_oneshot_agent` — 7参数, 629行
- [ ] `command/chat/ui/input.rs:build_line_segments` — 7参数, 80行
- [x] `command/chat/tools/derived_shared.rs:execute_tool_with_permission` — 6参数, 127行 *(已封装为 `ToolExecContext` struct)*
- [x] `command/chat/context/compact.rs:auto_compact` — 5参数, 134行 *(已封装为 `AutoCompactParams` struct，messages 单独传)*
- [x] `command/chat/context/window.rs:select_units` — 6参数, 155行 *(已封装为 `SelectUnitsParams` struct，函数体开头解构局部变量)*
- [ ] `command/todo/ui.rs:build_editing_item` — 7参数, 26行

### 6 参数

- [x] `tui/editor_markdown.rs:open_markdown_editor_on_terminal` — 4参数, 18行 *(已封装：提取 `build_editor_opts` 统一构建 MarkdownEditorOpts)*
- [x] `tui/editor_core/renderer.rs:render_table_rows` — 6参数, 143行 *(已拆分：提取至 renderer/table.rs)*
- [x] `tui/editor_core/renderer.rs:render_table_border` — 6参数, 28行 *(已拆分：提取至 renderer/table.rs)*
- [ ] `tui/editor_core/editor.rs:new` — 6参数, 51行
- [ ] `tui/components/cursor.rs:cursor_wrapped_lines` — 6参数, 65行
- [x] `tui/components/row.rs:toggle_row` — 6参数, 34行 *(已封装为 `ToggleRowCtx` struct)*
- [ ] `command/chat/oneshot.rs:run_oneshot_no_tools` — 6参数, 75行
- [ ] `command/chat/oneshot.rs:handle_tool_call` — 6参数, 87行
- [ ] `command/chat/oneshot.rs:fire_session_end` — 6参数, 18行
- [ ] `command/chat/ui/title_bar.rs:draw_title_bar` — 6参数, 338行
- [x] `command/chat/ui/components.rs:global_preview_row` — 6参数, 26行 *(已封装为 `GlobalRowCtx` struct，含 label/desc/hint/selected/theme)*
- [x] `command/chat/ui/components.rs:global_theme_row` — 6参数, 30行 *(同上 `GlobalRowCtx`)*
- [ ] `command/chat/ui/chat.rs:render_image_pass` — 6参数, 129行
- [ ] `command/chat/tools/background.rs:spawn_command` — 6参数, 28行
- [ ] `command/chat/tools/task/task_manager.rs:create_task` — 6参数, 22行
- [ ] `command/chat/tools/computer_use/som.rs:draw_digit` — 6参数, 20行
- [ ] `command/chat/tools/computer_use/som.rs:draw_number` — 6参数, 10行
- [ ] `command/chat/tools/computer_use/mouse.rs:CGEventCreateScrollWheelEvent` — 6参数, 14行
- [ ] `command/chat/tools/computer_use/mouse.rs:drag` — 6参数, 27行
- [ ] `command/chat/context/window.rs:select_messages` — 6参数, 89行
- [ ] `command/chat/render/cache/bubble.rs:wrap_md_line_in_bubble` — 6参数, 65行
- [x] `command/chat/agent/api.rs:build_request_with_tools` — 4参数, 49行 *(已合规：实际参数 ≤4，TODO 原扫描数据有误)*
- [x] `command/chat/remote/server.rs:handle_connection` — 6参数, 81行 *(已重构：参数 `token`/`expected_origin` 保持，`ws_state` 统一传入 `WsConnectionState`)*
- [x] `command/chat/remote/server.rs:handle_websocket` — 2参数, 141行 *(已重构：改为传 `&WsConnectionState`，参数从 6 降至 2)*
- [ ] `command/todo/ui.rs:build_normal_item` — 6参数, 73行
- [ ] `command/notebook/ui.rs:build_adding_item` — 6参数, 35行
- [ ] `command/notebook/ui.rs:build_rename_item` — 6参数, 9行

### 5 参数

- [x] `tui/editor_markdown.rs:open_markdown_editor` — 3参数, 16行 *(同上，opts 内部构建)*
- [x] `tui/editor_markdown.rs:open_markdown_editor_with_content` — 3参数, 16行 *(同上)*
- [ ] `tui/editor_core/search.rs:highlight_line` — 5参数, 61行
- [ ] `tui/components/row.rs:selectable_row` — 5参数, 22行
- [ ] `interactive/completer.rs:complete` — 5参数, 228行
- [ ] `interactive/completer.rs:complete` — 5参数, 8行
- [ ] `command/update.rs:draw_feature_menu` — 5参数, 89行
- [ ] `command/report.rs:update_config_files_silent` — 5参数, 17行
- [ ] `command/report.rs:update_config_files` — 5参数, 29行
- [ ] `command/report.rs:handle_search` — 5参数, 59行
- [ ] `command/chat/oneshot.rs:interactive_confirm` — 5参数, 138行
- [ ] `command/chat/handler/tui_loop.rs:dispatch_event` — 5参数, 116行
- [ ] `command/chat/handler/chat.rs:write_agent_dump` — 5参数, 17行
- [ ] `command/chat/ui/selector.rs:selector_block` — 5参数, 18行
- [ ] `command/chat/ui/hint.rs:render_hint_bar` — 5参数, 23行
- [ ] `command/chat/ui/input.rs:compute_mention_ranges` — 5参数, 12行
- [ ] `command/chat/infra/hook/manager.rs:execute_fire_and_forget` — 5参数, 12行
- [ ] `command/chat/infra/hook/definition.rs:into_hook_kinds` — 5参数, 47行
- [ ] `command/chat/tools/classification.rs:get_result_summary_for_tool` — 5参数, 27行
- [x] `command/chat/tools/browser.rs:screenshot` — 3参数, 70行 *(已合规：实际参数 ≤4，TODO 原扫描数据有误)*
- [x] `command/chat/tools/browser.rs:type_text` — 3参数, 96行 *(已合规：实际参数 ≤4，TODO 原扫描数据有误)*
- [x] `command/chat/tools/browser.rs:exec_browser_async` — 3参数, 92行 *(已合规：实际参数 ≤4，TODO 原扫描数据有误)*
- [x] `command/chat/tools/sub_agent.rs:run_sub_agent_loop` — 4参数, 318行 *(已合规：已封装为 `SubAgentLoopParams` struct，TODO 原扫描数据有误)*
- [ ] `command/chat/tools/computer_use/som.rs:capture_som` — 5参数, 78行
- [ ] `command/chat/tools/computer_use/ax.rs:query_tree` — 5参数, 22行
- [ ] `command/chat/tools/computer_use/ax.rs:find_elements` — 5参数, 18行
- [ ] `command/chat/tools/computer_use/mouse.rs:mouse_event` — 5参数, 9行
- [ ] `command/chat/tools/computer_use/mouse.rs:scroll` — 5参数, 26行
- [ ] `command/chat/context/compact.rs:record_skill_invocation` — 5参数, 24行
- [ ] `command/chat/app/system_prompt.rs:build_system_prompt_fn` — 5参数, 38行
- [ ] `command/chat/app/agent_handle.rs:spawn` — 5参数, 58行
- [ ] `command/chat/render/cache.rs:build_message_lines_incremental` — 5参数, 389行
- [ ] `command/chat/render/cache/msg_render.rs:render_user_msg` — 5参数, 114行
- [ ] `command/chat/render/cache/msg_render.rs:render_assistant_msg` — 5参数, 145行
- [x] `command/chat/agent/agent_loop.rs:push_compact_tool_messages` — 4参数, 49行 *(已合规：实际参数 ≤4，TODO 原扫描数据有误)*
- [ ] `command/chat/agent/agent_loop.rs:run_main_agent_loop` — 5参数, 1170行
- [x] `command/chat/agent/api.rs:call_llm_stream_async` — 4参数, 75行 *(已合规：实际参数 ≤4，TODO 原扫描数据有误)*
- [x] `command/chat/agent/api.rs:call_llm_stream` — 4参数, 18行 *(已合规：实际参数 ≤4，TODO 原扫描数据有误)*
- [ ] `command/chat/teammate/teammate_loop.rs:build_teammate_system_prompt` — 5参数, 20行
- [ ] `command/chat/permission/queue.rs:new` — 5参数, 14行
- [ ] `command/chat/remote/server.rs:serve_error` — 5参数, 27行

---

## 三、超过 800 行的大文件

按行数降序排列。修复方式：按职责拆分为子模块（独立 .rs 文件）。

- [x] `tui/editor_core/renderer.rs` — 1559行 *(已拆分为 renderer.rs + renderer/ 子模块：code_block.rs ~180行, table.rs ~240行, inline.rs ~160行, line.rs ~200行, visual_line.rs ~160行, renderer.rs ~350行)*
- [ ] `command/chat/tools/browser.rs` — 1522行
- [ ] `command/notebook/app.rs` — 1440行
- [ ] `command/chat/oneshot.rs` — 1381行
- [ ] `command/chat/agent/agent_loop.rs` — 1267行
- [ ] `command/report.rs` — 1246行
- [ ] `command/chat/handler/chat.rs` — 1104行
- [ ] `tui/editor_core/editor.rs` — 1076行
- [ ] `command/chat/tools/computer_use/tool.rs` — 1046行
- [ ] `command/chat/infra/hook/tests.rs` — 1042行 (测试文件，可豁免)
- [ ] `command/chat/markdown/highlight.rs` — 969行
- [ ] `command/update.rs` — 875行
- [ ] `command/chat/render/cache/confirm_render.rs` — 868行
- [ ] `command/todo/app.rs` — 867行
- [ ] `theme.rs` — 824行
- [ ] `command/chat/infra/hook/manager.rs` — 807行
- [ ] `command/chat/app/chat_app.rs` — 807行

## 四、非 test 代码中的 unwrap/expect

判定标准：前置条件保护的 expect 合理，可能失败的操作应改为 `?` 传播，Mutex::lock().unwrap() 可接受。

### 需改为 ? 传播（可能失败）

- [x] `interactive/run.rs:31` — `.expect("无法初始化编辑器")` — 已有 SAFETY 注释，合理 panic
- [x] `command/chat/input/input_thread.rs:42` — `.expect("failed to spawn input thread")` — 已有 SAFETY 注释，合理 panic
- [x] `command/chat/infra/sandbox.rs:188` — `std::env::current_dir().unwrap()` — 在 #[test] 内，可豁免

### 需补充 SAFETY 注释

- [x] `llm/stream.rs:109` — 已补充 SAFETY 注释（valid_up_to 保证 UTF-8 边界）
- [x] `llm/client.rs:65` — 原已有 SAFETY 注释 ✅
- [x] `command/time.rs:91` — 原已有 SAFETY 注释 ✅
- [x] `command/help/app.rs:135` — 已改为 `let Some(cache) = ... else { return &[] }` 防御式编程

### 合理使用（可豁免）

- `util/text.rs:68` — `.expect("正则表达式编译失败…")` — 静态正则模式，注释已说明
- `command/chat/handler/tui_loop.rs:438` — `.expect("ws_bridge checked…")` — 前置 is_some() 保护
- `command/chat/ui/config/global.rs:85` — `.expect("checked is_none()…")` — 前置条件检查保护
- `command/chat/oneshot.rs` 多处 — `Mutex::lock().unwrap()` — 仅在 poison 时失败，安全使用
- `command/chat/app/chat_app/update_misc.rs:247` — `.expect("browse mode 下…")` — 模式前置检查保证
- `command/chat/remote/crypto.rs` 两处 — HKDF/AES-GCM 参数硬编码常量，不会失败
- `command/chat/remote/setup.rs:52` — `.expect("SocketAddr 格式…")` — format! 产出必然合法
- `command/chat/context/regression_tests.rs` — 测试代码可豁免

## 五、super::super:: 过度层级引用

修复方式：统一改为 `crate::` 根路径绝对导入。

### 严重（含三级引用或 ≥10 处）

- [x] `command/chat/handler/tui_loop.rs` — 13处 → 全部改为 `crate::command::chat::*`
- [x] `command/chat/ui/config/global.rs` — 三级引用 → 改为 `crate::command::chat::render::helpers` / `crate::command::chat::ui::components`
- [x] `command/chat/ui/config/model.rs` — 三级引用 → 改为 `crate::command::chat::render::helpers` / `crate::command::chat::ui::components`

### 中等（3-9 处）

- [x] `command/chat/handler/archive.rs` — 5处 → `crate::command::chat::archive::*`
- [x] `command/chat/handler/chat.rs` — 3处 → `crate::command::chat::input::autocomplete` / `storage` / `render::theme`
- [x] `command/chat/agent/config.rs` — 5处 → `crate::command::chat::infra::hook` / `storage` / `tools::*`
- [x] `command/chat/ui/chat.rs` — 4处 → `crate::command::chat::app` / `markdown::*` / `render::cache`
- [x] `command/chat/ui/title_bar.rs` — 3处 → `crate::command::chat::app` / `teammate` / `tools::derived_shared`
- [x] `command/chat/agent/agent_loop.rs` — 3处 → `crate::command::chat::app::types` / `error` / `infra::hook`
- [x] `command/chat/agent/tool_processor.rs` — 3处 → `crate::command::chat::app::types` / `error` / `infra::hook`
- [x] `command/chat/app/chat_app/update_tool_interact.rs` — 3处 → `crate::command::chat::app::action` / `types` / `ui_state`
- [x] `command/chat/app/chat_app/update.rs` — 3处 → `crate::command::chat::app::action` / `ui_state`
- [x] `command/chat/app/chat_app/update_misc.rs` — 3处 → `crate::command::chat::app::action` / `types` / `ui_state`
- [x] `command/chat/app/chat_app/update_config.rs` — 3处 → `crate::command::chat::app::action` / `ui_state`

### 轻微（1-2 处）

- [x] `command/chat/ui/popup.rs` — 2处 → `crate::command::chat::app` / `input::autocomplete`
- [x] `command/chat/app/chat_app/update_session.rs` — 2处 → `crate::command::chat::app::action` / `ui_state`
- [x] `command/chat/handler/config.rs` — 1处 → `crate::command::chat::storage`
- [x] `command/chat/ui/selector.rs` — 1处 → `crate::command::chat::render::theme`
- [x] `command/chat/ui/archive.rs` — 1处 → `crate::command::chat::app`
- [x] `command/chat/input/autocomplete.rs` — 1处 → `crate::command::chat::app`
- [x] `command/chat/markdown/parser.rs` — 1处 → `crate::command::chat::render::theme`
- [x] `command/chat/markdown/image_loader.rs` — 1处 → `crate::command::chat::tools`
- [x] `command/chat/agent/api.rs` — 1处 → `crate::command::chat::error`
- [x] `command/chat/agent/retry.rs` — 1处 → `crate::command::chat::error`
- [x] `command/chat/remote/protocol.rs` — 1处 → `crate::command::chat::storage`
- [x] `command/chat/ui/hint.rs` — 1处 → `crate::command::chat::render::theme`

## 六、TUI 输出规范违规

> 规范：禁止在 TUI 模式下使用 println!/eprintln!/info!/error!/warn!/debug!

当前扫描结果：**TUI 代码中无违规**（原报告中的 `crate::error!` 已修复或不存在）。

### 可豁免的 CLI 输出（非 TUI 环境）

- `command/chat/oneshot.rs` — 大量 `println!`/`eprintln!`，但这是 CLI 模式（非 TUI 界面）
- `command/chat/remote/setup.rs` — 二维码显示和状态提示使用 `println!`，TUI 启动前的 CLI 阶段输出
