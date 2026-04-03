# j-cli Visual Architecture & Data Flow

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         TUI Main Loop                           │
│                    (handler/tui_loop.rs)                        │
└────────────────────┬────────────────────────────────────────────┘
                     │
        ┌────────────┼────────────┐
        │            │            │
        ▼            ▼            ▼
   ┌────────┐  ┌──────────┐  ┌───────────┐
   │ Render │  │ Poll     │  │ Handle    │
   │ (UI)   │  │ Backend  │  │ Input     │
   └────────┘  │ (Agent)  │  │ (Keys)    │
               └──────────┘  └───────────┘
```

---

## 1. Auto-Compact Flow

### Three-Layer Strategy

```
┌────────────────────────────────────────────────────────┐
│ Layer 1: micro_compact (Memory-only)                  │
│ - Replaces old tool results with placeholders         │
│ - Location: compact.rs:65-127                         │
│ - Cost: O(n) scanning                                 │
│ - Preserves: Recent 10 tool results by default        │
└────────────────────────────────────────────────────────┘
                        ↓
┌────────────────────────────────────────────────────────┐
│ Layer 2: auto_compact (LLM Summarization)             │
│ - Triggered when: tokens > 204,800                    │
│ - Location: compact.rs:174-246                        │
│ - Cost: 1 LLM call (20k token limit)                  │
│ - Output: Compact 2-message transcript                │
└────────────────────────────────────────────────────────┘
                        ↓
┌────────────────────────────────────────────────────────┐
│ Layer 3: CompactTool (User-triggered)                 │
│ - User/model calls via tool execution                 │
│ - Triggers Layer 2 (auto_compact)                     │
│ - Location: tools/compact.rs                          │
└────────────────────────────────────────────────────────┘
```

### Trigger Points

```
agent_loop (agent.rs:44-56)
    │
    ├─ micro_compact()          [Layer 1]
    │   └─ estimate_tokens() > threshold?
    │       └─ YES → call auto_compact() [Layer 2]
    │           ├─ save_transcript()
    │           ├─ LLM request (create_openai_client)
    │           └─ replace messages with summary
    │
    └─ (Stream sends ToolCallRequest)
        └─ CompactTool detected?
            └─ YES → call auto_compact() [Layer 2]
```

---

## 2. Toast System Architecture

### Data Flow

```
┌──────────────────────────────────────┐
│ Action triggers error/success        │
│ (e.g., clipboard, request fail)      │
└──────────────────────────────────────┘
                │
                ▼
┌──────────────────────────────────────────────────────┐
│ app.show_toast(msg, is_error)                        │
│ (app.rs:2633)                                        │
│                                                      │
│ Sets: ui.toast = Some((msg, is_error, now()))       │
└──────────────────────────────────────────────────────┘
                │
                ▼
        ┌───────────────┐
        │ Each frame    │
        │ (TUI loop)    │
        └───────────────┘
                │
                ▼
┌──────────────────────────────────────────────────────┐
│ draw_toast(f, area, app)                             │
│ (ui/chat.rs:882-926)                                 │
│                                                      │
│ Checks: app.ui.toast.is_some()                       │
│ - Calculates position (top-right corner)             │
│ - Calculates size from text width                    │
│ - Renders with themed colors (error vs success)     │
│ - Creates Paragraph widget with border              │
│ - f.render_widget(toast_area)                        │
└──────────────────────────────────────────────────────┘
                │
                ▼
        ┌───────────────┐
        │ Auto-dismissed│
        │ after timeout │
        │ (TOAST_DUR)   │
        └───────────────┘
```

### Toast Storage

```
ChatApp
  └─ ui: UIState
      └─ toast: Option<(String, bool, Instant)>
              ├─ String:  Message text
              ├─ bool:    true = error, false = success
              └─ Instant: Creation time
```

### Position Calculation

```
┌─────────────────────────────────────────────┐
│ Terminal Area (e.g., 120x40)                │
│                                             │
│ [Top-left]                  [Top-right]    │
│                             ┌───────────┐  │
│ (1,1) →                    │ Toast Box │  │
│                             │ x=width-w │  │
│                             │ y=1       │  │
│                             └───────────┘  │
│                                             │
│ y=40                                        │
└─────────────────────────────────────────────┘

x = area.width - toast_width - 1
y = 1
width = (text_width + 10).min(area.width).max(16)
height = 3
```

---

## 3. Title Bar Status Display

### Component Layout

```
┌──────────────────────────────────────────────────────────────────────┐
│ ┌─────────────────────────────────────────────────────────────────┐  │
│ │ 🦞  Sprite  │  💫  ModelName  │  📬  5 条消息  🔧 执行 tool_x  │  │
│ │             │                 │                   (or 📱 ...)  │  │
│ └─────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
```

### Status Priority Logic

```
is_loading == false
    └─ Display empty

