# Plan: AGENT.md 规范合规性全量修复

## 背景
根据合规性检查报告，需要修复 7 大类问题。部分工作已完成（fix1-4），剩余工作按类别并行分配给多个 agent。

## 已完成
- fix1: 修复 todo/ui.rs:584
- fix2: 修复 chat/ui/chat.rs:1075 draw_popup_list 调用处
- fix3: 修复 notebook/ui.rs 调用处
- fix4: 修复 server.rs:94 mut ws_state

## 待完成 — 并行 Agent 分配

### Agent A: 错误处理修复 (规范3)
- 替换 ~28 处生产代码中的 `unwrap()` 为安全替代
- 添加 3 处 `unsafe` 缺失的 `// SAFETY:` 注释
- 文件: shell.rs, plan.rs, agent.rs, computer_use/tool.rs, keyboard.rs, chat_app.rs, window.rs, teammate_loop.rs, permission/rules.rs, permission/queue.rs, tui_loop.rs, ui/config.rs, remote/setup.rs, storage.rs, update.rs, mouse.rs

### Agent B: 文档注释修复 (规范8)
- 为 15 个缺少 `///` 文档注释的 pub 成员添加文档
- 文件: util/shell_safety.rs, util/path_utils.rs, command/chat/infra/hook.rs, interactive/completer.rs

### Agent C: 函数参数 + 魔法值修复 (规范7)
- draw_popup_list (12参数) → 封装 Config struct
- colorize_tokens (11参数) → 封装 Config struct
- apply_static_placeholders (10参数) → 封装 Config struct
- run_headless_agent_loop (10参数) → 封装 Config struct
- classify_word (10参数) → 封装 Config struct
- handle_connection (9参数) → 封装 Config struct
- render_cursor_visual_line (9参数) → 封装 Config struct
- render_input_status_bar (8参数 x2) → 封装 Config struct
- render_visual_line (7参数) → 封装 Config struct
- draw_digit/draw_number (6参数) → 封装 Config struct
- drag (5参数) → 评估是否需要
- ~12处魔法值 → 提取为 const

### Agent D: clone 优化 (规范2)
- agent_loop.rs 中消息列表反复 clone → 改用引用或 Arc
- agent_loop.rs:1051 mem::take 替代 clone+clear
- tools/agent.rs clone Provider/Config → 改用 Arc 或引用
- tools/browser.rs 重复 clone → 优化
- chat_app.rs 归档 clone → 评估优化

### Agent E: mod.rs 迁移 + util 重命名 (规范6) [低优先级]
- 8 个 mod.rs → name.rs + name/ 模式
- util/ 模块评估是否需要重命名

## 执行策略
1. Agent A, B 并行（互不冲突的文件）
2. Agent C 的函数参数重构可能影响其他 agent 的文件 → 先完成参数重构
3. Agent D 的 clone 优化需要仔细分析生命周期 → 可能需要后续迭代
4. Agent E 低优先级，最后处理
5. 最终 cargo clippy + cargo fmt 验证
