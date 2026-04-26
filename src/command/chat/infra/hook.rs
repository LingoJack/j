pub mod definition;
pub mod executor;
pub mod manager;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export all public items used by external modules
pub use definition::HookDef;
pub use manager::HookManager;
pub use types::{HookContext, HookEvent, HookFilter, HookResult, HookType, OnError};
