# j-cli Visual Architecture Guide

## 🎬 Complete Request-Response Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ USER INPUT: Types message and presses Enter                                 │
└──────────────────────────────┬──────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ MAIN THREAD (UI Loop)                                                       │
│                                                                              │
│ 1. Set app.state.is_loading = true                                          │
│ 2. Spawn agent_loop in background                                           │
│ 3. Each frame: Render UI                                                    │
└──────────────────────────────┬──────────────────────────────────────────────┘
                               │
                    ┌──────────┴──────────┐
                    │                     │
                    ▼                     ▼
        ┌──────────────────────┐   ┌─────────────────────┐
        │ BACKGROUND THREAD    │   │ UI RENDERING        │
        │ (agent_loop)         │   │ (draw_chat_ui)      │
        │                      │   │                     │
        │ 1. micro_compact()   │   │ - draw_title_bar()  │
        │ 2. Check tokens      │   │   └─ Show loading   │
        │ 3. if > threshold    │   │     status          │
        │    auto_compact()    │   │                     │
        │ 4. Call LLM          │   │ - draw_messages()   │
        │                      │   │   └─ Render tools   │
        │ ✓ NO TOOLS          │   │                     │
        │    → Add text msg    │   │ - draw_toast()      │
        │    → Send Done       │   │   └─ Show notifs    │
        │                      │   │                     │
        │ ✗ TOOLS RETURNED    │   │ - tick_toast()      │
        │    → Add tool_calls  │   │   └─ Expire old     │
        │    → Send Tool       │   │                     │
        │      Request         │   └─────────────────────┘
        │    → WAIT for user   │
        │      confirmation    │
        └──────────────────────┘
                    │
                    ▼
        ┌──────────────────────┐
        │ TOOL CONFIRMATION UI │
        │                      │
        │ User sees:           │
        │ - Tool name          │
        │ - Parameters         │
        │ - Actions:           │
        │  Continue / Allow    │
        │  Refuse / Type       │
        └──────────────────────┘
                    │
        ┌───────────┴───────────┐
        │ User chooses          │
        ▼                       ▼
    ┌────────┐           ┌─────────┐
    │ REFUSE │           │ EXECUTE │
    └────────┘           └────┬────┘
        │                     │
        │                     ▼
        │            ┌──────────────────────┐
        │            │ Execute tools in     │
        │            │ background threads   │
        │            │                      │
        │            │ poll_results() gets: │
        │            │ ToolExecDoneMsg      │
        │            │                      │
        │            │ Update:              │
        │            │ active_tool_calls    │
        │            │ .status = Done/Failed│
        │            └──────────┬───────────┘
        │                       │
        │            ┌──────────┴────────────┐
        │            │                       │
        │            ▼                       ▼
        │     ┌────────────┐        ┌──────────────┐
        │     │ Add result │        │ Check: was   │
        │     │ as role=   │        │ CompactTool? │
        │     │ "tool" msg │        │              │
        │     └────────────┘        │ if YES →     │
        │            │              │ auto_compact │
        │            │              └──────┬───────┘
        │            └──────────┬──────────┘
        │                       │
        │       ┌───────────────┴────────────────┐
        │       │ More rounds?                   │
        │       │ (max 10 or user input?)        │
        │       ▼                                ▼
        │    Continue loop              Send Done to UI
        │    (back to LLM call)
        │
        └──────────────────────────┐
                                   │
                                   ▼
                        ┌──────────────────┐
                        │ app.state.       │
                        │ is_loading =     │
                        │ false            │
                        └──────────────────┘
