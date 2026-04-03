# J-CLI Tool Call Rendering - Visual Examples

## Example 1: Tool Call Request - Collapsed Mode

**Scenario**: User asks to read a file, AI calls the `Read` tool.

**Source**: `src/command/chat/render_cache.rs` lines 1101-1117

**Visual Output**:
```
  Sprite                           ← AI label

  📄 Read /Users/jack/file.rs… ← Collapsed tool call (60 char preview)

```

**Code that generates this**:
```rust
// Tool: Read
// Arguments: {"path": "/Users/jack/file.rs"}
// In collapsed mode (expand_tools == false):

lines.push(Line::from(vec![
    Span::styled("  ", Style::default()),                    // 2-space indent
    Span::styled(icon, Style::default().fg(tool_color)),    // 📄 (blue)
    Span::styled(" ", Style::default()),
    Span::styled(tc.name.clone(), ...),                     // "Read" (bold)
    Span::styled(" {args_preview}…", ...),                 // "/Users/jack..."
]));
```

---

## Example 2: Tool Call Request - Expanded Mode

**Scenario**: Same as above, but with `expand_tools == true` (toggled via Ctrl+O)

**Visual Output**:
```
  Sprite

  📄 Read ⏳                    ← Expanded header with status icon
    path: /Users/jack/file.rs  ← Parameter details (JSON expanded)

```

**Code that generates this**:
```rust
// Line 1: Tool header
lines.push(Line::from(vec![
    Span::styled("  ", Style::default()),
    Span::styled(icon, Style::default().fg(tool_color)),    // 📄 (blue)
    Span::styled(" ", Style::default()),
    Span::styled(tc.name.clone(), ...),                     // "Read" (bold)
    Span::styled(" ", Style::default()),
    Span::styled(status_icon, ...),                         // ⏳ (yellow)
]));

// Lines 2+: JSON parameters via render_json_params_enhanced()
// "    path: /Users/jack/file.rs"
```

---

## Example 3: Multiple Tool Calls

**Scenario**: AI wants to read 2 files and search for a pattern.

**Collapsed Mode Output**:
```
  Sprite

  📄 Read /file1.rs…
  
  📄 Read /file2.rs…
  
  🔍 Grep pattern: "function" /src…

```

**Expanded Mode Output**:
```
  Sprite

  📄 Read ⏳
    path: /file1.rs

  📄 Read ⏳
    path: /file2.rs

  🔍 Grep ⏳
    pattern: function
    path: /src
    flags: -r

```

---

## Example 4: Tool Result - Collapsed Mode

**Scenario**: Previous `Read` tool execution completed successfully.

**Visual Output**:
```
  Sprite

  📄 Read ⏳

  🔧 Read ✓ 45 lines, 2.3KB ← Tool result (header only in collapsed)

```

**Code that generates this**:
```rust
// Line: Tool result header (always shown, even collapsed)
lines.push(Line::from(vec![
    Span::styled("  ", Style::default()),
    Span::styled("🔧", ...),                                // Result icon
    Span::styled(" ", Style::default()),
    Span::styled("Read", ...),                              // Tool name (bold)
    Span::styled(" ", Style::default()),
    Span::styled("✓", ...),                                 // Status: Success (green)
    Span::styled(" ", Style::default()),
    Span::styled("45 lines, 2.3KB", ...),                   // Summary
]));
```

---

## Example 5: Tool Result - Expanded Mode (Normal Content)

**Scenario**: Same as above, but expanded mode shows the file content.

**Visual Output**:
```
  Sprite

  📄 Read ⏳

  🔧 Read ✓ 45 lines, 2.3KB
    use std::fs;
    use std::io;
    
    pub fn read_file(path: &str) -> io::Result<String> {
        fs::read_to_string(path)
    }
    
    ... (45 lines, showing first 100 lines)

```

---

## Example 6: Tool Result - Diff Content

**Scenario**: `Edit` tool modifies a file and returns diff output.

