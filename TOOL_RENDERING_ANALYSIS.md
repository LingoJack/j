# J-CLI Tool Call Rendering Analysis Report

## Overview
Tool calls in j-cli are rendered in the chat UI through a sophisticated system that handles both **assistant messages with tool_calls** (requests to execute tools) and **tool messages** (results from tool execution). The rendering is driven by the `render_cache.rs` module with support for expand/collapse modes controlled by the `expand_tools` flag.

---

## 1. Message Type Handling

### File: `src/command/chat/render_cache.rs` (Lines 122-196)

The rendering entry point is `build_message_lines_incremental()` which dispatches based on message role:

```rust
match m.role.as_str() {
    ROLE_USER => render_user_msg(...)
    ROLE_ASSISTANT => {
        if let Some(ref tool_calls) = m.tool_calls {
            render_tool_call_request_msg(tool_calls, ...)  // Tool call request
        } else {
            render_assistant_msg(...)                      // Regular assistant message
        }
    }
    ROLE_TOOL => {
        // Parse tool_call_id to find original tool_name
        render_tool_result_msg(content, label, ...)       // Tool execution result
    }
    ROLE_SYSTEM => { ... }
}
```

### Key Constants
- **File**: `src/command/chat/constants.rs`
- `ROLE_ASSISTANT = "assistant"`
- `ROLE_TOOL = "tool"`
- `ROLE_USER = "user"`
- `ROLE_SYSTEM = "system"`

---

## 2. Tool Call Request Messages (Assistant → Tool)

### Function: `render_tool_call_request_msg()`
**Location**: `src/command/chat/render_cache.rs`, Lines 1038-1120

#### Signature
```rust
pub fn render_tool_call_request_msg(
    tool_calls: &[super::storage::ToolCallItem],
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
    expand: bool,
)
```

#### Data Structure: `ToolCallItem`
**Location**: `src/command/chat/storage.rs`, Lines 93-99

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallItem {
    pub id: String,           // Unique tool call ID
    pub name: String,         // Tool name (e.g., "Bash", "Read", "Write")
    pub arguments: String,    // JSON string of tool arguments
}
```

#### Rendering Patterns

**Collapsed Mode** (when `expand_tools == false`):
```
  [ICON] [TOOL_NAME] [ARGS_PREVIEW]…
```
- Example: `📄 Read /path/to/file.rs (showing 60 chars max)…`
- Line 1101-1117

**Expanded Mode** (when `expand_tools == true`):
```
  [ICON] [TOOL_NAME] [STATUS_ICON]
    [PARAM_NAME]: [PARAM_VALUE]
    [PARAM_NAME]: [PARAM_VALUE]
    ...
```
- Example:
  ```
  ⚡ Bash ⏳
    command: ls -la /tmp
  ```
- Line 1066-1091

#### Tool Icons & Colors
**Location**: `src/command/chat/tools/classification.rs`, Lines 1-66

| Category | Icon | Color Theme | Tool Names |
|----------|------|-------------|-----------|
| File | 📄 | label_user (blue) | Read, Write, Edit, Glob, FileRead, FileWrite, FileEdit |
| Search | 🔍 | label_ai (green) | Grep, GrepTool |
| Execute | ⚡ | title_loading (yellow) | Bash, Task, TaskOutput, TaskCreate, TaskUpdate, TaskGet |
| Network | 🌐 | config_title (cyan) | WebFetch, WebSearch, WebBrowser |
| Plan | 📋 | label_ai (green) | EnterPlanMode, ExitPlanMode |
| Agent | 🤖 | title_loading (yellow) | Agent |
| Other | 🔧 | text_dim (gray) | Unknown tools |

#### Status Icons for Tool Calls
- `⏳` (Pending) - Waiting for user confirmation or execution
- Color: `title_loading` (yellow/orange)

#### JSON Parameter Rendering
**Function**: `render_json_params_enhanced()`, Lines 1123-1160

For tool arguments that are valid JSON:
1. Parse as JSON object
2. Iterate through key-value pairs
3. Format each parameter on separate line with indentation:
   ```
       [KEY]: [FORMATTED_VALUE]
   ```
4. Truncate values > display width + "…"
5. Use `format_json_value()` for smart display:
   - Strings: truncate @ 50 chars
   - Arrays: show as `[N items]`
   - Objects: show first 3 keys as `{key1, key2, key3}`

---

## 3. Tool Result Messages (Tool → Assistant)

### Function: `render_tool_result_msg()`
**Location**: `src/command/chat/render_cache.rs`, Lines 1165-1274

#### Signature
```rust
pub fn render_tool_result_msg(
    content: &str,              // Tool execution output
    label: &str,                // Tool name (parsed from tool_call_id lookup)
    bubble_max_width: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
    expand: bool,
)
```

#### Tool Name Resolution
**Location**: `src/command/chat/render_cache.rs`, Lines 152-175

The tool name is resolved by:
1. Get `tool_call_id` from the tool message
2. Search backwards through message history for matching assistant message
3. Find `ToolCallItem` with matching id in that message's `tool_calls` array
4. Extract `tool_name` from that item
5. Fallback to "工具结果" (Tool Result) if not found

#### Rendering Patterns

**Collapsed Mode** (when `expand_tools == false`):
```
  [ICON] [TOOL_NAME] [STATUS_ICON] [SUMMARY]
