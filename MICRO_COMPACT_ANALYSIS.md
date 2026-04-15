# micro_compact Function Implementation Analysis

## Overview
The `micro_compact` function is a Layer 1 compression strategy in the chat compaction system. It performs in-memory optimization by replacing old tool result messages with placeholder text, without requiring any LLM API calls.

**Location**: `src/command/chat/compact.rs` (lines 168-241)

---

## Function Signature

```rust
pub fn micro_compact(messages: &mut [ChatMessage], keep_recent: usize)
```

### Parameters
- `messages`: Mutable slice of chat messages to be compacted
- `keep_recent`: Number of most recent tool results to preserve without compaction (default: 10, defined in constants)

### Compaction Threshold
- `MICRO_COMPACT_BYTES_THRESHOLD`: 800 characters
- Only tool result messages with content exceeding 800 characters are eligible for compaction

---

## Complete Implementation Breakdown

### Step 1: Build tool_call_id → tool_name Mapping (Lines 173-183)

```rust
let mut tool_name_map: HashMap<String, String> = HashMap::new();
for msg in messages.iter() {
    if msg.role == ROLE_ASSISTANT
        && let Some(ref tcs) = msg.tool_calls
    {
        for tc in tcs {
            tool_name_map.insert(tc.id.clone(), tc.name.clone());
        }
    }
}
```

**What it does**:
- Iterates through all messages looking for **assistant role messages**
- Extracts assistant messages that have `tool_calls` field
- For each tool call, creates a mapping: `tool_call_id` → `tool_name`
- This mapping will later be used to identify which tool generated each result

**Data structures involved**:
- `ChatMessage` struct fields:
  - `role`: "assistant" | "tool" | "user" | "system"
  - `tool_calls`: Optional vector of `ToolCallItem` structs
  - Each `ToolCallItem` has: `id` (tool_call_id), `name` (tool_name), `arguments`

---

### Step 2: Identify All Tool Result Messages (Lines 185-191)

```rust
let tool_indices: Vec<usize> = messages
    .iter()
    .enumerate()
    .filter(|(_, msg)| msg.role == ROLE_TOOL)
    .map(|(i, _)| i)
    .collect();

if tool_indices.len() <= keep_recent {
    return;  // Not enough to compact
}
```

**What it does**:
- Scans all messages and collects indices of messages with `role == "tool"`
- Returns early if total tool results ≤ `keep_recent` (nothing to compact)
- This ensures recent tool results are preserved for context

**Example**:
```
messages = [user, assistant(tool_call_A), tool(result_A), assistant(tool_call_B), tool(result_B), ...]
keep_recent = 2

tool_indices = [2, 4, ...]  // indices of tool messages
If len(tool_indices) = 3, only indices 2 and 4 would be protected
Message at index 2 would be eligible for compaction
```

---

### Step 3: Compact Old Tool Results (Lines 197-230)

```rust
let to_compact = &tool_indices[..tool_indices.len() - keep_recent];
let mut compacted_count = 0;

const EXEMPT_TOOLS: &[&str] = &[
    LoadSkillTool::NAME,
    TaskTool::NAME,
    TodoWriteTool::NAME,
    TodoReadTool::NAME,
    EnterPlanModeTool::NAME,
    ExitPlanModeTool::NAME,
    AgentTool::NAME,
    AgentTeamTool::NAME,
    AskTool::NAME,
    crate::command::chat::tools::send_message::SendMessageTool::NAME,
    crate::command::chat::tools::create_teammate::CreateTeammateTool::NAME,
];

for &idx in to_compact {
    let msg = &messages[idx];
    if msg.content.chars().count() > MICRO_COMPACT_BYTES_THRESHOLD {
        let tool_call_id = msg.tool_call_id.clone().unwrap_or_default();
        let tool_name = tool_name_map
            .get(&tool_call_id)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        if EXEMPT_TOOLS.iter().any(|&t| t == tool_name) {
            continue;  // Skip exempt tools
        }
        messages[idx].content = format!("[Previous: used {}]", tool_name);
        compacted_count += 1;
    }
}
```

**What it does**:

1. **Selects candidates**: Takes all tool messages EXCEPT the last `keep_recent` ones
   ```
   If tool_indices = [2, 4, 6, 8, 10] and keep_recent = 2
   to_compact = [2, 4, 6]  (indices 8, 10 are preserved)
   ```

2. **Checks content size**: Only compact messages with > 800 characters

3. **Builds exempt tools list**: Certain tools are never compacted because their results carry critical workflow context:
   - `LoadSkill`: Skill definitions and instructions
   - `Task`/`Todo` tools: Task tracking and planning context
   - `Plan` tools: Workflow/plan execution state
   - `Agent`/`AgentTeam`: Agent collaboration context
   - `Ask`: User interaction history
   - `SendMessage`/`CreateTeammate`: Teammate collaboration context

4. **Skips exempt tools**: If the result came from an exempt tool, leaves it unmodified

5. **Replaces with placeholder**: For non-exempt tools, replaces content with:
   ```
   "[Previous: used {tool_name}]"
   ```

6. **Tracks compaction**: Increments counter for logging

**Example**:
```
Original message:
  role: "tool"
  tool_call_id: "call_xyz"
  content: "{very long JSON output from file read...}"  (5000 chars)

After compaction:
  role: "tool"
  tool_call_id: "call_xyz"
  content: "[Previous: used read]"
```

---

### Step 4: Logging (Lines 232-240)

```rust
if compacted_count > 0 {
    write_info_log(
        "micro_compact",
        &format!(
            "压缩了 {} 个旧 tool result（保留最近 {} 个）",
            compacted_count, keep_recent
        ),
    );
}
```