```

---

## 🔄 Auto-Compact Decision Tree

```
┌─────────────────────────────────────────────────────┐
│ Each Agent Loop Iteration                           │
└────────────────┬────────────────────────────────────┘
                 │
                 ▼
        ┌─────────────────────┐
        │ micro_compact()     │
        │ enabled: true       │
        │                     │
        │ Process:            │
        │ • Find all tool     │
        │   results > 800 B   │
        │ • Keep recent 10    │
        │ • Replace old with: │
        │  "[Previous: used   │
        │   {tool_name}]"    │
        └────────┬────────────┘
                 │
                 ▼
        ┌──────────────────────────┐
        │ estimate_tokens()        │
        │ (messages / 4)           │
        │                          │
        │ if tokens >              │
        │   token_threshold        │
        │   (204,800)              │
        └────────┬─────────────────┘
                 │
         ┌───────┴───────┐
         │               │
      NO │               │ YES
         │               │
         ▼               ▼
      ┌────┐      ┌────────────────────────┐
      │Done│      │ auto_compact()         │
      └────┘      │                        │
                  │ 1. save_transcript()   │
                  │    → .transcripts/     │
                  │       dir             │
                  │ 2. Call LLM summarize  │
                  │    (non-stream,        │
                  │     max 20K tokens)    │
                  │ 3. Replace msgs with:  │
                  │    - User: [Compressed│
                  │      Transcript: path] │
                  │      + summary         │
                  │    - Assistant:        │
                  │      "Understood..."   │
                  │ 4. Continue loop       │
                  │ (lower token count)    │
                  └────────────────────────┘
```

---

## 🎨 UI Layer Rendering Order

```
┌─────────────────────────────────────────────────────────────┐
│ Screen Layout (draw_chat_ui)                                │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ TITLE BAR (Height: 3)                                │   │
│  │ 🦞 Sprite │ 💫 gpt-4 │ 📬 42 条消息 🔧 执行 bash... │   │
│  │ └─ Contains: Model name, message count, loading info │   │
│  │    └─ if is_loading: shows tool status from         │   │
│  │       active_tool_calls[].status                     │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ MESSAGES AREA (Min Height: 5)                        │   │
│  │                                                       │   │
│  │ ┌─ MESSAGE 1: User text                           ┐  │   │
│  │ │  "How do I list files?"                         │  │   │
│  │ └────────────────────────────────────────────────┘  │   │
│  │                                                       │   │
│  │ ┌─ MESSAGE 2: Assistant tool call request        ┐  │   │
│  │ │                                                 │  │   │
│  │ │ (render_tool_call_request_msg)                 │  │   │
│  │ │ 📂 Bash ▶️                                      │  │   │
│  │ │   command: ls -la                              │  │   │
│  │ │   cwd: /home/user                              │  │   │
│  │ │                                                 │  │   │
│  │ └────────────────────────────────────────────────┘  │   │
│  │                                                       │   │
│  │ ┌─ MESSAGE 3: Tool execution confirmation       ┐  │   │
│  │ │ (render_tool_confirm_area - interactive)      │  │   │
│  │ │                                                 │  │   │
│  │ │ Continue  [Allow] Refuse Type                  │  │   │
│  │ │                                                 │  │   │
│  │ └────────────────────────────────────────────────┘  │   │
│  │                                                       │   │
│  │ ┌─ MESSAGE 4: Tool result                        ┐  │   │
│  │ │                                                 │  │   │
│  │ │ (render_tool_result_msg)                       │  │   │
│  │ │ 🔧 Bash ✓ Output (10 lines)                    │  │   │
│  │ │   drwxr-xr-x  12 user  staff    384 Apr 3 14:34 │  │   │
│  │ │   -rw-r--r--   1 user  staff    214 Apr 2 09:12 │  │   │
│  │ │   ...                                           │  │   │
│  │ │                                                 │  │   │
│  │ └────────────────────────────────────────────────┘  │   │
│  │                                                       │   │
│  │ ┌─ MESSAGE 5: Assistant response                ┐  │   │
│  │ │  "I've listed the files. Here's what I see..."│  │   │
│  │ └────────────────────────────────────────────────┘  │   │
│  │                                                       │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ INPUT AREA (Height: 5)                               │   │
│  │ > _                                                   │   │
│  │ (Text cursor here)                                   │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │ HINT BAR (Height: 1)                                 │   │
│  │ Esc: Cancel | Enter: Send | Ctrl+O: Toggle Tools     │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌────────────────────────────────────────────┐  (overlay)  │
│  │ ☑️ Changes saved to session                │  ← Toast    │
│  └────────────────────────────────────────────┘  (top-right)│
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 📊 Tool Call Rendering Logic

