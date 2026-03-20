pub mod fuzzy;
pub mod html_extract;
pub mod log;
pub mod md_render;
pub mod sync;
pub mod text;

// Re-export commonly used functions for convenience
pub use sync::safe_lock;
pub use text::remove_quotes;
