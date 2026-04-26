# AGENT.md 合规性审查 — 待修复清单

> 生成日期: 2026-04-26 | 基于代码实际扫描（非旧报告）
> 规范：函数行数 ≤ 120 行，参数 ≤ 4 个

---

## 一、函数行数 > 120 行

按行数降序排列。修复方式：按职责拆分为多个子函数。

- [ ] `command/chat/agent/agent_loop.rs:run_main_agent_loop` — 1170行, 5参数
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
- [ ] `command/chat/infra/hook/manager.rs:execute` — 313行, 4参数
- [ ] `command/chat/tools/teammate_tool.rs:execute` — 271行, 2参数
- [ ] `command/chat/tools/grep.rs:execute` — 258行, 2参数
- [ ] `command/chat/render/cache/tool_call_render.rs:render_tool_call_request_msg` — 258行, 4参数
- [ ] `interactive/parser.rs:parse_interactive_command` — 230行, 1参数
- [ ] `command/notebook/app.rs:handle_input_mode` — 229行, 2参数
- [ ] `interactive/completer.rs:complete` — 228行, 5参数
- [ ] `command/chat/tools/sub_agent.rs:execute` — 216行, 2参数
- [ ] `tui/editor_core/renderer.rs:render_visual_line` — 207行, 7参数
- [ ] `command/chat/ui/config.rs:draw_config_screen` — 201行, 3参数
- [ ] `util/shell_safety.rs:check_single_segment` — 196行, 1参数
- [ ] `tui/editor_core/renderer.rs:render_inline` — 190行, 1参数
- [ ] `tui/editor_core/renderer.rs:render_single_line_with_number` — 186行, 4参数
- [ ] `tui/editor_core/editor.rs:handle_input` — 181行, 1参数
- [ ] `command/system.rs:generate_zsh_completion` — 178行, 1参数
- [ ] `command/chat/tools/file/edit.rs:execute` — 178行, 2参数
- [ ] `command/chat/ui/components.rs:welcome_box` — 177行, 3参数
- [ ] `command/chat/ui/chat.rs:draw_messages` — 175行, 3参数
- [ ] `command/notebook/ui.rs:render_status_bar` — 169行, 3参数
- [ ] `command/chat/tools/shell.rs:execute` — 168行, 2参数
- [ ] `command/report.rs:handle_pull` — 167行, 1参数
- [ ] `command/chat/render/cache/tool_result_render.rs:render_tool_result_msg` — 165行, 9参数
- [ ] `command/chat/app/chat_app/update.rs:update` — 159行, 1参数
- [ ] `command/chat/context/window.rs:select_units` — 155行, 7参数
- [ ] `command/chat/tools/worktree.rs:execute` — 153行, 2参数
- [ ] `command/chat/tools/shell.rs:execute_background` — 152行, 4参数
- [ ] `command/chat/ui/archive.rs:draw_archive_list` — 149行, 3参数
- [ ] `command/update.rs:perform_update_curl` — 148行, 2参数
- [ ] `command/chat/app/message.rs:send_message_internal` — 148行, 1参数
- [ ] `command/chat/app/session_mgr.rs:save_session_state` — 146行, 0参数
- [ ] `command/chat/tools/web_search.rs:exec_search` — 145行, 1参数
- [ ] `command/chat/render/cache/msg_render.rs:render_assistant_msg` — 145行, 5参数
- [ ] `tui/editor_core/renderer.rs:render_table_rows` — 143行, 6参数
- [ ] `command/script.rs:handle_script` — 143行, 3参数
- [ ] `tui/editor_core/editor.rs:render` — 141行, 2参数
- [ ] `command/chat/remote/server.rs:handle_websocket` — 141行, 6参数
- [ ] `command/todo/ui.rs:render_status_bar` — 140行, 3参数
- [ ] `command/chat/oneshot.rs:interactive_confirm` — 138行, 5参数
- [ ] `command/chat/input/autocomplete.rs:get_filtered_files_for_at` — 135行, 1参数
- [ ] `command/chat/context/compact.rs:auto_compact` — 134行, 7参数
- [ ] `tui/editor_core/renderer.rs:render_cursor_visual_line` — 133行, 4参数
- [ ] `command/chat/tools/worktree.rs:execute` — 133行, 2参数
- [ ] `command/chat/tools/web_fetch.rs:exec_fetch` — 130行, 2参数
- [ ] `command/chat/ui/chat.rs:render_image_pass` — 129行, 6参数
- [ ] `command/chat/tools/hook.rs:handle_register` — 127行, 1参数
- [ ] `command/chat/tools/derived_shared.rs:execute_tool_with_permission` — 127行, 7参数

---

## 二、函数参数 > 4 个

按参数数降序排列。修复方式：封装为 Config/Options 结构体。