is_loading == true
    ├─ Find Executing tool?
    │   └─ YES: "🔧 执行 {tool_name}..."
    │
    ├─ Find PendingConfirm tool?
    │   └─ YES: "🔧 调用 {tool_name}..."
    │
    └─ Neither?
        └─ "⏳ 思考中..."
```

### Data Source

```
draw_title_bar() [ui/chat.rs:83]
    │
    ├─ app.state.is_loading        [app.rs:260]
    │   ├─ Set true at:   app.rs:2871 (start request)
    │   └─ Set false at:  app.rs:3365 (finish/cancel)
    │
    └─ app.tool_executor.active_tool_calls [app.rs:276]
        └─ Vec<ToolCallStatus>
            └─ Each has .status: ToolExecStatus
```

---

## 4. Tool Rendering Pipeline

### Message Rendering Architecture

```
┌────────────────────────────────────────────────┐
│ build_message_lines_incremental()              │
│ (render_cache.rs:50)                           │
│                                                │
│ Input: ChatApp, bubble_max_width, old_cache    │
│ Output: (msg_start_lines, per_msg_cache,      │
│          streaming_lines, stable_arc, offset)  │
└────────────────────────────────────────────────┘
        │
        │ For each message in session.messages[]
        │
        ├─────────────────────────────────────────┐
        │ Message Type Detection                   │
        └─────────────────────────────────────────┘
        │
        ├─ If role == "user"
        │   └─ render_user_message()
        │
        ├─ If role == "assistant"
        │   ├─ Has tool_calls?
        │   │   └─ render_tool_call_request_msg()
        │   │       └─ Lines: 1038-1120
        │   │           ├─ For each tool:
        │   │           │   ├─ Category icon + name
        │   │           │   ├─ Status icon
        │   │           │   └─ Args (JSON or text)
        │   │           │
        │   │           └─ Expand mode?
        │   │               ├─ YES: Full params
        │   │               └─ NO: 60-char preview
        │   │
        │   └─ Otherwise: render_assistant_message()
        │
        ├─ If role == "tool"
        │   └─ render_tool_result_msg()
        │       └─ Lines: 1165-1274
        │           ├─ Parse label (tool_name, is_error)
        │           ├─ Status header (icon + summary)
        │           │
        │           └─ If expand + content:
        │               ├─ Error? → First 20 lines
        │               ├─ Diff? → Color-coded diff
        │               ├─ Agent? → Tree format
        │               └─ Normal → First 100 lines
        │
        └─ If role == "system"
            └─ render_system_message()
```

### Tool Result Rendering Examples

```
Collapsed Mode:
┌─ Tool Call (request)
│  │ 🌐 fetch_web
│  │   (No details shown)
│  │
│  └─ Tool Result
│     │ 🔧 fetch_web ✅ HTTP 200 OK
│     │   (Only header shown)

Expanded Mode:
┌─ Tool Call (request)
│  │ 🌐 fetch_web ⏳
│  │   url: "https://example.com"
│  │   timeout: 30
│  │
│  └─ Tool Result
│     │ 🔧 fetch_web ✅ HTTP 200 OK
│     │   Content-Type: text/html
│     │   Content-Length: 12345
│     │   [First 100 lines of content...]
│     │   ... (5234 lines total, shown 100)
```

### Special Content Rendering

```
Diff Output:
  ├─ Parse ```diff blocks
  └─ Color code:
      ├─ "-" lines  → t.diff_del (red)
      ├─ "+" lines  → t.diff_add (green)
      └─ "@@" lines → t.diff_header (cyan)

Error Output:
  ├─ Prefix with "Error:" in error color
  ├─ Show first 20 lines only
  └─ Append "... (共 X 行，显示前 20 行)"

Agent Output (nested):
  ├─ Tree format: ├─ first line
  ├─              ├─ second line
  └─              └─ last line (max 30 shown)
```

---

## 5. Tool Execution State Machine

### State Diagram

