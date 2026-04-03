# J-CLI Tool Call Rendering - Documentation Index

## 📚 Documentation Files

This analysis consists of 3 comprehensive documents:

### 1. **TOOL_RENDERING_ANALYSIS.md** (506 lines)
   The complete deep-dive analysis covering:
   - Message type handling (ROLE_ASSISTANT, ROLE_TOOL, etc.)
   - Tool call request rendering (`render_tool_call_request_msg`)
   - Tool result rendering (`render_tool_result_msg`)
   - The `expand_tools` flag and cache invalidation
   - All tool categories and classifications
   - Performance optimizations (P0/P1/P2)
   - Theme integration and styling
   - File locations and line number references
   
   **Read this for**: Complete technical understanding

### 2. **TOOL_RENDERING_QUICK_REF.txt**
   Quick reference guide with:
   - Rendering flow overview
   - Tool categories table
   - Status icons reference
   - Key data structures
   - expand_tools toggle mechanism
   - Result summary calculation rules
   - Special content handling (diff/agent/error)
   - JSON parameter rendering logic
   - Performance optimization summary
   - Code location index
   
   **Read this for**: Quick lookups and key facts

### 3. **TOOL_RENDERING_EXAMPLES.md** (300+ lines)
   Visual examples including:
   - 15 realistic scenarios with code
   - Before/after UI examples
   - Multiple tool calls
   - Tool results with various output types
   - Cache invalidation behavior
   - JSON parameter display
   - Summary calculation examples
   - Performance characteristics
   
   **Read this for**: Visual understanding and practical examples

---

## 🎯 Core Concepts

### Message Flow
```
User Input
    ↓
AI (Claude) decides what tools to call
    ↓
Message with role=assistant & tool_calls=[ToolCallItem, ...]
    ↓ render_tool_call_request_msg()
    ↓ (Collapsed: icon + name + args preview)
    ↓ (Expanded: icon + name + status + full JSON params)
    
Tool Executor runs the tools
    ↓
Message with role=tool & tool_call_id & content (result)
    ↓ render_tool_result_msg()
    ↓ (Collapsed: icon + name + status + summary)
    ↓ (Expanded: above + full output content)
```

### The Two Rendering Modes
- **Collapsed** (default): Shows headers only, compact view
  - Toggle: OFF (expand_tools = false)
  - Tool calls: icon + name + args_preview
  - Tool results: icon + name + status + summary
  
- **Expanded**: Shows full details
  - Toggle: ON (expand_tools = true)
  - Tool calls: + full JSON parameters
  - Tool results: + full output content (up to limits)

### Tool Classification
7 categories with unique icons and colors:
- 📄 File (blue) - Read, Write, Edit, Glob
- 🔍 Search (green) - Grep
- ⚡ Execute (yellow) - Bash, Task, TaskCreate/Update/Get, TaskOutput
- 🌐 Network (cyan) - WebFetch, WebSearch, WebBrowser
- 📋 Plan (green) - EnterPlanMode, ExitPlanMode
- 🤖 Agent (yellow) - Agent
- 🔧 Other (gray) - Unknown tools

---

## 🗂️ File Structure