### > 7 参数

- [ ] `command/chat/ui/chat.rs:render_text_pass` — 10参数, 46行
- [ ] `command/chat/tools/derived_shared.rs:call_llm_non_stream` — 9参数, 52行
- [ ] `command/chat/render/cache/tool_result_render.rs:render_tool_result_msg` — 9参数, 165行

### 8 参数

- [ ] `tui/editor_core/editor.rs:open_markdown_editor_on_terminal` — 8参数, 36行
- [ ] `command/chat/tools/definition.rs:new` — 8参数, 78行
- [ ] `command/chat/remote/server.rs:run_server` — 8参数, 38行
- [ ] `command/notebook/app.rs:build_flat_entries_recursive` — 8参数, 79行

### 7 参数

- [ ] `tui/editor_core/renderer.rs:render_visual_line` — 7参数, 207行
- [ ] `tui/editor_core/editor.rs:open_markdown_editor` — 7参数, 28行
- [ ] `tui/editor_core/editor.rs:open_markdown_editor_with_content` — 7参数, 10行
- [ ] `tui/components/row.rs:text_field_row` — 7参数, 33行
- [ ] `tui/components/row.rs:toggle_list_item` — 7参数, 44行
- [ ] `command/chat/oneshot.rs:run_oneshot_agent` — 7参数, 629行
- [ ] `command/chat/ui/input.rs:build_line_segments` — 7参数, 80行
- [ ] `command/chat/tools/derived_shared.rs:execute_tool_with_permission` — 7参数, 127行
- [ ] `command/chat/context/compact.rs:auto_compact` — 7参数, 134行
- [ ] `command/chat/context/window.rs:select_units` — 7参数, 155行
- [ ] `command/todo/ui.rs:build_editing_item` — 7参数, 26行

### 6 参数

- [ ] `tui/editor_markdown.rs:open_markdown_editor_on_terminal` — 6参数, 18行
- [ ] `tui/editor_core/renderer.rs:render_table_rows` — 6参数, 143行
- [ ] `tui/editor_core/renderer.rs:render_table_border` — 6参数, 28行
- [ ] `tui/editor_core/editor.rs:new` — 6参数, 51行
- [ ] `tui/components/cursor.rs:cursor_wrapped_lines` — 6参数, 65行
- [ ] `tui/components/row.rs:toggle_row` — 6参数, 34行
- [ ] `command/chat/oneshot.rs:run_oneshot_no_tools` — 6参数, 75行
- [ ] `command/chat/oneshot.rs:handle_tool_call` — 6参数, 87行
- [ ] `command/chat/oneshot.rs:fire_session_end` — 6参数, 18行
- [ ] `command/chat/ui/title_bar.rs:draw_title_bar` — 6参数, 338行
- [ ] `command/chat/ui/components.rs:global_preview_row` — 6参数, 26行
- [ ] `command/chat/ui/components.rs:global_theme_row` — 6参数, 30行
- [ ] `command/chat/ui/chat.rs:render_image_pass` — 6参数, 129行
- [ ] `command/chat/tools/background.rs:spawn_command` — 6参数, 28行
- [ ] `command/chat/tools/task/task_manager.rs:create_task` — 6参数, 22行
- [ ] `command/chat/tools/computer_use/som.rs:draw_digit` — 6参数, 20行
- [ ] `command/chat/tools/computer_use/som.rs:draw_number` — 6参数, 10行
- [ ] `command/chat/tools/computer_use/mouse.rs:CGEventCreateScrollWheelEvent` — 6参数, 14行
- [ ] `command/chat/tools/computer_use/mouse.rs:drag` — 6参数, 27行
- [ ] `command/chat/context/window.rs:select_messages` — 6参数, 89行
- [ ] `command/chat/render/cache/bubble.rs:wrap_md_line_in_bubble` — 6参数, 65行
- [ ] `command/chat/agent/api.rs:build_request_with_tools` — 6参数, 49行
- [ ] `command/chat/remote/server.rs:handle_connection` — 6参数, 81行
- [ ] `command/chat/remote/server.rs:handle_websocket` — 6参数, 141行
- [ ] `command/todo/ui.rs:build_normal_item` — 6参数, 73行
- [ ] `command/notebook/ui.rs:build_adding_item` — 6参数, 35行
- [ ] `command/notebook/ui.rs:build_rename_item` — 6参数, 9行

### 5 参数

