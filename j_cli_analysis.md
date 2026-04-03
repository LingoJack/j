# j-cli Codebase Analysis: UI, Toast, Auto-Compact, and Tool Calls

## Overview
This document provides a detailed analysis of how key features work in the j-cli codebase:
1. `auto_compact` triggering mechanism
2. Toast notification system
3. Title bar loading/tool status display
4. Tool calls rendering in messages

---

## 1. AUTO_COMPACT: Triggering and Calling

### 1.1 Where `auto_compact` is Called

**File:** `src/command/chat/agent.rs`

**Call Sites:**

1. **Line 53** - Main agent loop, token threshold check:
   ```rust
   if compact::estimate_tokens(&messages) > compact_config.token_threshold {
       if let Err(e) = compact::auto_compact(&mut messages, &provider).await {
           write_error_log("agent_loop", &format!("auto_compact failed: {}", e));
       }
   }
   ```

2. **Line 319** - After tool execution (non-stream fallback):
   ```rust
   if compact_requested && compact_config.enabled {
       let _ = compact::auto_compact(&mut messages, &provider).await;
   }
   ```

3. **Line 402** - After tool execution (stream mode):
   ```rust
   if compact_requested && compact_config.enabled {
       let _ = compact::auto_compact(&mut messages, &provider).await;
   }
   ```

4. **Line 452** - After tool execution (non-stream mode):
   ```rust
   if compact_requested && compact_config.enabled {
       let _ = compact::auto_compact(&mut messages, &provider).await;
   }
   ```

### 1.2 Function Definition

**File:** `src/command/chat/compact.rs`
**Line:** 174-246

```rust
pub async fn auto_compact(
    messages: &mut Vec<ChatMessage>,
    provider: &ModelProvider,
) -> Result<(), String>
```

**What it does:**
1. Saves complete transcript to `.transcripts/` directory with Unix timestamp
2. Calls LLM (non-streaming, max_tokens=20000) to summarize conversation
3. Replaces messages with 2 messages:
   - User message: `[Conversation compressed. Transcript: {path}]\n\n{summary}`
   - Assistant message: `"Understood. I have the context from the summary. Continuing."`

### 1.3 Triggering Mechanism

**File:** `src/command/chat/agent.rs`
**Lines:** 45-57 (Main trigger in agent loop)

The agent loop has a 2-layer compaction system:

```
Layer 1: micro_compact - Replace old tool results with placeholders (free)
↓
Layer 2: if tokens > threshold → auto_compact (costs LLM tokens)
```

**Configuration:**
- **File:** `src/command/chat/compact.rs` lines 18-54
- `enabled`: true (default)
- `token_threshold`: 256 * 800 = 204,800 tokens
- `keep_recent`: 10 (keep recent 10 tool results in micro_compact)

**Layer 1: micro_compact** (Line 47)
- Replaces old tool results > 800 bytes with `"[Previous: used {tool_name}]"`
- Keeps most recent `keep_recent` tool results intact
- Exempts certain tools: LoadSkill, Task, TodoWrite, TodoRead, Ask

---

## 2. TOAST SYSTEM: How Notifications Work

### 2.1 Toast Data Structure

**File:** `src/command/chat/app.rs`
**Lines:** 138-139 (UIState definition)

```rust
pub struct UIState {
    // ...
    /// Toast 通知消息 (内容, 是否错误, 创建时间)
    pub toast: Option<(String, bool, std::time::Instant)>,
    // ...
}
```

### 2.2 Setting Toast Notifications

**File:** `src/command/chat/app.rs`
**Lines:** 2633-2635

```rust
pub fn show_toast(&mut self, msg: impl Into<String>, is_error: bool) {
    self.ui.toast = Some((msg.into(), is_error, std::time::Instant::now()));
}
```

**Usage pattern:**
```rust
app.show_toast("Success message", false);  // Green toast
app.show_toast("Error message", true);     // Red toast
```

### 2.3 Toast Duration and Cleanup

**File:** `src/command/chat/app.rs`
**Lines:** 2715-2721

```rust
pub fn tick_toast(&mut self) {
    if let Some((_, _, created)) = &self.ui.toast
        && created.elapsed().as_secs() >= TOAST_DURATION_SECS
    {
        self.ui.toast = None;
    }
}
```

**Duration constant:**
- **File:** `src/constants.rs` line 57
- `TOAST_DURATION_SECS = 4` seconds

### 2.4 Toast Rendering

**File:** `src/command/chat/ui/chat.rs`
**Lines:** 882-926

```rust
pub fn draw_toast(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    if let Some((ref msg, is_error, _)) = app.ui.toast {
        // Calculate position (top-right corner)
        let text_width = display_width(msg);
        let toast_width = (text_width + 10).min(area.width as usize).max(16) as u16;
        let x = area.width.saturating_sub(toast_width + 1);
        let y: u16 = 1;
        
        // Render with background and border
        let (icon, border_color, text_color) = if is_error {
            ("✖️", t.toast_error_border, t.toast_error_text)
        } else {
            ("☑️", t.toast_success_border, t.toast_success_text)
        };
        
        // Draw toast widget with rounded borders
    }
}
```

