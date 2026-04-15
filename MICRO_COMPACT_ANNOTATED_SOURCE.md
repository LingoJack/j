# micro_compact - Fully Annotated Source Code

## Complete Implementation with Line-by-Line Comments

**File**: `src/command/chat/compact.rs`  
**Lines**: 168-241

```rust
/// Layer 1: micro_compact - 替换旧 tool result 为占位符，保留最近 keep_recent 个
///
/// 纯内存操作，零 API 成本。
/// 将较早的 role="tool" 消息中内容长度 > MICRO_COMPACT_BYTES_THRESHOLD 的替换为 "[Previous: used {tool_name}]"
pub fn micro_compact(messages: &mut [ChatMessage], keep_recent: usize) {
    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 1: Build tool_call_id → tool_name Mapping
    // ═══════════════════════════════════════════════════════════════════════════════
    // Purpose: Create a lookup table to find tool names from tool_call IDs
    // 
    // The sequence works like this:
    //   1. Assistant sends message with tool_calls: [{id: "call_123", name: "Read", ...}]
    //   2. Tool executes and returns message with tool_call_id: "call_123"
    //   3. We need to know the tool name was "Read" to create placeholder
    //
    let mut tool_name_map: HashMap<String, String> = HashMap::new();
    
    // Iterate through ALL messages in the conversation
    for msg in messages.iter() {
        // Only look at ASSISTANT messages (they initiate tool calls)
        if msg.role == ROLE_ASSISTANT
            // Use guard clause: if msg.tool_calls is Some, bind it to 'tcs'
            && let Some(ref tcs) = msg.tool_calls
        {
            // An assistant can make MULTIPLE tool calls in one message
            // Example: "I'll read files AND search for patterns"
            //   tool_calls: [
            //     {id: "call_1", name: "Read", ...},
            //     {id: "call_2", name: "Grep", ...}
            //   ]
            for tc in tcs {
                // Map each tool_call_id → tool_name
                // "call_1" → "Read"
                // "call_2" → "Grep"
                tool_name_map.insert(tc.id.clone(), tc.name.clone());
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 2: Find All Tool Result Messages
    // ═══════════════════════════════════════════════════════════════════════════════
    // Purpose: Identify which messages are tool results (role == "tool")
    //
    // Example conversation:
    //   [0] user: "read file.rs"
    //   [1] assistant: "I'll read it" (tool_calls: [{id: "call_1", name: "Read"}])
    //   [2] tool: "contents..." (tool_call_id: "call_1")     ← This is a tool result
    //   [3] assistant: "The file has..." (tool_calls: [{id: "call_2", name: "Grep"}])
    //   [4] tool: "grep results" (tool_call_id: "call_2")    ← This is a tool result
    //   [5] user: "anything else?"
    //
    // After this step: tool_indices = [2, 4]
    //
    let tool_indices: Vec<usize> = messages
        .iter()
        .enumerate()  // Get both index and message
        .filter(|(_, msg)| msg.role == ROLE_TOOL)  // Keep only role="tool"
        .map(|(i, _)| i)  // Extract just the index
        .collect();

    // Early exit: if we have 10 or fewer tool results and keep_recent=10,
    // there's nothing to compact. All messages should stay.
    if tool_indices.len() <= keep_recent {
        return;
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 3: Identify Candidates for Compaction
    // ═══════════════════════════════════════════════════════════════════════════════
    // Purpose: Select old tool messages to compact (preserve recent ones)
    //
    // Strategy: Keep the LAST keep_recent tool messages intact,
    //           compact everything before them
    //
    // Example:
    //   tool_indices = [2, 4, 6, 8, 10]  (5 tool results total)
    //   keep_recent = 2                  (keep last 2)
    //   
    //   tool_indices.len() - keep_recent = 5 - 2 = 3
    //   to_compact = &tool_indices[..3]  = [2, 4, 6]
    //   protected  = [8, 10]             (last 2 stay intact)
    //
    let to_compact = &tool_indices[..tool_indices.len() - keep_recent];

    // Track how many messages were actually compacted (for logging)
    let mut compacted_count = 0;

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 3a: Define Exempt Tools (Never Compacted)
    // ═══════════════════════════════════════════════════════════════════════════════
    // Some tool results carry CRITICAL workflow context and should NEVER be compacted
    // even if they're old and large. Examples:
    //
    // - LoadSkill: Returns full skill definitions & instructions
    //              Removing this breaks the workflow the skill implements
    //
    // - Task/Todo tools: Return task state and planning context
    //                    Removing this loses task tracking
    //
    // - Plan tools: Return workflow execution state
    //               Removing this breaks workflow continuity
    //
    // - Agent/AgentTeam: Return collaboration context
    //                    Removing this breaks multi-agent workflows
    //
    // - Ask: Returns user interaction history
    //        Removing this loses context of what user confirmed/rejected
    //
    // - SendMessage/CreateTeammate: Return team communication state
    //                               Removing this breaks team coordination
    //
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
        // Teammate 工具结果不压缩（承载协作上下文）
        crate::command::chat::tools::send_message::SendMessageTool::NAME,
        crate::command::chat::tools::create_teammate::CreateTeammateTool::NAME,
    ];

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 4: Process Each Candidate for Compaction
    // ═══════════════════════════════════════════════════════════════════════════════
    //
    for &idx in to_compact {
        let msg = &messages[idx];
        
        // ─────────────────────────────────────────────────────────────────────────
        // 4a. Check content size
        // ─────────────────────────────────────────────────────────────────────────
        // Only compact LARGE messages (> 800 characters)
        // Rationale: Small messages don't save much token space,
        //            so leave them intact for readability
        //
        // msg.content.chars().count() counts Unicode characters
        // (not bytes, important for emoji/CJK characters)
        //
        if msg.content.chars().count() > MICRO_COMPACT_BYTES_THRESHOLD {
            
            // ─────────────────────────────────────────────────────────────────────
            // 4b. Look up the tool name
            // ─────────────────────────────────────────────────────────────────────
            // Get the tool_call_id from this tool result message
            // Use it to look up the tool name from our mapping
            //
            // The chain of information:
            //   tool result message.tool_call_id = "call_123"
            //          ↓
            //   tool_name_map["call_123"] = "Read"
            //          ↓
            //   use "Read" in the placeholder
            //
            let tool_call_id = msg.tool_call_id.clone().unwrap_or_default();
            let tool_name = tool_name_map
                .get(&tool_call_id)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            
            // ─────────────────────────────────────────────────────────────────────
            // 4c. Check if tool is exempt
            // ─────────────────────────────────────────────────────────────────────
            // If this result came from an exempt tool, SKIP it
            // Don't compact, keep it intact
            //
            if EXEMPT_TOOLS.iter().any(|&t| t == tool_name) {
                continue;  // Jump to next candidate
            }
            
            // ─────────────────────────────────────────────────────────────────────
            // 4d. Replace with placeholder
            // ─────────────────────────────────────────────────────────────────────
            // If we got here, the message is:
            //   ✓ Old (outside keep_recent)
            //   ✓ Large (> 800 chars)
            //   ✓ From a non-exempt tool
            //
            // Replace its content with a placeholder that tells the model:
            //   "I previously used the Read tool"
            //
            // This placeholder preserves semantic meaning without the detail
            //
            messages[idx].content = format!("[Previous: used {}]", tool_name);
            compacted_count += 1;
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // STEP 5: Log Results
    // ═══════════════════════════════════════════════════════════════════════════════
    // If we compacted anything, log it for debugging
    //
    if compacted_count > 0 {
        write_info_log(
            "micro_compact",
            &format!(
                "压缩了 {} 个旧 tool result（保留最近 {} 个）",
                compacted_count, keep_recent
            ),
        );
    }
}
```

