# j-cli Codebase Architecture Analysis

## Overview
This document provides a comprehensive guide to understanding five key architectural components of the j-cli chat system, focusing on files in `src/command/chat/`.

---

## 1. Auto Compact Trigger & Call Chain

### How auto_compact is Triggered

The `auto_compact` function implements **Layer 2** of a 3-layer context compaction strategy. It's triggered in these scenarios:

#### Primary Trigger - Automatic Token Threshold (agent.rs)
- **File**: `src/command/chat/agent.rs`
- **Lines**: 44-56
- **Trigger Logic**:
  ```
  Before each AI request:
  1. micro_compact() → Layer 1 (replaces old tool results)
  2. Check if token estimate > token_threshold
  3. If yes: call auto_compact() → Layer 2 (LLM summary)
  ```

#### Call Sites for auto_compact()

1. **Automatic threshold check** (Lines 44-56)
   - Located: `src/command/chat/agent.rs:53`
   - Triggered when: `compact::estimate_tokens(&messages) > compact_config.token_threshold`
   - Logs: "auto_compact triggered (token threshold exceeded)"

2. **Manual compact tool trigger** (Lines 316-322, 399-405, 449-455)
   - Located: `src/command/chat/agent.rs:319, :402, :452`
   - Triggered when: User/model calls CompactTool
   - Check: `if compact_requested && compact_config.enabled`

### auto_compact Implementation

- **File**: `src/command/chat/compact.rs`
- **Lines**: 174-246
- **Function Signature**:
  ```rust
  pub async fn auto_compact(
      messages: &mut Vec<ChatMessage>,
      provider: &ModelProvider,
  ) -> Result<(), String>
  ```

**Steps**:
1. **Save Transcript** (Line 179)
   - Saves full conversation to `.jcli/transcripts/transcript_<timestamp>.jsonl`
   - Uses `save_transcript()` helper

2. **Build Summary Prompt** (Lines 182-192)
   - Truncates conversation to 80,000 chars
   - Includes context preservation instruction for skills/workflows

3. **LLM Call** (Lines 206-212)
   - Non-streaming request to LLM
   - `max_tokens=20000`
   - Uses `create_openai_client(provider)`

4. **Replace Messages** (Lines 225-243)
   - Clears original messages
   - Replaces with 2 messages:
     - User message: "[Conversation compressed. Transcript: ...]\n\n{summary}"
     - Assistant message: "Understood. I have the context from the summary. Continuing."

### Configuration

**File**: `src/command/chat/compact.rs:18-54`

```rust
pub struct CompactConfig {
    pub enabled: bool,                    // default: true
    pub token_threshold: usize,           // default: 256 * 800 = 204,800 tokens
    pub keep_recent: usize,               // default: 10 (recent tool results to keep)
}
```

---

## 2. Toast System

### Toast Storage & Initialization

- **File**: `src/command/chat/app.rs`
- **Structure**: `UIState` (Lines 128-160)
- **Toast Field** (Line 139):
  ```rust
  pub toast: Option<(String, bool, std::time::Instant)>,
  // Tuple: (message, is_error, creation_time)
  ```
- **Initialization** (Line 1321): `toast: None`

### Show Toast Function

- **File**: `src/command/chat/app.rs`
- **Lines**: 2632-2635
- **Function**:
  ```rust
  pub fn show_toast(&mut self, msg: impl Into<String>, is_error: bool) {
      self.ui.toast = Some((msg.into(), is_error, std::time::Instant::now()));
  }
  ```

### Toast Trigger Points

Usage throughout app.rs:
1. **Request Failures** (Line 1597): `self.show_toast(format!("请求失败: {}", e), true);`
2. **Clipboard Operations**:
   - Line 1902: Success - "已复制第 N 条消息"
   - Line 1911: Failure - "复制到剪切板失败"
3. Various other success/error scenarios

### Toast Rendering

- **File**: `src/command/chat/ui/chat.rs`
- **Lines**: 882-926
- **Function**: `pub fn draw_toast(f: &mut ratatui::Frame, area: Rect, app: &ChatApp)`