```
build_message_lines_incremental()
    │
    ├─ For each message in session.messages:
    │
    ├─ if role == "assistant" && has tool_calls
    │   │
    │   └─ render_tool_call_request_msg(tool_calls, width, lines, theme, expand)
    │      │
    │      ├─ if expand == true
    │      │   │
    │      │   └─ For each tool:
    │      │       ├─ Line 1: icon + name + status
    │      │       │         🔧 Bash ⏳
    │      │       ├─ Lines 2+: Full parameter details
    │      │       │           command: "ls -la"
    │      │       │           cwd: "/home/user"
    │      │       └─ Blank line between tools
    │      │
    │      └─ if expand == false
    │          │
    │          └─ For each tool (single line):
    │              icon + name + 60-char args preview
    │              🔧 Bash ls -la /home/user /var/lo…
    │
    ├─ if role == "tool"
    │   │
    │   └─ Find tool_name from tool_call_id
    │      (search backwards for matching assistant msg)
    │      │
    │      └─ render_tool_result_msg(content, tool_name, width, lines, theme, expand)
    │         │
    │         ├─ if expand == true
    │         │   │
    │         │   └─ Line 1: 🔧 {tool_name} {status_icon} {summary}
    │         │      Lines 2+: Full output (indented)
    │         │              drwxr-xr-x  12 user  staff  384 Apr 3 14:34
    │         │              -rw-r--r--   1 user  staff  214 Apr 2 09:12
    │         │              ...
    │         │
    │         └─ if expand == false
    │             │
    │             └─ Line 1 only: 🔧 {tool_name} {status_icon} {summary}
    │
    ├─ if in ToolConfirm mode
    │   │
    │   └─ render_tool_confirm_area()
    │      Interactive user confirmation UI
    │
    └─ Process continues for all messages
```

---

## 🔌 Data Structure Relationships

```
ChatApp
├─ state: ChatState
│  ├─ session: ChatSession
│  │  └─ messages: Vec<ChatMessage>
│  │     ├─ role: "user" / "assistant" / "tool" / "system"
│  │     ├─ content: String (main text)
│  │     ├─ tool_calls: Option<Vec<ToolCallItem>>
│  │     │  ├─ id: "call_abc123"
│  │     │  ├─ name: "bash" / "file_edit" / ...
│  │     │  └─ arguments: JSON string
│  │     ├─ tool_call_id: Option<String> (for role="tool")
│  │     └─ images: Option<Vec<...>>
│  │
│  ├─ is_loading: bool (true while waiting for agent)
│  ├─ streaming_content: Arc<Mutex<String>>
│  └─ agent_config: AgentConfig
│
├─ ui: UIState
│  ├─ toast: Option<(
│  │   msg: String,
│  │   is_error: bool,
│  │   created: Instant
│  │)>
│  ├─ mode: ChatMode
│  │  ├─ Chat
│  │  ├─ ToolConfirm
│  │  ├─ Browse
│  │  └─ ...
│  ├─ expand_tools: bool (Ctrl+O toggle)
│  ├─ theme: Theme
│  │  ├─ toast_success_bg: Color
│  │  ├─ toast_error_bg: Color
│  │  ├─ title_loading: Color
│  │  └─ ... (many more colors)
│  └─ ...
│
└─ tool_executor: ToolExecutor
   ├─ active_tool_calls: Vec<ToolCallStatus>
   │  ├─ tool_call_id: String
   │  ├─ tool_name: String
   │  ├─ arguments: String
   │  ├─ confirm_message: String
   │  └─ status: ToolExecStatus
   │     ├─ PendingConfirm
   │     ├─ Executing
   │     ├─ Done(String)      ← Contains summary
   │     ├─ Failed(String)    ← Contains error message
   │     └─ Rejected
   ├─ pending_tool_idx: usize
   ├─ tools_executing_count: usize
   ├─ tool_exec_rx: Option<mpsc::Receiver<ToolExecDoneMsg>>
   └─ tool_result_tx: Option<mpsc::SyncSender<ToolResultMsg>>
```

---

## 🌊 Message Flow: From LLM Response to Rendering