---

## Data Structure Relationships

### ChatMessage Structure
```rust
pub struct ChatMessage {
    pub role: String,                           // "user", "assistant", "tool", "system"
    pub content: String,                        // The message text (what gets compacted)
    pub tool_calls: Option<Vec<ToolCallItem>>,  // Only set on assistant messages
    pub tool_call_id: Option<String>,           // Only set on tool result messages
    pub images: Option<Vec<ImageData>>,         // For multimodal messages
}
```

### ToolCallItem Structure
```rust
pub struct ToolCallItem {
    pub id: String,         // "call_123" - matches tool_result.tool_call_id
    pub name: String,       // "Read", "Glob", etc.
    pub arguments: String,  // JSON args for the tool
}
```

---

## Message Flow Example

```
┌─ User Request ─────────────────────────────┐
│ "Read the main.rs file and show me errors" │
└──────────────────────────────────────────┬─┘
                                           ↓
        ┌─ Assistant Message ────────────────────┐
        │ role: "assistant"                      │
        │ content: "I'll read the file..."       │
        │ tool_calls: [                          │
        │   {                                    │
        │     id: "call_abc123",    ←─────┐     │
        │     name: "Read",              │      │
        │     arguments: "..."           │      │
        │   }                            │      │
        │ ]                              │      │
        └───────────────────────────────┬──────┘
                                        │
                            The ID "call_abc123"
                            is the LINK
                                        │
        ┌─ Tool Result Message ──────────┼──────┐
        │ role: "tool"                   │      │
        │ tool_call_id: "call_abc123" ←─┘      │
        │ content: "fn main() {          │      │
        │           println!(...);       │      │
        │           ...                  │      │
        │         }"                     │      │
        │          (2000 characters)     │      │
        └────────────────────────────────┬──────┘
                                        ↓
        ┌─ During micro_compact ─────────────────┐
        │ 1. Extract call_abc123 → "Read"        │
        │ 2. Check: 2000 chars > 800? YES        │
        │ 3. Check: "Read" in exempt? NO         │
        │ 4. Replace content:                    │
        │    "[Previous: used Read]"             │
        └────────────────────────────────────────┘
                                        ↓
        ┌─ After micro_compact ──────────────────┐
        │ role: "tool"                           │
        │ tool_call_id: "call_abc123"            │
        │ content: "[Previous: used Read]"       │
        │          (20 characters - 100x saving) │
        └────────────────────────────────────────┘
```