**Layout**:
- **Position**: Top-right corner (overlapping layer)
- **Size**: `(text_width + 10).min(area.width).max(16)` x `3` height
- **Location Calculation**: `x = area.width - toast_width - 1`, `y = 1`

**Styling**:
- **Error Toast**:
  - Icon: ✖️
  - Background: `t.toast_error_bg`
  - Border: `t.toast_error_border`
  - Text: `t.toast_error_text`
- **Success Toast**:
  - Icon: ☑️
  - Background: `t.toast_success_bg`
  - Border: `t.toast_success_border`
  - Text: `t.toast_success_text`

**Rendering Code** (Lines 908-922):
```rust
let toast_widget = Paragraph::new(Line::from(vec![
    Span::styled(format!(" {} ", icon), Style::default()),
    Span::styled(msg.as_str(), Style::default().fg(text_color)),
]))
.block(
    Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(background))
);
f.render_widget(toast_widget, toast_area);
```

**Call Site**:
- **File**: `src/command/chat/ui/chat.rs`
- **Line**: 59 (in `pub fn draw_chat_ui()`)
- **Function Call**: `draw_toast(f, size, app);`

---

## 3. Title Bar Loading/Tool Status Display

### Title Bar Structure

- **File**: `src/command/chat/ui/chat.rs`
- **Lines**: 83-162
- **Function**: `pub fn draw_title_bar(f: &mut ratatui::Frame, area: Rect, app: &ChatApp)`

### Components Displayed

1. **Application Icon & Name** (Lines 111-118)
   - Icon: 🦞
   - Text: "Sprite"

2. **Model Name** (Lines 120-126)
   - Icon: 💫
   - Shows active model (bold, title_model color)

3. **Message Count** (Lines 128-131)
   - Format: "📬 N 条消息"

4. **Loading/Tool Status** (Lines 87-109)
   - **Priority 1**: Show executing tool
   - **Priority 2**: Show pending confirm tool
   - **Fallback**: Show "⏳ 思考中..." (thinking)

### Loading/Tool Status Logic

**Source Data**:
- **File**: `src/command/chat/app.rs`
- **Field**: `app.state.is_loading` (Line 260)
  - Set to `true` when starting AI request (Line 2871)
  - Set to `false` when finishing/cancelled (Line 3365)

**Tool Status Detection** (Lines 87-109):

```rust
let loading = if app.state.is_loading {
    // Priority 1: Check for Executing tool
    let tool_info = app
        .tool_executor
        .active_tool_calls
        .iter()
        .find(|tc| matches!(tc.status, ToolExecStatus::Executing))
        .map(|tc| format!(" 🔧 执行 {}...", tc.tool_name))
        .or_else(|| {
            // Priority 2: Check for PendingConfirm tool
            app.tool_executor
                .active_tool_calls
                .iter()
                .find(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))
                .map(|tc| format!(" 🔧 调用 {}...", tc.tool_name))
        });
    if let Some(info) = tool_info {
        info
    } else {
        " ⏳ 思考中...".to_string()  // Fallback
    }
} else {
    String::new()
};
```

### Remote Connection Indicator

- **Lines**: 141-152
- **Condition**: `app.remote_connected`
- **Display**: "📱 远程已连接" (Remote Connected)

---

## 4. Tool Calls Rendering in Message Area

### Rendering Architecture

**File**: `src/command/chat/render_cache.rs`

The rendering uses an incremental caching system optimized for performance:

#### Build Message Lines Incremental

- **Lines**: 50-61
- **Function**: `pub fn build_message_lines_incremental()`
- **Returns**: 
  ```
  (msg_start_lines, per_msg_cache, streaming_lines, stable_lines_arc, stable_offset)
  ```
- **Caching Layers**:
  - **P0**: Per-message line caching
  - **P1**: Streaming message incremental rendering
  - **P2**: Direct indexing (no flat Vec assembly)

### Tool Call Request Messages