```
                        ┌─────────────────┐
                        │  (No tool call) │
                        └────────┬────────┘
                                 │
                    ┌────────────▼─────────────┐
                    │ Stream receives          │
                    │ ToolCallRequest          │
                    │ (agent.rs)               │
                    └────────────┬─────────────┘
                                 │
        ┌────────────────────────┼────────────────────────┐
        │                        │                        │
        ▼                        ▼                        ▼
   ┌────────────┐        ┌──────────────┐         ┌──────────────┐
   │ HOOK ABORT │        │ DENY CONFIG  │         │ INITIALIZE   │
   │ (Failed)   │        │ (Failed)     │         │ TOOL         │
   └────┬───────┘        └──────┬───────┘         └──────┬───────┘
        │                       │                        │
        └───────────────────────┼────────────────────────┘
                                │
        ┌───────────────────────▼───────────────────────┐
        │ Needs Confirmation?                           │
        │ (permission + sandbox checks)                 │
        └───────────────────────┬───────────────────────┘
                                │
                    ┌───────────┴──────────┐
                    │                      │
                YES │                      │ NO
                    ▼                      ▼
          ┌──────────────────┐    ┌──────────────────┐
          │  PendingConfirm  │    │   Executing      │
          │  (wait for user) │    │  (run in thread) │
          └────────┬─────────┘    └────────┬─────────┘
                   │                       │
        ┌──────────┴──────────┐            │
        │ User action?        │            │
        └──────────┬──────────┘            │
         ▲         │                       │
         │    ┌────┴──────┬───────┐        │
         │    │           │       │        │
      REJECT  │        CONFIRM   │        │
         │    │           │       │        │
         │    ▼           ▼       ▼        │
         │  ┌─────┐  ┌─────────┐ │        │
         └──│Rejectd│  Execute   │        │
            └─────┘  └─────┬─────┘        │
                           │              │
                    ┌──────▼──────────────┘
                    │
                    ▼
            ┌──────────────┐
            │ Tool Finishes│
            │ (result)     │
            └────────┬─────┘
                     │
            ┌────────▼────────┐
            │ Is Error?        │
            └────────┬────────┘
                     │
         ┌───────────┴─────────┐
         │                     │
        YES                    NO
         │                     │
         ▼                     ▼
    ┌─────────┐          ┌────────┐
    │ Failed  │          │ Done   │
    │(msg)    │          │(msg)   │
    └────┬────┘          └────┬───┘
         │                    │
         └────────┬───────────┘
                  │
         ┌────────▼──────────┐
         │ active_tool_calls │
         │ .clear()          │
         └───────────────────┘
```

### Status Enum

```rust
pub enum ToolExecStatus {
    PendingConfirm,        // User input needed
    Executing,             // Background thread running
    Done(String),          // Success → summary
    Rejected,              // User said no
    Failed(String),        // Error → message
}
```

### Usage in active_tool_calls

```
ToolExecutor {
    active_tool_calls: Vec<ToolCallStatus> [
        {
            tool_call_id: "call_abc123",
            tool_name: "fetch_web",
            arguments: r#"{"url":"https://..."}"#,
            confirm_message: "Run fetch_web with url=https://... ?",
            status: PendingConfirm,
        },
        {
            tool_call_id: "call_def456",
            tool_name: "execute_command",
            arguments: r#"{"command":"ls -la"}"#,
            confirm_message: "Execute: ls -la",
            status: Executing,
        },
        {
            tool_call_id: "call_ghi789",
            tool_name: "read_file",
            arguments: r#"{"path":"/etc/hosts"}"#,
            confirm_message: "",
            status: Done("Read 15 lines".to_string()),
        },
    ],
    pending_tool_idx: 0,
    tools_executing_count: 1,
    // ... more
}
```

---

## File Relationships Map

```
┌──────────────────────────────────────────────────────────────┐
│                        Main Flow                             │
│                                                              │
│  app.rs (ChatApp)                                            │
│    ├─ state: AppState                                        │
│    │   └─ is_loading: bool [2871, 3365]                    │
│    │   └─ session: ChatSession                              │
│    │       └─ messages: Vec<ChatMessage>                    │
│    │                                                         │
│    ├─ ui: UIState                                            │
│    │   └─ toast: Option<(msg, is_error, time)> [139]       │
│    │   └─ mode: ChatMode                                    │
│    │                                                         │
│    ├─ tool_executor: ToolExecutor [273]                     │
│    │   └─ active_tool_calls: Vec<ToolCallStatus> [276]      │
│    │       └─ Each has .status: ToolExecStatus [65]         │
│    │                                                         │
│    └─ show_toast() method [2633]                            │
│        └─ Sets ui.toast                                     │
│                                                              │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│                      Rendering Flow                          │
│                                                              │
│  ui/chat.rs                                                  │
│    ├─ draw_title_bar() [83]                                 │
│    │   └─ Reads: is_loading, active_tool_calls             │
│    │                                                         │
│    └─ draw_toast() [882]                                    │
│        └─ Reads: app.ui.toast                               │
│                                                              │
│  render_cache.rs                                             │
│    ├─ build_message_lines_incremental() [50]               │
│    │   ├─ Reads: session.messages                           │
│    │   └─ Delegates tool rendering:                         │
│    │                                                         │
│    ├─ render_tool_call_request_msg() [1038]                │
│    │   └─ For assistant messages with tool_calls            │
│    │                                                         │
│    └─ render_tool_result_msg() [1165]                       │
│        └─ For tool role messages                            │
│                                                              │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│                    Compaction Flow                           │
│                                                              │
│  agent.rs (run_agent_loop)                                   │
│    ├─ Calls micro_compact() [47]                            │
│    └─ If threshold exceeded, calls:                         │
│        └─ compact.rs: auto_compact() [174]                  │
│            ├─ Saves transcript                              │
│            ├─ Calls LLM for summary                         │
│            └─ Replaces messages                             │
│                                                              │
│  compact.rs                                                  │
│    ├─ CompactConfig [20]                                    │
│    ├─ estimate_tokens() [57]                                │
│    ├─ micro_compact() [65]                                  │
│    └─ auto_compact() [174]                                  │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

