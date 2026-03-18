# Chat Module Architecture

> 本文档描述 `src/command/chat/` 模块的数据流向和架构设计，供后续开发参考。

## 目录

1. [总体架构](#1-总体架构)
2. [文件结构](#2-文件结构)
3. [核心数据结构](#3-核心数据结构)
4. [数据流向](#4-数据流向)
5. [主事件循环（5 阶段）](#5-主事件循环5-阶段)
6. [Action 枚举（完整分类）](#6-action-枚举完整分类)
7. [Handler 层](#7-handler-层)
8. [后台流式处理](#8-后台流式处理)
9. [工具执行流水线](#9-工具执行流水线)
10. [渲染管线](#10-渲染管线)
11. [跨线程通信](#11-跨线程通信)
12. [关键设计模式](#12-关键设计模式)
13. [未来改进方向](#13-未来改进方向)

---

## 1. 总体架构

采用 **Redux-like 单向数据流** 模式：

```
┌─────────────────────────────────────────────────────────┐
│                      Main Event Loop                     │
│                     (handler/mod.rs)                      │
│                                                           │
│  Phase 1: Tick ──────── Action::TickToast                │
│      │                                                    │
│  Phase 2: Poll Backend ─ poll_stream_actions()           │
│      │                   → Vec<Action>                   │
│      │                   → app.update(action)            │
│      │                                                    │
│  Phase 3: Render ─────── draw_chat_ui(f, &mut app)      │
│      │                   (只在 needs_redraw=true 时)     │
│      │                                                    │
│  Phase 4: Collect Input                                  │
│      │   KeyEvent → handler_fn(app, key)                 │
│      │            → app.update(Action::Xxx)              │
│      │   Mouse    → app.update(Action::Scroll)           │
│      │                                                    │
│  Phase 5: Side-effects ─ 全屏编辑器等                    │
│                           → Action::ShowToast            │
└─────────────────────────────────────────────────────────┘
```

核心原则：
- **所有状态变更** 通过 `app.update(Action)` 集中处理
- Handler 只负责 **KeyEvent → Action 映射**，不直接改状态（少量例外）
- 后台事件通过 `poll_stream_actions()` 收集为 `Vec<Action>` 后统一 dispatch

---

## 2. 文件结构

```
src/command/chat/
├── mod.rs              # 模块入口
├── app.rs              # 核心：ChatApp, UIState, ChatState, ToolExecutor,
│                       #       AgentHandle, Action, update(), poll_stream_actions()
├── agent.rs            # Agent 后台线程（run_agent_loop）
├── model.rs            # 数据模型 I/O（AgentConfig, ChatSession, 加载/保存）
├── api.rs              # LLM API 调用
├── permission.rs       # .jcli 权限配置（JcliConfig）
├── skill.rs            # Skill 加载和构建
├── config.rs           # 配置字段映射
├── archive.rs          # 会话归档功能
├── autocomplete.rs     # @ 和 @file: 补全逻辑
├── render.rs           # 消息行构建（build_message_lines_incremental）
├── constant.rs         # 常量定义
├── theme.rs            # 主题系统
│
├── handler/            # 事件处理层（KeyEvent → Action）
│   ├── mod.rs          # 主事件循环（5 阶段）+ handler 分发
│   ├── chat.rs         # Chat 模式处理
│   ├── browse.rs       # Browse 模式处理
│   ├── config.rs       # Config/ToolToggle/SkillToggle/SelectModel 处理
│   ├── archive.rs      # ArchiveConfirm/ArchiveList 处理
│   └── tool_confirm.rs # ToolConfirm + Ask 模式处理
│
├── ui/                 # 渲染层
│   ├── mod.rs          # UI 模块入口
│   ├── chat.rs         # 主渲染函数 draw_chat_ui + 子组件
│   ├── config.rs       # 配置界面渲染
│   └── archive.rs      # 归档界面渲染
│
├── markdown/           # Markdown 渲染
│   ├── mod.rs
│   ├── parser.rs       # Markdown → ratatui Lines
│   ├── highlight.rs    # 语法高亮
│   ├── image_cache.rs  # 终端图片缓存
│   └── image_loader.rs # 异步图片加载
│
└── tools/              # 工具实现
    ├── mod.rs          # ToolRegistry
    ├── shell.rs        # Shell 命令执行
    ├── ask.rs          # Ask 结构化交互工具
    ├── web_search.rs   # 网页搜索
    ├── web_fetch.rs    # 网页内容获取
    ├── browser.rs      # 浏览器自动化
    ├── grep.rs         # Grep 搜索
    ├── task.rs         # 任务管理
    ├── skill.rs        # Skill 执行
    ├── background.rs   # 后台任务管理器
    └── file/           # 文件操作工具
        ├── mod.rs
        ├── read.rs
        ├── write.rs
        ├── edit.rs
        └── glob.rs
```

---

## 3. 核心数据结构

### 3.1 ChatApp（根状态）

```
ChatApp
├── ui: UIState                    # 前端 UI 状态（55 字段）
├── state: ChatState               # 后端数据状态（7 字段）
├── tool_executor: ToolExecutor    # 工具执行器（9 字段 + 8 方法）
├── agent: Option<AgentHandle>     # Agent 生命周期句柄（2 字段 + 3 方法）
├── tool_registry: Arc<ToolRegistry>        # 工具注册表（共享）
├── jcli_config: Arc<JcliConfig>            # .jcli 权限配置（共享）
├── background_manager: Arc<BackgroundManager>  # 后台任务管理器（共享）
├── ask_response_tx: Option<Sender<String>>     # Ask 工具响应通道
└── ask_request_rx: Option<Receiver<AskRequest>> # Ask 工具请求通道
```

### 3.2 UIState（前端 55 字段）

| 分组 | 字段 | 类型 | 说明 |
|------|------|------|------|
| **输入框** | input | String | 输入缓冲区 |
| | cursor_pos | usize | 光标位置 |
| **模式** | mode | ChatMode | 当前模式（10 种） |
| **滚动** | scroll_offset | u16 | 消息列表滚动偏移 |
| | auto_scroll | bool | 是否自动滚动到底部 |
| **浏览** | browse_msg_index | usize | 浏览模式选中消息 |
| | browse_scroll_offset | u16 | 消息内细粒度滚动 |
| **模型选择** | model_list_state | ListState | ratatui 列表状态 |
| **Toast** | toast | Option<(String, bool, Instant)> | 通知消息 |
| **渲染缓存** | msg_lines_cache | Option<MsgLinesCache> | 消息行缓存 |
| | last_rendered_streaming_len | usize | 流式节流：上次渲染长度 |
| | last_stream_render_time | Instant | 流式节流：上次渲染时间 |
| **配置界面** | config_provider_idx | usize | 选中 provider |
| | config_field_idx | usize | 选中字段 |
| | config_editing | bool | 是否编辑中 |
| | config_edit_buf | String | 编辑缓冲区 |
| | config_edit_cursor | usize | 编辑光标 |
| **主题** | theme | Theme | 当前主题 |
| **归档** | archives | Vec\<ChatArchive\> | 归档列表缓存 |
| | archive_list_index | usize | 列表选中索引 |
| | archive_default_name | String | 默认归档名 |
| | archive_custom_name | String | 自定义归档名 |
| | archive_editing_name | bool | 是否编辑名称 |
| | archive_edit_cursor | usize | 名称编辑光标 |
| | restore_confirm_needed | bool | 还原确认标志 |
| **@ 补全** | at_popup_active | bool | 弹窗激活 |
| | at_popup_filter | String | 过滤文本 |
| | at_popup_start_pos | usize | @ 在 input 中的位置 |
| | at_popup_selected | usize | 选中项索引 |
| **文件补全** | file_popup_active | bool | 弹窗激活 |
| | file_popup_start_pos | usize | @file: 起始位置 |
| | file_popup_filter | String | 路径过滤 |
| | file_popup_selected | usize | 选中项索引 |
| **工具交互** | tool_interact_selected | usize | 选项索引 (0=continue, 1=allow, 2=refuse, 3=type) |
| | tool_interact_typing | bool | 是否输入拒绝原因 |
| | tool_interact_input | String | 拒绝原因文本 |
| | tool_interact_cursor | usize | 输入光标 |
| **Ask 交互** | tool_ask_mode | bool | 是否 Ask 模式 |
| | tool_ask_questions | Vec\<AskQuestion\> | 问题列表 |
| | tool_ask_current_idx | usize | 当前问题索引 |
| | tool_ask_answers | Vec\<AskAnswer\> | 答案列表 |
| | tool_ask_selections | Vec\<bool\> | 多选状态 |
| | tool_ask_cursor | usize | 选项游标 |
| **编辑标志** | pending_system_prompt_edit | bool | 待编辑系统提示词 |
| | pending_style_edit | bool | 待编辑回复风格 |
| **图片** | image_cache | Arc\<Mutex\<ImageCache\>\> | 终端图片缓存 |
| **开关菜单** | tool_toggle_index | usize | 工具开关选中索引 |
| | skill_toggle_index | usize | Skill 开关选中索引 |

### 3.3 ChatState（后端 7 字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| agent_config | AgentConfig | Agent 配置（providers, theme, tools 等） |
| session | ChatSession | 当前对话（messages: Vec\<ChatMessage\>） |
| streaming_content | Arc\<Mutex\<String\>\> | 正在接收的流式内容（跨线程共享） |
| is_loading | bool | 是否等待 AI 回复 |
| loaded_skills | Vec\<Skill\> | 已加载的 Skills |
| queued_tasks | Arc\<Mutex\<Vec\<String\>\>\> | 排队任务列表（跨线程共享） |
| pending_user_messages | Arc\<Mutex\<Vec\<ChatMessage\>\>\> | 待处理消息队列（跨线程共享） |

### 3.4 ToolExecutor（9 字段 + 8 方法）

| 字段 | 说明 |
|------|------|
| active_tool_calls | 当前活跃的工具调用状态列表 |
| pending_tool_idx | ToolConfirm 模式中待处理工具索引 |
| tool_confirm_entered_at | 进入 ToolConfirm 的时间（超时用） |
| pending_tool_execution | 是否有待执行工具 |
| tools_executing_count | 后台执行中的工具数量 |
| tool_cancelled | 取消标志 (AtomicBool) |
| tool_exec_tx/rx | 工具执行结果通道 |
| tool_result_tx | 工具结果发送通道（→Agent 线程） |

| 方法 | 说明 |
|------|------|
| poll_results() | 轮询后台工具执行结果 |
| execute_batch() | 批量执行 Executing 状态的工具 |
| execute_current() | 执行当前待确认工具 |
| reject_current(reason) | 拒绝当前工具 |
| allow_and_execute() | 允许并记住规则 |
| advance() | 推进到下一个 PendingConfirm |
| cancel() | 取消所有工具执行 |
| reset() | 重置所有工具状态 |

### 3.5 AgentHandle（2 字段 + 3 方法）

| 字段 | 说明 |
|------|------|
| stream_rx | 接收后台流式消息的 channel |
| cancel_token | 流式请求取消令牌 (CancellationToken) |

| 方法 | 说明 |
|------|------|
| spawn(...) | 启动 Agent 后台线程 |
| cancel() | 取消当前流式请求 |
| poll() | 非阻塞获取所有可用 StreamMsg |

### 3.6 ChatMode（10 种模式）

```rust
Chat            // 正常对话（焦点在输入框）
SelectModel     // 模型选择
Browse          // 消息浏览（选中消息并复制）
Help            // 帮助屏幕
Config          // 配置编辑
ArchiveConfirm  // 归档确认（输入归档名称）
ArchiveList     // 归档列表（查看和还原）
ToolConfirm     // 工具调用确认
ToolToggle      // 工具开关子菜单
SkillToggle     // Skill 开关子菜单
```

---

## 4. 数据流向

### 4.1 全局流向图

```
                     ┌──────────────┐
                     │  User Input  │
                     │  (Keyboard)  │
                     └──────┬───────┘
                            │ KeyEvent
                            ▼
                ┌───────────────────────┐
                │    Handler 层          │
                │  (handler/*.rs)       │
                │                       │
                │  match key.code {     │
                │    'j' => Action::    │
                │      BrowseNavigate   │
                │      (Down)           │
                │  }                    │
                └───────────┬───────────┘
                            │ Action
                            ▼
            ┌───────────────────────────────┐
            │         app.update(action)     │
            │         (app.rs:~1900 行)      │
            │                               │
            │  match action {               │
            │    BrowseNavigate(dir) => {   │
            │      // 直接修改 ui/state     │
            │    }                          │
            │  }                            │
            └───────────────┬───────────────┘
                            │ State Mutation
                            ▼
            ┌───────────────────────────────┐
            │      ChatApp 状态树            │
            │  ├── ui: UIState             │
            │  ├── state: ChatState        │
            │  └── tool_executor           │
            └───────────────┬───────────────┘
                            │
                            ▼
            ┌───────────────────────────────┐
            │     draw_chat_ui(f, app)      │
            │     (ui/chat.rs)              │
            │                               │
            │  读取 app 状态 → 渲染到终端    │
            └───────────────────────────────┘
```

### 4.2 后台流式数据流向

```
┌────────────────┐  StreamMsg  ┌──────────────────────┐
│  Agent Thread  │────────────▶│  agent.stream_rx      │
│  (agent.rs)    │             │  (mpsc::Receiver)     │
│                │             └──────────┬────────────┘
│  run_agent_    │                        │
│  loop()        │                        ▼
│                │             ┌──────────────────────┐
│  • 调用 LLM    │             │ poll_stream_actions() │
│  • 收到 chunk  │             │ (app.rs)             │
│  • 收到 tool   │             │                      │
│    call        │             │ StreamMsg → Action:   │
│  • 等待 tool   │             │  Chunk → StreamChunk  │
│    result      │             │  Done  → StreamDone   │
│  • 继续下一轮  │             │  Error → StreamError  │
└────────────────┘             └──────────┬────────────┘
        ▲                                 │ Vec<Action>
        │                                 ▼
        │                      ┌──────────────────────┐
        │  ToolResultMsg       │   for action in       │
        │  (执行结果回传)       │     stream_actions    │
        ├──────────────────────│   { app.update(a); }  │
        │                      └──────────────────────┘
        │
┌───────┴────────┐
│ Tool Execution │
│ (后台线程)      │
│                │
│ registry       │
│  .execute()    │
└────────────────┘
```

### 4.3 Ask 工具数据流向

```
Agent Thread                    TUI Main Thread
     │                               │
     │ AskRequest(questions,         │
     │   response_tx)                │
     │──────────────────────────────▶│ ask_request_rx.recv()
     │                               │
     │                               │ init_ask_mode()
     │                               │ ui.tool_ask_mode = true
     │                               │ ui.tool_ask_questions = [...]
     │                               │
     │                               │ ◀── 用户交互 ──▶
     │                               │ AskNavigate / AskInputChar
     │                               │ AskSubmitAnswer
     │                               │
     │       JSON response           │
     │◀──────────────────────────────│ ask_response_tx.send(json)
     │                               │
     │ (继续 agent loop)             │
```

---

## 5. 主事件循环（5 阶段）

位于 `handler/mod.rs` 的 `run_chat_tui_internal()`。

```rust
loop {
    // ═══════════════════════════════════════════════
    // Phase 1: Tick — 定时器和周期性状态更新
    // ═══════════════════════════════════════════════
    // • 检查 Toast 过期 → Action::TickToast
    // • 如果 Toast 消失 → needs_redraw = true

    // ═══════════════════════════════════════════════
    // Phase 2: Poll Backend — 后台事件收集与分发
    // ═══════════════════════════════════════════════
    // • poll_stream_actions() → Vec<Action>
    // • 逐个 app.update(action) 分发
    // • 检查 pending_tool_execution → 强制重绘
    // • ToolConfirm 超时 → Action::ExecutePendingTool
    // • 流式节流：bytes_delta>=200 || time>=200ms → 重绘

    // ═══════════════════════════════════════════════
    // Phase 3: Render — 条件重绘
    // ═══════════════════════════════════════════════
    // • 只在 needs_redraw=true 时调用 terminal.draw()
    // • 渲染后更新流式节流状态
    // • needs_redraw = false

    // ═══════════════════════════════════════════════
    // Phase 4: Collect Input — 事件采集与处理
    // ═══════════════════════════════════════════════
    // • poll_timeout: 150ms(加载中) / 500ms(ToolConfirm) / 1000ms(空闲)
    // • 批量消费所有待处理事件（非阻塞循环）
    // • KeyEvent → 按 mode 分发到对应 handler
    // • Mouse ScrollUp/Down → Action::Scroll(Direction)
    // • Resize → needs_redraw = true

    // ═══════════════════════════════════════════════
    // Phase 5: Side-effects — 全屏编辑器等
    // ═══════════════════════════════════════════════
    // • pending_system_prompt_edit → 打开编辑器
    // • pending_style_edit → 打开编辑器
    // • 编辑结果 → Action::ShowToast
}
```

### needs_redraw 触发条件

| 触发源 | 条件 |
|--------|------|
| Toast 过期 | `had_toast && app.ui.toast.is_none()` |
| 后台事件 | `!stream_actions.is_empty()` |
| 待执行工具 | `pending_tool_execution == true` |
| ToolConfirm 超时 | 超时到达或倒计时变化 |
| 流式节流 | `bytes_delta >= 200 \|\| time >= 200ms` |
| 加载完成 | `was_loading && !is_loading` |
| ToolConfirm 模式 | 始终重绘（倒计时显示） |
| 任何键盘/鼠标事件 | `needs_redraw = true` |
| 窗口 Resize | `needs_redraw = true` |
| 编辑器返回 | `needs_redraw = true` |

---

## 6. Action 枚举（完整分类）

共 ~85 个变体，按功能分为 14 类：

### Chat 输入和文本编辑（6 个）
| Action | 说明 |
|--------|------|
| SendMessage | 发送当前输入框内容 |
| InsertChar(char) | 光标处插入字符 |
| DeleteChar | 删除光标前字符 (Backspace) |
| DeleteForward | 删除光标后字符 (Delete) |
| MoveCursor(Direction) | 移动光标 |
| ClearInput | 清空输入框 |

### 弹窗交互（10 个）
| Action | 说明 |
|--------|------|
| AtPopupActivate | 激活 @ 补全弹窗 |
| AtPopupClose | 关闭 @ 补全弹窗 |
| AtPopupFilter(String) | 更新过滤文本 |
| AtPopupNavigate(Dir) | 弹窗内导航 |
| AtPopupConfirm | 确认选择 |
| FilePopupActivate | 激活文件补全弹窗 |
| FilePopupClose | 关闭文件补全弹窗 |
| FilePopupFilter(String) | 更新路径过滤 |
| FilePopupNavigate(Dir) | 弹窗内导航 |
| FilePopupConfirm | 确认选择 |

### 流式生命周期（5 个）
| Action | 说明 | 来源 |
|--------|------|------|
| StreamChunk | 收到流式文本块 | poll_stream_actions() |
| ToolCallRequest(Vec) | LLM 请求执行工具 | poll_stream_actions() |
| StreamDone | 流式完成 | poll_stream_actions() |
| StreamError(String) | 流式错误 | poll_stream_actions() |
| StreamCancelled | 用户取消 | poll_stream_actions() |

### 工具执行和确认（5 个）
| Action | 说明 |
|--------|------|
| ExecutePendingTool | 执行当前待确认工具 |
| RejectPendingTool | 拒绝工具（无原因） |
| RejectPendingToolWithReason(String) | 拒绝工具（带原因） |
| AllowAndExecutePendingTool | 允许并记住规则到 .jcli |
| ToolExecDone(ToolExecDoneMsg) | 工具后台执行完成 |

### Ask 工具交互（8 个）
| Action | 说明 |
|--------|------|
| AskNavigate(Dir) | 上一题/下一题 |
| AskOptionNavigate(Dir) | 选项上下移动 |
| AskSingleSelect | 单选确认 |
| AskToggleMultiSelect | 多选切换 |
| AskInputChar(char) | 自由文本输入 |
| AskDeleteChar | 自由文本删除 |
| AskSubmitAnswer | 提交当前答案 |
| AskCancel | 取消所有问题 |

### 工具交互区（4 个）
| Action | 说明 |
|--------|------|
| ToolInteractNavigate(Dir) | 选项导航 (Continue→Allow→Refuse→Type) |
| ToolInteractInputChar(char) | 拒绝原因输入 |
| ToolInteractDeleteChar | 拒绝原因删除 |
| ToolInteractConfirm | 确认当前选项 |

### 模式切换和导航（7 个）
| Action | 说明 |
|--------|------|
| EnterMode(ChatMode) | 进入指定模式 |
| ExitToChat | 返回 Chat 模式 |
| Scroll(Dir) | 滚动消息 |
| PageScroll(Dir) | 分页滚动 |
| BrowseNavigate(Dir) | 浏览模式选择消息 |
| BrowseFineScroll(Dir) | 浏览模式细粒度滚动 |
| BrowseCopyMessage | 复制选中消息 |

### 配置编辑（18 个）
| Action | 说明 |
|--------|------|
| ConfigNavigate(Dir) | 选择字段 |
| ConfigSwitchProvider(Dir) | 切换 provider |
| ConfigEnter | 开始编辑或触发操作 |
| ConfigEditChar(char) | 编辑输入 |
| ConfigEditDelete | 编辑删除 |
| ConfigEditMoveCursor(Dir) | 编辑光标移动 |
| ConfigEditSubmit | 提交编辑 |
| ConfigAddProvider | 添加 provider |
| ConfigDeleteProvider | 删除 provider |
| ConfigSetActiveProvider | 设为活跃 provider |
| EnterToolToggleMenu | 进入工具开关菜单 |
| EnterSkillToggleMenu | 进入 Skill 开关菜单 |
| ToggleMenuNavigate(Dir) | 开关菜单导航 |
| ToggleMenuToggle | 切换当前项 |
| ToggleMenuEnableAll | 全部启用 |
| ToggleMenuDisableAll | 全部禁用 |
| ModelSelectNavigate(Dir) | 模型列表导航 |
| ModelSelectConfirm | 确认切换模型 |

### 归档管理（12 个）
| Action | 说明 |
|--------|------|
| StartArchiveConfirm | 启动归档确认 |
| ArchiveConfirmEditName | 开始编辑归档名 |
| ArchiveConfirmMoveCursor(Dir) | 名称编辑光标移动 |
| ArchiveConfirmInputChar(char) | 名称输入字符 |
| ArchiveConfirmDeleteChar | 名称删除字符 |
| ArchiveWithDefault | 使用默认名保存 |
| ArchiveWithCustom | 使用自定义名保存 |
| ClearSession | 清空会话 |
| StartArchiveList | 打开归档列表 |
| ArchiveListNavigate(Dir) | 列表导航 |
| RestoreArchive | 还原归档 |
| DeleteArchive | 删除归档 |

### 模型和主题（3 个）
| Action | 说明 |
|--------|------|
| SwitchModel | 进入模型选择 (Ctrl+T) |
| SwitchTheme | 切换主题 |
| ToggleStreamMode | 切换流式/批处理 |

### 流式控制（2 个）
| Action | 说明 |
|--------|------|
| CancelStream | 取消流式请求 (Esc) |
| CancelToolsOnly | 只取消工具，不中断 Agent |

### UI 管理（3 个）
| Action | 说明 |
|--------|------|
| ShowToast(String, bool) | 显示通知 (内容, 是否错误) |
| TickToast | 检查 Toast 过期 |
| SaveConfig | 保存配置 |

### 快速操作（3 个）
| Action | 说明 |
|--------|------|
| CopyLastAiReply | 复制最后 AI 回复 (Ctrl+Y) |
| ShowHelp | 显示帮助 (F1/?) |
| OpenLogWindows | 打开日志窗口 (Ctrl+G) |

### 应用控制（1 个）
| Action | 说明 |
|--------|------|
| Quit | 正常退出 (Ctrl+C) |

---

## 7. Handler 层

每个 handler 函数接收 `(app: &mut ChatApp, key: KeyEvent)`，将按键映射为 Action 后调用 `app.update(action)`。

### Handler → Mode 映射

| Handler 函数 | 负责的 ChatMode | 文件 |
|-------------|----------------|------|
| handle_chat_mode | Chat | handler/chat.rs |
| handle_browse_mode | Browse | handler/browse.rs |
| handle_config_mode | Config | handler/config.rs |
| handle_tool_toggle_mode | ToolToggle | handler/config.rs |
| handle_skill_toggle_mode | SkillToggle | handler/config.rs |
| handle_select_model | SelectModel | handler/config.rs |
| handle_archive_confirm_mode | ArchiveConfirm | handler/archive.rs |
| handle_archive_list_mode | ArchiveList | handler/archive.rs |
| handle_tool_confirm_mode | ToolConfirm | handler/tool_confirm.rs |
| Help 模式 | （直接在 mod.rs 中处理） | handler/mod.rs |

### Hybrid Patterns（混合模式说明）

大部分 handler 遵循纯 Action 映射，但以下场景保留了直接状态修改：

1. **chat.rs 弹窗逻辑**: @ 和 @file: 补全弹窗的过滤、选中等涉及复杂字符串操作，部分直接修改 `ui.at_popup_*` / `ui.file_popup_*`
2. **browse.rs**: 退出浏览模式时直接清除 `ui.msg_lines_cache = None`
3. **config.rs**: 工具开关中 `tools_enabled` 的直接切换，退出时直接调用 `save_agent_config()`
4. **tool_confirm.rs**: 输入模式下 `tool_interact_typing`、`tool_interact_input` 的直接修改
5. **archive.rs**: 归档校验函数的直接调用，`restore_confirm_needed` 标志的直接修改

这些是刻意保留的务实决策——纯 Action 化在这些场景下会增加不必要的复杂度。

---

## 8. 后台流式处理

### poll_stream_actions() 详细流程

```
poll_stream_actions() → Vec<Action>
│
├── 1. tool_executor.poll_results()
│      轮询后台工具执行完成的结果
│
├── 2. ask_request_rx.try_recv()
│      检查 Ask 工具是否有新请求 → init_ask_mode()
│
├── 3. 检查 pending_tool_execution
│      ├── 如果有待执行工具:
│      │   ├── 检查 jcli denied 规则 → Failed
│      │   ├── 检查 jcli allowed 规则 → Executing
│      │   └── 其他 → PendingConfirm → 进入 ToolConfirm 模式
│      └── pending_tool_execution = false
│
├── 4. 如果有 Executing 状态工具:
│      tool_executor.execute_batch() → 启动后台线程执行
│
├── 5. 如果所有工具完成:
│      advance() → 下一个 PendingConfirm 或返回 Chat
│      收集 Done 的工具结果 → tool_result_tx.send()
│
└── 6. agent.poll() → Vec<StreamMsg>
       ├── Chunk     → push Action::StreamChunk
       ├── Done      → push Action::StreamDone
       ├── Error(e)  → push Action::StreamError(e)
       ├── Cancelled → push Action::StreamCancelled
       └── ToolCallRequest(calls) → 设置 tool_executor 状态
                                     (不产生 Action，延迟到下一帧)
```

### StreamMsg → Action 映射

| StreamMsg | Action | 备注 |
|-----------|--------|------|
| Chunk | StreamChunk | auto_scroll 设为 true |
| Done | StreamDone | 结束加载，处理排队任务 |
| Error(e) | StreamError(e) | 显示错误 Toast |
| Cancelled | StreamCancelled | 结束加载 |
| ToolCallRequest(calls) | （无立即 Action） | 设置 pending_tool_execution=true，下一帧处理 |

---

## 9. 工具执行流水线

```
LLM 返回 tool_calls
        │
        ▼
poll_stream_actions() 收到 ToolCallRequest
        │
        ▼
设置 active_tool_calls + pending_tool_execution=true
        │
        ▼ (下一帧)
┌───────────────────────────────┐
│ 对每个 tool_call 检查权限:     │
│                               │
│ .jcli denied? ──YES──▶ Failed │
│       │                       │
│      NO                       │
│       │                       │
│ .jcli allowed? ──YES──▶ Executing (自动执行) │
│       │                       │
│      NO                       │
│       │                       │
│ requires_confirmation?        │
│   YES → PendingConfirm       │
│    NO → Executing             │
└───────────────┬───────────────┘
                │
    ┌───────────┴───────────┐
    │                       │
    ▼                       ▼
PendingConfirm          Executing
(进入 ToolConfirm)      (批量执行)
    │                       │
    │ 用户选择:              │ 后台线程
    │ ├─ Continue → Executing│ registry.execute()
    │ ├─ Allow → 写入 .jcli  │     │
    │ │         → Executing  │     ▼
    │ ├─ Refuse → Rejected   │ ToolExecDoneMsg
    │ └─ Timeout → Executing │     │
    │       (如果配置了)      │     ▼
    └───────────────────────┘ poll_results()
                │                  │
                ▼                  ▼
         所有工具完成 → ToolResultMsg → Agent 线程
                                        继续下一轮
```

### ToolExecStatus 状态机

```
PendingConfirm ──┬── Continue ──▶ Executing ──▶ Done(summary)
                 ├── Allow ─────▶ Executing ──▶ Done(summary)
                 ├── Refuse ────▶ Rejected
                 ├── Timeout ───▶ Executing ──▶ Done(summary)
                 └── Cancel ────▶ (reset all)
                                  Executing ──▶ Failed(error)
```

---

## 10. 渲染管线

### draw_chat_ui 布局

```
┌──────────────────────────────────────────┐
│ Title Bar (3 lines)                       │
│ 模型名 | 消息数 | 加载指示器              │
├──────────────────────────────────────────┤
│                                          │
│ Messages Area (Min(5))                   │
│                                          │
│ 按 mode 分发:                            │
│  Chat/Browse/ToolConfirm → draw_messages │
│  Help         → draw_help                │
│  SelectModel  → draw_model_selector      │
│  Config       → draw_config_screen       │
│  ArchiveConfirm → draw_archive_confirm   │
│  ArchiveList  → draw_archive_list        │
│  ToolToggle   → draw_tool_toggle         │
│  SkillToggle  → draw_skill_toggle        │
│                                          │
├──────────────────────────────────────────┤
│ Input Area (5 lines)                      │
│ 输入框 + 光标                             │
├──────────────────────────────────────────┤
│ Hint Bar (1 line)                         │
│ 底部快捷键提示                            │
└──────────────────────────────────────────┘

叠加层 (Overlays):
  ├── Toast (右上角通知)
  ├── @ 补全弹窗
  └── @file: 文件补全弹窗
```

### 消息渲染缓存机制

`MsgLinesCache` 提供增量渲染能力：

```
缓存命中条件（全部满足则复用）:
  • msg_count 不变
  • last_msg_len 不变
  • streaming_len 不变
  • is_loading 不变
  • bubble_max_width 不变（窗口未 resize）
  • browse_index 不变
  • tool_confirm_idx 不变

缓存失效 → 增量重建:
  • per_msg_lines: 按消息粒度缓存，只重建变化的消息
  • streaming_stable_lines: 流式内容的已完成段落缓存
  • streaming_stable_offset: 记录已缓存到的字节位置
```

### prepare_for_render() / prepare_scroll_state()

这两个方法已实现但标记为 `#[allow(dead_code)]`，设计用于在主循环 Phase 3 之前调用，将渲染中的状态变更提前到渲染前：

```
// 未来激活方式:
// Phase 3 之前:
app.prepare_for_render();      // 预计算消息行缓存
let (h, max) = ...;            // 获取可视区域
app.prepare_scroll_state(h, max); // 预调整滚动偏移
// Phase 3:
terminal.draw(|f| draw_chat_ui(f, &app));  // 理想: &app（只读）
```

---

## 11. 跨线程通信

| 通道 | 类型 | 方向 | 用途 |
|------|------|------|------|
| agent.stream_rx | mpsc::Receiver\<StreamMsg\> | Agent→TUI | 流式消息 |
| tool_exec_tx/rx | mpsc::channel\<ToolExecDoneMsg\> | Tool线程→TUI | 工具执行结果 |
| tool_result_tx | mpsc::SyncSender\<ToolResultMsg\> | TUI→Agent | 工具结果回传 |
| ask_response_tx | mpsc::Sender\<String\> | TUI→Agent | Ask 工具答案 |
| ask_request_rx | mpsc::Receiver\<AskRequest\> | Agent→TUI | Ask 工具请求 |

| 共享状态 | 类型 | 用途 |
|----------|------|------|
| streaming_content | Arc\<Mutex\<String\>\> | 实时流式内容 |
| queued_tasks | Arc\<Mutex\<Vec\<String\>\>\> | 排队任务 |
| pending_user_messages | Arc\<Mutex\<Vec\<ChatMessage\>\>\> | 待处理消息 |
| image_cache | Arc\<Mutex\<ImageCache\>\> | 终端图片缓存 |
| tool_cancelled | Arc\<AtomicBool\> | 工具取消标志 |

---

## 12. 关键设计模式

### 模式 1: Redux 单向数据流
所有状态变更经过 `update(Action)` 中心化处理。Handler 只做 KeyEvent→Action 映射。

### 模式 2: Arc\<Mutex\<T\>\> 跨线程共享
`streaming_content`、`queued_tasks`、`pending_user_messages` 通过 Arc\<Mutex\> 在 TUI 线程和 Agent 线程间共享。

### 模式 3: 流式节流
防止过度渲染：只在 `bytes_delta >= 200 || time_elapsed >= 200ms` 时重绘。

### 模式 4: 延迟工具执行
`pending_tool_execution` 标志让工具状态在 Frame N 初始化，Frame N+1 实际执行，确保 UI 一致性。

### 模式 5: 缓存失效
`msg_lines_cache = None` 在任何影响渲染的状态变化时设置。渲染时检查缓存有效性决定是否重建。

### 模式 6: 混合事件模型
- 用户输入 → handler → Action（同步路径）
- 后台事件 → poll_stream_actions() → Action（异步路径）
- 副作用 → flags 在 Phase 5 检查（延迟路径）

### 模式 7: 权限门控
工具执行前三级检查：jcli denied → jcli allowed → requires_confirmation。

---

## 13. 未来改进方向

1. **激活 pre-render 阶段**: 在主循环 Phase 3 前调用 `prepare_for_render()` 和 `prepare_scroll_state()`，将渲染函数签名从 `&mut ChatApp` 改为 `&ChatApp`（受限于 ratatui 的 `render_stateful_widget` API）

2. **消除 Hybrid Patterns**: 将 handler 中残留的直接状态修改逐步迁移到 Action

3. **Action 日志/回放**: 利用 Action 枚举实现状态变更日志或时间旅行调试

4. **状态快照测试**: 基于 Action→update() 的纯函数特性，编写状态转换的单元测试

5. **Handler 纯函数化**: 将 handler 签名改为 `fn(state: &ChatApp, key: KeyEvent) -> Vec<Action>`，完全消除对 `&mut ChatApp` 的依赖