```
- Example: `🔧 Bash ✓ 12 lines, 450 characters`
- Shows only header line; content is hidden

**Expanded Mode** (when `expand_tools == true`):
```
  [ICON] [TOOL_NAME] [STATUS_ICON] [SUMMARY]
    [CONTENT - up to 100 lines]
    ... (total N lines, showing first 100)
```

#### Status Icons for Tool Results
- `✓` (Success) - Tool executed successfully
- `✗` (Failed) - Tool execution failed
- Color: Green for Success, Red for Failed

#### Result Summary Calculation
**Function**: `get_result_summary()`, `src/command/chat/tools/classification.rs`, Lines 156-180

```
- Empty output: "无输出"
- 1 line, < 100 chars: "{N} 字符" (e.g., "42 字符")
- 1 line, >= 100 chars: "{size}KB" (e.g., "1.2KB")
- Multiple lines, < 1KB: "{N} 行, {M} 字符" (e.g., "12 行, 450 字符")
- Multiple lines, >= 1KB: "{N} 行, {size}KB" (e.g., "45 行, 2.3KB")
```

#### Special Content Handling (Expanded Mode)

**Error Results** (Lines 1220-1247):
- Shows "Error:" label in red/error color
- Displays first 20 lines of error message
- Shows truncation notice if > 20 lines
- Used when content matches error pattern

**Diff Content** (Lines 1248-1250):
- Special rendering if content contains ` ```diff ` blocks
- **Function**: `render_diff_content()`, Lines 1277-1321
- Color highlighting:
  - Lines starting with `- `: Red (theme.diff_del)
  - Lines starting with `+ `: Green (theme.diff_add)
  - Lines starting with `@@ `: Cyan (theme.diff_header)
  - Other lines: Dim

**Agent Results** (Lines 1251-1253):
- Nested tree-style display
- **Function**: `render_agent_result_nested()`, Lines 1324-1357
- Format:
  ```
      ├─ [LINE 1]
      ├─ [LINE 2]
      └─ [LINE N]
  ```
- Shows max 30 lines with continuation notice

**Normal Results** (Lines 1255-1264):
- Simple text display with indentation
- Max 100 lines shown, with truncation notice

---

## 4. The `expand_tools` Flag

### Purpose
Controls whether tool calls and results are shown in collapsed or expanded view.

### Definition
**Location**: `src/command/chat/app.rs`

```rust
pub struct UiState {
    pub expand_tools: bool,  // Default: false (collapsed)
}

pub struct MsgLinesCache {
    pub expand_tools: bool,  // Cached for invalidation detection
}
```

### Toggle Mechanism
**Location**: `src/command/chat/app.rs` (Key Handler)

```rust
// Ctrl+O toggles expand_tools
if key == crossterm::event::KeyCode::Char('o') && modifiers.contains(KeyModifiers::CONTROL) {
    self.ui.expand_tools = !self.ui.expand_tools;
    // Mark cache dirty so next render rebuilds with new expand value
}
```

### Cache Invalidation
**Location**: `src/command/chat/render_cache.rs`, Lines 84-89

