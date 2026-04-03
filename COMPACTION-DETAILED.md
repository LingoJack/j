# J-CLI Compaction System - Detailed Analysis

## Overview: Why Compaction?

In long conversations, the message history accumulates tokens. Three-layer compaction prevents context window overflow:

1. **Micro Compact**: In-memory placeholder replacement (free)
2. **Auto Compact**: LLM summarization (costs tokens, saves more)
3. **Explicit Compact**: User/tool-triggered (manual control)

## Layer 1: Micro Compact (In-Memory)

### Location
`src/command/chat/compact.rs`, lines 61-127
Called: Every agent loop iteration (line 47 in `agent.rs`)

### Algorithm

```rust
fn micro_compact(messages: &mut [ChatMessage], keep_recent: usize)
```

### What It Does

1. **Build tool name mapping** (lines 66-76)
   - Scans all assistant messages for `tool_calls`
   - Creates `tool_call_id → tool_name` map
   - Example: `"call_123" → "Bash"`

2. **Find tool result messages** (lines 78-84)
   - Collects indices of all `role="tool"` messages
   - These contain results from previous tool executions

3. **Determine compaction targets** (lines 86-91)
   - If fewer than `keep_recent` tool messages: do nothing
   - Otherwise: compact messages older than the last `keep_recent`
   - Default `keep_recent = 10`

4. **Replace with placeholders** (lines 102-116)
   - For each old tool result message:
     - Check content size: if > 800 bytes
     - Skip if tool is exempt (see below)
     - Replace content with: `[Previous: used {tool_name}]`
   - Log count of compacted messages

### Exempt Tools

Tools NOT compacted even if large (lines 94-100):
```rust
const EXEMPT_TOOLS: &[&str] = &[
    LoadSkillTool::NAME,      // Skills carry workflow instructions
    TaskTool::NAME,           // Task definitions important for tracking
    TodoWriteTool::NAME,      // Todo state management
    TodoReadTool::NAME,       // Todo state management
    AskTool::NAME,            // Ask dialogs may be needed context
];
```

**Why exempt?** These tools deliver state/instructions, not just informational results.

### Example

Before micro_compact:
```
Message 1: role=assistant, content="I'll search for that file"
Message 2: role=tool (Bash), tool_call_id=call_456
           content="... 1500 bytes of file listing ..."

Message 3: role=assistant, content="Found 3 matches, let me read them"
Message 4: role=tool (Read), tool_call_id=call_789
           content="... 2000 bytes of file content ..."

Message 5: role=assistant, content="I'm loading the skill"
Message 6: role=tool (LoadSkill), tool_call_id=call_abc
           content="... 3000 bytes of skill definition ... [EXEMPT]"
```

After micro_compact (keep_recent=3):
```
Message 1: role=assistant, content="I'll search for that file"
Message 2: role=tool (Bash), tool_call_id=call_456
           content="[Previous: used Bash]"          ← Compacted!

Message 3: role=assistant, content="Found 3 matches, let me read them"
Message 4: role=tool (Read), tool_call_id=call_789
           content="[Previous: used Read]"          ← Compacted!

Message 5: role=assistant, content="I'm loading the skill"
Message 6: role=tool (LoadSkill), tool_call_id=call_abc
           content="... 3000 bytes of skill definition ... [KEPT]"
```

### Performance
- **Cost**: O(n) scan, very fast
- **Token saved**: Highly variable (depends on tool result sizes)
- **Data loss**: Minimal (just tool results, not logic)
- **Frequency**: Every agent loop iteration

---

## Layer 2: Auto Compact (LLM Summarization)

### Location
`src/command/chat/compact.rs`, lines 174-246
Called: When token threshold exceeded (lines 48-56 in `agent.rs`)

### Trigger Condition

```rust
if compact::estimate_tokens(&messages) > compact_config.token_threshold {
    auto_compact(&mut messages, &provider).await
}
```

