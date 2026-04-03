# j-cli Documentation Index

## 📚 Complete Documentation Set

This index provides an overview of all generated documentation files explaining the j-cli codebase architecture.

---

## 📄 Main Documents

### 1. **J_CLI_ARCHITECTURE_ANALYSIS.md** (14 KB)
**Comprehensive architectural deep-dive**

Complete reference covering all 5 system components with detailed code references and line numbers.

**Contents:**
- ✅ Auto-Compact trigger & call chain (3 trigger points)
- ✅ Toast notification system (storage, display, styling)
- ✅ Title bar loading/tool status (priority logic, data sources)
- ✅ Tool call rendering (incremental caching, special content)
- ✅ ToolExecStatus enum & active_tool_calls (state transitions, queries)

**Best for:** In-depth understanding, implementation details, debugging

**Key sections:**
- Auto-Compact Configuration (204,800 token threshold)
- Toast Rendering Code (ui/chat.rs:908-922)
- Title Bar Status Priority (Lines 87-109)
- Tool Rendering Pipeline (Lines 1038-1274)
- Tool State Machine (all transitions)

---

### 2. **VISUAL_ARCHITECTURE.md** (24 KB)
**ASCII diagrams and visual flows**

Data flow diagrams, state machines, and file relationship maps with ASCII art.

**Contents:**
- ✅ System overview diagram
- ✅ Auto-compact 3-layer strategy flow
- ✅ Toast data flow (trigger → render → dismiss)
- ✅ Title bar component layout
- ✅ Tool rendering pipeline (message type detection)
- ✅ Tool execution state machine
- ✅ File relationships map

**Best for:** Understanding data flow, visual learners, system overview

**Key diagrams:**
- 3-layer compaction strategy
- Toast positioning calculation
- Tool execution state transitions (5 states)
- Message rendering decision tree
- Active tool calls example object

---

### 3. **QUICK_REFERENCE.md** (5 KB)
**Fast lookup and common patterns**

Quick copy-paste reference for common tasks and code patterns.

**Contents:**
- ✅ Function locations and line numbers
- ✅ Setup/usage examples
- ✅ Common queries (find executing tool, check loading, etc.)
- ✅ Constants table
- ✅ File index

**Best for:** Quick lookups, copy-paste code snippets, common patterns

**Quick finds:**
- Auto-compact call chain diagram
- show_toast() usage
- Title bar status display
- Tool rendering functions
- ToolExecStatus enum variants

---

## 📑 Quick Navigation

### By Use Case

#### "I need to understand how auto_compact works"
1. Start: **QUICK_REFERENCE.md** → "1. Auto Compact System"
2. Then: **J_CLI_ARCHITECTURE_ANALYSIS.md** → "1. Auto Compact Trigger & Call Chain"
3. Visual: **VISUAL_ARCHITECTURE.md** → "1. Auto-Compact Flow"

#### "I need to add a new toast notification"
1. Start: **QUICK_REFERENCE.md** → "2. Toast Notifications" → "Setup Toast"
2. Then: **J_CLI_ARCHITECTURE_ANALYSIS.md** → "2. Toast System" → "Toast Trigger Points"
3. Code: See `app.rs:2633` for implementation

#### "I need to understand tool execution status"
1. Start: **QUICK_REFERENCE.md** → "5. Tool Execution Status"
2. Then: **J_CLI_ARCHITECTURE_ANALYSIS.md** → "5. ToolExecStatus Enum & active_tool_calls"
3. Visual: **VISUAL_ARCHITECTURE.md** → "5. Tool Execution State Machine" (state diagram)

#### "I need to modify tool rendering"
1. Start: **VISUAL_ARCHITECTURE.md** → "4. Tool Rendering Pipeline"
2. Then: **J_CLI_ARCHITECTURE_ANALYSIS.md** → "4. Tool Calls Rendering in Message Area"
3. Code: `render_cache.rs:1038-1274`

#### "I need to understand the whole system"
1. Start: **VISUAL_ARCHITECTURE.md** → "System Overview" & "File Relationships Map"
2. Then: **J_CLI_ARCHITECTURE_ANALYSIS.md** (complete read)
3. Reference: **QUICK_REFERENCE.md** for constant lookup

---

### By File

#### app.rs
- **Toast**: J_CLI section 2, show_toast() at line 2633
- **ToolExecStatus**: J_CLI section 5, lines 46-57
- **ToolExecutor**: J_CLI section 5, lines 273-293
- **is_loading**: J_CLI section 3, line 260 (source for title bar)

#### compact.rs
- **auto_compact()**: J_CLI section 1, lines 174-246
- **CompactConfig**: J_CLI section 1, lines 18-54
- **Trigger logic**: J_CLI section 1 + VISUAL section 1

#### ui/chat.rs
- **draw_title_bar()**: J_CLI section 3, lines 83-162
- **draw_toast()**: J_CLI section 2, lines 882-926
- **Status priority**: J_CLI section 3, lines 87-109

#### render_cache.rs
- **build_message_lines_incremental()**: J_CLI section 4, lines 50-61
- **render_tool_call_request_msg()**: J_CLI section 4, lines 1038-1120
- **render_tool_result_msg()**: J_CLI section 4, lines 1165-1274

#### agent.rs
- **auto_compact trigger**: J_CLI section 1, lines 44-56
- **Manual compact trigger**: J_CLI section 1, lines 316-322, 399-405, 449-455