**Visual Output**:
```
  Sprite

  📄 Edit ⏳

  🔧 Edit ✓ 12 lines, 480 chars
    ```diff
    - pub fn old_name() {
    + pub fn new_name() {
        println!("Hello");
    ```

```

**Special Rendering** (from `render_diff_content()`, lines 1277-1321):
- Lines starting with `- ` → Red (theme.diff_del)
- Lines starting with `+ ` → Green (theme.diff_add)
- Lines starting with `@@ ` → Cyan (theme.diff_header)
- Context lines → Dim

---

## Example 7: Tool Result - Error

**Scenario**: `Bash` command failed.

**Visual Output**:
```
  Sprite

  ⚡ Bash ⏳

  🔧 Bash ✗ 失败              ← Status is ✗ (red)
    Error:                    ← "Error:" label in red
      command not found: xyz  ← Error message (up to 20 lines)
      ...

```

---

## Example 8: Tool Result - Agent Output (Nested)

**Scenario**: `Agent` tool returns multi-line structured output.

**Visual Output**:
```
  Sprite

  🤖 Agent ⏳

  🔧 Agent ✓ 45 lines, 3.2KB
    ├─ Analyzing codebase structure
    ├─ Found 12 files in src/ directory
    ├─ Identified 3 main modules
    └─ Summary: Well-organized monolithic structure
    ... (30 lines with tree prefixes)

```

**Code** (from `render_agent_result_nested()`, lines 1324-1357):
- First N-1 lines use `├─ ` prefix
- Last line uses `└─ ` prefix
- Shows max 30 lines

---

## Example 9: Tool Categories & Colors

**All tools in one view**:

```
  Sprite                       ← AI assistant name

  📄 Read ⏳                   ← File (blue icon)
  
  🔍 Grep ⏳                   ← Search (green icon)
  
  ⚡ Bash ⏳                   ← Execute (yellow icon)
  
  🌐 WebFetch ⏳               ← Network (cyan icon)
  
  📋 EnterPlanMode ⏳          ← Plan (green icon)
  
  🤖 Agent ⏳                  ← Agent (yellow icon)
  
  🔧 UnknownTool ⏳            ← Other (gray icon)

```

---

## Example 10: Cache Invalidation Behavior

**Scenario**: User presses Ctrl+O to toggle expand_tools

**Before Toggle** (collapsed):
```
  ⚡ Bash ls -la /tmp…
```

**After Toggle** (expanded):
```
  ⚡ Bash ⏳
    command: ls -la /tmp
```

**What happens internally**:
1. User presses Ctrl+O
2. `self.ui.expand_tools = !self.ui.expand_tools`
3. Next render calls `build_message_lines_incremental()` with new expand value
4. Cache check fails: `old_cache.expand_tools (true) != expand (false)` ❌
5. All tool messages are re-rendered with new layout
6. New cache is stored with `expand_tools: false`

---

## Example 11: JSON Parameter Rendering

**Scenario**: Complex tool parameters in expanded mode

**Visual Output**:
```
  ⚡ Bash ⏳
    command: ls -la
    timeout: 30
    working_dir: /home/user
    environment: {"PATH": "/usr/bin:/bin", "HOME"…}  ← Object truncated

  🔍 Grep ⏳
    pattern: (import|use)
    flags: -r
    paths: [3 items]           ← Array shown as "[N items]"
    max_count: 100
    context_lines: 2

```

**Code** (from `format_json_value()`, classification.rs:122-153):
```rust
// Strings: truncate @ 50 chars
serde_json::Value::String(s) if s.len() > 50 => format!("\"{}...\"", &s[..47])

// Arrays: show count
serde_json::Value::Array(arr) => format!("[{} items]", arr.len())

// Objects: show first 3 keys
serde_json::Value::Object(obj) if !obj.is_empty() => {
    let keys: Vec<&str> = obj.keys().take(3).map(|s| s.as_str()).collect();
    format!("{{{}}}", keys.join(", "))
}
```

---

## Example 12: Summary Calculation Examples

**From `get_result_summary()` (classification.rs:156-180)**:

| Output | Summary |
|--------|---------|
| "" (empty) | "无输出" |
| "42" (1 line, 2 chars) | "2 字符" |
| "x".repeat(100) (1 line, 100 chars) | "100B" → "100 字符" |
| "line\n" repeated 10x, total 500 bytes | "10 行, 500 字符" |
| "line\n" repeated 100x, total 5KB | "100 行, 5.0KB" |

---

## Example 13: Tool Name Resolution

**Scenario**: Tool result message needs to know what tool was called.

**Message History**:
```
[0] User: "read a file"

[1] Assistant with tool_calls:
    id: "call_xyz789"
    name: "Read"
    arguments: "{\"path\": \"/file.rs\"}"

[2] Tool message:
    role: "tool"
    tool_call_id: "call_xyz789"
    content: "file content here..."
```

**Rendering of Message [2]**:
1. Get `tool_call_id`: "call_xyz789"
2. Search backwards from index [1] → find message [1]
3. Find `ToolCallItem` with id="call_xyz789" → get name="Read"
4. Use "Read" as label for display

**Output**:
```
  🔧 Read ✓ 12 lines, 450 chars   ← Tool name resolved!
    file content here...
```

---

## Example 14: Truncation of Long Output

**Scenario**: Tool returns 250 lines of output, but max display is 100 lines.

**Visual Output**:
```
  🔧 Bash ✓ 250 lines, 25KB
    Line 1
    Line 2
    ...
    Line 100
    ... (共 250 行，显示前 100 行)

```

**Code** (render_cache.rs:1266-1272):
```rust
let total_lines = clean.lines().count();
if total_lines > 100 {
    lines.push(Line::from(Span::styled(
        format!("    ... (共 {} 行，显示前 100 行)", total_lines),
        Style::default().fg(theme.text_dim),
    )));
}
```

---

## Example 15: UI State & expand_tools Flag

**Application State** (app.rs):
```rust
pub struct UiState {
    pub expand_tools: bool,  // Default: false (collapsed)
    // ... other fields
}

pub struct MsgLinesCache {
    pub expand_tools: bool,  // Track what mode was used for this cache
    // ... other fields
}
```

**Toggle Shortcut**: Ctrl+O

**Behavior**:
- OFF (default): Tool calls and results show only headers
- ON: Tool calls show parameters, results show full content (up to limits)
- Change triggers complete re-render of all tool messages

---

## Rendering Performance Notes

### Cache Hit (all fields match):
```
┌─ Cached render line (ZERO RECOMPUTE) ───────────┐
│ - bubble_max_width: same ✓                       │
│ - expand_tools: same ✓                           │
│ - content length: same ✓                         │
│ - is_selected: same ✓                           │
│ → Use old_per.lines directly (Arc clone O(1))   │
└─────────────────────────────────────────────────┘
```

### Cache Miss (expand_tools changed):
```
┌─ Re-render entire message ──────────────────────┐
│ - Invalidation detected: expand != old_expand   │
│ - Rebuild all tool message renderings            │
│ - Store new cache with new expand_tools value   │
└─────────────────────────────────────────────────┘
```

### Streaming Messages (P1 optimization):
```
┌─ Incremental markdown parsing ──────────────────┐
│ 1. Find stable paragraph boundary               │
│ 2. Cache already-parsed stable paragraphs       │
│ 3. Re-parse only last incomplete paragraph      │
│ 4. Reuse stable_lines via Arc (no copy)         │
└─────────────────────────────────────────────────┘
```

