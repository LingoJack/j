# CRITICAL BUG: Session Exit Missing Cleanup

## Summary
When a user exits a chat session, the TeammateManager.stop_all() method is never called, leaving all active teammate threads running in the background.

## Impact
- **Resource Leak**: Threads accumulate over multiple sessions
- **Memory Leak**: Pending message queues, snapshots, and thread handles remain allocated
- **LLM Token Waste**: Idle teammates still consume LLM API connections
- **CPU Usage**: Idle polling threads sleep but still consume resources
- **Unpredictable Cleanup**: Teammates eventually exit after ~2 minutes of idle timeout

## Location
File: `src/command/chat/handler/tui_loop.rs`
Lines: 604-645 (Session exit flow)

## Current Code
```rust
// tui_loop.rs session exit (BUGGY - no cleanup)
// ... user exits chat session ...
// ... no manager.stop_all() called ...
```

## Evidence
From the exploration:
- create_teammate.rs line 152: `is_running = Arc::new(AtomicBool::new(true))`
- create_teammate.rs line 247: `is_running_clone.store(false, Ordering::Relaxed)` (only set to false when thread exits)
- teammate_loop.rs lines 104-108: Loop only exits when cancel_token is cancelled OR idle_rounds >= 120
- manager.rs lines 256-294: stop_all() exists but is never called on session exit

## Reproduction
1. Start a chat session
2. Create multiple teammates with CreateTeammate tool
3. Exit the session immediately (don't wait for idle timeout)
4. Verify teammates continue running (check thread count)
5. Wait ~2 minutes for idle timeout

## Fix Required

**File**: `src/command/chat/handler/tui_loop.rs`  
**Location**: Session exit handler (around lines 604-645)  
**Priority**: HIGH

### Implementation

```rust
// Before session cleanup, add:
if let Ok(mut manager) = self.teammate_manager.lock() {
    write_info_log("ChatApp", "Stopping all teammates...");
    manager.stop_all();
    
    // Optional: Wait for completion with timeout
    for (name, handle) in &manager.teammates {
        match handle.wait_for_completion(std::time::Duration::from_secs(5)) {
            Ok(_) => write_info_log("ChatApp", &format!("Teammate '{}' stopped cleanly", name)),
            Err(e) => write_info_log("ChatApp", &format!("Teammate '{}' stop timeout: {}", name, e)),
        }
    }
}
```

### Testing

After fix, verify:
1. Create multiple teammates
2. Exit session immediately
3. Verify `is_running` is set to false for all teammates
4. Verify no threads are left running
5. Verify memory is released
6. Verify no LLM API connections remain active

## Related Methods

### TeammateManager::stop_all()
Located: `src/command/chat/teammate/manager.rs` lines 256-271
```rust
pub fn stop_all(&mut self) {
    for (_, handle) in self.teammates.iter_mut() {
        handle.cancel_token.cancel();
    }
}
```

### TeammateManager::cleanup_finished()
Located: `src/command/chat/teammate/manager.rs` lines 276-294
Can be called after stop_all() to join threads and remove finished teammates:
```rust
pub fn cleanup_finished(&mut self) {
    self.teammates.retain(|_, handle| {
        if handle.is_running.load(Ordering::Relaxed) {
            true
        } else {
            let _ = handle.thread_handle.take().and_then(|h| h.join().ok());
            false
        }
    });
}
```

## Session Exit Flow (Current)
```
User presses Ctrl+C or /exit
  ↓
tui_loop::run() returns
  ↓
session cleanup begins
  ↓
⚠️  NO manager.stop_all() call ⚠️
  ↓
Program exits
  ↓
Teammates continue running in background (~2 minutes)
  ↓
Teammates eventually timeout via idle detection
```

## Session Exit Flow (Fixed)
```
User presses Ctrl+C or /exit
  ↓
tui_loop::run() returns
  ↓
session cleanup begins
  ↓
✓ manager.stop_all() called
  ✓ All cancel_tokens cancelled
  ✓ All teammates notified to stop
  ↓
✓ Optional: wait_for_completion() called
  ✓ All threads joined with timeout
  ↓
✓ cleanup_finished() called (optional)
  ✓ All teammates removed from HashMap
  ✓ All thread_handles consumed
  ↓
Program exits cleanly
  ↓
No orphaned threads or resources
```

## Notes
- The fix is simple: one method call in the session exit handler
- No API changes needed
- No new dependencies required
- Fully backward compatible
- Should be applied immediately to prevent resource leaks

## Related Files
- `src/command/chat/teammate/manager.rs`: Contains stop_all() and cleanup_finished()
- `src/command/chat/teammate/teammate_loop.rs`: Respects cancel_token
- `src/command/chat/tools/create_teammate.rs`: Spawns threads with cancel_token
- `src/command/chat/app/chat_app.rs`: Contains session management
