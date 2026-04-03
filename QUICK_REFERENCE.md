# j-cli Quick Reference Guide

## 1. Auto Compact System

### Call Chain
```
agent.rs:45-56 → estimate_tokens() > token_threshold
                ↓
              agent.rs:53 → compact::auto_compact()
                ↓
              compact.rs:174-246 → LLM summarization
```

### Key Functions
- `auto_compact()` - compact.rs:174
- `estimate_tokens()` - compact.rs:57
- `micro_compact()` - compact.rs:65 (Layer 1)

### Config
- Location: `CompactConfig` (compact.rs:20)
- Token threshold: 204,800 (256 * 800)
- Summary max tokens: 20,000

---

## 2. Toast Notifications

### Setup Toast
```rust
// In app.rs - line 2633
pub fn show_toast(&mut self, msg: impl Into<String>, is_error: bool) {
    self.ui.toast = Some((msg.into(), is_error, std::time::Instant::now()));
}
```

### Call It
```rust
self.show_toast("Message here", false);  // success
self.show_toast("Error message", true);  // error
```

### Rendering
- File: ui/chat.rs:882-926
- Position: Top-right corner
- Auto-dismiss: via `TOAST_DURATION_SECS` constant

### Storage
- Field: `app.ui.toast` (app.rs:139)
- Type: `Option<(String, bool, Instant)>` = (message, is_error, created_at)

---

## 3. Title Bar Status

### Loading Status Display
**File**: ui/chat.rs:83-162 (draw_title_bar)

**Priority Order**:
1. Executing tool: "🔧 执行 {name}..."
2. Pending confirm tool: "🔧 调用 {name}..."
3. Thinking: "⏳ 思考中..."
4. Idle: (empty)

### Key Fields
- `app.state.is_loading` - Set to true/false to trigger
- `app.tool_executor.active_tool_calls` - List of current tools

### Status Toggle
- Start: app.rs:2871 → `self.state.is_loading = true`
- End: app.rs:3365 → `self.state.is_loading = false`

---

## 4. Tool Rendering

### Tool Call Request
**File**: render_cache.rs:1038-1120

```rust
pub fn render_tool_call_request_msg(
    tool_calls: &[ToolCallItem],
    bubble_max_width: usize,
    expand: bool,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
)
```

### Tool Result
**File**: render_cache.rs:1165-1274

```rust
pub fn render_tool_result_msg(
    content: &str,
    label: &str,
    bubble_max_width: usize,
    expand: bool,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
)
```

### Rendering Modes
- **Collapsed**: Header only
- **Expanded**: Full content with formatting

### Special Handling
- Diff blocks: Color-coded (+/-)
- Errors: First 20 lines shown
- Agent results: Tree format with ├─/└─

---

## 5. Tool Execution Status

### Enum Definition
**File**: app.rs:46-57

```rust
pub enum ToolExecStatus {
    PendingConfirm,      // Waiting for user approval
    Executing,           // Currently running
    Done(String),        // Completed with summary
    Rejected,            // User rejected
    Failed(String),      // Execution failed
}
```

### Active Tool Calls
**File**: app.rs:273-293 (ToolExecutor struct)

```rust
pub struct ToolExecutor {
    pub active_tool_calls: Vec<ToolCallStatus>,  // Current tools
    pub pending_tool_idx: usize,                 // Current tool index
    pub tools_executing_count: usize,            // How many running
    // ... more fields
}
```

### State Transitions
```
Add to active_tool_calls (app.rs:3221)
    ↓
PendingConfirm or Executing? (app.rs:3227-3229)
    ↓
User confirms? → Set to Executing (app.rs:483)
    ↓
Tool completes → Set to Done/Failed (app.rs:366-368)
```

### Common Queries
```rust
// Find executing tool
active_tool_calls
    .iter()
    .find(|tc| matches!(tc.status, ToolExecStatus::Executing))

// Find pending confirmation
active_tool_calls
    .iter()
    .find(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))

// Any pending?
active_tool_calls
    .iter()
    .any(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))
```

---

## File Index

```
src/command/chat/
├── app.rs                      # Main app state, ToolExecStatus, show_toast()
├── agent.rs                    # Auto-compact trigger logic
├── compact.rs                  # auto_compact(), CompactConfig
├── render_cache.rs             # Tool rendering functions
└── ui/
    └── chat.rs                 # draw_title_bar(), draw_toast()
```

---

## Common Patterns

### Add a new toast
```rust
// In app.rs or wherever ChatApp is accessible
app.show_toast("Your message", is_error);
```

### Check if AI is thinking
```rust
if app.state.is_loading {
    // Show loading indicator
}
```

### Get current tool being executed
```rust
app.tool_executor
    .active_tool_calls
    .iter()
    .find(|tc| matches!(tc.status, ToolExecStatus::Executing))
    .map(|tc| &tc.tool_name)
```

### Trigger auto-compact manually
```rust
compact::auto_compact(&mut messages, &provider).await
```

### Check message cache validity
```rust
// In render_cache.rs
let can_reuse = old_cache
    .map(|c| c.bubble_max_width == bubble_max_width && c.expand_tools == expand)
    .unwrap_or(false)
```

---

## Constants

| Name | Value | Location |
|------|-------|----------|
| Token Threshold | 204,800 | compact.rs:39 |
| Max Summary Tokens | 20,000 | compact.rs:202 |
| Tool Result Truncate | 100 lines | render_cache.rs:1256 |
| Error Result Truncate | 20 lines | render_cache.rs:1231 |
| Toast Duration | ? | constants.rs |

