# j-cli Quick Reference Guide

## 🎯 Quick Answer Guide

### Q1: How is `auto_compact` triggered?
- **When:** When token count exceeds 204,800 tokens (256 * 800)
- **Where called:** 4 places in `agent.rs` (lines 53, 319, 402, 452)
- **What it does:** 
  1. Saves transcript to `.transcripts/` 
  2. Calls LLM to summarize
  3. Replaces all messages with summary

### Q2: How does the toast system work?
- **Storage:** `app.ui.toast = Some((msg, is_error, timestamp))`
- **Show:** `app.show_toast("msg", false/true)` in `app.rs:2633`
- **Clear:** `tick_toast()` clears after 4 seconds `app.rs:2715`
- **Render:** `draw_toast()` in top-right corner `ui/chat.rs:882`

### Q3: How does title bar show loading status?
- **Checks:** `app.state.is_loading` flag
- **Priority 1:** Show executing tool: "🔧 执行 {tool_name}..."
- **Priority 2:** Show pending tool: "🔧 调用 {tool_name}..."
- **Fallback:** "⏳ 思考中..." generic indicator

### Q4: How are tool calls displayed?
- **Request messages:** `render_tool_call_request_msg()` in `render_cache.rs:1038`
  - Expanded: Full details with JSON
  - Collapsed: Name + 60-char preview
- **Result messages:** `render_tool_result_msg()` in `render_cache.rs:1165`
  - Shows "🔧 {name} {status} {summary}"
  - Errors shown with red styling
  - Full content optional (expand mode)

### Q5: Where are ToolExecStatus and active_tool_calls?
- **Enum:** `app.rs:44-57` (5 states: PendingConfirm, Executing, Done, Rejected, Failed)
- **Struct:** `app.rs:60-66` (ToolCallStatus with tool_name, arguments, status)
- **Active list:** `app.rs:276` in ToolExecutor (Vec<ToolCallStatus>)

---

## 🗂️ File Structure

```
src/command/chat/
├── app.rs
│   ├── ToolExecStatus enum (line 44)
│   ├── ToolCallStatus struct (line 60)
│   ├── UIState toast field (line 138)
│   ├── ChatState is_loading (line 259)
│   ├── ToolExecutor active_tool_calls (line 276)
│   ├── show_toast() (line 2633)
│   └── tick_toast() (line 2715)
├── agent.rs
│   ├── auto_compact call 1 - tokens check (line 53)
│   ├── auto_compact call 2 - after tool (line 319)
│   ├── auto_compact call 3 - after tool (line 402)
│   ├── auto_compact call 4 - after tool (line 452)
│   └── process_tool_calls() flow (line 587)
├── compact.rs
│   ├── CompactConfig struct (line 18)
│   ├── micro_compact() (line 65)
│   ├── save_transcript() (line 130)
│   └── auto_compact() (line 174)
├── ui/chat.rs
│   ├── draw_title_bar() (line 83)
│   ├── Tool status logic (line 87)
│   └── draw_toast() (line 882)
├── render_cache.rs
│   ├── build_message_lines_incremental() (line 50)
│   ├── Tool call rendering (line 135)
│   ├── Tool result rendering (line 177)
│   ├── render_tool_call_request_msg() (line 1038)
│   └── render_tool_result_msg() (line 1165)
├── handler/tui_loop.rs
│   └── Toast polling (line 259)
└── constants.rs
    └── TOAST_DURATION_SECS = 4 (line 57)
```

---

## 🔄 Data Flow

### Message Lifecycle with Tool Calls

```
1. User sends message
   └─→ app.state.is_loading = true
   
2. Background agent loop starts
   └─→ Calls LLM
   
3. LLM returns tool_calls
   └─→ agent.rs:639 sends StreamMsg::ToolCallRequest(tool_items)
   └─→ UI receives and enters ToolConfirm mode
   
4. User confirms tool execution
   └─→ ToolExecutor marks tool as Executing
   └─→ Tool status = ToolExecStatus::Executing
   
5. Tool execution completes
   └─→ ToolExecDoneMsg received
   └─→ poll_results() updates ToolCallStatus.status
   └─→ Tool results added to messages as role="tool"
   
6. UI renders
   └─→ draw_title_bar checks active_tool_calls status
   └─→ draw_messages calls render_tool_call_request_msg()
   └─→ Then calls render_tool_result_msg() for results
```

---

## 📊 State Machine: ToolExecStatus

```
                    ┌─────────────────┐
                    │  PendingConfirm │
                    └────────┬────────┘
                             │
                   ┌─────────┼─────────┐
                   │         │         │
                   ▼         ▼         ▼
            ┌─────────┐  ┌──────┐  ┌────────┐
            │ Rejected│  │Cancel │  │Executing│
            └─────────┘  └──────┘  └────┬───┘
                                        │
                            ┌───────────┼───────────┐
                            ▼           ▼           ▼
                    ┌──────────┐  ┌────────────┐
                    │Done(str) │  │Failed(str) │
                    └──────────┘  └────────────┘
```