Default threshold: **204,800 tokens** (256 * 800)
Rough estimation: `json_length / 4` (4 chars ≈ 1 token)

### Process

#### Step 1: Save Full Transcript (lines 178-168)
```rust
fn save_transcript(messages: &[ChatMessage]) -> Option<String>
```
- Creates `.transcripts/` directory in agent data
- Saves all messages as JSONL (one message per line)
- Filename: `transcript_{unix_timestamp}.jsonl`
- **Purpose**: Preserve complete history for debugging/recovery
- **Logged**: Path to transcript file

#### Step 2: Truncate Conversation (lines 182-184)
```rust
let conversation_text = serde_json::to_string(messages).unwrap_or_default();
let truncated: String = conversation_text.chars().take(80000).collect();
```
- Serialize all messages to JSON
- Keep first 80,000 characters
- Prevents sending huge prompts to LLM

#### Step 3: Build Summary Request (lines 186-192)
```rust
let summary_prompt = format!(
    "Summarize this conversation for continuity. Include: \
     1) What was accomplished, 2) Current state, 3) Key decisions made. \
     4) If a skill/workflow was actively being followed, preserve its key steps and current progress so the model can continue following it. \
     Be concise but preserve critical details.\n\n{}",
    truncated
);
```

**Key instruction**: "If a skill/workflow was actively being followed, preserve its key steps and current progress"

This is **critical** for workflow preservation!

#### Step 4: Call LLM (lines 194-212)
- Uses **non-streaming** request (easier for text extraction)
- Single-turn: no tools, just summarization
- `max_tokens=20000` (allow large summaries)
- API: OpenAI (same provider as main agent)

#### Step 5: Extract Summary (lines 214-218)
```rust
let summary = response
    .choices
    .first()
    .and_then(|c| c.message.content.clone())
    .unwrap_or_else(|| "(empty summary)".to_string());
```

#### Step 6: Replace Message History (lines 225-243)
```rust
messages.clear();
messages.push(ChatMessage {
    role: "user",
    content: format!(
        "[Conversation compressed. Transcript: {}]\n\n{}",
        transcript_path, summary
    ),
    ...
});
messages.push(ChatMessage {
    role: "assistant",
    content: "Understood. I have the context from the summary. Continuing.",
    ...
});
```

**Result:**
- Only 2 messages remain: one user (summary), one assistant (ack)
- Transcript location provided for reference
- Model acknowledges context is loaded

### Performance & Cost
- **Token cost**: ~80K chars input + 20K output = ~25K tokens ≈ $0.25
- **Token savings**: Typically 100K-200K+ tokens freed
- **ROI**: Usually positive, saves more than it costs
- **Timing**: Happens automatically when needed

### Graceful Degradation
```rust
if let Err(e) = compact::auto_compact(&mut messages, &provider).await {
    write_error_log("agent_loop", &format!("auto_compact failed: {}", e));
    // Continue with original messages! Don't crash.
}
```
- If LLM call fails: error logged, conversation continues
- Original messages unchanged
- Provides robustness against API issues

---

## Layer 3: Explicit Compact Tool

### Location
`src/command/chat/tools/compact.rs`, lines 1-45

### Tool Definition

```rust
pub struct CompactTool;

impl Tool for CompactTool {
    fn name(&self) -> &str { "Compact" }
    
    fn description(&self) -> &str {
        "Trigger conversation compression to free up context window. \
         Use this when the conversation is getting long and you want to \
         summarize and compress the history to continue working efficiently."
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "focus": {
                    "type": "string",
                    "description": "What to preserve in the summary (optional)"
                }
            }
        })
    }
}
```

### How It Works

1. **Tool call**: Agent/user can call `Compact` tool
2. **Optional parameter**: `focus` string (not currently used by LLM, for future)
3. **Execution**: Returns "Compression requested" (lines 34-40)
4. **Agent detection**: Main loop checks for `CompactTool` call (line 606 in `agent.rs`)
5. **Triggering**: If detected, calls `auto_compact()` (lines 318-319, 451-452, etc.)