```
src/command/chat/
├── render_cache.rs          ← PRIMARY FILE (1443 lines)
│   ├── build_message_lines_incremental()  [Lines 50-337]
│   ├── render_user_msg()                  [Lines 407-490]
│   ├── render_assistant_msg()             [Lines 493-541]
│   ├── render_tool_call_request_msg()     [Lines 1038-1120] ← Tool calls
│   ├── render_tool_result_msg()           [Lines 1165-1274] ← Tool results
│   ├── render_json_params_enhanced()      [Lines 1123-1160] ← JSON params
│   ├── render_diff_content()              [Lines 1277-1321] ← Diff handling
│   ├── render_agent_result_nested()       [Lines 1324-1357] ← Agent results
│   └── [helper functions]
│
├── tools/
│   ├── mod.rs                             ← Tool registry
│   └── classification.rs                  ← Categories & icons
│       ├── ToolCategory enum              [Lines 1-66]
│       ├── ToolStatus enum                [Lines 68-120]
│       ├── format_json_value()            [Lines 122-153]
│       └── get_result_summary()           [Lines 156-180]
│
├── storage.rs
│   ├── ToolCallItem struct                [Lines 93-99]
│   ├── ChatMessage struct                 [Lines 117-125]
│   └── [message storage/serialization]
│
├── app.rs
│   ├── UiState struct
│   │   └── expand_tools: bool             ← Toggle flag
│   ├── MsgLinesCache struct
│   │   └── expand_tools: bool             ← Cache tracking
│   └── [TUI state management]
│
├── markdown/
│   ├── mod.rs                             ← Export markdown_to_lines
│   ├── parser.rs                          ← Markdown parsing (with syntax highlight)
│   ├── highlight.rs                       ← Code block highlighting
│   └── image_cache.rs                     ← Image placeholder handling
│
└── constants.rs
    ├── ROLE_ASSISTANT = "assistant"
    ├── ROLE_TOOL = "tool"
    ├── ROLE_USER = "user"
    └── ROLE_SYSTEM = "system"
```

---

## 🔑 Key Functions

### Main Entry Point
- `build_message_lines_incremental()` [render_cache.rs:50-337]
  - Dispatches rendering based on message role
  - Handles cache invalidation (P0 optimization)
  - Manages streaming incremental rendering (P1)
  
### Tool Call Rendering
- `render_tool_call_request_msg()` [render_cache.rs:1038-1120]
  - Renders what tools the AI wants to call
  - Uses category for icon/color
  - Shows pending status ⏳
  - Supports collapsed/expanded modes

### Tool Result Rendering
- `render_tool_result_msg()` [render_cache.rs:1165-1274]
  - Renders what tools returned
  - Uses category for icon/color
  - Shows success ✓ or failed ✗ status
  - Always shows header + summary
  - Optionally shows content (if expanded)

### Parameter Rendering
- `render_json_params_enhanced()` [render_cache.rs:1123-1160]
  - Formats JSON tool arguments
  - One parameter per line: "    key: value"
  - Uses `format_json_value()` for smart display

### Special Content Handling
- `render_diff_content()` [render_cache.rs:1277-1321]
  - Color-codes diff lines (-, +, @@)
  - Max 100 lines
  
- `render_agent_result_nested()` [render_cache.rs:1324-1357]
  - Tree-style output with ├─ and └─ prefixes
  - Max 30 lines

---

## 🎨 Visual Elements

### Icons by Category
| Category | Icon | Theme | Usage |
|----------|------|-------|-------|
| File | 📄 | label_user | Read, Write, Edit, Glob |
| Search | 🔍 | label_ai | Grep |
| Execute | ⚡ | title_loading | Bash, Task, etc. |
| Network | 🌐 | config_title | Web tools |
| Plan | 📋 | label_ai | Plan mode |
| Agent | 🤖 | title_loading | Agent |
| Other | 🔧 | text_dim | Unknown |

### Status Icons
| Status | Icon | Color | Meaning |
|--------|------|-------|---------|
| Pending | ⏳ | title_loading | Waiting to execute |
| Success | ✓ | label_ai | Completed successfully |
| Failed | ✗ | toast_error_border | Execution failed |

### Text Modifiers
- `Modifier::BOLD` - Tool names, status indicators
- `Modifier::UNDERLINED` - Markdown headings

---

## ⚙️ Performance & Caching

### Three-Level Optimization (P0/P1/P2)

**P0: Message-Level Caching**
- Each message's render lines cached independently
- Reuse when: width, expand_tools, content length, is_selected all match
- Invalidates individually