Logs compaction summary for debugging and monitoring.

---

## How Tool Results Map to Tool Calls

The key linking mechanism between tool calls and tool results:

### In an Assistant Message with tool_calls:
```rust
ChatMessage {
    role: "assistant",
    content: "I'll read the file for you.",
    tool_calls: Some(vec![
        ToolCallItem {
            id: "call_123",           // Unique identifier
            name: "Glob",             // Tool name
            arguments: "{...}"        // JSON arguments
        }
    ]),
    tool_call_id: None,
    ...
}
```

### In the Corresponding Tool Result Message:
```rust
ChatMessage {
    role: "tool",
    content: "file1.rs\nfile2.rs\n...",
    tool_calls: None,
    tool_call_id: Some("call_123"),   // References the tool_call.id
    ...
}
```

### Flow in micro_compact:
1. Find assistant message with `tool_calls`
2. Extract `tool_calls[i].id` → `tool_name` mapping
3. Later find tool message with `tool_call_id` matching that id
4. Use the mapping to get the tool name for the placeholder

---

## Integration with the Agent Loop

### Where it's called (src/command/chat/agent.rs, line 123):

```rust
// ── Layer 1: micro_compact（替换旧 tool results）──
// ── Layer 2: if tokens > threshold → auto_compact（LLM 摘要）──
if compact_config.enabled {
    compact::micro_compact(&mut messages, compact_config.keep_recent);
    if compact::estimate_tokens(&messages) > compact_config.token_threshold {
        // If still too large, trigger Layer 2 (auto_compact)
        if let Err(e) = 
            compact::auto_compact(&mut messages, &provider, &invoked_skills).await 
        {
            write_error_log("agent_loop", &format!("auto_compact failed: {}", e));
        }
    }
}
```

### When it triggers:
- **Every agent loop round** (if enabled in config)
- Before sending messages to the LLM
- After draining pending user messages
- Before checking if Layer 2 (auto_compact) is needed

---

## Compaction Strategy Summary

| Layer | Name | Trigger | Cost | Operation |
|-------|------|---------|------|-----------|
| 1 | `micro_compact` | Every round | 0 (memory-only) | Replace old tool results with placeholders |
| 2 | `auto_compact` | When tokens > threshold | LLM cost | Summarize entire conversation + preserve skills |

---

## Constants Used

```rust
// From src/command/chat/constants.rs
pub const ROLE_ASSISTANT: &str = "assistant";
pub const ROLE_TOOL: &str = "tool";
pub const MICRO_COMPACT_BYTES_THRESHOLD: usize = 800;        // 800 chars
pub const COMPACT_KEEP_RECENT: usize = 10;                   // Keep 10 recent
pub const COMPACT_TOKEN_THRESHOLD: usize = 256 * 800;        // ~205K tokens
```

---

## Tradeoffs & Design Decisions

### ✅ Advantages:
1. **Zero cost**: Runs in memory, no API calls
2. **Preserves recent context**: Keeps `keep_recent` tool results intact
3. **Maintains semantic value**: Placeholders like `[Previous: used Glob]` tell the model what happened
4. **Exempt critical tools**: Never compacts results that carry essential workflow state
5. **Early reduction**: Prevents unnecessary API calls to Layer 2 (auto_compact)

### ⚠️ Tradeoffs:
1. **Loses detail**: Placeholder loses actual tool output (files read, test results, etc.)
2. **Older results gone first**: Compacts oldest tool results regardless of tool type
3. **Content-based only**: Only considers message size, not semantic importance
4. **May not be enough**: If token budget still exceeded, triggers expensive auto_compact

### 🎯 Optimal Use Cases:
- Long conversations with many small tool calls (file reads, grep, shell commands)
- Preserving recent context while reducing context window usage
- Avoiding auto_compact API calls when just cleaning up old results

---

## Example Scenario

```
Initial messages (tokens = 250K):
[1] user: "Read file.rs and analyze it"
[2] assistant: I'll read the file. (tool_call: id=call_1, name=Read)
[3] tool: [contents of file.rs - 5000 chars]
[4] assistant: The file has... (tool_call: id=call_2, name=Grep)
[5] tool: [grep results - 3000 chars]
[6] assistant: I found... (tool_call: id=call_3, name=Shell)
[7] tool: [shell output - 2000 chars]
[8] user: "Summarize this"

After micro_compact(keep_recent=2):
- Message [3] is outside keep_recent and > 800 chars
  - tool_call_id in [3] maps to tool_name "Read"
  - "Read" is NOT in EXEMPT_TOOLS
  - Replaces [3].content with "[Previous: used Read]"
- Message [5] is within keep_recent (newest 2), stays intact
- Message [7] is within keep_recent, stays intact

Result (tokens ≈ 180K):
[1] user: "Read file.rs and analyze it"
[2] assistant: I'll read the file.
[3] tool: "[Previous: used Read]"
[4] assistant: The file has...
[5] tool: [grep results - 3000 chars]  ← KEPT
[6] assistant: I found...
[7] tool: [shell output - 2000 chars]   ← KEPT
[8] user: "Summarize this"
```

---

## Related Functions

1. **`estimate_tokens(messages)`**: Rough token counting (~4 chars = 1 token)
2. **`auto_compact(messages, provider, invoked_skills)`**: Layer 2, uses LLM summarization
3. **`build_invoked_skills_attachment(map)`**: Preserves skill definitions across compaction
4. **`record_skill_invocation(...)`**: Tracks which skills were used in the session

