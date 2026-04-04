# Tool Results Flow Investigation Report

## Overview
This report documents the complete flow of tool results from tool execution back to the agent loop in the codebase at `/Users/jacklingo/dev_custom/j`.

---

## 1. ToolResultMsg Struct Definition

**File:** `/Users/jacklingo/dev_custom/j/src/command/chat/app.rs`
**Lines:** 69-76

```rust
/// 主线程 → 后台线程的工具结果消息
pub struct ToolResultMsg {
    pub tool_call_id: String,
    pub result: String,
    #[allow(dead_code)]
    pub is_error: bool,
    /// 工具返回的图片数据（用于多模态模型）
    pub images: Vec<crate::command::chat::tools::ImageData>,
}
```

### Fields Analysis:
- **`tool_call_id: String`** - Identifier to match results with the original tool call request
- **`result: String`** - The textual output from the tool execution
- **`is_error: bool`** - Flag indicating whether tool execution resulted in an error (currently marked as `#[allow(dead_code)]`)
- **`images: Vec<crate::command::chat::tools::ImageData>`** - ✅ **YES, it HAS an `images` field for storing image data returned by tools**

---

## 2. ToolResultMsg Construction/Sending Points

### Point 1: Tool Execution in Background Thread (Worker Pool)

**File:** `/Users/jacklingo/dev_custom/j/src/command/chat/app.rs`
**Lines:** 359-439 (The `execute_batch` method)
**Specific Construction:** Lines 430-435

```rust
pub fn execute_batch(&mut self, registry: &Arc<ToolRegistry>) {
    // ... setup code ...
    
    for (tool_call_id, tool_name, arguments) in tasks {
        let result_tx = result_tx.clone();
        let completed_results = Arc::clone(&completed_results);
        let registry = Arc::clone(registry);
        let cancelled = Arc::clone(&self.tool_cancelled);
        std::thread::spawn(move || {
            // Execute tool and catch panics
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                registry.execute(&tool_name, &arguments, &cancelled)
            }));
            
            let (output, is_error, images) = match result {
                Ok(exec_result) => {
                    (exec_result.output, exec_result.is_error, exec_result.images)
                }
                Err(panic_info) => {
                    // ... error handling ...
                    (format!("[Tool panic] {}", msg), true, vec![])
                }
            };
            
            // ★ CONSTRUCTION POINT 1: Creating ToolResultMsg
            let _ = tx.send(ToolResultMsg {
                tool_call_id,
                result: output,
                is_error,
                images,  // ✅ Images are extracted from ToolResult and passed
            });
        });
    }
}
```

### Point 2: User Confirms Tool Execution

**File:** `/Users/jacklingo/dev_custom/j/src/command/chat/app.rs`
**Lines:** 442-524
**Specific Construction:** Lines 514-519

```rust
pub fn execute_current(&mut self, registry: &Arc<ToolRegistry>) -> Option<ChatMode> {
    // ... validation and setup ...
    
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            registry.execute(&tool_name, &arguments, &cancelled)
        }));
        
        let (output, is_error, images) = match result {
            Ok(exec_result) => (exec_result.output, exec_result.is_error, exec_result.images),
            Err(_) => {/* error handling */}
        };
        
        // ★ CONSTRUCTION POINT 2: Creating ToolResultMsg
        if let Some(ref tx) = result_tx {
            let _ = tx.send(ToolResultMsg {
                tool_call_id,
                result: output,
                is_error,
                images,  // ✅ Images are passed
            });
        }
    });
}
```

### Point 3: User Rejects Tool Execution

**File:** `/Users/jacklingo/dev_custom/j/src/command/chat/app.rs`
**Lines:** 526-552
**Specific Construction:** Lines 543-548

```rust
pub fn reject_current(&mut self, reason: &str) -> Option<ChatMode> {
    // ...
    
    // ★ CONSTRUCTION POINT 3: Creating ToolResultMsg for rejection
    if let Some(ref tx) = self.tool_result_tx {
        let _ = tx.send(ToolResultMsg {
            tool_call_id,
            result: reject_msg,
            is_error: true,
            images: vec![],  // No images for rejection
        });
    }
}
```

### Point 4: Tool Execution Denied by .jcli/ Config

**File:** `/Users/jacklingo/dev_custom/j/src/command/chat/app.rs`
**Lines:** 3050-3062
**Specific Construction:** Lines 3057-3062

```rust
for tc in &self.tool_executor.active_tool_calls {
    if let ToolExecStatus::Failed(ref msg) = tc.status
        && let Some(ref tx) = self.tool_executor.tool_result_tx
    {
        // ★ CONSTRUCTION POINT 4: Creating ToolResultMsg for denied tools
        let _ = tx.send(ToolResultMsg {
            tool_call_id: tc.tool_call_id.clone(),
            result: msg.clone(),
            is_error: true,
            images: vec![],  // No images for denied execution
        });
    }
}
```

