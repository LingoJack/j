pub mod color_adapt;
pub mod fuzzy;
pub mod log;
pub mod md_render;
pub mod sync;
pub mod text;

// Re-export commonly used functions for convenience
pub use sync::{LockFileGuard, safe_lock};
pub use text::remove_quotes;