---

## 🎨 UI Rendering Stack

```
draw_chat_ui() [ui/chat.rs:22]
├─ draw_title_bar() [line 40]
│  └─ Shows: "🔧 {tool_name}..." if app.state.is_loading
│     └─ Checks: active_tool_calls[*].status
├─ draw_messages() [line 49]
│  └─ build_message_lines_incremental() [render_cache.rs:50]
│     ├─ For each message:
│     │  ├─ role="assistant" with tool_calls
│     │  │  └─ render_tool_call_request_msg() [line 1038]
│     │  ├─ role="tool"
│     │  │  └─ render_tool_result_msg() [line 1165]
│     │  └─ role="user" / other content
│     └─ render_tool_confirm_area() if in ToolConfirm mode [line 577]
├─ draw_input() [line 53]
├─ draw_hint_bar() [line 56]
└─ draw_toast() [line 59]
   └─ Shows: ☑️ green or ✖️ red box (top-right)
   └─ Auto-clears: after 4 seconds [tick_toast()]
```

---

## 💾 Data Structures

### Toast Notification
```rust
app.ui.toast: Option<(
    msg: String,           // Notification text
    is_error: bool,        // Success (false) or Error (true)
    created: Instant,      // Timestamp
)>

// Set: app.show_toast("message", is_error)
// Clear: After 4 seconds or manually set to None
```

### Tool Call Execution Status
```rust
app.tool_executor.active_tool_calls: Vec<ToolCallStatus>
    ├─ tool_call_id: String      // UUID
    ├─ tool_name: String         // "bash", "file_edit", etc.
    ├─ arguments: String         // JSON
    ├─ confirm_message: String   // Description for user
    └─ status: ToolExecStatus    // Current execution state
        ├─ PendingConfirm        // Awaiting user confirmation
        ├─ Executing            // Running now
        ├─ Done(summary)        // Completed successfully
        ├─ Rejected             // User rejected
        └─ Failed(summary)      // Execution error
```

### Message with Tool Calls
```rust
ChatMessage {
    role: "assistant",
    content: "Here's what I'll do...",  // Can be empty
    tool_calls: Some(vec![
        ToolCallItem {
            id: "call_abc123",
            name: "bash",
            arguments: r#"{"command":"ls"}"#,
        },
        // ...
    ]),
    tool_call_id: None,
    images: None,
}

// Followed by tool result:
ChatMessage {
    role: "tool",
    content: "file1.txt\nfile2.txt",
    tool_calls: None,
    tool_call_id: Some("call_abc123"),
    images: None,
}
```

---

## ⚙️ Configuration

### Compaction Settings
```rust
CompactConfig {
    enabled: true,                    // default
    token_threshold: 256 * 800,       // ~204K tokens
    keep_recent: 10,                  // tool results to preserve
}

micro_compact triggers when:
    - Tool result > 800 bytes (MICRO_COMPACT_BYTES_COUNT_THRESHOLD)
    - Keep most recent 10, compress older ones
    - Exempt tools: LoadSkill, Task, TodoWrite, TodoRead, Ask

auto_compact triggers when:
    - Total tokens > threshold
    - Saves transcript to disk
    - Calls LLM for summary
    - Graceful degradation on error
```

### Toast Settings
```rust
TOAST_DURATION_SECS = 4  // Auto-dismiss after 4 seconds
toast_position = top-right corner
toast_padding = 1 char from edge
toast_icons:
    - Success (false): "☑️"
    - Error (true): "✖️"
```

---

## 🔍 Debug Tips

### Find auto_compact calls:
```bash
grep -n "auto_compact" src/command/chat/agent.rs
# Lines: 53, 319, 402, 452
```

### Find toast usage:
```bash
grep -n "show_toast\|\.toast\s*=" src/command/chat/app.rs
# Lines: 2634 (set), 2719 (clear)
```

### Find tool rendering:
```bash
grep -n "render_tool" src/command/chat/render_cache.rs
# Lines: 135-136 (request), 177-184 (result)
```

### Check loading status:
```bash
grep -n "is_loading" src/command/chat/ui/chat.rs
# Lines: 87, 202, 223, 249, 273, 546, 699, 710, 747
```

---

## 📝 Common Patterns

### Show notification to user:
```rust
self.show_toast("Operation successful", false);  // Green
self.show_toast("Error occurred", true);         // Red
```

### Check if loading:
```rust
if app.state.is_loading {
    // Show loading indicator
}
```

### Access active tools:
```rust
for tool_call in &app.tool_executor.active_tool_calls {
    if matches!(tool_call.status, ToolExecStatus::Executing) {
        // Tool is running
    }
}
```

### Render tool info:
```rust
let expand = app.ui.expand_tools;  // Toggle with Ctrl+O
render_tool_call_request_msg(tool_calls, bubble_width, lines, theme, expand);
```