```
1. LLM RESPONSE
   ├─ Content: "I'll run ls for you"
   ├─ ToolCalls:
   │  └─ { id: "call_123", name: "bash", arguments: "{...}" }
   └─ finish_reason: tool_calls
      
2. AGENT LOOP (agent.rs:387-407)
   ├─ Split into 2 messages if has both text + tools:
   │  ├─ Message A: role=assistant, content="I'll run ls", tool_calls=None
   │  └─ Message B: role=assistant, content="", tool_calls=[...] ← This one!
   │
   └─ Send StreamMsg::ToolCallRequest(tool_items)
      
3. UI RECEIVES (app.rs)
   ├─ Switch to ChatMode::ToolConfirm
   ├─ Populate ToolExecutor::active_tool_calls
   └─ Each tool status = PendingConfirm
      
4. USER INTERACTS (handler/*)
   ├─ If allows: status = Executing
   └─ Tool runs in background thread
      
5. TOOL COMPLETES (app.rs:312-392)
   ├─ receive ToolExecDoneMsg
   ├─ Update active_tool_calls status
   │  └─ Done(summary) OR Failed(error)
   │
   └─ Add to messages as role="tool"
      └─ { role: "tool", content: "output...", tool_call_id: "call_123" }
      
6. NEXT AGENT LOOP
   ├─ Add tool result to messages
   ├─ Check if token limit exceeded
   └─ If compact tool was called → auto_compact()
      
7. UI RENDERING (ui/chat.rs + render_cache.rs)
   ├─ For Message B (tool_calls):
   │  └─ render_tool_call_request_msg()
   │     └─ 🔧 Bash ▶️
   │        command: ls
   │
   └─ For Message C (tool result):
      └─ render_tool_result_msg()
         └─ 🔧 Bash ✓
            (contents shown if expanded)
```

---

## ⏱️ Toast Lifecycle

```
Trigger Event
    │
    ▼
┌──────────────────────────────┐
│ app.show_toast(msg, is_error)│  (app.rs:2633)
│                              │
│ Sets:                        │
│ app.ui.toast = Some((        │
│   msg.into(),               │
│   is_error,                 │
│   Instant::now()            │
│ ))                          │
└──────┬───────────────────────┘
       │
       ▼
  [Rendered in draw_toast()]
  Top-right corner of screen
  Width: auto (content + 10)
  Height: 3
  
  ┌─ Success ──────────────┐
  │ ☑️ Message            │
  └──────────────────────┘
       (Green theme)
  
  or
  
  ┌─ Error ────────────────┐
  │ ✖️ Error message      │
  └──────────────────────┘
       (Red theme)
       │
       ▼
 [Each frame]
 app.tick_toast() (app.rs:2715)
       │
       if elapsed() >= 4 seconds
       │
       ▼
 app.ui.toast = None
       │
       ▼
  [Toast disappears]
```

---

## 🔀 ToolExecStatus Transitions

```
        User sends request
              │
              ▼
    ┌─────────────────────┐
    │  PendingConfirm     │
    │  (Awaiting user)    │
    └────────┬────────────┘
             │
             ├─────────────┬──────────────┬──────────────┐
             │             │              │              │
         User allows    Timeout       User refuses   (other)
             │             │              │              │
             ▼             ▼              ▼              ▼
    ┌────────────┐ ┌─────────────┐ ┌──────────┐   ┌──────────┐
    │ Executing  │ │ Executing   │ │ Rejected │   │(reserved)│
    │            │ │ (auto)      │ │          │   │          │
    └─────┬──────┘ └─────────────┘ └──────────┘   └──────────┘
          │
          ├─────────────────┬────────────────┐
          │                 │                │
      Tool passes      Tool fails       User cancels
          │                 │                │
          ▼                 ▼                ▼
    ┌────────────┐  ┌──────────────┐  ┌──────────┐
    │ Done(str)  │  │ Failed(str)  │  │ Rejected │
    │            │  │              │  │          │
    │ {summary}  │  │ {error msg}  │  │          │
    └────────────┘  └──────────────┘  └──────────┘
         │                 │
         └─────────┬───────┘
                   │
              UI shows
              ✓ or ✖️ icon
              in title bar
              & messages
```