### When to Use

User/Agent should use when:
- Conversation feels long
- Want to resume work in new session
- Running long multi-step workflows
- Need to clear mental state before next phase

### Workflow Preservation

The `focus` parameter is **reserved for future** workflow tracking:
```rust
"focus": {
    "type": "string",
    "description": "What to preserve in the summary (optional)"
}
```

Could be used like: `{"focus": "Current status of feature X, steps completed so far"}`

---

## Complete Compaction Flow

```
Agent Loop Iteration N
  │
  ├─ Step 1: Drain pending user messages
  │
  ├─ Step 2: Run micro_compact()
  │   ├─ Scan tool messages
  │   ├─ Keep recent 10
  │   ├─ Replace old large results with placeholders
  │   └─ Log count compacted
  │
  ├─ Step 3: Check token count
  │   ├─ estimate_tokens() = json_length / 4
  │   └─ Compare to threshold (default 204,800)
  │
  ├─ Step 4: If threshold exceeded
  │   ├─ save_transcript() to ~/.jdata/transcripts/
  │   ├─ Truncate conversation to 80K chars
  │   ├─ Build summary prompt (with workflow preservation instruction)
  │   ├─ Call LLM non-streaming
  │   ├─ Extract summary
  │   ├─ Replace messages with [summary] + [ack]
  │   └─ Log completion
  │
  ├─ Step 5: Build LLM request
  │   ├─ Add system prompt
  │   ├─ Add current messages
  │   ├─ Add tools
  │   └─ Send to API
  │
  ├─ Step 6: Process response
  │   ├─ If tool_calls
  │   │   ├─ Check if CompactTool called
  │   │   ├─ If yes, trigger auto_compact() (Layer 3)
  │   │   └─ Execute tools
  │   └─ Else (text response)
  │       └─ Add to messages
  │
  └─ Loop continues if pending user messages or tool calls remain
```

---

## Important Design Points

### 1. Workflow Preservation Priority

The compaction prompt explicitly instructs the LLM:
> "If a skill/workflow was actively being followed, preserve its key steps and current progress so the model can continue following it."

This ensures that:
- Skill/tool sequences are maintained
- Multi-step workflows can resume
- Current progress is not lost
- Instructions for "what's next" are preserved

### 2. Exempt Tools Philosophy

Exempt tools carry **state and instructions**, not just results:
- `LoadSkill` → workflow instructions
- `Task` → task definitions and tracking
- `TodoWrite/TodoRead` → state management
- `Ask` → dialog context

These are kept because they're instructions for the next iteration.

### 3. Graceful Degradation

All compaction is **non-blocking**:
- Network error in auto_compact? → Log and continue
- Micro_compact always succeeds (no I/O)
- Explicit compact tool just signals intent

System doesn't crash or hang if compression fails.

### 4. Session Continuity

After auto_compact:
```
Message history is: [user_msg(summary), assistant_msg(ack)]
Agent continues from here with new messages
Next tool results will be added fresh
Micro_compact will protect them
```

Model can seamlessly continue from the summary.

---

## Configuration

### CompactConfig (in config.yaml)

```yaml
[compact]
enabled = true                    # Enable/disable compaction
token_threshold = 204800          # Auto-compact trigger (default: 256 * 800)
keep_recent = 10                  # Micro-compact: keep this many recent tool results
```

### Debug: Check Compaction

To see if compaction happened:
```bash
# Check logs
tail -f ~/.jdata/logs/agent_loop.log

# Look for:
# - "micro_compact triggered"
# - "auto_compact triggered"
# - "Transcript saved: ..."
```

### Manual Compaction

User can call during chat:
```
/Compact

Parameters (optional):
- focus: "What to preserve"
```

Or agent can detect it's getting long and call it autonomously.