**P1: Incremental Streaming**
- Find stable paragraph boundary (check for ``` balance)
- Completed paragraphs cached in `streaming_stable_lines` (Arc)
- Only last incomplete paragraph re-parsed each render
- Arc clone O(1) instead of Vec clone O(n)

**P2: Direct Indexing**
- `msg_start_lines` map: message index → line offset
- No assembly of flat Vec
- Direct index into per-message caches

### Cache Invalidation Conditions
A message cache is reused only if ALL of these match:
1. `bubble_max_width == old.bubble_max_width` (terminal width)
2. `expand_tools == old.expand_tools` (toggle state)
3. `m.content.len() == old.content_len` (content size)
4. `is_selected == old.is_selected` (browse highlight)

If **ANY** condition fails, the message is re-rendered from scratch.

---

## 🎮 User Interactions

### Toggle expand_tools
- **Shortcut**: Ctrl+O
- **Effect**: Flips `expand_tools` boolean
- **Result**: Invalidates all tool message caches
- **Next render**: All tool calls/results show in new mode

### Result Summary
- **Collapsed**: Shows: icon + name + status + summary
- **Expanded**: Shows: above + full content (with line limits)

### Tool Call Details
- **Collapsed**: Shows: icon + name + 60-char args preview
- **Expanded**: Shows: icon + name + status + full JSON params

---

## 📊 Content Display Limits

| Content Type | Collapsed | Expanded | Limit |
|---|---|---|---|
| Tool call params | Preview only | Full JSON | N/A |
| Tool result output | None | Shown | 100 lines |
| Error messages | None | Shown | 20 lines |
| Agent tree output | None | Shown | 30 lines |
| Diff content | None | Shown | 100 lines |

---

## 🔍 Data Flow Example

### Example: User asks to "read main.rs"

```
1. User types: "read main.rs"
   ↓
2. AI responds with tool_calls:
   {
     role: "assistant",
     content: "I'll read that file for you",
     tool_calls: [{
       id: "call_123",
       name: "Read",
       arguments: "{\"path\": \"main.rs\"}"
     }]
   }
   ↓
3. Rendering (collapsed):
   🔧 Sprite
   📄 Read main.rs
   ↓
4. Rendering (expanded via Ctrl+O):
   🔧 Sprite
   📄 Read ⏳
     path: main.rs
   ↓
5. Tool executor runs Read("main.rs")
   ↓
6. Tool sends result:
   {
     role: "tool",
     tool_call_id: "call_123",
     content: "fn main() { println!(\"Hello\"); }"
   }
   ↓
7. Rendering (collapsed):
   🔧 Sprite
   📄 Read ⏳
   🔧 Read ✓ 1 line, 38 字符
   ↓
8. Rendering (expanded):
   🔧 Sprite
   📄 Read ⏳
   🔧 Read ✓ 1 line, 38 字符
     fn main() { println!("Hello"); }
```

---

## 🚀 Quick Start Guide

### To understand tool rendering:
1. Start with **TOOL_RENDERING_QUICK_REF.txt** (5 min read)
2. Look at **TOOL_RENDERING_EXAMPLES.md** for visuals (15 min)
3. Dive into **TOOL_RENDERING_ANALYSIS.md** for deep understanding (30 min)

### To modify tool rendering:
1. Find the relevant function in `render_cache.rs`
2. Understand the rendering flow from Examples
3. Check classification.rs for icon/color logic
4. Check app.rs for expand_tools flag behavior
5. Remember cache invalidation conditions

### To add a new tool category:
1. Add variant to `ToolCategory` enum (classification.rs:6)
2. Add icon in `icon()` method (classification.rs:42-52)
3. Add color in `color()` method (classification.rs:55-65)
4. Update `from_name()` to classify your tool (classification.rs:25-39)
5. Add tool name to appropriate branch

---

## 📝 Notes

- All tool rendering is in `render_cache.rs` (~1443 lines)
- Tool classification/icons in `classification.rs` (~193 lines)
- Storage structures in `storage.rs` (ToolCallItem, ChatMessage)
- Markdown rendering separate but integrated (markdown/parser.rs)
- UI state managed in `app.rs` (expand_tools flag)
- Performance optimized with 3-level caching
- All content has display limits to prevent UI overflow
- Special handling for diff, agent, and error outputs

---

**Last Updated**: April 2026
**Status**: Analysis Complete ✓
**Coverage**: 100% of tool call rendering in j-cli