---

## Integration Points

### Called From: `src/command/chat/agent.rs` Line 123

```rust
// Every agent loop round:
if compact_config.enabled {
    compact::micro_compact(&mut messages, compact_config.keep_recent);
    
    // Then check if Layer 2 is needed:
    if compact::estimate_tokens(&messages) > compact_config.token_threshold {
        if let Err(e) = compact::auto_compact(&mut messages, &provider, &invoked_skills).await {
            write_error_log("agent_loop", &format!("auto_compact failed: {}", e));
        }
    }
}
```

### Data Flow in Agent Loop

```
Round N:
  1. Drain pending user messages
  2. IF compact enabled:
     a. Call micro_compact()  ← This function
     b. Estimate tokens
     c. IF tokens > 205K tokens:
        └─ Call auto_compact() (Layer 2)
  3. Send messages to LLM
  4. Process tool calls
  5. Loop back to Round N+1
```

---

## Key Insights

### 1. Lazy Linking via tool_call_id

The function doesn't pre-compute all links. Instead:
- First pass: Build `tool_call_id → tool_name` map
- Second pass: When processing tool results, look them up on-demand
- This keeps the algorithm simple and cache-friendly

### 2. Graceful Degradation

If tool name not found:
```rust
.unwrap_or_else(|| "unknown".to_string())
```
The function still works, using placeholder: `[Previous: used unknown]`

### 3. Content Size vs. Message Count

The function uses TWO levels of filtering:
1. **Message count**: Only compact if more than `keep_recent` exist
2. **Content size**: Only compact messages larger than 800 chars

This preserves both recent context AND small messages that don't waste space.

### 4. Exempt Tools Preserve Workflow

Certain tools are NEVER compacted regardless of size or age, because:
- **LoadSkill**: Model needs full instructions to follow skill
- **Task/Todo**: Model needs task state to continue work
- **Plan**: Model needs progress to continue workflow
- **Ask**: Model needs history of what user confirmed

This ensures LLM can continue work without re-loading critical context.

### 5. Token Savings

For a typical scenario with 10 tool results (5 old, 5 recent):
- Old results: ~5000 chars each = 1250 tokens
- After compaction: ~20 chars each = 5 tokens
- **Savings: 6225 tokens per round** (in-memory, no API cost!)

---

## Performance Notes

| Operation | Complexity | Cost |
|-----------|-----------|------|
| Build tool_name_map | O(m) | m = total tool_calls across all messages |
| Find tool messages | O(n) | n = total messages |
| Process candidates | O(c) | c = candidates (typically n/2 - n) |
| Total | O(n) | Linear scan, single pass |

For 100-message history: < 1ms on modern hardware