```rust
// Check if expand_tools changed - if so, invalidate cache
let can_reuse_per_msg = old_cache
    .map(|c| c.bubble_max_width == bubble_max_width && c.expand_tools == expand)
    .unwrap_or(false);
```

When `expand_tools` changes, all cached tool message renderings are invalidated and re-rendered.

---

## 5. Markdown Rendering Integration

### Markdown-to-Lines Pipeline
**Location**: `src/command/chat/markdown/`

Files:
- `mod.rs` - Export `markdown_to_lines`
- `parser.rs` - Main markdown parsing logic
- `highlight.rs` - Syntax highlighting for code blocks
- `image_cache.rs` - Image placeholder handling

### Usage in Rendering
Called by `render_assistant_msg()` when rendering regular assistant text responses to parse markdown formatting.

---

## 6. Tool Categories & Classification

### Implementation
**File**: `src/command/chat/tools/classification.rs`

### ToolCategory Enum
```rust
pub enum ToolCategory {
    File,      // Read, Write, Edit, Glob
    Search,    // Grep
    Execute,   // Bash, Task, TaskOutput, TaskCreate, TaskUpdate, TaskGet
    Network,   // WebFetch, WebSearch, WebBrowser
    Plan,      // EnterPlanMode, ExitPlanMode
    Agent,     // Agent
    Other,     // Unknown
}
```

### ToolStatus Enum
```rust
pub enum ToolStatus {
    Pending,   // ⏳
    Running,   // ⏱
    Success,   // ✓
    Failed,    // ✗
    Rejected,  // ⊘
}
```

---

## 7. Available Tool Names (Complete List)

### File Operations
- `Read` - Read file contents
- `Write` - Write file contents
- `Edit` - Edit file with line numbers
- `Glob` - File pattern matching
- `FileRead`, `FileWrite`, `FileEdit` - Aliases

### Search
- `Grep` - Regex pattern search
- `GrepTool` - Alias

### Execution
- `Bash` - Shell command execution
- `Task` - Task management
- `TaskOutput` - Get task output
- `TaskCreate` - Create task
- `TaskUpdate` - Update task
- `TaskGet` - Get task info

### Network
- `WebFetch` - Fetch web content
- `WebSearch` - Web search
- `WebBrowser` - Browser tool

### Planning
- `EnterPlanMode` - Enter planning mode
- `ExitPlanMode` - Exit planning mode

### AI Agents
- `Agent` - Sub-agent execution

### Utilities
- `Ask` - Ask user questions (interactive)
- `TaskOutput` - Background task output
- `Compact` - Context compaction
- `RegisterHook` - Register hooks
- `ComputerUse` - Computer control
- `LoadSkill` - Load/execute skills
- `Todo` - Todo management (TodoRead, TodoWrite)

---

## 8. Rendering Flow Summary

### Tool Call Request (Assistant Message with tool_calls)

```
build_message_lines_incremental()
  ├─ Check if m.role == ROLE_ASSISTANT
  ├─ Check if m.tool_calls.is_some()
  └─ render_tool_call_request_msg()
      ├─ For each ToolCallItem in tool_calls:
      │   ├─ Determine category from tool name
      │   ├─ Get icon & color from category
      │   ├─ Get status icon (Pending)
      │   └─ If expand_tools:
      │       ├─ Line 1: [ICON] [NAME] [STATUS]
      │       └─ Lines 2+: JSON params (render_json_params_enhanced)
      │       └─ Else: [ICON] [NAME] [ARGS_PREVIEW]…
      └─ Add blank lines between multiple tool calls
```

### Tool Result (Tool Role Message)

```
build_message_lines_incremental()
  ├─ Check if m.role == ROLE_TOOL
  ├─ Lookup tool_name via tool_call_id
  └─ render_tool_result_msg()
      ├─ Get category & color from tool_name
      ├─ Determine status (Success/Failed) from content
      ├─ Get summary via get_result_summary()
      ├─ Line 1: [ICON] [NAME] [STATUS] [SUMMARY]
      └─ If expand_tools && content not empty:
          ├─ Check content type (diff/agent/error/normal)
          ├─ Render special formatting if needed
          └─ Show up to 100 lines of content
```