- **Lines**: 1038-1120
- **Function**: `pub fn render_tool_call_request_msg()`
- **Parameters**:
  - `tool_calls: &[ToolCallItem]`
  - `bubble_max_width: usize`
  - `expand: bool` (from `app.ui.expand_tools`)

**Rendering Logic**:

1. **Tool Information Header** (Lines 1066-1076):
   - Category icon (from `ToolCategory::from_name()`)
   - Tool name (bold, category color)
   - Status icon (from `ToolStatus::Pending`)

2. **Arguments Display**:
   - If expand mode & JSON args: `render_json_params_enhanced()` (Line 1081)
   - If expand mode & non-JSON: Line-wrapped raw text
   - If collapsed: 60-char preview with ellipsis

3. **Spacing**: Blank line between multiple tool calls (Line 1055)

### Tool Result Messages

- **Lines**: 1165-1274
- **Function**: `pub fn render_tool_result_msg()`
- **Parameters**:
  - `content: &str` (tool output)
  - `label: &str` (tool name info)
  - `expand: bool`

**Rendering Logic**:

1. **Result Header** (Lines 1197-1209):
   - Tool icon (🔧)
   - Tool name (category color, bold)
   - Status icon + color (Success/Failed)
   - Result summary (from `get_result_summary()`)

2. **Content Rendering** (Lines 1211-1273):
   - **Collapsed**: Only header shown
   - **Error Results** (Lines 1220-1247):
     - "Error:" label in error color
     - First 20 lines of error
     - "... (共 N 行，显示前 20 行)" truncation message
   - **Diff Content** (Lines 1248-1250):
     - Special rendering for `\`\`\`diff\n` blocks
     - Color-coded: `-` (deletion), `+` (addition), `@@` (header)
   - **Agent Results** (Lines 1251-1253):
     - Nested tree rendering with `├─` and `└─` prefixes
   - **Normal Results** (Lines 1255-1272):
     - Indented display, first 100 lines
     - Truncation message if longer

### Tool Confirm Content

- **Lines**: 877-1036
- **Function**: `fn render_tool_confirm_content()`
- **Triggered**: When `app.ui.mode == ChatMode::ToolConfirm`

**Components**:
1. Tool name and description
2. Arguments display (JSON formatted)
3. Confirmation options (Yes/No with keyboard shortcuts)
4. User guide text

### Tool Confirm Area

- **Lines**: 577-623
- **Function**: `fn render_tool_confirm_area()`
- **Called from**: `build_message_lines_incremental()` (Line 324)

---

## 5. ToolExecStatus Enum & active_tool_calls

### ToolExecStatus Enum Definition

- **File**: `src/command/chat/app.rs`
- **Lines**: 46-57
- **Variants**:
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

### ToolCallStatus Structure

- **Lines**: 60-66
- **Fields**:
  ```rust
  pub struct ToolCallStatus {
      pub tool_call_id: String,
      pub tool_name: String,
      pub arguments: String,
      pub confirm_message: String,
      pub status: ToolExecStatus,
  }
  ```

### ToolExecutor Structure

- **Lines**: 273-293
- **Key Fields**:
  ```rust
  pub struct ToolExecutor {
      /// 当前活跃的工具调用状态列表
      pub active_tool_calls: Vec<ToolCallStatus>,
      /// ToolConfirm 模式中当前待处理工具的索引
      pub pending_tool_idx: usize,
      /// 进入 ToolConfirm 模式的时间（用于超时自动执行）
      pub tool_confirm_entered_at: std::time::Instant,
      /// 标记是否正在处理某个待确认工具
      pub pending_tool_execution: bool,
      /// 后台线程计数（有多少个工具在执行）
      pub tools_executing_count: usize,
      // ... channels for communication
  }
  ```

### active_tool_calls Usage Patterns

#### 1. Status Transitions

