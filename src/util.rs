pub mod color_adapt;
pub mod fuzzy;
pub mod html_extract;
pub mod log;
pub mod md_render;
pub mod path_utils;
pub mod shell_safety;
pub mod sync;
pub mod text;

// Re-export commonly used functions for convenience
pub use sync::{LockFileGuard, safe_lock};
pub use text::remove_quotes;