---

## 3. StreamMsg Enum Definition

**File:** `/Users/jacklingo/dev_custom/j/src/command/chat/app.rs`
**Lines:** 30-42

```rust
/// 后台线程发送给 TUI 的消息类型
pub enum StreamMsg {
    /// 收到一个流式文本块
    Chunk,
    /// LLM 请求执行工具（附带完整工具调用列表）
    ToolCallRequest(Vec<ToolCallItem>),
    /// 流式响应完成
    Done,
    /// 发生错误
    Error(String),
    /// 用户主动取消
    Cancelled,
}
```

### StreamMsg Variants:
- **`Chunk`** - Streaming text received from LLM
- **`ToolCallRequest(Vec<ToolCallItem>)`** - LLM requests tool execution with the list of tools to call
- **`Done`** - Streaming response completed
- **`Error(String)`** - An error occurred during processing
- **`Cancelled`** - User cancelled the operation

---

## 4. ToolResult Struct Definition (from tools module)

**File:** `/Users/jacklingo/dev_custom/j/src/command/chat/tools/mod.rs`
**Lines:** 52-71

### Supporting Type: ImageData

```rust
/// 图片数据（用于多模态工具返回）
#[derive(Debug, Clone)]
pub struct ImageData {
    /// base64 编码的图片数据
    pub base64: String,
    /// MIME 类型（如 "image/png", "image/jpeg"）
    pub media_type: String,
}
```

### Main ToolResult Struct

```rust
/// 工具执行结果
pub struct ToolResult {
    /// 返回给 LLM 的内容
    pub output: String,
    /// 是否执行出错
    pub is_error: bool,
    /// 可选的图片数据（用于多模态模型，由 agent loop 决定是否注入）
    pub images: Vec<ImageData>,
}
```

### Fields Analysis:
- **`output: String`** - The text output from tool execution
- **`is_error: bool`** - Whether the tool execution resulted in an error
- **`images: Vec<ImageData>`** - ✅ **YES, ToolResult HAS an images field**

---

## 5. Tool Results Reception in Agent Loop

**File:** `/Users/jacklingo/dev_custom/j/src/command/chat/agent.rs`
**Function:** `process_tool_calls`
**Lines:** 587-720

### Reception Point: Waiting for Results

```rust
fn process_tool_calls(
    tool_items: Vec<ToolCallItem>,
    assistant_text: String,
    messages: &mut Vec<ChatMessage>,
    tx: &mpsc::Sender<StreamMsg>,
    tool_result_rx: &mpsc::Receiver<ToolResultMsg>,  // ← Receiver channel
    pending_user_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    hook_manager: &HookManager,
    supports_vision: bool,
    shared_messages: &Arc<Mutex<Vec<ChatMessage>>>,
    streaming_content: &Arc<Mutex<String>>,
    compact_config: &CompactConfig,
) -> Result<bool, ()> {
    // ... setup ...
    
    let mut tool_results: Vec<ToolResultMsg> = Vec::new();
    for _ in &tool_items {
        match tool_result_rx.recv() {  // ← Blocking receive from channel
            Ok(result) => tool_results.push(result),
            Err(_) => return Err(()),
        }
    }
    
    log_tool_results(&tool_items, &tool_results);
    
    // Process each tool result
    for result in tool_results {
        let mut result_content = result.result;
        let result_images = result.images;  // ✅ Images extracted from ToolResultMsg
        
        // Find tool name
        let tool_name = tool_items
            .iter()
            .find(|t| t.id == result.tool_call_id)
            .map(|t| t.name.clone());
        
        // Execute PostToolExecution hook
        if hook_manager.has_hooks_for(HookEvent::PostToolExecution) {
            // ... hook processing ...
        }
        
        // Create tool message
        let tool_msg = ChatMessage {
            role: ROLE_TOOL.to_string(),
            content: result_content,
            tool_calls: None,
            tool_call_id: Some(result.tool_call_id.clone()),
            images: None,
        };
        messages.push(tool_msg.clone());
        
        // ★ KEY POINT: Image Injection for Vision Models
        // If model supports vision AND tool returned images, inject as user message
        if supports_vision && !result_images.is_empty() {
            let tool_label = tool_name.as_deref().unwrap_or("unknown");
            let img_msg = ChatMessage {
                role: ROLE_USER.to_string(),
                content: format!(
                    "[{tool_label} 返回了以下图片，请描述图片内容并继续帮助完成任务]\n"
                ),
                tool_calls: None,
                tool_call_id: None,
                images: Some(
                    result_images
                        .into_iter()
                        .map(|img| super::storage::ImageData {
                            base64: img.base64,
                            media_type: img.media_type,
                        })
                        .collect(),
                ),
            };
            messages.push(img_msg.clone());
            push_shared(shared_messages, img_msg);
        }
    }
    
    drain_pending_user_messages(messages, pending_user_messages);
    Ok(compact_requested)
}
```