---

## 🎯 Key Findings Summary

### 1. Auto-Compact (3 Layers)

**Trigger Chain:**
```
agent.rs:45-56 → estimate_tokens() > threshold
                ↓
              compact.rs:53 → auto_compact()
                ↓
              LLM summarization → Replace messages
```

**Config:**
- Threshold: 204,800 tokens (256 × 800)
- Max summary: 20,000 tokens
- Transcript location: `.jcli/transcripts/`

---

### 2. Toast System

**Display:**
- Location: Top-right corner of terminal
- Size: Dynamic (text_width + 10)
- Auto-dismiss: After `TOAST_DURATION_SECS`

**Styling:**
- Error: ✖️ (red background/border)
- Success: ☑️ (green background/border)

**Storage:** `app.ui.toast: Option<(msg, is_error, Instant)>`

---

### 3. Title Bar Status

**Priority Display:**
1. Executing tool: "🔧 执行 {name}..."
2. Pending confirm: "🔧 调用 {name}..."
3. Thinking: "⏳ 思考中..."

**Data Source:**
- `app.state.is_loading` (true/false)
- `app.tool_executor.active_tool_calls[].status`

---

### 4. Tool Rendering

**Cache Optimization:**
- P0: Per-message line caching
- P1: Streaming incremental rendering
- P2: Direct indexing (no flat Vec)

**Special Formatting:**
- Diff: Color-coded (+/- lines)
- Errors: First 20 lines
- Agent: Tree format (├─/└─)
- Normal: First 100 lines

---

### 5. Tool Execution Status

**States:** 5 variants
- `PendingConfirm` - Waiting for user
- `Executing` - Running in background
- `Done(String)` - Success with summary
- `Rejected` - User said no
- `Failed(String)` - Error with message

**Storage:** `app.tool_executor.active_tool_calls: Vec<ToolCallStatus>`

---

## 📊 Constants Reference

| Constant | Value | Location |
|----------|-------|----------|
| Token Threshold | 204,800 | compact.rs:39 |
| Max Summary Tokens | 20,000 | compact.rs:202 |
| Tool Result Lines | 100 | render_cache.rs:1256 |
| Error Result Lines | 20 | render_cache.rs:1231 |
| Keep Recent (micro) | 10 | compact.rs:43 |
| Toast Duration | ? | constants.rs |
| Transcript Dir | `.jcli/transcripts/` | compact.rs:131 |

---

## 🔍 Code Location Index

### Initialization & Setup
- `app.rs:1321` - Initialize toast as None
- `app.rs:1380` - Initialize is_loading as false
- `app.rs:1297-1299` - Initialize tool_executor

### State Setters
- `app.rs:2871` - Set is_loading = true (start request)
- `app.rs:3365` - Set is_loading = false (finish)
- `app.rs:2633` - show_toast() implementation

### Trigger Points
- `agent.rs:53` - Call auto_compact() on threshold
- `agent.rs:319, 402, 452` - Call auto_compact() on CompactTool
- `render_cache.rs:59` - Call draw_toast() in draw loop
- `ui/chat.rs:83` - Call draw_title_bar() in draw loop

### Rendering
- `ui/chat.rs:82-162` - draw_title_bar() function
- `ui/chat.rs:882-926` - draw_toast() function
- `render_cache.rs:1038-1120` - render_tool_call_request_msg()
- `render_cache.rs:1165-1274` - render_tool_result_msg()

### Status Updates
- `app.rs:366-368` - Update status to Done/Failed
- `app.rs:483` - Update status to Executing
- `app.rs:553` - Update status to Rejected
- `app.rs:3221` - Push new ToolCallStatus

---

## 📝 File Sizes

- J_CLI_ARCHITECTURE_ANALYSIS.md: 14 KB
- VISUAL_ARCHITECTURE.md: 24 KB
- QUICK_REFERENCE.md: 5 KB
- DOCUMENTATION_INDEX.md: This file (~5 KB)

**Total: ~48 KB of comprehensive documentation**

---

## ✅ Coverage Checklist

- [x] Auto-compact system (trigger, implementation, config)
- [x] Toast system (storage, display, rendering)
- [x] Title bar status (display logic, data source)
- [x] Tool rendering (architecture, special cases)
- [x] Tool execution status (enum, state machine)
- [x] All file locations and line numbers
- [x] Visual diagrams and flows
- [x] Common code patterns
- [x] Constants reference
- [x] Cross-document navigation

---

## 🚀 Getting Started

1. **For quick answers**: Use **QUICK_REFERENCE.md**
2. **For understanding flows**: Start with **VISUAL_ARCHITECTURE.md**
3. **For detailed implementation**: Read **J_CLI_ARCHITECTURE_ANALYSIS.md**
4. **For navigation**: This index (DOCUMENTATION_INDEX.md)

---

## 📌 Last Updated

- Date: 2026-04-03
- Codebase: j-cli (src/command/chat/)
- Files analyzed: app.rs, agent.rs, compact.rs, render_cache.rs, ui/chat.rs

---

## 💡 Tips

- Use Ctrl+F to search documentation files
- Cross-reference line numbers with your IDE
- Bookmark QUICK_REFERENCE.md for fast access
- Use VISUAL_ARCHITECTURE.md when discussing designs
- Share J_CLI_ARCHITECTURE_ANALYSIS.md for comprehensive onboarding