**Visual appearance:**
- **Position:** Top-right corner (x offset by 1, y at 1)
- **Success:** ☑️ Green border/text
- **Error:** ✖️ Red border/text
- **Duration:** 4 seconds then disappears

**Polling for expiration:**
- **File:** `src/command/chat/handler/tui_loop.rs` lines 259-261
- Called each frame in TUI loop

---

## 3. TITLE BAR: Loading and Tool Status Display

### 3.1 Title Bar Drawing Function

**File:** `src/command/chat/ui/chat.rs`
**Lines:** 83-150+

```rust
pub fn draw_title_bar(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let model_name = app.active_model_name();
    let msg_count = app.state.session.messages.len();
    
    let loading = if app.state.is_loading {
        // Priority 1: Show executing tool
        // Priority 2: Show pending confirm tool
        // Fallback: Show "⏳ 思考中..."
    } else {
        String::new()
    };
```

### 3.2 Tool Status Display Logic

**File:** `src/command/chat/ui/chat.rs`
**Lines:** 87-109

```rust
let loading = if app.state.is_loading {
    // Priority 1: Find tool in Executing status
    let tool_info = app
        .tool_executor
        .active_tool_calls
        .iter()
        .find(|tc| matches!(tc.status, ToolExecStatus::Executing))
        .map(|tc| format!(" 🔧 执行 {}...", tc.tool_name))
        .or_else(|| {
            // Priority 2: Find tool in PendingConfirm status
            app.tool_executor
                .active_tool_calls
                .iter()
                .find(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))
                .map(|tc| format!(" 🔧 调用 {}...", tc.tool_name))
        });
    if let Some(info) = tool_info {
        info
    } else {
        // Fallback: Generic thinking indicator
        " ⏳ 思考中...".to_string()
    }
} else {
    String::new()
};
```

### 3.3 is_loading Flag

**File:** `src/command/chat/app.rs`
**Lines:** 259-260

```rust
pub struct ChatState {
    // ...
    /// 是否正在等待 AI 回复
    pub is_loading: bool,
    // ...
}
```

**Set/Cleared by:**
- Set to `true` when user sends message (before agent loop starts)
- Set to `false` when agent loop completes or is cancelled

### 3.4 Title Bar Components

The title bar shows:
```
🦞 Sprite  │  💫  {model_name}  │  📬  {msg_count} 条消息  {loading_info}  │  📱 远程已连接
```

---

## 4. TOOL_EXECSTATUS: Enum and States

### 4.1 ToolExecStatus Enum

**File:** `src/command/chat/app.rs`
**Lines:** 44-57

```rust
pub enum ToolExecStatus {
    /// 等待用户确认
    PendingConfirm,
    /// 执行中
    Executing,
    /// 完成（摘要）
    Done(String),
    /// 用户拒绝
    Rejected,
    /// 执行失败
    Failed(String),
}
```

### 4.2 ToolCallStatus Structure

**File:** `src/command/chat/app.rs`
**Lines:** 60-66

```rust
pub struct ToolCallStatus {
    pub tool_call_id: String,
    pub tool_name: String,
    pub arguments: String,
    pub confirm_message: String,
    pub status: ToolExecStatus,
}
```

### 4.3 Active Tool Calls Management

**File:** `src/command/chat/app.rs`
**Lines:** 274-293 (ToolExecutor struct)

```rust
pub struct ToolExecutor {
    /// 当前活跃的工具调用状态列表
    pub active_tool_calls: Vec<ToolCallStatus>,
    /// ToolConfirm 模式中当前待处理工具的索引
    pub pending_tool_idx: usize,
    /// 进入 ToolConfirm 模式的时间（用于超时自动执行）
    pub tool_confirm_entered_at: std::time::Instant,
    // ... other fields
}
```

---

## 5. TOOL CALLS: Rendering in Messages

### 5.1 Tool Call Request Message Rendering

**File:** `src/command/chat/render_cache.rs`
**Lines:** 1038-1120

```rust
pub fn render_tool_call_request_msg(
    tool_calls: &[super::storage::ToolCallItem],
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
    expand: bool,
)
```

**Two rendering modes:**

1. **Expanded mode** (`expand: true`):
   - Shows tool icon + name + status (first line)
   - Shows parameter details (JSON formatted with pretty printing)
   - Multiple lines per tool

2. **Collapsed mode** (`expand: false`):
   - Shows tool icon + name + parameter preview (60 chars max)
   - Single line per tool

**Tool Categories and Icons:**
- Categories defined in `src/command/chat/tools/classification.rs`
- Each category has an emoji icon and color

### 5.2 Tool Result Message Rendering

**File:** `src/command/chat/render_cache.rs`
**Lines:** 1165-1237