---

## 6. Tool Results Logging

**File:** `/Users/jacklingo/dev_custom/j/src/command/chat/agent.rs`
**Lines:** 567-581

```rust
/// 记录工具调用结果日志
fn log_tool_results(tool_items: &[ToolCallItem], tool_results: &[ToolResultMsg]) {
    let mut log_content = String::new();
    for (i, result) in tool_results.iter().enumerate() {
        let (tool_name, tool_args) = tool_items
            .get(i)
            .map(|t| (t.name.as_str(), t.arguments.as_str()))
            .unwrap_or(("unknown", ""));
        log_content.push_str(&format!(
            "- [{}] {}({}): {}\n",
            result.tool_call_id, tool_name, tool_args, result.result
        ));
    }
    write_info_log("工具调用结果", &log_content);
}
```

---

## 7. Channel Creation and Thread Coordination

**File:** `/Users/jacklingo/dev_custom/j/src/command/chat/app.rs`
**Lines:** 655-668 (in `AgentHandle::new_with_system_prompt_fn`)

```rust
pub fn new_with_system_prompt_fn<F>(
    // ... parameters ...
    shared_messages: Arc<Mutex<Vec<ChatMessage>>>,
) -> (Self, mpsc::SyncSender<ToolResultMsg>) {
    let (stream_tx, stream_rx) = mpsc::channel::<StreamMsg>();
    let (tool_result_tx, tool_result_rx) = mpsc::sync_channel::<ToolResultMsg>(16);  // ← Channel creation
    
    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();
    
    std::thread::spawn(move || {
        // Agent loop runs here
        // Receives tool_result_rx for incoming results
    });
    
    // Return handle and sender for external code
    (Self { stream_rx, cancel_token }, tool_result_tx)
}
```

---

## Summary of the Complete Flow

1. **Agent Loop Sends Tool Request** → `StreamMsg::ToolCallRequest(Vec<ToolCallItem>)` sent via `stream_tx`

2. **Main Thread/UI Receives Request** → Displays tool calls for user confirmation

3. **Tool Execution Triggered** → One of 4 scenarios:
   - User confirms execution → background thread runs tool
   - User rejects execution → rejection message sent
   - Tool denied by config → denial message sent
   - Batch execution → parallel thread pool

4. **Tool Registry Executes Tool** → `ToolRegistry::execute()` returns `ToolResult` with:
   - `output: String`
   - `is_error: bool`
   - `images: Vec<ImageData>` ✅

5. **ToolResultMsg Created and Sent** → Worker thread/main thread sends via `tool_result_tx`:
   - `tool_call_id` (to match with request)
   - `result` (extracted from ToolResult.output)
   - `is_error` (extracted from ToolResult.is_error)
   - `images` (extracted from ToolResult.images) ✅

6. **Agent Loop Receives Results** → `process_tool_calls()` receives from `tool_result_rx`

7. **Images Processed** → For vision-capable models:
   - Images injected as separate `ChatMessage` with role = ROLE_USER
   - Images converted to `storage::ImageData` format
   - Added to message history for next LLM call

8. **LLM Processes Results** → Next iteration with tool results + images in context

---

## Bug Investigation Implications

### ✅ CONFIRMED: Images ARE being carried through the flow

1. **ToolResult** (tools/mod.rs) has `images: Vec<ImageData>` field
2. **ToolResultMsg** (app.rs) has `images: Vec<...ImageData>` field
3. **Images are extracted** from ToolResult when creating ToolResultMsg (lines 430-435, 514-519)
4. **Images are used** in agent loop to inject vision messages (lines 694-715)

### Potential Issues to Investigate:

1. **Images field initialization**: Check if tools are actually populating the `images` field when returning results
2. **ImageData conversion**: The `ImageData` from tools module is converted to `storage::ImageData` in agent.rs - check if this conversion preserves data correctly
3. **Vision model detection**: Check `supports_vision` flag - if false, images won't be injected even if present
4. **Empty images vector**: Many ToolResultMsg constructions use `vec![]` for images (rejection, denial cases) - this is correct
5. **Tool result display**: The text summary in UI shows only first 60 chars (lines 411-419) but images should still be sent to agent

---

## Key Files Summary

| File | Purpose | Key Lines |
|------|---------|-----------|
| `src/command/chat/app.rs` | ToolResultMsg definition & construction | 69-76, 430-435, 514-519, 543-548, 3057-3062 |
| `src/command/chat/app.rs` | StreamMsg enum definition | 30-42 |
| `src/command/chat/tools/mod.rs` | ToolResult definition & ImageData | 52-71 |
| `src/command/chat/agent.rs` | Tool results reception & processing | 587-720 |
| `src/command/chat/agent.rs` | Results logging | 567-581 |