- **Initialize** (Line 3141): `self.tool_executor.active_tool_calls.clear();`
- **Add** (Line 3221):
  ```rust
  self.tool_executor.active_tool_calls.push(ToolCallStatus {
      tool_call_id: tc.id.clone(),
      tool_name: tc.name.clone(),
      arguments: tc.arguments.clone(),
      confirm_message: confirm_msg,
      status: if needs_confirm {
          ToolExecStatus::PendingConfirm
      } else {
          ToolExecStatus::Executing
      },
  });
  ```

- **Update Status** (Lines 365-369, 483, 553):
  ```rust
  self.active_tool_calls[idx].status = match {
      Done(summary),
      Failed(message),
      Executing,
      Rejected,
  }
  ```

#### 2. Querying & Filtering

- **Find Executing** (Lines 89-93, ui/chat.rs):
  ```rust
  app.tool_executor
      .active_tool_calls
      .iter()
      .find(|tc| matches!(tc.status, ToolExecStatus::Executing))
  ```

- **Find PendingConfirm** (Lines 96-99, ui/chat.rs):
  ```rust
  app.tool_executor
      .active_tool_calls
      .iter()
      .find(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))
  ```

- **Has Pending Confirm** (Lines 598-602):
  ```rust
  pub fn has_pending_confirm(&self) -> bool {
      self.active_tool_calls
          .iter()
          .any(|tc| matches!(tc.status, ToolExecStatus::PendingConfirm))
  }
  ```

- **Count by Status** (Line 399):
  ```rust
  let tasks: Vec<...> = self
      .active_tool_calls
      .iter()
      .filter(|tc| matches!(tc.status, ToolExecStatus::Executing))
      .map(...)
  ```

#### 3. Reset & Cleanup

- **Reset** (Lines 640-643):
  ```rust
  pub fn reset(&mut self) {
      self.active_tool_calls.clear();
      self.pending_tool_idx = 0;
      // ...
  }
  ```

- **Clear after finish** (Line 3368):
  ```rust
  self.tool_executor.active_tool_calls.clear();
  ```

#### 4. Tool Execution Flow

**Sequence**:
1. **Stream receives ToolCallRequest** → `active_tool_calls.clear()` & populate (Line 3141-3231)
2. **Permission/Hook checks** → Mark as `Failed` if denied (Line 3176, 3199)
3. **Determine confirm needed** → Set status to `PendingConfirm` or `Executing` (Line 3227-3229)
4. **Execute** → Status changes to `Executing` (Line 483)
5. **Complete** → Status changes to `Done` or `Failed` (Line 366-368)
6. **UI Display** → Rendered in title bar and message area (ui/chat.rs:89-100)

### Current Display Patterns

**Tool Display in Title Bar** (Lines 87-109, ui/chat.rs):
- Shows tool name with status: "🔧 执行 {tool_name}..." or "🔧 调用 {tool_name}..."

**Tool Display in Message Area** (render_cache.rs):
- Tool call request: `render_tool_call_request_msg()` (Lines 1038+)
- Tool result: `render_tool_result_msg()` (Lines 1165+)
- Tool confirmation: `render_tool_confirm_content()` (Lines 877+)

---

## Key Files Reference

| Component | Primary File | Lines | Secondary Files |
|-----------|--------------|-------|-----------------|
| auto_compact | compact.rs | 174-246 | agent.rs:44-56, 316-455 |
| Toast | app.rs | 2632-2635 | ui/chat.rs:882-926 |
| Title Bar | ui/chat.rs | 83-162 | app.rs:260, 2871, 3365 |
| Tool Rendering | render_cache.rs | 1038-1274 | ui/chat.rs:89-100 |
| ToolExecStatus | app.rs | 46-57 | app.rs:273-293, 3141-3231 |
| active_tool_calls | app.rs | 273-293 | Throughout app.rs, render_cache.rs, ui/chat.rs |

---

## Constants & Configuration

- **Toast duration**: `TOAST_DURATION_SECS` (from crate::constants)
- **Token threshold**: `256 * 800` = 204,800 tokens
- **Max token summary**: 20,000 tokens
- **Transcript truncation**: 80,000 chars
- **Tool result truncation**: 100 lines (20 for errors)