- [ ] `tui/editor_markdown.rs:open_markdown_editor` — 5参数, 16行
- [ ] `tui/editor_markdown.rs:open_markdown_editor_with_content` — 5参数, 16行
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
- [ ] `command/chat/tools/browser.rs:screenshot` — 5参数, 70行
- [ ] `command/chat/tools/browser.rs:type_text` — 5参数, 96行
- [ ] `command/chat/tools/browser.rs:exec_browser_async` — 5参数, 92行
- [ ] `command/chat/tools/sub_agent.rs:run_sub_agent_loop` — 5参数, 318行
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
- [ ] `command/chat/agent/agent_loop.rs:push_compact_tool_messages` — 5参数, 49行
- [ ] `command/chat/agent/agent_loop.rs:run_main_agent_loop` — 5参数, 1170行
- [ ] `command/chat/agent/api.rs:call_llm_stream_async` — 5参数, 75行
- [ ] `command/chat/agent/api.rs:call_llm_stream` — 5参数, 18行
- [ ] `command/chat/teammate/teammate_loop.rs:build_teammate_system_prompt` — 5参数, 20行
- [ ] `command/chat/permission/queue.rs:new` — 5参数, 14行
- [ ] `command/chat/remote/server.rs:serve_error` — 5参数, 27行

---

## 三、超过 800 行的大文件

按行数降序排列。修复方式：按职责拆分为子模块（独立 .rs 文件）。

- [ ] `tui/editor_core/renderer.rs` — 1559行
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

- [ ] `interactive/run.rs:31` — `.expect("无法初始化编辑器")` → 应传播错误
- [ ] `command/chat/input/input_thread.rs:42` — `.expect("failed to spawn input thread")` → 线程创建可能失败，应传播错误
- [ ] `command/chat/infra/sandbox.rs:188` — `std::env::current_dir().unwrap()` → 应处理错误

### 需补充 SAFETY 注释

- [ ] `llm/stream.rs:109` — `.expect("valid_uplo…")` — 逻辑正确（UTF-8 边界切割），但需标注 SAFETY
- [ ] `llm/client.rs:65` — `.expect("ChatRequest must serialize…")` — serde 序列化必然成功，需 SAFETY 注释
- [ ] `command/time.rs:91` — `.expect("进度条模板格式错误")` — 编译期常量模式，需 SAFETY 注释
- [ ] `command/chat/help/app.rs:135` — `.expect("缓存应该在…")` — 依赖前置逻辑保证，建议改为 if let 防御式

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

- [ ] `command/chat/handler/tui_loop.rs` — 13处 `super::super::` 引用
- [ ] `command/chat/ui/config/global.rs` — 含 `super::super::super::` 三级引用
- [ ] `command/chat/ui/config/model.rs` — 含 `super::super::super::` 三级引用

### 中等（3-9 处）

- [ ] `command/chat/handler/archive.rs` — 5处内联 `super::super::` 调用
- [ ] `command/chat/handler/chat.rs` — 3个 `super::super::` use 语句
- [ ] `command/chat/agent/config.rs` — 5个 `super::super::` use 语句
- [ ] `command/chat/ui/chat.rs` — 4个 `super::super::` use 语句
- [ ] `command/chat/ui/title_bar.rs` — 3个 `super::super::` use 语句
- [ ] `command/chat/agent/agent_loop.rs` — 3个 `super::super::` use 语句
- [ ] `command/chat/agent/tool_processor.rs` — 3个 `super::super::` use 语句
- [ ] `command/chat/app/chat_app/update_tool_interact.rs` — 3处
- [ ] `command/chat/app/chat_app/update.rs` — 3处（含 2 处内联）
- [ ] `command/chat/app/chat_app/update_misc.rs` — 3个 use 语句
- [ ] `command/chat/app/chat_app/update_config.rs` — 3处（含 1 处内联）

### 轻微（1-2 处）

- [ ] `command/chat/ui/popup.rs` — 2处
- [ ] `command/chat/app/chat_app/update_session.rs` — 2处
- [ ] `command/chat/handler/config.rs` — 1处
- [ ] `command/chat/ui/selector.rs` — 1处
- [ ] `command/chat/ui/archive.rs` — 1处
- [ ] `command/chat/input/autocomplete.rs` — 1处
- [ ] `command/chat/markdown/parser.rs` — 1处
- [ ] `command/chat/markdown/image_loader.rs` — 1处（内联）
- [ ] `command/chat/agent/api.rs` — 1处
- [ ] `command/chat/agent/retry.rs` — 1处
- [ ] `command/chat/remote/protocol.rs` — 1处

## 六、TUI 输出规范违规

> 规范：禁止在 TUI 模式下使用 println!/eprintln!/info!/error!/warn!/debug!

当前扫描结果：**TUI 代码中无违规**（原报告中的 `crate::error!` 已修复或不存在）。

### 可豁免的 CLI 输出（非 TUI 环境）

- `command/chat/oneshot.rs` — 大量 `println!`/`eprintln!`，但这是 CLI 模式（非 TUI 界面）
- `command/chat/remote/setup.rs` — 二维码显示和状态提示使用 `println!`，TUI 启动前的 CLI 阶段输出