---

## 9. Line-by-Line Rendering Details

### Tool Call Header (Expanded)
**Location**: Lines 1066-1076

```rust
lines.push(Line::from(vec![
    Span::styled("  ", Style::default()),          // 2-space indent
    Span::styled(icon, Style::default().fg(tool_color)),
    Span::styled(" ", Style::default()),           // 1-space separator
    Span::styled(tc.name.clone(), Style::default()
        .fg(tool_color).add_modifier(Modifier::BOLD)),
    Span::styled(" ", Style::default()),           // 1-space separator
    Span::styled(status_icon, Style::default()
        .fg(status_color)),
]));
```

### Tool Result Header (Always Shown)
**Location**: Lines 1197-1209

```rust
lines.push(Line::from(vec![
    Span::styled("  ", Style::default()),          // 2-space indent
    Span::styled(icon, Style::default().fg(tool_color)),
    Span::styled(" ", Style::default()),           // 1-space separator
    Span::styled(tool_name.clone(), Style::default()
        .fg(tool_color).add_modifier(Modifier::BOLD)),
    Span::styled(" ", Style::default()),           // 1-space separator
    Span::styled(status_icon, Style::default()
        .fg(status_color)),
    Span::styled(" ", Style::default()),           // 1-space separator
    Span::styled(summary, Style::default()
        .fg(status_color)),
]));
```

---

## 10. Key Files & Line Number Reference

| File | Purpose | Key Lines |
|------|---------|-----------|
| `render_cache.rs` | Main rendering logic | 1-1443 |
| `render_cache.rs` | Tool call request | 1038-1120 |
| `render_cache.rs` | Tool result | 1165-1274 |
| `render_cache.rs` | JSON params | 1123-1160 |
| `render_cache.rs` | Diff rendering | 1277-1321 |
| `render_cache.rs` | Agent results | 1324-1357 |
| `storage.rs` | ToolCallItem struct | 93-99 |
| `storage.rs` | ChatMessage struct | 117-125 |
| `classification.rs` | Tool categories | 1-66 |
| `classification.rs` | Result summary | 156-180 |
| `app.rs` | expand_tools flag | Line ~400 |
| `markdown/parser.rs` | Markdown parsing | 1+ |

---

## 11. Styling & Theme Integration

### Theme Properties Used
- `theme.label_ai` - Green (for file/search categories)
- `theme.label_user` - Blue (for file operations)
- `theme.title_loading` - Yellow/Orange (for execution/agent)
- `theme.config_title` - Cyan (for network)
- `theme.text_dim` - Gray (for params/details)
- `theme.text_normal` - Normal text color
- `theme.text_white` - White text
- `theme.diff_del` - Red (diff deletions)
- `theme.diff_add` - Green (diff additions)
- `theme.diff_header` - Cyan (diff headers)
- `theme.toast_error_border` - Red (errors)

### Text Modifiers
- `Modifier::BOLD` - Tool names & status
- `Modifier::UNDERLINED` - Headings (in markdown)

---

## 12. Performance Optimizations

### P0: Message-Level Caching
- Per-message rendering cached
- Reuse when content unchanged
- Invalidated when: width changes, expand_tools changes, or content changes

### P1: Incremental Streaming
- Only last incomplete paragraph is re-parsed for streaming messages
- Completed paragraphs cached in `streaming_stable_lines` (Arc for zero-copy)

### P2: Direct Indexing
- `msg_start_lines` map: message index → line offset
- No flat Vec assembly, just index into per-message caches

### Cache Invalidation Conditions
```rust
can_reuse_per_msg = old_cache.map(|c|
    c.bubble_max_width == bubble_max_width &&
    c.expand_tools == expand &&
    old_per.content_len == m.content.len() &&
    old_per.is_selected == is_selected
).unwrap_or(false)
```

---

## Summary

Tool calls in j-cli are rendered through a sophisticated two-phase system:

1. **Tool Requests** (Assistant messages): Show what tools will be called, with icons/colors by category
2. **Tool Results** (Tool messages): Show execution status and output

Both respect the `expand_tools` toggle for compact/detailed views, use smart caching for performance, and integrate category-based visual classification with themed colors and icons.