```rust
pub fn render_tool_result_msg(
    content: &str,
    label: &str,
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
    expand: bool,
)
```

**Format:**
- First line: `🔧 {tool_name} {status_icon} {summary}`
- Expanded: Shows full content with indentation
- Collapsed: Shows only the first line
- Error results: Displayed with error styling, first 20 lines

### 5.3 Message Rendering Integration

**File:** `src/command/chat/render_cache.rs`
**Lines:** 130-185 (build_message_lines_incremental)

Tool calls are rendered in 3 places:

1. **Tool Call Request** - When LLM wants to call tools (role: assistant)
   ```
   Lines 135-136: render_tool_call_request_msg()
   ```

2. **Tool Result** - When tool returns result (role: tool)
   ```
   Lines 177-184: render_tool_result_msg()
   ```

3. **Tool Confirmation UI** - When awaiting user confirmation
   ```
   Lines 324: render_tool_confirm_area()
   ```

### 5.4 Message Structure in Storage

**File:** `src/command/chat/storage.rs` (ChatMessage)

```rust
pub struct ChatMessage {
    pub role: String,              // "user", "assistant", "tool", "system"
    pub content: String,           // Main content
    pub tool_calls: Option<Vec<ToolCallItem>>,  // For role="assistant" with tool calls
    pub tool_call_id: Option<String>,           // For role="tool" to link to request
    pub images: Option<Vec<(u32, u32, Vec<u8>)>>, // Image data
}

pub struct ToolCallItem {
    pub id: String,                // UUID for the tool call
    pub name: String,              // Tool name (e.g., "bash", "file_edit")
    pub arguments: String,         // JSON arguments string
}
```

### 5.5 Tool Confirmation Area Rendering

**File:** `src/command/chat/render_cache.rs`
**Lines:** 577-876 (render_tool_confirm_area)

Renders the interactive tool confirmation interface:
- Shows all pending tool calls
- Allows user to: Continue, Allow (execute), Refuse, or Type custom input
- Updates status for each tool as user interacts

---

## 6. Message Flow Diagram

```
User Input
    ↓
[TUI Event Handler] → finish_loading() or send_message()
    ↓
[ChatApp::state.is_loading = true]
    ↓
[Spawn Agent Loop in Background]
    ├─→ Layer 1: micro_compact (free memory optimization)
    ├─→ Layer 2: auto_compact if tokens > threshold
    ├─→ Call LLM (streaming or non-streaming)
    ├─→ Receive tool_calls from LLM
    │   ├─→ Send StreamMsg::ToolCallRequest to UI
    │   ├─→ Wait for user confirmation
    │   ├─→ Execute tools (call process_tool_calls)
    │   ├─→ Receive tool results
    │   ├─→ If compact tool called: trigger auto_compact
    │   └─→ Continue loop if more rounds needed
    └─→ Send StreamMsg::Done to UI
        ↓
[TUI Main Loop Renders]
    ├─→ draw_title_bar shows loading status
    ├─→ draw_messages renders tool calls and results
    │   ├─→ Tool call requests (with expand toggle)
    │   ├─→ Tool results (with error styling)
    │   └─→ Tool confirmation area (interactive)
    ├─→ draw_toast shows notifications
    └─→ tick_toast clears old toasts (> 4 sec)
```

---

## 7. Configuration and Constants

### Compaction Configuration
- **File:** `src/command/chat/compact.rs`
- Token threshold: 204,800 (256 * 800)
- Keep recent: 10 tool results
- Micro-compact threshold: 800 bytes

### Toast Configuration
- **Duration:** 4 seconds
- **File:** `src/constants.rs` line 57

### Tool Execution
- **Default max rounds:** 10 rounds per agent loop
- **Tool confirm timeout:** Auto-execute if user inactive

---

## 8. Key Files Reference

| Component | File | Lines |
|-----------|------|-------|
| auto_compact definition | `src/command/chat/compact.rs` | 174-246 |
| auto_compact calls | `src/command/chat/agent.rs` | 53, 319, 402, 452 |
| Toast struct | `src/command/chat/app.rs` | 138-139 |
| show_toast method | `src/command/chat/app.rs` | 2633-2635 |
| tick_toast method | `src/command/chat/app.rs` | 2715-2721 |
| draw_toast function | `src/command/chat/ui/chat.rs` | 882-926 |
| draw_title_bar function | `src/command/chat/ui/chat.rs` | 83-150+ |
| Tool status logic | `src/command/chat/ui/chat.rs` | 87-109 |
| ToolExecStatus enum | `src/command/chat/app.rs` | 44-57 |
| ToolCallStatus struct | `src/command/chat/app.rs` | 60-66 |
| ToolExecutor struct | `src/command/chat/app.rs` | 274-293 |
| render_tool_call_request_msg | `src/command/chat/render_cache.rs` | 1038-1120 |
| render_tool_result_msg | `src/command/chat/render_cache.rs` | 1165-1237 |
| Message rendering integration | `src/command/chat/render_cache.rs` | 130-185 |
